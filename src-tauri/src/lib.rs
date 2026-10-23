use tauri::Manager;

mod capture;
mod export;
mod region;
mod slots;
mod storage;
mod window;

pub fn run() {
    tauri::Builder::default()
        .manage(capture::CaptureState::default())
        .manage(slots::SlotState::default())
        .manage(window::WindowState::default())
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
        .setup(|app| {
            window::restore(app.handle());
            window::remember(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(
                    event,
                    tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
                )
            {
                window::remember(window.app_handle());
            }
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                window::save(window.app_handle());
            }
            if matches!(event, tauri::WindowEvent::Destroyed) {
                capture::window_closed(window.app_handle(), window.label());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Slotshot")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                window::save(app);
            }
        });
}
