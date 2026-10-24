use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::{
    export,
    region::Selection,
    slots::{self, SlotState},
};

const MAX_PIXELS: u64 = 64 * 1024 * 1024;
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Destination {
    Store,
    Save,
}

#[derive(Clone, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Outcome {
    Cancelled,
    Stored {
        slot: usize,
        width: u32,
        height: u32,
    },
    Saved {
        path: String,
        width: u32,
        height: u32,
    },
}

struct Snapshot {
    label: String,
    png: Vec<u8>,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    display_width: u32,
    display_height: u32,
    ready: bool,
}

#[derive(Debug)]
enum Completion {
    Ready,
    Selected { index: usize, selection: Selection },
    Cancelled,
    Failed(String),
}

struct Session {
    snapshots: Vec<Snapshot>,
    sender: tauri::async_runtime::Sender<Completion>,
    submitted: bool,
}

#[derive(Default)]
pub struct CaptureState {
    busy: AtomicBool,
    session: Mutex<Option<Session>>,
}

fn state_error() -> String {
    "Capture state is unavailable. Restart Slotshot.".into()
}

#[cfg(target_os = "macos")]
fn permission() -> Result<(), String> {
    let access = core_graphics::access::ScreenCaptureAccess;
    if !access.preflight() && !access.request() {
        let executable = std::env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "unknown executable".into());
        return Err(format!(
            "Screen Recording access was not granted (preflight=false, request=false); no screenshot was attempted. Running executable: {executable}. Enable the canonical ~/Applications/Slotshot.app in System Settings → Privacy & Security → Screen & System Audio Recording, then fully quit and reopen it. An enabled entry for an older signature may need removal and re-adding; see README."
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn permission() -> Result<(), String> {
    Ok(())
}

fn snapshots() -> Result<Vec<Snapshot>, String> {
    permission()?;
    let monitors = xcap::Monitor::all().map_err(|e| format!("Could not list displays: {e}"))?;
    if monitors.is_empty() || monitors.len() > 16 {
        return Err("Capture requires between 1 and 16 active displays.".into());
    }
    let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let mut total_pixels = 0;
    let mut result = Vec::new();
    for (index, monitor) in monitors.into_iter().enumerate() {
        let metadata_error = |e| format!("Could not read display geometry: {e}");
        let display_width = monitor.width().map_err(metadata_error)?;
        let display_height = monitor.height().map_err(metadata_error)?;
        let x = monitor.x().map_err(metadata_error)?;
        let y = monitor.y().map_err(metadata_error)?;
        // macOS desktop geometry is in points; Windows geometry is in pixels.
        #[cfg(target_os = "macos")]
        let scale = f64::from(monitor.scale_factor().map_err(metadata_error)?);
        #[cfg(not(target_os = "macos"))]
        let scale = 1.0;
        let estimated = f64::from(display_width) * f64::from(display_height) * scale * scale;
        if !estimated.is_finite()
            || estimated <= 0.0
            || estimated + total_pixels as f64 > MAX_PIXELS as f64
        {
            return Err("The connected displays exceed the capture memory limit (64 megapixels). Disconnect a display or lower its resolution.".into());
        }
        let image = monitor.capture_image().map_err(|e| {
            format!("Screen capture backend failed after the permission check passed: {e}. Retry capture; if it persists, report this backend error separately from a permission denial.")
        })?;
        let (width, height) = image.dimensions();
        total_pixels += u64::from(width) * u64::from(height);
        if width == 0 || height == 0 || total_pixels > MAX_PIXELS {
            return Err(
                "The captured displays are empty or exceed the 64 megapixel memory limit.".into(),
            );
        }
        result.push(Snapshot {
            label: format!("region-{id}-{index}"),
            png: export::png(&image)?,
            width,
            height,
            x,
            y,
            display_width,
            display_height,
            ready: false,
        });
    }
    Ok(result)
}

async fn create_overlay(
    app: &AppHandle,
    label: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html?region".into()))
        .title("Slotshot — drag a region, Escape to cancel")
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|e| format!("Could not open selection window: {e}"))?;
    #[cfg(target_os = "macos")]
    {
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e| format!("Could not size selection window: {e}"))?;
        window
            .set_position(tauri::LogicalPosition::new(x, y))
            .map_err(|e| format!("Could not position selection window: {e}"))?;
        let native_window = window.clone();
        window
            .run_on_main_thread(move || {
                if let Ok(ptr) = native_window.ns_window() {
                    // Tauri owns this NSWindow. It is used only on the main thread,
                    // and this reference does not outlive the closure/window.
                    let ns_window = unsafe { &*ptr.cast::<objc2_app_kit::NSWindow>() };
                    ns_window.setLevel(objc2_app_kit::NSScreenSaverWindowLevel);
                }
            })
            .map_err(|e| format!("Could not configure selection window: {e}"))?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let (sender, mut receiver) = tauri::async_runtime::channel(1);
        let native_window = window.clone();
        // Run in one main-thread callback: moving processes WM_DPICHANGED before
        // applying the final physical size. Otherwise Tao preserves logical size
        // when crossing DPI scales and stretches an already-sized overlay.
        window
            .run_on_main_thread(move || {
                let result = (|| {
                    let position = tauri::PhysicalPosition::new(x, y);
                    let size = tauri::PhysicalSize::new(width, height);
                    native_window.set_position(position)?;
                    native_window.set_size(size)?;
                    if native_window.outer_position()? != position
                        || native_window.inner_size()? != size
                    {
                        return Err(tauri::Error::WindowNotFound);
                    }
                    Ok(())
                })();
                let _ = sender.try_send(result);
            })
            .map_err(|e| format!("Could not configure selection window: {e}"))?;
        receiver
            .recv()
            .await
            .ok_or("Selection window setup was interrupted.")?
            .map_err(|e| format!("Could not align selection window with the display: {e}"))?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PickerVisibility {
    Hidden,
    Minimized,
    Visible,
}

fn cleanup(app: &AppHandle, visibility: PickerVisibility) {
    let state = app.state::<CaptureState>();
    // Clear state first so Destroyed callbacks cannot cancel the next capture.
    if let Ok(mut session) = state.session.lock() {
        *session = None;
    }
    for (label, window) in app.webview_windows() {
        if label.starts_with("region-") {
            let _ = window.destroy();
        }
    }
    match visibility {
        PickerVisibility::Visible => crate::desktop::show(app),
        PickerVisibility::Minimized => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.minimize();
            }
        }
        PickerVisibility::Hidden => {}
    }
    state.busy.store(false, Ordering::Release);
}

async fn capture(
    app: &AppHandle,
    destination: Destination,
    slot: Option<usize>,
    revision: u64,
) -> Result<Outcome, String> {
    let main = app
        .get_webview_window("main")
        .ok_or("Main window is unavailable.")?;
    main.hide()
        .map_err(|e| format!("Could not hide Slotshot: {e}"))?;
    let snapshots = tauri::async_runtime::spawn_blocking(|| {
        // One settling delay after hiding the launcher; no background polling.
        std::thread::sleep(std::time::Duration::from_millis(150));
        snapshots()
    })
    .await
    .map_err(|e| format!("Capture worker failed: {e}"))??;
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    let state = app.state::<CaptureState>();
    // Build hidden windows only after ALL snapshots exist: no overlay can be captured.
    // Do not hold the session mutex across window creation (webview requests need it).
    let overlays: Vec<_> = snapshots
        .iter()
        .map(|s| (s.label.clone(), s.x, s.y, s.display_width, s.display_height))
        .collect();
    *state.session.lock().map_err(|_| state_error())? = Some(Session {
        snapshots,
        sender,
        submitted: false,
    });
    for (label, x, y, width, height) in overlays {
        create_overlay(app, &label, x, y, width, height).await?;
    }
    let completion = await_ready(&mut receiver, std::time::Duration::from_secs(15)).await?;
    let completion = if matches!(completion, Completion::Ready) {
        let labels: Vec<_> = state
            .session
            .lock()
            .map_err(|_| state_error())?
            .as_ref()
            .ok_or("Capture is no longer active.")?
            .snapshots
            .iter()
            .map(|s| s.label.clone())
            .collect();
        for label in labels {
            app.get_webview_window(&label)
                .ok_or("Selection window closed unexpectedly.")?
                .show()
                .map_err(|e| format!("Could not display selection: {e}"))?;
        }
        // Focus one overlay for Escape; clicking another display transfers focus.
        if let Some(window) = app
            .webview_windows()
            .into_iter()
            .find(|(l, _)| l.starts_with("region-"))
            .map(|(_, w)| w)
        {
            window
                .set_focus()
                .map_err(|e| format!("Could not focus selection: {e}"))?;
        }
        receiver
            .recv()
            .await
            .ok_or("Selection windows closed unexpectedly.")?
    } else {
        completion
    };
    let session = state
        .session
        .lock()
        .map_err(|_| state_error())?
        .take()
        .ok_or("Capture is no longer active.")?;
    match completion {
        Completion::Ready => Err("Selection initialized more than once. Retry capture.".into()),
        Completion::Cancelled => Ok(Outcome::Cancelled),
        Completion::Failed(message) => Err(message),
        Completion::Selected { index, selection } => {
            let snapshot = session
                .snapshots
                .into_iter()
                .nth(index)
                .ok_or("Display is no longer available.")?;
            // Drop all other display images before decoding the selected display.
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let region = selection.pixels(snapshot.width, snapshot.height)?;
                let image =
                    image::load_from_memory_with_format(&snapshot.png, image::ImageFormat::Png)
                        .map_err(|e| format!("Could not decode captured image: {e}"))?
                        .crop_imm(region.x, region.y, region.width, region.height)
                        .into_rgba8();
                let (width, height) = image.dimensions();
                match destination {
                    Destination::Store => {
                        let slot = slots::with_store(&app, |s| s.assign(slot, revision, &image))?;
                        Ok(Outcome::Stored {
                            slot,
                            width,
                            height,
                        })
                    }
                    Destination::Save => {
                        let path = export::save(
                            &slots::with_store(&app, |s| s.export_directory())?,
                            &image,
                        )?;
                        Ok(Outcome::Saved {
                            path: path.to_string_lossy().into_owned(),
                            width,
                            height,
                        })
                    }
                }
            })
            .await
            .map_err(|e| format!("Image export worker failed: {e}"))?
        }
    }
}

