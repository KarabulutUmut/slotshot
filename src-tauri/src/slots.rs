use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use crate::storage::{ExportResult, Store, View};
use tauri::{AppHandle, Manager, WebviewWindow};

#[derive(Default)]
pub struct SlotState {
    store: Mutex<Option<Store>>,
    active: Arc<AtomicBool>,
}

pub struct Operation(Arc<AtomicBool>);
impl Drop for Operation {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl SlotState {
    pub fn begin(&self) -> Result<Operation, String> {
        if self.active.swap(true, Ordering::AcqRel) {
            return Err("Finish the current capture, folder dialog, or native drag first.".into());
        }
        Ok(Operation(self.active.clone()))
    }
}

pub fn with_store<T>(
    app: &AppHandle,
    action: impl FnOnce(&mut Store) -> Result<T, String>,
) -> Result<T, String> {
    let state = app.state::<SlotState>();
    let mut guard = state
        .store
        .lock()
        .map_err(|_| "Storage is unavailable. Restart Slotshot.")?;
    if guard.is_none() {
        *guard = Some(Store::open(
            app.path()
                .app_local_data_dir()
                .map_err(|e| e.to_string())?
                .join("slots-v1"),
            app.path().app_config_dir().map_err(|e| e.to_string())?,
            app.path()
                .picture_dir()
                .map_err(|e| format!("Could not locate Pictures: {e}"))?,
        )?);
    }
    action(guard.as_mut().ok_or("Storage is unavailable.")?)
}

pub fn main_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("This operation requires the picker window.".into())
    }
}

#[tauri::command]
pub async fn slot_view(app: AppHandle, window: WebviewWindow) -> Result<View, String> {
    main_only(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        with_store(&app, |s| {
            // No cleanup while a drag/capture owns immutable source generations.
            let mut view = s.view();
            if !app.state::<SlotState>().active.load(Ordering::Acquire)
                && let Err(error) = s.reconcile()
            {
                view.warning = Some(match view.warning {
                    Some(warning) => format!("{warning} {error}"),
                    None => error,
                });
            }
            Ok(view)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn slot_thumbnail(
    app: AppHandle,
    window: WebviewWindow,
    id: usize,
) -> Result<tauri::ipc::Response, String> {
    main_only(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        with_store(&app, |s| s.thumbnail(id).map(tauri::ipc::Response::new))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn clear_slots(
    app: AppHandle,
    window: WebviewWindow,
    ids: Vec<usize>,
    revision: u64,
) -> Result<(), String> {
    main_only(&window)?;
    let operation = app.state::<SlotState>().begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        with_store(&app, |s| s.clear(&ids, revision))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resize_slots(
    app: AppHandle,
    window: WebviewWindow,
    count: usize,
    revision: u64,
    confirmed: bool,
) -> Result<(), String> {
    main_only(&window)?;
    let operation = app.state::<SlotState>().begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        with_store(&app, |s| s.resize(count, revision, confirmed))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_slots(
    app: AppHandle,
    window: WebviewWindow,
    ids: Vec<usize>,
) -> Result<ExportResult, String> {
    main_only(&window)?;
    let operation = app.state::<SlotState>().begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        with_store(&app, |s| s.export_selected(&ids))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn configure_exports(
    app: AppHandle,
    window: WebviewWindow,
    use_default: bool,
) -> Result<bool, String> {
    main_only(&window)?;
    let _operation = app.state::<SlotState>().begin()?;
    let default = app.path().picture_dir().map_err(|e| e.to_string())?;
    let destination = if use_default {
        Some(default)
    } else {
        rfd::AsyncFileDialog::new()
            .set_title("Choose Slotshot export folder")
            .set_directory(default)
            .pick_folder()
            .await
            .map(|file| file.path().to_path_buf())
    };
    if let Some(destination) = destination {
        tauri::async_runtime::spawn_blocking(move || {
            with_store(&app, |s| s.configure(destination))
        })
        .await
        .map_err(|e| e.to_string())??;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn finish_drag(
    lease: &Mutex<Option<Operation>>,
    sender: &tauri::async_runtime::Sender<Result<String, String>>,
    result: Result<drag::DragResult, String>,
) {
    if let Ok(mut guard) = lease.lock() {
        guard.take();
    }
    let _ = sender.try_send(result.map(|outcome| match outcome {
        drag::DragResult::Dropped => "Files offered to destination.".into(),
        drag::DragResult::Cancel => "Drag cancelled or rejected.".into(),
    }));
}

#[tauri::command]
pub async fn drag_slots(
    app: AppHandle,
    window: WebviewWindow,
    ids: Vec<usize>,
) -> Result<String, String> {
    main_only(&window)?;
    let operation = app.state::<SlotState>().begin()?;
    let app_for_files = app.clone();
    let (paths, icon) = tauri::async_runtime::spawn_blocking(move || {
        with_store(&app_for_files, |s| {
            let paths = s.drag_files(&ids)?;
            let icon = s.thumbnail(ids[0])?;
            Ok((paths, icon))
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    let native_window = window.clone();
    window
        .run_on_main_thread(move || {
            // Keep the lease until the OS completion callback (async on macOS).
            // The callback can be retained by AppKit, so explicitly take/drop it.
            let lease = Arc::new(Mutex::new(Some(operation)));
            let callback_lease = lease.clone();
            let result_sender = sender.clone();
            let result = drag::start_drag(
                &native_window,
                drag::DragItem::Files(paths),
                drag::Image::Raw(icon),
                move |result, _| {
                    finish_drag(&callback_lease, &result_sender, Ok(result));
                },
                drag::Options {
                    mode: drag::DragMode::Copy,
                    ..Default::default()
                },
            );
            if let Err(e) = result {
                finish_drag(
                    &lease,
                    &sender,
                    Err(format!("Could not start native file drag: {e}")),
                );
            }
        })
        .map_err(|e| format!("Could not schedule native file drag: {e}"))?;
    receiver
        .recv()
        .await
        .ok_or("Native drag ended without a result.")?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_drag_error_and_cancellation_release_sources_and_reach_caller() {
        for result in [
            Err("Could not start native file drag: unsupported handle".into()),
            Ok(drag::DragResult::Cancel),
            Ok(drag::DragResult::Dropped),
        ] {
            let state = SlotState::default();
            let lease = Mutex::new(Some(state.begin().unwrap()));
            let (sender, mut receiver) = tauri::async_runtime::channel(1);
            assert!(state.begin().is_err());
            let failed = result.is_err();
            finish_drag(&lease, &sender, result);
            let delivered = receiver.try_recv().unwrap();
            assert_eq!(delivered.is_err(), failed);
            assert!(lease.lock().unwrap().is_none());
            assert!(state.begin().is_ok());
        }
    }

    #[test]
    fn operation_guard_blocks_mutation_until_completion_or_error() {
        let state = SlotState::default();
        let lease = state.begin().unwrap();
        assert!(state.begin().is_err());
        drop(lease); // Both cancelled and completed drags release the same guard.
        assert!(state.begin().is_ok());
    }
}
