//! Temporary Task 003B.3 comparison. No persistence or screenshot access.
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Material {
    Default,
    Regular,
    Clear,
    Hud,
}

#[tauri::command]
pub fn material_preview_enabled(window: tauri::WebviewWindow) -> bool {
    window.label() == "main" && cfg!(all(debug_assertions, target_os = "macos"))
}

#[tauri::command]
pub fn preview_material(window: tauri::WebviewWindow, mode: Material) -> Result<(), String> {
    if !material_preview_enabled(window.clone()) {
        return Err("Material comparison is available only in the macOS debug picker.".into());
    }
    #[cfg(all(debug_assertions, target_os = "macos"))]
    {
        use tauri::window::{Color, Effect, EffectState, EffectsBuilder};
        let mut effects = EffectsBuilder::new()
            .radius(12.0)
            .state(EffectState::Active);
        effects = match mode {
            Material::Default | Material::Hud => effects.effect(Effect::HudWindow),
            Material::Regular => effects
                .effect(Effect::LiquidGlassRegular)
                .effect(Effect::HudWindow)
                .color(Color(20, 24, 32, 28)),
            Material::Clear => effects
                .effect(Effect::LiquidGlassClear)
                .effect(Effect::HudWindow)
                .color(Color(20, 24, 32, 18)),
        };
        // Tauri schedules native replacement on the main thread and removes the
        // previous material. Its success means queued, not a rendering guarantee.
        window
            .set_effects(effects.build())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(all(debug_assertions, target_os = "macos")))]
    {
        let _ = mode;
        Err("Material comparison is unavailable.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_accepts_only_the_four_named_modes() {
        for mode in ["default", "regular", "clear", "hud"] {
            assert!(serde_json::from_value::<Material>(serde_json::json!(mode)).is_ok());
        }
        for mode in ["acrylic", "", "Regular", "arbitrary"] {
            assert!(serde_json::from_value::<Material>(serde_json::json!(mode)).is_err());
        }
    }
}
