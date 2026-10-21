use tauri::Manager;

mod capture;
mod export;
mod region;

pub fn run() {
    tauri::Builder::default()
        .manage(capture::CaptureState::default())
        .invoke_handler(tauri::generate_handler![
            capture::capture_region,
            capture::region_snapshot,
            capture::region_ready,
            capture::finish_region,
            capture::cancel_region,
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                capture::window_closed(window.app_handle(), window.label());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Slotshot");
}
