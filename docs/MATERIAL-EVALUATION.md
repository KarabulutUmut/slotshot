# Task 003B.3 — native material evaluation

This is a temporary comparison of the existing picker UI against
[Template 1](design/Slotshot%20Mac%20Utility%20Showcase.png). No replacement material
is approved. The user is taking the controlled comparison screenshots manually.

## Installed support

Inspected on macOS 26.6.2, arm64, on 23 October 2026. Lockfiles were unchanged:
Rust Tauri **2.12.5**, tauri-utils **2.10.2**, tauri-runtime-wry **2.12.3**,
Wry **0.57.0**, transitive window-vibrancy **0.8.1**, frontend API **2.12.4**.
No package, crate, plugin, or OS permission was added.

| Mode             | Built-in native integration                      | Comparison treatment                                    | Limits / tradeoffs                                                                                          |
| ---------------- | ------------------------------------------------ | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Existing default | `HudWindow` / `NSVisualEffectView`               | Original roughly 85%-opaque charcoal/navy gradient      | Preserved at every startup; the CSS shell masks much of the material.                                       |
| A — Regular      | `LiquidGlassRegular` / `NSGlassEffectView`, 26+  | Native dark tint RGBA(20,24,32,28); shell alpha 13–19%  | More substantial glass; likely more consistent contrast than Clear. Full desktop comparison remains needed. |
| B — Clear        | `LiquidGlassClear` / `NSGlassEffectView`, 26+    | Native dark tint RGBA(20,24,32,18); shell alpha 25–31%  | Most background-sensitive; empty-slot and secondary-label readability need bright/busy-background checks.   |
| C — HUD          | `HudWindow` / `NSVisualEffectView`, macOS 10.14+ | Same lighter shell as A; native active state, radius 12 | Provisional preference for Template 1's restrained dark utility surface and older-macOS support.            |

All three are real native effects, switched using Rust `WebviewWindow::set_effects`.
Tauri clears the previous view before applying the next effect. A/B request HUD as
the older-macOS fallback. This fallback has been source-inspected, not run on an
older OS. Tauri queues application on the main thread; successful IPC is not proof
of rendering success. Native effect failures can be logged without an IPC error.
The `state` property applies to HUD, not Liquid Glass. `interactive` needs macOS
27+ in the installed stack and is not enabled in this macOS 26 experiment.

The window and webview are transparent; `:root`, body, and React root add no opaque
background. Only the comparison shell tints change; no shell CSS blur stands in
for a native effect. Existing badge/dialog CSS blur remains local to those elements.
There is no screenshot saturation, image opacity, or image-content adjustment.
The native dark theme, overlay titlebar, hidden title, resize limits, and OS shadow
are unchanged. Windows keeps its existing Acrylic/CSS fallback and native titlebar;
the selector is unavailable there. Windows does not implement these macOS materials.

## Manual comparison

1. Run `npm ci`, then `npm run app:test:mac`. Use only
   `~/Applications/Slotshot.app`, identifier `io.github.karabulutumut.slotshot`.
   The canonical workflow signs with the existing Apple Development certificate
   and verifies the installed bundle and executable path.
2. Open **Settings → Development material comparison**. Optional **Synthetic
   thumbnails** shows three generated, non-sensitive images and two empty slots
   without modifying any stored PNG, thumbnail, manifest, or setting. Selection
   still works; capture, exports, deletion, count changes, folder changes, and native
   drag are disabled until the checkbox is off. Compare one selected slot.
3. Set the window size/position once. Automated native window snapshots used
   **794 × 263 logical pixels** (1588 × 526 screenshot pixels); any fixed useful
   size works. Keep the same desktop/background, selection, focus, and geometry
   for A, B, C. Close Settings before each capture. Save images outside Git.
4. Repeat A/B/C over a visibly different background, including a bright or busy
   region. Look for background colors/shapes affecting the shell while thumbnail
   colors stay unchanged. An isolated app-window snapshot can omit the underlying
   desktop and cannot establish this. Capture the actual desktop composition.
