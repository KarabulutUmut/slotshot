use crate::{
    export,
    region::Selection,
    slots::{self, SlotState},
    timing::Timing,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
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
    Choosing {
        capture: u64,
        width: u32,
        height: u32,
    },
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
#[derive(Clone, Debug, PartialEq)]
struct Display {
    id: u32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale: f64,
}
impl Display {
    fn label(&self) -> String {
        format!("region-{}", self.id)
    }
    fn read(monitor: &xcap::Monitor) -> Result<Self, String> {
        let error = |e| format!("Could not read display geometry: {e}");
        Ok(Self {
            id: monitor.id().map_err(error)?,
            x: monitor.x().map_err(error)?,
            y: monitor.y().map_err(error)?,
            width: monitor.width().map_err(error)?,
            height: monitor.height().map_err(error)?,
            scale: f64::from(monitor.scale_factor().map_err(error)?),
        })
    }
}
#[derive(Debug)]
enum Completion {
    Ready,
    Selected { index: usize, selection: Selection },
    Slot(usize),
    Cancelled,
    Failed(String),
}
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Selecting,
    Acquiring,
    Choosing,
}
struct Session {
    id: u64,
    displays: Vec<Display>,
    sender: tauri::async_runtime::Sender<Completion>,
    submitted: bool,
    ready_sent: bool,
    phase: Phase,
    timing: Timing,
}
#[derive(Default)]
pub struct CaptureState {
    busy: AtomicBool,
    session: Mutex<Option<Session>>,
    ready: Mutex<HashSet<String>>,
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

fn displays() -> Result<Vec<Display>, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("Could not list displays: {e}"))?;
    if monitors.is_empty() || monitors.len() > 16 {
        return Err("Capture requires between 1 and 16 active displays.".into());
    }
    let displays = monitors
        .iter()
        .map(Display::read)
        .collect::<Result<Vec<_>, _>>()?;
    if displays
        .iter()
        .any(|d| d.width == 0 || d.height == 0 || !d.scale.is_finite() || d.scale <= 0.0)
    {
        return Err("Invalid display geometry. Retry capture.".into());
    }
    Ok(displays)
}
// Geometry-only prewarming. No permission request, screenshot, timer, or recording while idle.
pub fn prewarm(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let operation = match app.state::<SlotState>().begin() {
            Ok(op) => op,
            Err(_) => return,
        };
        if let Ok(Ok(displays)) = tauri::async_runtime::spawn_blocking(displays).await {
            for display in displays {
                let _ = create_overlay(
                    &app,
                    &display.label(),
                    display.x,
                    display.y,
                    display.width,
                    display.height,
                )
                .await;
            }
        }
        drop(operation);
    });
}
fn acquire(
    display: &Display,
    selection: Selection,
    timing: Timing,
) -> Result<image::RgbaImage, String> {
    permission()?;
    let monitor = xcap::Monitor::all()
        .map_err(|e| format!("Could not list displays: {e}"))?
        .into_iter()
        .find(|m| m.id().ok() == Some(display.id))
        .ok_or("Display disconnected. Retry capture.")?;
    if Display::read(&monitor)? != *display {
        return Err("Display geometry changed during selection. Retry capture.".into());
    }
    // XCap takes desktop points on macOS and physical monitor pixels on Windows.
    let region = selection.pixels(display.width, display.height)?;
    let scale = if cfg!(target_os = "macos") {
        display.scale
    } else {
        1.0
    };
    let pixels = f64::from(region.width) * f64::from(region.height) * scale * scale;
    if pixels > MAX_PIXELS as f64 {
        return Err("Selected region exceeds the 64 megapixel memory limit.".into());
    }
    let image = monitor.capture_region(region.x, region.y, region.width, region.height)
        .map_err(|e| format!("Screen capture backend failed after the permission check passed: {e}. Retry capture."))?;
    timing.mark("screen_acquisition_completed");
    if image.width() == 0
        || image.height() == 0
        || u64::from(image.width()) * u64::from(image.height()) > MAX_PIXELS
    {
        return Err("Captured region is empty or exceeds the 64 megapixel memory limit.".into());
    }
    Ok(image)
}
async fn create_overlay(
    app: &AppHandle,
    label: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let window = if let Some(window) = app.get_webview_window(label) {
        window
    } else {
        WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html?region".into()))
            .title("Slotshot — drag a region, Escape to cancel")
            .transparent(true)
            .background_color(tauri::window::Color(0, 0, 0, 0))
            .content_protected(true)
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
            .map_err(|e| format!("Could not open selection window: {e}"))?
    };
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

async fn hide_overlays(app: &AppHandle) -> Result<(), String> {
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let result = handle
            .webview_windows()
            .into_iter()
            .filter(|(label, _)| label.starts_with("region-"))
            .try_for_each(|(_, window)| {
                window.hide().map_err(|e| e.to_string())?;
                if window.is_visible().map_err(|e| e.to_string())? {
                    return Err("Selection overlay could not be hidden; capture aborted.".into());
                }
                Ok(())
            });
        let _ = sender.try_send(result);
    })
    .map_err(|e| e.to_string())?;
    receiver
        .recv()
        .await
        .ok_or("Overlay dismissal was interrupted.")?
}

