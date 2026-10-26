//! Native entry points share the capture engine and never delegate capture to React.
use crate::{
    capture::{self, Destination, Outcome},
    shortcuts, slots,
};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, Manager, WebviewWindow,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    sequence: u64,
    busy: bool,
    outcome: Option<Outcome>,
    message: Option<String>,
    error: bool,
    settings: bool,
}
#[derive(Default)]
pub struct DesktopState(Mutex<View>);
fn publish(
    app: &AppHandle,
    busy: bool,
    outcome: Option<Outcome>,
    message: Option<String>,
    error: bool,
    settings: bool,
) {
    let state = app.state::<DesktopState>();
    let mut view = state.0.lock().unwrap();
    view.sequence += 1;
    view.busy = busy;
    view.outcome = outcome;
    view.message = message;
    view.error = error;
    view.settings = settings;
    let _ = app.emit_to("main", "desktop-state", view.clone());
}
#[tauri::command]
pub fn desktop_view(app: AppHandle, window: WebviewWindow) -> Result<View, String> {
    slots::main_only(&window)?;
    Ok(app
        .state::<DesktopState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone())
}
pub fn choosing(app: &AppHandle, capture: u64, width: u32, height: u32) {
    publish(
        app,
        true,
        Some(Outcome::Choosing {
            capture,
            width,
            height,
        }),
        None,
        false,
        false,
    );
}
pub fn capture_finished(app: &AppHandle, result: &Result<Outcome, String>) {
    match result {
        Ok(outcome) => publish(app, false, Some(outcome.clone()), None, false, false),
        Err(error) => publish(app, false, None, Some(error.clone()), true, false),
    }
}
pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
// Also used by second-launch and macOS Dock reopen. Never raise a picker over overlays.
pub fn open(app: &AppHandle) {
    dispatch(app, "open");
}
#[derive(Debug, PartialEq)]
enum WindowAction {
    Show,
    Hide,
    Settings,
    Quit,
    Capture,
    Ignore,
}
fn window_action(action: &str, visible: bool, minimized: bool) -> WindowAction {
    match action {
        "quit" => WindowAction::Quit,
        "close" => WindowAction::Hide,
        "toggle" if visible && !minimized => WindowAction::Hide,
        "toggle" | "open" => WindowAction::Show,
        "settings" => WindowAction::Settings,
        "store" | "save" => WindowAction::Capture,
        _ => WindowAction::Ignore,
    }
}
pub fn dispatch(app: &AppHandle, action: &str) {
    dispatch_timed(
        app,
        action,
        crate::timing::Timing::new("native_action_received"),
    );
}
pub fn dispatch_timed(app: &AppHandle, action: &str, timing: crate::timing::Timing) {
    let window = app.get_webview_window("main");
    let effect = window_action(
        action,
        window
            .as_ref()
            .is_some_and(|w| w.is_visible().unwrap_or(false)),
        window
            .as_ref()
            .is_some_and(|w| w.is_minimized().unwrap_or(false)),
    );
    if effect == WindowAction::Ignore {
        return;
    }
    if effect == WindowAction::Quit {
        crate::window::save(app);
        capture::shutdown(app);
        app.exit(0);
        return;
    }
    let Some(state) = app.try_state::<slots::SlotState>() else {
        return;
    };
    let Ok(operation) = state.begin() else { return };
    match effect {
        WindowAction::Capture => {
            let destination = if action == "store" {
                Destination::Store
            } else {
                Destination::Save
            };
            let app = app.clone();
            publish(&app, true, None, None, false, false);
            tauri::async_runtime::spawn(async move {
                let worker = app.clone();
                let preparation = tauri::async_runtime::spawn_blocking(move || {
                    slots::with_store(&worker, |s| {
                        let view = s.view();
                        if view.export_directory.is_none() {
                            return Ok(None);
                        }
                        Ok(Some(()))
                    })
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|v| v);
                match preparation {
                    Ok(Some(())) => {}
                    Ok(None) => {
                        drop(operation);
                        show(&app);
                        publish(
                            &app,
                            false,
                            None,
                            Some("Choose an export folder before capturing.".into()),
                            false,
                            true,
                        );
                        return;
                    }
                    Err(error) => {
                        drop(operation);
                        show(&app);
                        publish(&app, false, None, Some(error), true, false);
                        return;
                    }
                };
                let result = capture::run_capture(&app, destination, operation, timing).await;
                capture_finished(&app, &result);
            });
        }
        WindowAction::Hide => {
            if let Some(window) = window {
                crate::window::save(app);
                let _ = window.hide();
            }
        }
        WindowAction::Settings => {
            show(app);
            publish(app, false, None, None, false, true);
        }
        WindowAction::Show => show(app),
        WindowAction::Ignore | WindowAction::Quit => {}
    }
}
fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let view = shortcuts::view(app);
    let label = |title: &str, index: usize| match &view.config.keys[index] {
        Some(key) => format!(
            "{title} ({}){}",
            display_key(key),
            if view.errors[index].is_some() {
                " — unavailable"
            } else {
                ""
            }
        ),
        None => title.to_string(),
    };
    let open = MenuItem::with_id(app, "open", label("Open Slotshot", 2), true, None::<&str>)?;
    let store = MenuItem::with_id(app, "store", label("Store in Slot", 0), true, None::<&str>)?;
    let save = MenuItem::with_id(app, "save", label("Save to Device", 1), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Slotshot", true, None::<&str>)?;
    Menu::with_items(app, &[&open, &store, &save, &settings, &quit])
}
fn display_key(key: &str) -> String {
    let key = key.replace("Key", "").replace("Digit", "");
    if cfg!(target_os = "macos") {
        key.replace("Control", "⌃")
            .replace("Alt", "⌥")
            .replace("Shift", "⇧")
            .replace("Super", "⌘")
            .replace("+", "")
    } else {
        key.replace("Control", "Ctrl")
    }
}
pub fn refresh_menu(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("slotshot") {
        match menu(app).and_then(|menu| tray.set_menu(Some(menu))) {
            Ok(()) => {}
            Err(error) => eprintln!("Could not update tray menu: {error}"),
        }
    }
}
fn template_icon() -> tauri::image::Image<'static> {
    // Small pixel-aligned crop corners; alpha-only monochrome template on macOS.
    let mut rgba = vec![0u8; 22 * 22 * 4];
    for y in 0..22usize {
        for x in 0..22usize {
            let corner = ((3..=8).contains(&x) || (13..=18).contains(&x)) && (y == 3 || y == 18)
                || ((3..=8).contains(&y) || (13..=18).contains(&y)) && (x == 3 || x == 18);
            if corner {
                rgba[(y * 22 + x) * 4 + 3] = 255;
            }
        }
    }
    tauri::image::Image::new_owned(rgba, 22, 22)
}
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let icon = if cfg!(target_os = "macos") {
        template_icon()
    } else {
        app.default_window_icon()
            .expect("bundled application icon")
            .clone()
    };
    TrayIconBuilder::with_id("slotshot")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Slotshot")
        .menu(&menu(app)?)
        // macOS status items do not support right-click menus independently;
        // left click opens the minimal menu for immediate access. Windows left click opens the picker.
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| dispatch(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if !cfg!(target_os = "macos")
                && matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                )
            {
                open(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle_close_quit_and_second_launch_preserve_native_lifecycle() {
        assert_eq!(window_action("toggle", true, false), WindowAction::Hide);
        assert_eq!(window_action("toggle", false, false), WindowAction::Show);
        assert_eq!(window_action("toggle", true, true), WindowAction::Show);
        for visible in [true, false] {
            assert_eq!(window_action("close", visible, false), WindowAction::Hide);
            assert_eq!(window_action("quit", visible, false), WindowAction::Quit);
            // Second-launch callback and Dock reopen dispatch this same action.
            assert_eq!(window_action("open", visible, false), WindowAction::Show);
        }
    }
}
