# Task 002: native region capture

This document records Task 002 and its still-present implementation, including
legacy Copy Region and native clipboard writes. Task 003A changes documentation
only. The approved target replaces Copy Region with Store in Slot and Save to
Device, persistent local PNG slots, and native file drag-out; none is implemented
yet. See [product requirements](PRODUCT.md), [architecture](ARCHITECTURE.md), and
[Template 1 design](DESIGN.md). The clipboard dependencies, behavior, and manual
tests below describe the legacy capture build, not a final product requirement.

## Decision and dependencies

Use [XCap 0.9.8](https://github.com/nashaofu/xcap) for one-shot display snapshots and
[arboard 3.7](https://docs.rs/arboard/3.7.0/arboard/) for actual image clipboard writes.
Their maintained macOS and Windows backends avoid maintaining two independent
capture and clipboard integrations. We use XCap's `capture_image`, not recording.
The published macOS backend uses Core Graphics window-list capture. That Apple API
is deprecated in favor of ScreenCaptureKit; future macOS compatibility needs ongoing
validation. A direct ScreenCaptureKit/Windows Graphics Capture implementation would
require more platform-specific lifecycle code. System screenshot executables offer
inconsistent selection and cancellation across platforms. No Tauri plugin is needed.

Other direct dependencies:

- `image` 0.25, defaults disabled and PNG enabled: crop, encode, and decode. Shared
  with XCap/arboard; those crates enable some additional formats internally.
- `serde` 1 with derive: typed payloads/results, already in Tauri's dependency tree.
- `tokio` 1, time feature only: bounded asynchronous overlay initialization deadline.
  It reuses Tauri's existing runtime and adds no separate runtime or worker service.
- macOS-only `core-graphics` 0.25: capture permission preflight/request wrappers.
- macOS-only `objc2-app-kit` 0.3.2, `NSWindow`: public window-level API to cover the
  menu bar, called on the main thread. Both macOS crates already occur in the native
  dependency tree. No private transparency API is enabled.
- `@tauri-apps/api` 2: official IPC and native-runtime detection.

Lockfiles intentionally include the new dependencies, including XCap's dependencies
for unsupported Linux targets. Runtime support remains macOS and Windows. macOS
10.15+ permission APIs are used; this API floor is not a tested minimum OS promise.

## Behavior and limitations

Click **Copy Region** or **Save Region**, then drag on any display in any direction.
A white border and outside dimming mark the rectangle. Release finishes. Escape or
**Cancel** cancels without modifying the clipboard or creating a file. An empty click
leaves selection active with a retry message. Copy writes native image content. Save
exports `Slotshot-<Unix milliseconds>-<suffix>.png` to the resolved Pictures folder
without overwriting files. The launcher returns with dimensions, path, cancellation,
or an error. Permission denial, capture failure, busy clipboard, invalid coordinates,
PNG encoding, Pictures lookup, directory creation, and write failures are reported.

All monitors are snapshotted before any overlay exists, so no border/dimming/message
is captured. Selection uses a frozen view; subsequent desktop changes are not captured.
Regions are clamped to the display where the drag starts. Cross-display rectangles
and stitching are deferred. Each display uses its own actual snapshot dimensions,
including Retina/HiDPI. Reconfigure or reconnect displays outside an active capture;
retry after changes. The total accepted area is 64 megapixels across at most 16
screens; temporary PNGs and webviews add overhead beyond RGBA memory.

Mirroring, fullscreen Spaces, virtual desktops, HDR/color profiles, protected video,
remote desktops, rotated displays, and clipboard receiving apps need native testing.
Protected content may be black or unavailable. The backend does not request cursor
inclusion. Successful export alone does not prove color fidelity or cursor behavior
for every display. No screenshots are cached, uploaded, or logged. Save exports
persist intentionally; clipboard images belong to the OS after copy.

## Validation record

Automated checks and real OS workflows are distinct. Unit tests use synthetic
coordinates/images and require no screen permission. Real capture on macOS requires
Screen Recording permission; it cannot be verified by unit tests or compilation.
Local validation on macOS 26.6.2:

| Validation                                               | Result                                              |
| -------------------------------------------------------- | --------------------------------------------------- |
| `npm ci`, `npm run check`, `npm run build`               | Passed                                              |
| Rust formatting and Clippy (`--locked`, warnings denied) | Passed                                              |
| Rust tests (`--locked`)                                  | 7 passed; synthetic data only                       |
| Tauri debug native build (`--no-bundle`, `--locked`)     | Passed                                              |
| Unsigned local `.app` build                              | Passed before final lifecycle fixes; not a release  |
| Launcher and denied-permission UI                        | Visible in user-supplied screenshot                 |
| Real region capture, image paste, Pictures export        | Unverified: Screen Recording permission not granted |
| Multi-monitor/mixed-DPI/Windows runtime workflows        | Unverified                                          |

The [Task 002 pull request](https://github.com/KarabulutUmut/slotshot/pull/1) records
CI results for the final revision. Windows CI compilation does not establish
Windows runtime behavior.

## Exact macOS manual test steps

1. Install the README prerequisites. Run `npm ci` and `npm run tauri -- dev`.
   To test production assets/CSP, run
   `npm run tauri -- build --debug --no-bundle -- --locked`, then
   `./src-tauri/target/debug/slotshot`. For a local unsigned app identity use
   `npm run tauri -- build --debug --bundles app -- --locked` and open
   `src-tauri/target/debug/bundle/macos/Slotshot.app`.
2. Prepare non-private colored shapes/text in another window. Click **Copy Region**.
   Grant Screen Recording in **System Settings → Privacy & Security → Screen &
   System Audio Recording**. The entry may be Slotshot or the terminal/development
   host. Fully quit and reopen the app after changing permission.
3. Drag around the test content and release. Confirm `Copied W × H image` appears.
   In **Preview → File → New from Clipboard** (Command-N), inspect content, sharpness,
   colors, and dimensions. Confirm no white boundary, dimming, instructions, or Cancel
   button is present. This tests actual image paste, not text/path. Try a second
   image-capable app if available.
4. Click **Save Region**, drag right-to-left/bottom-to-top, release, and open the
   reported Pictures PNG in Preview. Inspect the same points. Save repeatedly and
   verify unique filenames and unchanged earlier files.
5. Start capture and press Escape; repeat with **Cancel**. Verify the launcher
   returns, clipboard stays unchanged, and no PNG appears. Click without dragging,
   confirm the empty-selection message, then drag a valid region in that session.
6. On Retina, capture a known small rectangle and the screen edge-to-edge. In
   Preview's inspector verify physical pixel dimensions (a 100-point region in a
   2× snapshot is 200 pixels). Check right/bottom edges for clipping.
7. With an external display, repeat on each monitor, including left/above primary
   and mixed scaling where available. Verify snapshot alignment and crop content.
   Dragging past an edge stays on its originating display. Repeat after changing
   layout between captures; check Escape after clicking each display.
8. Revoke Screen Recording, quit/reopen, and retry. Verify an actionable error and
   usable launcher; restore permission and retry. In a disposable test account,
   make Pictures unwritable, verify an error and no partial PNG, then restore access.
9. Repeat capture/cancel several times and try closing an overlay with the window
   close shortcut. Confirm the launcher recovers and buttons re-enable. Quit when
   done. No slots, hotkeys, settings, or editing tools are part of Task 002.
