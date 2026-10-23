//! Picker geometry only. Screenshot storage and capture never use this file.
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{LogicalSize, Manager, PhysicalPosition};

#[derive(Default)]
pub struct WindowState(Mutex<Option<Geometry>>);

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Geometry {
    x: i32,
    y: i32,
    width: f64,
    height: f64,
}

// Positions are physical desktop coordinates (including negative origins); sizes
// are logical so moving between Retina and standard displays keeps UI proportions.
fn bounded(saved: Geometry, area: (i32, i32, u32, u32), scale: f64) -> Option<Geometry> {
    if !saved.width.is_finite() || !saved.height.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let (x, y, width, height) = area;
    // Leave room for native borders/title bars, including Windows decorations.
    let width = (f64::from(width) / scale - 16.0).max(400.0);
    let height = (f64::from(height) / scale - 40.0).max(200.0);
    let width = saved.width.clamp(400.0, width.min(1280.0));
    let height = saved.height.clamp(200.0, height.min(720.0));
    let right = f64::from(x) + f64::from(area.2) - width * scale - 16.0 * scale;
    let bottom = f64::from(y) + f64::from(area.3) - height * scale - 40.0 * scale;
    Some(Geometry {
        x: f64::from(saved.x).clamp(f64::from(x), right.max(f64::from(x))) as i32,
        y: f64::from(saved.y).clamp(f64::from(y), bottom.max(f64::from(y))) as i32,
        width,
        height,
    })
}

pub fn restore(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(path) = app.path().app_config_dir() else {
        return;
    };
    let saved = std::fs::read(path.join("picker-window.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Geometry>(&bytes).ok());
    let Ok(monitors) = window.available_monitors() else {
        return;
    };
    let monitor = saved.and_then(|saved| {
        monitors.iter().find(|monitor| {
            let area = monitor.work_area();
            i64::from(saved.x) >= i64::from(area.position.x)
                && i64::from(saved.y) >= i64::from(area.position.y)
                && i64::from(saved.x) < i64::from(area.position.x) + i64::from(area.size.width)
                && i64::from(saved.y) < i64::from(area.position.y) + i64::from(area.size.height)
        })
    });
    let primary = window.primary_monitor().ok().flatten();
    let Some(monitor) = monitor.or(primary.as_ref()).or(monitors.first()) else {
        return;
    };
    let area = monitor.work_area();
    let initial = saved.unwrap_or(Geometry {
        x: area.position.x
            + ((f64::from(area.size.width) - 660.0 * monitor.scale_factor()) / 2.0) as i32,
        y: area.position.y
            + ((f64::from(area.size.height) - 252.0 * monitor.scale_factor()) / 2.0) as i32,
        width: 660.0,
        height: 252.0,
    });
    if let Some(geometry) = bounded(
        initial,
        (
            area.position.x,
            area.position.y,
            area.size.width,
            area.size.height,
        ),
        monitor.scale_factor(),
    ) {
        let _ = window.set_size(LogicalSize::new(geometry.width, geometry.height));
        let _ = window.set_position(PhysicalPosition::new(geometry.x, geometry.y));
    }
}

// Resize/move events only update memory; no disk writes or native size changes.
pub fn remember(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_maximized().unwrap_or(false)
        || window.is_minimized().unwrap_or(false)
        || window.is_fullscreen().unwrap_or(false)
    {
        return;
    }
    let (Ok(position), Ok(size), Ok(scale)) = (
        window.outer_position(),
        window.inner_size(),
        window.scale_factor(),
    ) else {
        return;
    };
    let size = size.to_logical::<f64>(scale);
    let geometry = Geometry {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };
    if let Ok(mut saved) = app.state::<WindowState>().0.lock() {
        *saved = Some(geometry);
    }
}

// The window can already be destroyed by Exit. Keep the last normal geometry
// in memory so Cmd+Q and NSRunningApplication termination also preserve it.
pub fn save(app: &tauri::AppHandle) {
    remember(app);
    let state = app.state::<WindowState>();
    let Ok(saved) = state.0.lock() else { return };
    let Some(geometry) = *saved else { return };
    let Ok(directory) = app.path().app_config_dir() else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec(&geometry) else {
        return;
    };
    let write = || -> std::io::Result<()> {
        use std::io::Write;
        std::fs::create_dir_all(&directory)?;
        let mut file = tempfile::NamedTempFile::new_in(&directory)?;
        file.write_all(&bytes)?;
        file.persist(directory.join("picker-window.json"))?;
        Ok(())
    };
    if let Err(error) = write() {
        eprintln!("Could not remember picker geometry: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removed_display_and_oversized_preferences_stay_in_work_area() {
        let saved = Geometry {
            x: 9000,
            y: -8000,
            width: 3000.0,
            height: 2000.0,
        };
        let restored = bounded(saved, (0, 50, 1440, 850), 2.0).unwrap();
        assert_eq!(restored.width, 704.0);
        assert_eq!(restored.height, 385.0);
        assert_eq!(restored.x, 0);
        assert_eq!(restored.y, 50);
    }
    #[test]
    fn negative_display_origin_and_deliberate_resize_survive() {
        let saved = Geometry {
            x: -1500,
            y: 100,
            width: 840.0,
            height: 380.0,
        };
        let restored = bounded(saved, (-1920, 0, 1920, 1080), 1.0).unwrap();
        assert_eq!(restored.x, saved.x);
        assert_eq!(restored.y, saved.y);
        assert_eq!(restored.width, saved.width);
        assert_eq!(restored.height, saved.height);
    }
    #[test]
    fn invalid_or_tiny_sizes_are_rejected_or_clamped() {
        let saved = Geometry {
            x: 0,
            y: 0,
            width: 1.0,
            height: 1.0,
        };
        let restored = bounded(saved, (0, 0, 1920, 1080), 1.0).unwrap();
        assert_eq!((restored.width, restored.height), (400.0, 200.0));
        assert!(
            bounded(
                Geometry {
                    width: f64::NAN,
                    ..saved
                },
                (0, 0, 1920, 1080),
                1.0
            )
            .is_none()
        );
    }
}