5. Compare secondary labels, enabled/disabled controls, blue selection, empty
   frames, native controls, shadows, resizing and keyboard focus. Check all three
   with **System Settings → Accessibility → Display → Reduce transparency** on,
   then restore the original setting. Check native controls without automation's
   purple window-sharing indicator covering the traffic lights.
6. Turn synthetic thumbnails off before testing real capture/cancel, Store in Slot
   into an empty slot, independent export, and drag. Existing occupied slots should
   not be replaced for this comparison. Check sources and separately exported PNGs
   remain intact. Repeat relevant native workflows on Windows when available.
7. Select **Existing default**, or quit/reopen to restore the existing appearance
   and real slots. Neither material nor synthetic-preview state is persisted.

## Observations and validation limits

- All three styles rendered distinct native window appearances and switched
  without restarting or crashing the canonical app. Settings, numbered sample
  thumbnails, and selection remained available. There is no browser-preview evidence
  being substituted for the native run.
- Automated isolated snapshots had identical dimensions and synthetic content, but
  their neutral/composited backgrounds differ from the visible desktop. They are
  unsuitable for judging desktop translucency or declaring a winner. A delayed
  desktop capture of C showed background blur and color response, but the backdrop
  was Codex rather than the intended controlled Preview image. It is not an A/B/C
  comparison. Screenshots remain outside Git; the user opted to capture them.
- HUD became a dark, readable surface when macOS Reduce Transparency was enabled.
  The OS preference was verified off before testing and restored off afterward.
  Regular/Clear reduced-transparency behavior still needs manual verification.
- Main-process snapshots during interaction showed approximately 97–98 MiB RSS;
  cumulative CPU time rose from 3.62 to 4.29 seconds across the observation period.
  These are debug-process observations, exclude WebKit and WindowServer, and are
  not controlled idle, frame-time, GPU, or per-material benchmarks. No performance
  ranking is justified.
- Automated UI interaction was paused when concurrent user changes were detected.
  Material-specific resizing, traffic-light behavior without sharing UI, completed
  capture/export/drag regression, and Windows runtime remain unverified in this task.
  The earlier native workflow results in README are historical, not rerun evidence.
- `npm ci`, `npm run check` (24 tests), `npm run build`, locked Rust formatting,
  Clippy with warnings denied, Rust tests (22), and the locked debug native build
  passed. Canonical build/install, stable signing, and strict signature verification
  passed. A Rust test rejects material names outside the four supported values.

**Provisional recommendation:** start the visual decision with **C**, which keeps
the traditional utility-window character and broad macOS availability while exposing
more of the native surface. Keep **A** as the modern alternative if the real desktop
comparisons show a better edge/refraction treatment and stable contrast. **B** needs
the strongest readability scrutiny. This is a tradeoff recommendation pending the
user's controlled screenshots, not a completed visual selection.

## Public and private API distinction

Apple documents both
[`NSGlassEffectView`](https://developer.apple.com/documentation/appkit/nsglasseffectview)
and [`NSVisualEffectView`](https://developer.apple.com/documentation/appkit/nsvisualeffectview)
as public AppKit APIs. Do not label Liquid Glass itself a private API because an
unrelated third-party plugin does so.

[Tauri's built-in configuration](https://v2.tauri.app/reference/config/#windoweffectsconfig)
supports the two Liquid Glass styles and HUD.
[Since Tauri 2.12.1](https://v2.tauri.app/release/tauri/v2.12.1/), the
`macos-private-api` feature/config flag is no longer needed. In this repository it
is retained, but removing it would not establish App Store eligibility: inspected
Wry 0.57.0 still calls the private WebKit `drawsBackground` KVC key when configuring
transparent webviews. The public native material does not remove that existing
integration risk. A distribution/API audit and potentially a different transparency
implementation are needed before a Mac App Store claim. Local Apple Development
signing is not notarization, distribution signing, or App Store approval.

The temporary selector, synthetic previews, two narrow command grants, and comparison
CSS should be removed after a material is explicitly approved. Until then, keep the
existing material as the default; all comparison state resets on restart.