#[tauri::command]
pub async fn capture_region(
    app: AppHandle,
    window: WebviewWindow,
    destination: Destination,
    slot: Option<usize>,
    revision: u64,
    replace: bool,
) -> Result<Outcome, String> {
    if window.label() != "main" {
        return Err("Capture must start from the main window.".into());
    }
    let operation = app.state::<SlotState>().begin()?;
    run_capture(&app, destination, slot, revision, replace, operation).await
}

pub fn active(app: &AppHandle) -> bool {
    app.state::<CaptureState>().busy.load(Ordering::Acquire)
}

// Both native actions and the restricted main-window command use this path.
// The operation is acquired before native dispatch, so repeated presses cannot queue captures.
pub async fn run_capture(
    app: &AppHandle,
    destination: Destination,
    slot: Option<usize>,
    revision: u64,
    replace: bool,
    _operation: slots::Operation,
) -> Result<Outcome, String> {
    let main = app
        .get_webview_window("main")
        .ok_or("Main window is unavailable.")?;
    let previous = if !main.is_visible().map_err(|e| e.to_string())? {
        PickerVisibility::Hidden
    } else if main.is_minimized().map_err(|e| e.to_string())? {
        PickerVisibility::Minimized
    } else {
        PickerVisibility::Visible
    };
    let app_for_validation = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        slots::with_store(&app_for_validation, |s| {
            // Setup is required for either destination, including native actions.
            s.export_directory()?;
            if matches!(destination, Destination::Store) {
                s.validate_capture_target(slot, revision, replace)?;
            }
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    if app
        .state::<CaptureState>()
        .busy
        .swap(true, Ordering::AcqRel)
    {
        return Err("A capture is already in progress.".into());
    }
    let result = capture(app, destination, slot, revision).await;
    cleanup(app, restore_visibility(previous, &result));
    result
}

fn restore_visibility(
    previous: PickerVisibility,
    result: &Result<Outcome, String>,
) -> PickerVisibility {
    if matches!(result, Ok(Outcome::Stored { .. })) {
        PickerVisibility::Visible
    } else {
        previous
    }
}

pub fn shutdown(app: &AppHandle) {
    // Exit must release snapshots and destroy all temporary overlays without revealing the picker.
    cleanup(app, PickerVisibility::Hidden);
}

#[tauri::command]
pub fn region_snapshot(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<tauri::ipc::Response, String> {
    let state = app.state::<CaptureState>();
    let session = state.session.lock().map_err(|_| state_error())?;
    let snapshot = session
        .as_ref()
        .and_then(|s| s.snapshots.iter().find(|s| s.label == window.label()))
        .ok_or("This selection window is no longer active.")?;
    Ok(tauri::ipc::Response::new(snapshot.png.clone()))
}

#[tauri::command]
pub fn region_ready(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    let state = app.state::<CaptureState>();
    let mut session = state.session.lock().map_err(|_| state_error())?;
    let session = session.as_mut().ok_or("Capture is no longer active.")?;
    if session.submitted {
        return Err("Capture has already finished.".into());
    }
    let snapshot = session
        .snapshots
        .iter_mut()
        .find(|s| s.label == window.label())
        .ok_or("This selection window is no longer active.")?;
    if !snapshot.ready {
        snapshot.ready = true;
        if session.snapshots.iter().all(|s| s.ready) {
            session
                .sender
                .try_send(Completion::Ready)
                .map_err(|_| "Selection initialization was interrupted.")?;
        }
    }
    Ok(())
}

async fn await_ready(
    receiver: &mut tauri::async_runtime::Receiver<Completion>,
    deadline: std::time::Duration,
) -> Result<Completion, String> {
    tokio::time::timeout(deadline, receiver.recv())
        .await
        .map_err(|_| {
            "Selection windows did not initialize within 15 seconds. Retry capture.".to_string()
        })?
        .ok_or_else(|| "Selection windows closed unexpectedly.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_visibility_restores_on_cancel_failure_and_save_but_store_reveals() {
        for previous in [
            PickerVisibility::Hidden,
            PickerVisibility::Minimized,
            PickerVisibility::Visible,
        ] {
            for result in [
                Ok(Outcome::Cancelled),
                Err("Snapshot failed".into()),
                Ok(Outcome::Saved {
                    path: "export.png".into(),
                    width: 10,
                    height: 10,
                }),
            ] {
                assert_eq!(restore_visibility(previous, &result), previous);
            }
            assert_eq!(
                restore_visibility(
                    previous,
                    &Ok(Outcome::Stored {
                        slot: 1,
                        width: 10,
                        height: 10
                    })
                ),
                PickerVisibility::Visible
            );
        }
    }

    #[test]
    fn initialization_timeout_recovers_even_when_sender_remains_alive() {
        tauri::async_runtime::block_on(async {
            let (_sender, mut receiver) = tauri::async_runtime::channel(1);
            assert!(
                await_ready(&mut receiver, std::time::Duration::from_millis(1))
                    .await
                    .unwrap_err()
                    .contains("did not initialize")
            );
        });
    }

    #[test]
    fn readiness_and_early_cancellation_are_delivered_without_waiting_for_timeout() {
        tauri::async_runtime::block_on(async {
            for completion in [Completion::Ready, Completion::Cancelled] {
                let (sender, mut receiver) = tauri::async_runtime::channel(1);
                sender.try_send(completion).unwrap();
                assert!(matches!(
                    await_ready(&mut receiver, std::time::Duration::from_secs(1)).await,
                    Ok(Completion::Ready | Completion::Cancelled)
                ));
            }
        });
    }
}

fn complete(
    app: &AppHandle,
    label: &str,
    selection: Option<Selection>,
    failed: bool,
) -> Result<(), String> {
    let state = app.state::<CaptureState>();
    let mut session = state.session.lock().map_err(|_| state_error())?;
    let session = session.as_mut().ok_or("Capture is no longer active.")?;
    let index = session
        .snapshots
        .iter()
        .position(|s| s.label == label)
        .ok_or("Unknown selection window.")?;
    if session.submitted {
        return Err("Capture has already finished.".into());
    }
    let completion = if let Some(selection) = selection {
        let snapshot = &session.snapshots[index];
        selection.pixels(snapshot.width, snapshot.height)?;
        Completion::Selected { index, selection }
    } else if failed {
        Completion::Failed(
            "The selection window could not display its snapshot. Retry capture.".into(),
        )
    } else {
        Completion::Cancelled
    };
    session
        .sender
        .try_send(completion)
        .map_err(|_| "Capture has already finished.")?;
    session.submitted = true;
    Ok(())
}

#[tauri::command]
pub fn finish_region(
    app: AppHandle,
    window: WebviewWindow,
    selection: Selection,
) -> Result<(), String> {
    complete(&app, window.label(), Some(selection), false)
}

#[tauri::command]
pub fn cancel_region(app: AppHandle, window: WebviewWindow, failed: bool) -> Result<(), String> {
    complete(&app, window.label(), None, failed)
}

pub fn window_closed(app: &AppHandle, label: &str) {
    if label.starts_with("region-") {
        let _ = complete(app, label, None, false);
    }
}
