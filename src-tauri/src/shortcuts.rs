//! Rust owns OS registrations; the webview may only submit validated preferences.
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path, sync::Mutex};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub const ACTIONS: [&str; 3] = ["store", "save", "toggle"];
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    version: u8,
    pub keys: [Option<String>; 3],
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            keys: ["1", "2", "3"].map(|key| Some(format!("Control+Alt+Shift+{key}"))),
        }
    }
}
#[derive(Clone, Serialize)]
pub struct View {
    pub config: Config,
    pub errors: [Option<String>; 3],
}
struct Runtime {
    view: View,
    registered: HashSet<Shortcut>,
    held: HashSet<u32>,
    recording: bool,
}
pub struct ShortcutStateData(Mutex<Runtime>);
impl Default for ShortcutStateData {
    fn default() -> Self {
        Self(Mutex::new(Runtime {
            view: View {
                config: Config::default(),
                errors: Default::default(),
            },
            registered: HashSet::new(),
            held: HashSet::new(),
            recording: false,
        }))
    }
}

pub fn parse(value: &str) -> Result<Shortcut, String> {
    if value.len() > 100 {
        return Err("Shortcut is too long.".into());
    }
    let key: Shortcut = value.parse().map_err(|_| {
        "Use modifiers followed by one letter, number, or function key.".to_string()
    })?;
    let mods = key.mods;
    let name = key.key.to_string();
    let ordinary = name.strip_prefix("Key").is_some_and(|s| s.len() == 1)
        || name.strip_prefix("Digit").is_some_and(|s| s.len() == 1)
        || name
            .strip_prefix('F')
            .and_then(|s| s.parse::<u8>().ok())
            .is_some_and(|n| (1..=24).contains(&n));
    if !ordinary
        || mods.bits().count_ones() < 2
        || !mods.intersects(Modifiers::CONTROL | Modifiers::SUPER)
    {
        return Err("Use at least two modifiers, including Control or Command, and a letter, number, or function key.".into());
    }
    // Conservative deny list: standard editing/quit keys, OS capture, session and application switching.
    let editing = matches!(
        key.key,
        Code::KeyC
            | Code::KeyV
            | Code::KeyX
            | Code::KeyZ
            | Code::KeyA
            | Code::KeyQ
            | Code::KeyW
            | Code::KeyH
            | Code::KeyM
    );
    let mac_capture = mods.contains(Modifiers::SUPER | Modifiers::SHIFT)
        && matches!(
            key.key,
            Code::Digit3 | Code::Digit4 | Code::Digit5 | Code::Digit6
        );
    let windows_capture = mods.contains(Modifiers::SUPER)
        || (mods.contains(Modifiers::CONTROL | Modifiers::SHIFT) && key.key == Code::KeyS);
    let system_function = (mods.contains(Modifiers::ALT) && key.key == Code::F4)
        || (mods.contains(Modifiers::CONTROL)
            && matches!(key.key, Code::F2 | Code::F3 | Code::F4 | Code::F8));
    if editing || mac_capture || (!cfg!(target_os = "macos") && windows_capture) || system_function
    {
        return Err("That combination is reserved for editing or an operating-system action. Choose another shortcut.".into());
    }
    if cfg!(target_os = "macos")
        && name
            .strip_prefix('F')
            .and_then(|s| s.parse::<u8>().ok())
            .is_some_and(|n| n > 20)
    {
        return Err("This key is unsupported on macOS. Use letters, numbers, or F1–F20.".into());
    }
    if !cfg!(target_os = "macos") && key.key == Code::F12 {
        return Err("F12 is reserved by Windows for debugging. Choose another key.".into());
    }
    Ok(key)
}
fn validate(config: &Config) -> Result<HashSet<Shortcut>, String> {
    if config.version != 1 {
        return Err("Unsupported shortcut preference version.".into());
    }
    let mut keys = HashSet::new();
    for key in config.keys.iter().flatten() {
        if !keys.insert(parse(key)?) {
            return Err("Each Slotshot action must have a different shortcut.".into());
        }
    }
    Ok(keys)
}
fn load(path: &Path) -> Result<Config, String> {
    let config = match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| format!("Could not read shortcuts: {e}"))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => return Err(e.to_string()),
    };
    validate(&config)?;
    Ok(config)
}
fn persist(path: &Path, config: &Config) -> Result<(), String> {
    use std::io::Write;
    let directory = path.parent().ok_or("Missing preference directory.")?;
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(config).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

trait Registrar {
    fn register(&self, key: Shortcut) -> Result<(), String>;
    fn unregister(&self, key: Shortcut) -> Result<(), String>;
}
impl Registrar for AppHandle {
    fn register(&self, key: Shortcut) -> Result<(), String> {
        self.global_shortcut()
            .register(key)
            .map_err(|e| format!("Could not register {key}. It may be in use by another app: {e}"))
    }
    fn unregister(&self, key: Shortcut) -> Result<(), String> {
        self.global_shortcut()
            .unregister(key)
            .map_err(|e| e.to_string())
    }
}
// Track each successful OS call. Even if compensation fails, dispatch only keys
// actually registered and show which previous assignments need attention.
fn transition(
    registrar: &impl Registrar,
    active: &mut HashSet<Shortcut>,
    wanted: &HashSet<Shortcut>,
    save: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let before = active.clone();
    let change = (|| {
        for key in wanted.difference(&before) {
            registrar.register(*key)?;
            active.insert(*key);
        }
        for key in before.difference(wanted) {
            registrar.unregister(*key)?;
            active.remove(key);
        }
        save()
    })();
    if let Err(mut error) = change {
        for key in active.clone().difference(&before) {
            match registrar.unregister(*key) {
                Ok(()) => {
                    active.remove(key);
                }
                Err(e) => {
                    error.push_str(&format!(" Cleanup failed: {e}. Quit and reopen Slotshot."));
                }
            }
        }
        for key in before.difference(&active.clone()) {
            match registrar.register(*key) {
                Ok(()) => {
                    active.insert(*key);
                }
                Err(e) => {
                    error.push_str(&format!(" Restoring the previous shortcut failed: {e}."));
                }
            }
        }
        return Err(error);
    }
    Ok(())
}
fn registration_errors(runtime: &mut Runtime) {
    runtime.view.errors = runtime.view.config.keys.each_ref().map(|key| {
        key.as_ref()
            .and_then(|key| parse(key).ok())
            .filter(|key| !runtime.registered.contains(key))
            .map(|_| {
                "Unavailable. Choose a different shortcut or retry after the other app releases it."
                    .into()
            })
    });
}
pub fn setup(app: &AppHandle) {
    let path = app
        .path()
        .app_config_dir()
        .map(|p| p.join("shortcuts.json"));
    let state = app.state::<ShortcutStateData>();
    let mut runtime = state.0.lock().unwrap();
    match path.map_err(|e| e.to_string()).and_then(|p| load(&p)) {
        Ok(config) => runtime.view.config = config,
        Err(error) => {
            // Preserve unreadable preferences; do not silently overwrite them on startup.
            runtime.view.config.keys = [None, None, None];
            runtime.view.errors = std::array::from_fn(|_| Some(error.clone()));
            return;
        }
    }
    register_initial(app, &mut runtime);
}
fn register_initial(registrar: &impl Registrar, runtime: &mut Runtime) {
    for (index, key) in runtime.view.config.keys.iter().enumerate() {
        if let Some(key) = key {
            let key = parse(key).expect("validated preferences");
            match registrar.register(key) {
                Ok(()) => {
                    runtime.registered.insert(key);
                }
                Err(error) => runtime.view.errors[index] = Some(error),
            }
        }
    }
}
fn pressed_once(held: &mut HashSet<u32>, id: u32, state: ShortcutState) -> bool {
    match state {
        ShortcutState::Released => {
            held.remove(&id);
            false
        }
        ShortcutState::Pressed => held.insert(id),
    }
}

pub fn handle(app: &AppHandle, key: &Shortcut, event: tauri_plugin_global_shortcut::ShortcutEvent) {
    let timing = crate::timing::Timing::new("hotkey_received");
    let state = app.state::<ShortcutStateData>();
    // Registration can dispatch to the native thread. Never block that thread
    // waiting on a preference transaction that is itself waiting on registration.
    let Ok(mut runtime) = state.0.try_lock() else {
        return;
    };
    if !runtime.registered.contains(key) || !pressed_once(&mut runtime.held, key.id(), event.state)
    {
        return;
    }
    if runtime.recording {
        use tauri::Emitter;
        let _ = app.emit_to("main", "shortcut-recorded", key.to_string());
        return;
    }
    let action = runtime
        .view
        .config
        .keys
        .iter()
        .position(|value| value.as_ref().and_then(|s| parse(s).ok()).as_ref() == Some(key));
    drop(runtime);
    if let Some(index) = action {
        crate::desktop::dispatch_timed(app, ACTIONS[index], timing);
    }
}
pub fn view(app: &AppHandle) -> View {
    app.state::<ShortcutStateData>()
        .0
        .lock()
        .unwrap()
        .view
        .clone()
}
#[tauri::command]
pub async fn shortcut_view(app: AppHandle, window: WebviewWindow) -> Result<View, String> {
    crate::slots::main_only(&window)?;
    tauri::async_runtime::spawn_blocking(move || view(&app))
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn configure_shortcuts(
    app: AppHandle,
    window: WebviewWindow,
    config: Config,
) -> Result<View, String> {
    crate::slots::main_only(&window)?;
    let wanted = validate(&config)?;
    let operation = app.state::<crate::slots::SlotState>().begin()?;
    let worker = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let path = worker
            .path()
            .app_config_dir()
            .map_err(|e| e.to_string())?
            .join("shortcuts.json");
        let state = worker.state::<ShortcutStateData>();
        let mut runtime = state.0.lock().map_err(|e| e.to_string())?;
        let result = transition(&worker, &mut runtime.registered, &wanted, || {
            persist(&path, &config)
        });
        if result.is_ok() {
            runtime.view.config = config;
        }
        runtime.held.clear();
        registration_errors(&mut runtime);
        result.map(|()| runtime.view.clone())
    })
    .await
    .map_err(|e| e.to_string())?;
    crate::desktop::refresh_menu(&app);
    result
}

