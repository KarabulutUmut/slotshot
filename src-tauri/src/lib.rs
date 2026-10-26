use tauri::Manager;

mod capture;
mod desktop;
mod export;
mod region;
mod shortcuts;
mod slots;
mod storage;
mod timing;
mod window;

pub fn run() {
    let mut context = tauri::generate_context!();
    if std::env::args().any(|arg| arg == "--background") {
        for window in &mut context.config_mut().app.windows {
            if window.label == "main" {
                window.visible = false;
                window.focus = false;
            }
        }
    }
    // Configure before native windows are created, in both debug and release.
    // Keep the shared HUD/Acrylic configuration unchanged on Windows.
    #[cfg(target_os = "macos")]
    let context = {
        use tauri::window::{Color, Effect, EffectState, EffectsBuilder};
        let mut context = context;
        if let Some(picker) = context
            .config_mut()
            .app
            .windows
            .iter_mut()
            .find(|w| w.label == "main")
        {
            picker.window_effects = Some(
                EffectsBuilder::new()
                    .effect(Effect::LiquidGlassRegular)
                    .effect(Effect::HudWindow)
                    .state(EffectState::Active)
                    .radius(12.0)
                    .color(Color(20, 24, 32, 28))
                    .build(),
            );
        }
        context
    };
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            desktop::open(app)
        }))
        .manage(shortcuts::ShortcutStateData::default())
        .manage(desktop::DesktopState::default())
        .manage(capture::CaptureState::default())
        .manage(slots::SlotState::default())
        .manage(window::WindowState::default());
    let builder = builder.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(shortcuts::handle)
            .build(),
    );
    builder
        .invoke_handler(tauri::generate_handler![
            shortcuts::shortcut_view,
            shortcuts::shortcut_recording,
            shortcuts::configure_shortcuts,
            desktop::desktop_view,
            slots::slot_view,
            slots::slot_thumbnail,
            slots::clear_slots,
            slots::resize_slots,
            slots::save_slots,
            slots::configure_exports,
            slots::drag_slots,
            capture::capture_region,
            capture::choose_capture_slot,
            capture::region_ready,
            capture::region_interactive,
            capture::finish_region,
            capture::cancel_region,
        ])
        .setup(|app| {
            // Open the existing locked store once; a second launch exits in the first plugin.
            slots::with_store(app.handle(), |_| Ok(()))?;
            shortcuts::setup(app.handle());
            desktop::setup(app.handle())?;
            window::restore(app.handle());
            window::remember(app.handle());
            capture::prewarm(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Focused(false)) {
                shortcuts::stop_recording(window.app_handle());
            }
            if window.label() == "main"
                && matches!(
                    event,
                    tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
                )
            {
                window::remember(window.app_handle());
            }
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                desktop::dispatch(window.app_handle(), "close");
            }
            if matches!(event, tauri::WindowEvent::Destroyed) {
                capture::window_closed(window.app_handle(), window.label());
            }
        })
        .build(context)
        .expect("error while building Slotshot")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                window::save(app);
                capture::shutdown(app);
            }
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::RunEvent::Reopen { .. }) {
                desktop::open(app);
            }
        });
}