async fn cleanup(app: &AppHandle, visibility: PickerVisibility) {
    let state = app.state::<CaptureState>();
    // Pending pixels exist only in the capture future, never a temporary file.
    if let Ok(mut session) = state.session.lock() {
        *session = None;
    }
    let _ = hide_overlays(app).await;
    match visibility {
        PickerVisibility::Visible => crate::desktop::show(app),
        PickerVisibility::Minimized => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.minimize();
            }
        }
        PickerVisibility::Hidden => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
    }
    state.busy.store(false, Ordering::Release);
}

fn signal_ready(state: &CaptureState) -> Result<(), String> {
    let ready = state.ready.lock().map_err(|_| state_error())?;
    let mut guard = state.session.lock().map_err(|_| state_error())?;
    if let Some(session) = guard.as_mut()
        && !session.ready_sent
        && session.displays.iter().all(|d| ready.contains(&d.label()))
    {
        session
            .sender
            .try_send(Completion::Ready)
            .map_err(|_| "Selection initialization was interrupted.")?;
        session.ready_sent = true;
    }
    Ok(())
}

async fn capture(
    app: &AppHandle,
    destination: Destination,
    revision: u64,
    timing: Timing,
) -> Result<Outcome, String> {
    let main = app
        .get_webview_window("main")
        .ok_or("Main window is unavailable.")?;
    main.hide()
        .map_err(|e| format!("Could not hide Slotshot: {e}"))?;
    timing.mark("picker_hidden");
    let displays = tauri::async_runtime::spawn_blocking(displays)
        .await
        .map_err(|e| e.to_string())??;
    timing.mark("display_enumeration_completed");
    let labels: HashSet<_> = displays.iter().map(Display::label).collect();
    // Display hot-plug keeps the pool bounded to currently connected displays.
    for (label, window) in app.webview_windows() {
        if label.starts_with("region-") && !labels.contains(&label) {
            app.state::<CaptureState>()
                .ready
                .lock()
                .map_err(|_| state_error())?
                .remove(&label);
            let _ = window.destroy();
        }
    }
    let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let (sender, mut receiver) = tauri::async_runtime::channel(4);
    let state = app.state::<CaptureState>();
    *state.session.lock().map_err(|_| state_error())? = Some(Session {
        id,
        displays: displays.clone(),
        sender,
        submitted: false,
        ready_sent: false,
        phase: Phase::Selecting,
        timing,
    });
    for display in &displays {
        create_overlay(
            app,
            &display.label(),
            display.x,
            display.y,
            display.width,
            display.height,
        )
        .await?;
    }
    timing.mark("overlay_creation_completed");
    signal_ready(&state)?;
    match await_ready(&mut receiver, std::time::Duration::from_secs(15)).await? {
        Completion::Ready => {}
        Completion::Cancelled => return Ok(Outcome::Cancelled),
        Completion::Failed(message) => return Err(message),
        _ => return Err("Unexpected selection initialization.".into()),
    }
    for display in &displays {
        let window = app
            .get_webview_window(&display.label())
            .ok_or("Selection window closed unexpectedly.")?;
        window.show().map_err(|e| e.to_string())?;
    }
    // Focus one for Escape. Clicking any other display transfers keyboard focus.
    let window = app
        .get_webview_window(&displays[0].label())
        .ok_or("Selection window closed unexpectedly.")?;
    window.set_focus().map_err(|e| e.to_string())?;
    timing.mark("overlay_shown_focused");
    for display in &displays {
        app.emit_to(display.label(), "region-activate", id)
            .map_err(|e| e.to_string())?;
    }
    let (index, selection) = match receiver
        .recv()
        .await
        .ok_or("Selection windows closed unexpectedly.")?
    {
        Completion::Selected { index, selection } => (index, selection),
        Completion::Cancelled => return Ok(Outcome::Cancelled),
        Completion::Failed(message) => return Err(message),
        _ => return Err("Unexpected selection result.".into()),
    };
    timing.mark("selection_completed");
    // Complete native hide calls on the UI thread before asking the OS for pixels.
    // Protected overlays are an additional safeguard, not a replacement for dismissal.
    hide_overlays(app).await?;
    timing.mark("overlays_hidden");
    let display = displays[index].clone();
    let (image, png) = tauri::async_runtime::spawn_blocking(move || {
        let image = acquire(&display, selection, timing)?;
        let png = export::png(&image)?;
        timing.mark("png_encoding_completed");
        timing.mark("final_region_capture_encoding_completed");
        Ok::<_, String>((image, png))
    })
    .await
    .map_err(|e| format!("Capture worker failed: {e}"))??;
    let (width, height) = image.dimensions();
    let slot = match destination {
        Destination::Save => None,
        Destination::Store => {
            {
                let mut guard = state.session.lock().map_err(|_| state_error())?;
                let session = guard.as_mut().ok_or("Capture is no longer active.")?;
                session.phase = Phase::Choosing;
                session.submitted = false;
            }
            crate::desktop::choosing(app, id, width, height);
            crate::desktop::show(app);
            match receiver
                .recv()
                .await
                .ok_or("Destination selection was interrupted.")?
            {
                Completion::Slot(slot) => Some(slot),
                Completion::Cancelled => return Ok(Outcome::Cancelled),
                _ => return Err("Unexpected capture destination.".into()),
            }
        }
    };
    let worker = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let outcome = match slot {
            Some(slot) => {
                let slot = slots::with_store(&worker, |s| {
                    s.assign_encoded(Some(slot), revision, &image, &png)
                })?;
                Outcome::Stored {
                    slot,
                    width,
                    height,
                }
            }
            None => {
                let path = export::save_bytes(
                    &slots::with_store(&worker, |s| s.export_directory())?,
                    &png,
                )?;
                Outcome::Saved {
                    path: path.to_string_lossy().into_owned(),
                    width,
                    height,
                }
            }
        };
        timing.mark("storage_completed");
        Ok(outcome)
    })
    .await
    .map_err(|e| format!("Storage worker failed: {e}"))?
}

