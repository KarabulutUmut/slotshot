use tauri::Manager;

mod capture;
mod export;
mod region;
mod slots;
mod storage;

pub fn run() {
    tauri::Builder::default()
        .manage(capture::CaptureState::default())
        .manage(slots::SlotState::default())
        .invoke_handler(tauri::generate_handler![
            slots::slot_view,
            slots::slot_thumbnail,
            slots::clear_slots,
            slots::resize_slots,
            slots::save_slots,
            slots::configure_exports,
            slots::drag_slots,
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
