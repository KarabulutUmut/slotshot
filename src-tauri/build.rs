fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "shortcut_view",
            "shortcut_recording",
            "configure_shortcuts",
            "desktop_view",
            "slot_view",
            "slot_thumbnail",
            "clear_slots",
            "resize_slots",
            "save_slots",
            "configure_exports",
            "drag_slots",
            "capture_region",
            "region_snapshot",
            "region_ready",
            "finish_region",
            "cancel_region",
        ]),
    ))
    .expect("failed to build Slotshot")
}