// Only suppress Slotshot's own three registrations while the focused Settings
// recorder is active. This is not global keyboard monitoring.
pub fn stop_recording(app: &AppHandle) {
    if let Some(state) = app.try_state::<ShortcutStateData>()
        && let Ok(mut runtime) = state.0.try_lock()
    {
        runtime.recording = false;
    }
}
#[tauri::command]
pub fn shortcut_recording(
    app: AppHandle,
    window: WebviewWindow,
    recording: bool,
) -> Result<(), String> {
    crate::slots::main_only(&window)?;
    if recording && (!window.is_focused().unwrap_or(false) || crate::capture::active(&app)) {
        return Err("Focus Settings before recording a shortcut.".into());
    }
    app.state::<ShortcutStateData>()
        .0
        .try_lock()
        .map_err(|_| "Shortcuts are being updated. Try again.")?
        .recording = recording;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[derive(Default)]
    struct Fake {
        active: RefCell<HashSet<Shortcut>>,
        conflict: Option<Shortcut>,
        unregister_failure: Option<Shortcut>,
    }
    impl Registrar for Fake {
        fn register(&self, key: Shortcut) -> Result<(), String> {
            if self.conflict == Some(key) {
                return Err("Owned by another app".into());
            }
            assert!(self.active.borrow_mut().insert(key));
            Ok(())
        }
        fn unregister(&self, key: Shortcut) -> Result<(), String> {
            if self.unregister_failure == Some(key) {
                return Err("OS unregister failed".into());
            }
            assert!(self.active.borrow_mut().remove(&key));
            Ok(())
        }
    }
    #[test]
    fn repeats_do_not_queue_capture_and_release_allows_the_next_press() {
        let key = parse("Control+Alt+Shift+1").unwrap();
        let mut held = HashSet::new();
        let state = crate::slots::SlotState::default();
        assert!(pressed_once(&mut held, key.id(), ShortcutState::Pressed));
        let capture = state.begin().unwrap();
        for _ in 0..5 {
            assert!(!pressed_once(&mut held, key.id(), ShortcutState::Pressed));
            assert!(state.begin().is_err());
        }
        assert!(!pressed_once(&mut held, key.id(), ShortcutState::Released));
        assert!(pressed_once(&mut held, key.id(), ShortcutState::Pressed));
        assert!(state.begin().is_err());
        drop(capture);
        assert!(!pressed_once(&mut held, key.id(), ShortcutState::Released));
        assert!(pressed_once(&mut held, key.id(), ShortcutState::Pressed));
        assert!(state.begin().is_ok());
    }
    #[test]
    fn defaults_are_valid_independent_and_platform_appropriate() {
        let defaults = Config::default();
        assert_eq!(validate(&defaults).unwrap().len(), 3);
        for (index, key) in defaults.keys.iter().enumerate() {
            assert_eq!(
                parse(key.as_ref().unwrap()).unwrap().key,
                [Code::Digit1, Code::Digit2, Code::Digit3][index]
            );
        }
    }
    #[test]
    fn parsing_rejects_protected_invalid_modifier_only_and_duplicate_assignments() {
        for key in [
            "",
            "Control+Alt",
            "A",
            "Shift+A",
            "Ctrl+Ctrl+K",
            "Ctrl+V",
            "Control+Shift+C",
            "Super+Shift+4",
            "Alt+Control+Delete",
            "Control+Alt+F4",
            "Control+Alt+Space",
            "Control+Alt+K+L",
        ] {
            assert!(parse(key).is_err(), "{key}");
        }
        if !cfg!(target_os = "macos") {
            assert!(parse("Super+Shift+S").is_err());
        }
        let mut config = Config::default();
        config.keys[1] = Some("shift+ctrl+alt+Digit1".into());
        assert!(validate(&config).unwrap_err().contains("different"));
        config.keys[1] = None;
        assert_eq!(validate(&config).unwrap().len(), 2);
    }
    #[test]
    fn persisted_configuration_reloads_and_malformed_data_is_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shortcuts.json");
        assert_eq!(load(&path).unwrap(), Config::default());
        let mut config = Config::default();
        config.keys[0] = Some("Control+Alt+K".into());
        config.keys[1] = None;
        persist(&path, &config).unwrap();
        assert_eq!(load(&path).unwrap(), config);
        std::fs::write(&path, b"broken preferences").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"broken preferences");
    }
    #[test]
    fn conflicts_and_disk_failure_rollback_while_reassignment_and_disable_commit() {
        let old = validate(&Config::default()).unwrap();
        let mut config = Config::default();
        config.keys[0] = Some("Control+Alt+K".into());
        config.keys[1] = None;
        let wanted = validate(&config).unwrap();
        let new_key = parse("Control+Alt+K").unwrap();
        for failure in ["conflict", "disk", "unregister", "none"] {
            let registrar = Fake {
                active: RefCell::new(old.clone()),
                conflict: (failure == "conflict").then_some(new_key),
                unregister_failure: (failure == "unregister")
                    .then(|| parse("Control+Alt+Shift+1").unwrap()),
            };
            let mut active = old.clone();
            let result = transition(&registrar, &mut active, &wanted, || {
                if failure == "disk" {
                    Err("Disk full".into())
                } else {
                    Ok(())
                }
            });
            assert_eq!(result.is_ok(), failure == "none");
            assert_eq!(active, *registrar.active.borrow());
            assert_eq!(
                active,
                if failure == "none" {
                    wanted.clone()
                } else {
                    old.clone()
                }
            );
        }
    }
    #[test]
    fn swapping_existing_assignments_needs_no_os_reregistration() {
        let config = Config::default();
        let before = validate(&config).unwrap();
        let mut swapped = config;
        swapped.keys.swap(0, 2);
        let wanted = validate(&swapped).unwrap();
        let fake = Fake {
            active: RefCell::new(before.clone()),
            ..Default::default()
        };
        transition(&fake, &mut before.clone(), &wanted, || Ok(())).unwrap();
        assert_eq!(*fake.active.borrow(), wanted);
    }
    #[test]
    fn one_unavailable_default_does_not_block_the_other_actions() {
        let registrar = Fake {
            conflict: Some(parse("Control+Alt+Shift+1").unwrap()),
            ..Default::default()
        };
        let state = ShortcutStateData::default();
        let mut runtime = state.0.lock().unwrap();
        register_initial(&registrar, &mut runtime);
        assert_eq!(registrar.active.borrow().len(), 2);
        assert!(runtime.view.errors[0].is_some());
        assert!(runtime.view.errors[1..].iter().all(Option::is_none));
        assert_eq!(runtime.registered, *registrar.active.borrow());
    }
}