#[tauri::command]
pub async fn capture_region(
    app: AppHandle,
    window: WebviewWindow,
    destination: Destination,
) -> Result<Outcome, String> {
    slots::main_only(&window)?;
    let operation = app.state::<SlotState>().begin()?;
    let result = run_capture(&app, destination, operation, Timing::new("button_received")).await;
    crate::desktop::capture_finished(&app, &result);
    result
}
pub fn active(app: &AppHandle) -> bool {
    app.state::<CaptureState>().busy.load(Ordering::Acquire)
}

pub async fn run_capture(
    app: &AppHandle,
    destination: Destination,
    _operation: slots::Operation,
    timing: Timing,
) -> Result<Outcome, String> {
    timing.mark("native_command_started");
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
    let worker = app.clone();
    let revision = tauri::async_runtime::spawn_blocking(move || {
        slots::with_store(&worker, |s| {
            s.export_directory()?;
            Ok(s.view().revision)
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
    let result = capture(app, destination, revision, timing).await;
    cleanup(app, restore_visibility(previous, &result)).await;
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
    if let Ok(mut session) = app.state::<CaptureState>().session.lock() {
        *session = None;
    }
    for (label, window) in app.webview_windows() {
        if label.starts_with("region-") {
            let _ = window.destroy();
        }
    }
}
#[tauri::command]
pub fn region_ready(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    if !window.label().starts_with("region-") {
        return Err("Unknown selection window.".into());
    }
    let state = app.state::<CaptureState>();
    state
        .ready
        .lock()
        .map_err(|_| state_error())?
        .insert(window.label().into());
    signal_ready(&state)
}
#[tauri::command]
pub fn region_interactive(
    app: AppHandle,
    window: WebviewWindow,
    capture: u64,
) -> Result<(), String> {
    let state = app.state::<CaptureState>();
    let guard = state.session.lock().map_err(|_| state_error())?;
    let session = guard.as_ref().ok_or("Capture is no longer active.")?;
    validate_overlay(session, window.label(), capture)?;
    if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
        session.timing.mark("overlay_interactive");
    }
    Ok(())
}
fn validate_overlay(session: &Session, label: &str, id: u64) -> Result<usize, String> {
    if session.id != id || session.phase != Phase::Selecting || session.submitted {
        return Err("Selection is no longer active.".into());
    }
    session
        .displays
        .iter()
        .position(|d| d.label() == label)
        .ok_or("Unknown selection window.".into())
}
fn complete(
    app: &AppHandle,
    label: &str,
    id: u64,
    selection: Option<Selection>,
    failed: bool,
) -> Result<(), String> {
    let state = app.state::<CaptureState>();
    let mut guard = state.session.lock().map_err(|_| state_error())?;
    let session = guard.as_mut().ok_or("Capture is no longer active.")?;
    let index = validate_overlay(session, label, id)?;
    let completion = if let Some(selection) = selection {
        let display = &session.displays[index];
        selection.pixels(display.width, display.height)?;
        Completion::Selected { index, selection }
    } else if failed {
        Completion::Failed("Selection window failed. Retry capture.".into())
    } else {
        Completion::Cancelled
    };
    session
        .sender
        .try_send(completion)
        .map_err(|_| "Capture has already finished.")?;
    session.submitted = true;
    session.phase = Phase::Acquiring;
    Ok(())
}
#[tauri::command]
pub fn finish_region(
    app: AppHandle,
    window: WebviewWindow,
    capture: u64,
    selection: Selection,
) -> Result<(), String> {
    complete(&app, window.label(), capture, Some(selection), false)
}
#[tauri::command]
pub fn cancel_region(
    app: AppHandle,
    window: WebviewWindow,
    capture: u64,
    failed: bool,
) -> Result<(), String> {
    complete(&app, window.label(), capture, None, failed)
}
#[tauri::command]
pub fn choose_capture_slot(
    app: AppHandle,
    window: WebviewWindow,
    capture: u64,
    slot: Option<usize>,
) -> Result<(), String> {
    slots::main_only(&window)?;
    let state = app.state::<CaptureState>();
    let mut guard = state.session.lock().map_err(|_| state_error())?;
    let session = guard
        .as_mut()
        .ok_or("No screenshot is waiting for a destination.")?;
    validate_choice(session, capture, slot)?;
    if let Some(slot) = slot {
        slots::with_store(&app, |s| {
            s.validate_target(Some(slot), s.view().revision).map(|_| ())
        })?;
    }
    session
        .sender
        .try_send(slot.map_or(Completion::Cancelled, Completion::Slot))
        .map_err(|_| "Destination has already been chosen.")?;
    session.submitted = true;
    Ok(())
}
fn validate_choice(session: &Session, id: u64, slot: Option<usize>) -> Result<(), String> {
    if session.id != id || session.phase != Phase::Choosing || session.submitted {
        return Err("Destination selection is no longer active.".into());
    }
    if slot.is_some_and(|s| !(1..=10).contains(&s)) {
        return Err("Invalid slot number.".into());
    }
    Ok(())
}
pub fn window_closed(app: &AppHandle, label: &str) {
    if !label.starts_with("region-") {
        return;
    }
    let state = app.state::<CaptureState>();
    if let Ok(mut ready) = state.ready.lock() {
        ready.remove(label);
    }
    if let Ok(mut guard) = state.session.lock()
        && let Some(session) = guard.as_mut()
        && session.displays.iter().any(|d| d.label() == label)
        && !session.submitted
    {
        let _ = session.sender.try_send(Completion::Cancelled);
        session.submitted = true;
    }
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

    fn session(phase: Phase) -> Session {
        let (sender, _) = tauri::async_runtime::channel(4);
        Session {
            id: 42,
            displays: vec![Display {
                id: 7,
                x: -1512,
                y: -100,
                width: 1512,
                height: 982,
                scale: 2.0,
            }],
            sender,
            submitted: false,
            ready_sent: false,
            phase,
            timing: Timing::new("test"),
        }
    }

    #[test]
    fn destination_rejects_wrong_session_phase_duplicates_and_invalid_slots() {
        let mut pending = session(Phase::Selecting);
        assert!(validate_choice(&pending, 42, Some(1)).is_err());
        pending.phase = Phase::Acquiring;
        assert!(validate_choice(&pending, 42, None).is_err());
        pending.phase = Phase::Choosing;
        for slot in 1..=10 {
            assert!(validate_choice(&pending, 42, Some(slot)).is_ok());
        }
        assert!(validate_choice(&pending, 42, None).is_ok());
        for slot in [0, 11, usize::MAX] {
            assert!(validate_choice(&pending, 42, Some(slot)).is_err());
        }
        assert!(validate_choice(&pending, 41, Some(1)).is_err());
        pending.submitted = true;
        assert!(validate_choice(&pending, 42, Some(1)).is_err());
        assert!(validate_choice(&pending, 42, None).is_err());
    }

    #[test]
    fn reused_overlays_reject_previous_capture_messages_and_other_displays() {
        let mut selecting = session(Phase::Selecting);
        assert_eq!(validate_overlay(&selecting, "region-7", 42).unwrap(), 0);
        assert!(validate_overlay(&selecting, "main", 42).is_err());
        assert!(validate_overlay(&selecting, "region-8", 42).is_err());
        assert!(validate_overlay(&selecting, "region-7", 41).is_err());
        selecting.submitted = true;
        assert!(validate_overlay(&selecting, "region-7", 42).is_err());
    }

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
