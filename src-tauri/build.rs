fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "capture_region",
            "region_snapshot",
            "region_ready",
            "finish_region",
            "cancel_region",
        ]),
    ))
    .expect("failed to build Slotshot")
}
