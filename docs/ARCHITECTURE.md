# Architecture

One React/TypeScript frontend with plain CSS and local UI state; one Rust/Tauri 2
backend owns capture, trusted paths, persistence, exports, native dialogs, and drag.
No server, database, application network client, accounts, analytics, or cloud.

## Code and command boundaries

- `src/App.tsx`: primary picker, setup/settings, confirmations, and action results.
  `selection.ts` holds tested click/range/toggle rules. React retains metadata and
  bounded thumbnail Blob URLs, never every full-resolution PNG. URLs are revoked.
- `capture.rs` / `RegionOverlay.tsx`: on-demand frozen snapshots and temporary
  selection webviews; `region.rs` validates normalized coordinates/pixel conversion.
- `storage.rs`: versioned metadata, immutable PNG generations, thumbnails,
  settings, cleanup/recovery, collision-safe exports, and input/path validation.
- `slots.rs`: window-restricted commands, blocking filesystem workers, native
  directory dialog, native drag, and shared operation guard.
- `export.rs`: PNG encoding and `create_new` exports, including partial-file cleanup.
- `window.rs`: event-driven, in-memory picker geometry; bounded restore from a
  separate preference file, written on close/hide and normal quit. No screenshot paths
  or operations change.

The main window can request capture, slot views/thumbnails, Clear/count changes,
exports, configuration, and drag. Overlay windows receive only their own snapshot,
readiness, finish, and cancel commands. Rust repeats window/slot/revision checks.
`build.rs` generates command ACLs; there are no general filesystem, clipboard,
shell, or drag-plugin grants. The frontend supplies slot IDs, never source paths.
Only the native folder dialog can choose a custom export path. CSP permits local
Blob images, with no arbitrary remote content. Standard paste shortcuts are untouched.

## Storage and recovery

Tauri’s `app_local_data_dir()/slots-v1` holds `manifest.json`, `store.lock`, and
`image-<generation>.png` / `thumb-<generation>.png`. On macOS this is
`~/Library/Application Support/io.github.karabulutumut.slotshot/slots-v1`; Windows
uses the OS LocalAppData location resolved by Tauri. `app_config_dir()/settings.json`
stores the versioned export setting (the app-data parent on macOS; the OS app-config
location on Windows). No paths depend on the checkout. Default count is five.

A Rust mutex serializes filesystem access. An OS file lock rejects parallel app
instances using the same storage. An operation lease excludes conflicting capture,
settings, export, mutation, and drag requests; a native drag retains it until its
OS completion/cancellation callback. Dropping a failed operation releases the lease.
Source generations therefore remain stable while offered to another application.

Rust validates 1–10 counts, occupied unique IDs, revisions, metadata versions,
bounded dimensions, and generation basenames. Managed metadata/images must be
regular files, not symlinks; source canonical paths must stay in the managed root.
Invalid metadata stops loading without sweeping files. Missing/unreadable sources
show a warning and can be restored or explicitly cleared. Export directories are
canonicalized and rejected inside managed image/config storage, including symlink
aliases; export destination checks repeat on use.

Replacement writes a new full PNG and a maximum 320×240 thumbnail to unique
immutable generations. `tempfile` stages on the same filesystem, syncs content,
and atomically replaces metadata/files (rename on Unix, MoveFileExW on Windows).
The new files precede metadata; obsolete generations are removed only after the
metadata commit. Unix directory entries are synced. A failed pre-commit replacement
keeps the old assignment; a post-commit cleanup error reports that the change was
committed and deletion needs retry. This is crash recovery, not a guarantee against
all hardware/power-loss/filesystem failures.

Clear/count reduction first commits the new assignments, then removes unreferenced
PNGs/thumbnails. Revision checks reject stale confirmations; reductions with occupied
slots additionally require explicit confirmation. Increasing count adds empty slots.
Startup/Reload reconciles committed metadata against owned generation names and
`.stage-` files, permanently deleting abandoned/obsolete files. Unknown files are
left alone, exports are never swept, and cleanup failures remain visible/retryable.
A missing thumbnail regenerates on demand from its original PNG. Normal deletion
is not secure erasure. Windows open-file/rename failures require runtime testing;
they surface as errors, with obsolete generations retried on Reload/restart.

## Independent exports

First-run setup offers Pictures or a native folder picker; Settings changes the
folder without moving prior exports. A create/sync/delete probe checks write access.
Save to Device encodes only its selected region. Save Selected reads one source at
a time, creates independent full-resolution PNGs, and reports successful paths and
per-slot failures. `create_new` prevents overwrites; failed write/sync removes only
that attempt’s incomplete export. Internal cleanup never touches export folders.

## Capture engine and limits

XCap 0.9.8 provides one-shot snapshots; no recording service. Its macOS backend
uses Core Graphics window-list capture, deprecated in favor of ScreenCaptureKit;
future OS compatibility needs validation. macOS-only Core Graphics permission
checks and AppKit window levels are platform-gated. `image` handles PNG/cropping;
`serde`/`serde_json` provide typed metadata/IPC; Tokio’s time feature bounds overlay
initialization on Tauri’s existing runtime.

Capture hides the picker, waits once for 150 ms, then snapshots every display
sequentially before creating any overlay. A display-specific binary IPC response
becomes a temporary Blob URL. Overlays appear after all images report readiness;
a 15-second initialization deadline recovers bootstrap failures. Selection has no
timeout; Escape, Cancel, or closing an overlay releases the session. Snapshots
stay in memory until success/cancellation/error; only the final destination writes.

One session accepts at most 16 displays and 64 megapixels total (256 MiB RGBA before
encoding, not a peak-memory claim). Rust crops original pixels using normalized
selection fractions, independent of webview devicePixelRatio. macOS uses desktop
points; Windows uses physical bounds, including negative origins. Windows movement
and final sizing run together on the main thread and verify resulting bounds to
avoid DPI transition stretching. Regions stay on the originating display; no stitching.

Reconfigure displays between captures. Mirroring, fullscreen Spaces, virtual desktops,
HDR/profiles, protected content, remote desktops, rotated/mixed-DPI displays, and
cursor inclusion require runtime checks. Protected content may be black/unavailable;
the backend does not request cursor inclusion. See README for permission recovery
and exact native tests. Historical capture details remain in Git history and PR #1.

## Native drag and window effects

[CrabNebula drag-rs](https://github.com/crabnebula-dev/drag-rs), pinned to 2.1.1,
is called directly from Rust on the main thread. This avoids the plugin’s generic
frontend arbitrary-path API and an extra npm dependency. One `DragItem::Files`
contains validated occupied-slot PNG paths; a bounded thumbnail supplies the icon.
`DragMode::Copy` maps to NSDragOperationCopy / DROPEFFECT_COPY. Only the OS file
transfer mechanism is used; there are no system clipboard Copy/Paste operations.
Cancellation/rejection/error is reported and sources remain internal.

Native drag initialization/completion differs: macOS completes asynchronously;
Windows DoDragDrop runs a modal OS loop. The guard survives both. If an OS callback
never arrives, operations remain blocked to protect sources; quit/reopen to recover.
An accepted OS drop does not establish that a receiving app imported every file.
Test one/multiple PNGs with Finder/Explorer, ChatGPT, and common file-accepting apps.
Multiple-file acceptance is destination-dependent. No destination matrix is claimed
until those native tests have been observed.

`rfd` 0.17.2 supplies only native folder dialogs (Linux features disabled); no
frontend filesystem/dialog permission is granted. `tempfile` supplies cross-platform
atomic replacement and private staging files, already present transitively. No new
npm runtime package is needed; arboard and its unused clipboard dependencies are
removed.

Task 003B.4 finalizes the user-approved Liquid Glass Regular material. Before native
window creation, the macOS-only block in `lib.rs` overrides the main window's
Tauri `WindowEffectsConfig` in both debug and release: `LiquidGlassRegular`, then
`HudWindow` fallback, active HUD state, radius 12, and RGBA(20,24,32,28) native tint.
Tauri 2.12.5 applies Regular on macOS 26+ and falls back to HUD on older macOS
(the HUD material requires 10.14+). `state` does not affect Liquid Glass;
interactive glass is not enabled. This uses the already-locked transitive
window-vibrancy 0.8.1 integration, without new libraries or effect IPC permissions.
The shared configuration and opaque charcoal/navy CSS tint remain unchanged on
Windows, which requests Acrylic. Only the macOS shell gets the approved lighter
CSS tint; screenshot images remain unfiltered. The macOS theme stays dark,
titlebar stays overlay/hidden-title, and native shadows/resize limits are unchanged.
Native materials respond to Reduce Transparency; CSS additionally uses an opaque
surface when WebKit reports `prefers-reduced-transparency`. No polling is added.
The development selector, synthetic previews, temporary commands and grants, and
separate evaluation document are removed. No screenshot-storage migration occurs.

In locked Tauri 2.12.5, the retained `macos-private-api` flag is a no-op (since
[Tauri 2.12.1](https://v2.tauri.app/release/tauri/v2.12.1/)); Wry 0.57.0 still uses
the private WebKit `drawsBackground` KVC key for transparency. Removing the flag
alone is insufficient for a Mac App Store API audit. Native
[`NSGlassEffectView`](https://developer.apple.com/documentation/appkit/nsglasseffectview)
and [`NSVisualEffectView`](https://developer.apple.com/documentation/appkit/nsvisualeffectview)
are public AppKit APIs; choosing them does not remove that WebView integration risk.
Mac App Store distribution needs a separate audit and potentially a different
transparency implementation. Local Apple Development signing is not notarization
or App Store approval. The glass implementation needs no material plugin.

## Shortcuts, tray, and window lifecycle

`shortcuts.rs` owns the official `tauri-plugin-global-shortcut` 2.4.1 registrations,
physical key parsing, conservative protected-key validation, duplicate detection,
pressed/released de-duplication, and a versioned `app_config_dir()/shortcuts.json`.
On macOS its global-hotkey 0.8 backend uses public Carbon shared registration.
Native tests confirmed that macOS can accept an already registered combination,
including a shared key; this cannot establish exclusive availability. Exclusive registration can suppress another app's shared registration and does
not reliably reject that shared assignment. The app retains the official plugin and tells
macOS users to choose an unused combination when another app responds. Windows
reports ordinary registration conflicts through RegisterHotKey; failures enter the
transaction rollback path. macOS supports F1–F20; Windows reserves F12 for its debugger.
No shortcut plugin command is granted to a webview. The main window receives only
restricted preference/view/recording commands. Recording listens inside the focused
Settings webview; only Slotshot's existing registered keys are forwarded to that
recorder. Focus loss/unmount ends recording. There is no keyboard monitor or
Accessibility permission. Rust registration is independent of React rerenders.

A preference transaction registers additions before removing obsolete keys, then
atomically replaces the preference file using tempfile. Any registration, removal,
or persistence failure compensates toward the previous registration set and retains
the previous configuration. Each successful OS call updates the tracked set; rare
compensation failures report affected assignments and recommend quit/reopen rather
than claim success. Startup registers each assignment independently so a single
conflict cannot disable unrelated shortcuts. Corrupt preferences are preserved,
with assignments disabled and an explanation until the user resets/configures them.
Runtime state mutexes never block the main thread while a registration worker is
waiting for that same thread.

`desktop.rs` owns the built-in Tauri tray/menu and native action dispatch. macOS uses
a 22-pixel alpha template crop-corner icon and a left-click native menu; Windows uses
the existing bundled color icon, left-click activation, and a right-click menu.
Shortcut labels update after preference transactions. Open and toggle unminimize
and focus the same webview without a storage reload. Closing prevents native
window destruction and hides the picker; explicit Quit and normal OS quit destroy
overlays and exit. The normal Dock/taskbar and native effects remain intact.

The official `tauri-plugin-single-instance` 2.5.4 initializes first, before storage
opens, and routes subsequent launches to the existing picker. macOS Dock reopen uses
the same guarded open action. The OS file lock remains a separate storage safety
boundary. `--background` sets initial visibility before native window creation;
ordinary launch shows the picker. No autostart dependency or setting is installed.

Native capture acquires the same operation lease synchronously before asynchronous
dispatch, so repeated presses cannot queue captures, conflict with native drag,
change settings, or show the picker over active overlays. Preparation loads current
metadata, checks setup, and validates the first empty slot. Full-slot/setup failures
show the existing picker instructions/setup. The restricted `capture_region` command
still accepts only the main window; overlays retain their previous command scope.
Both native and frontend entry points call `run_capture`, which repeats destination,
replacement confirmation, and revision validation and uses the existing capture
engine. No native action supplies replacement authorization.

Cleanup drops snapshots/session data and destroys every temporary overlay. It
restores prior hidden/visible/minimized state on cancellation/failure/Save; successful Store reveals
the new selection. A Rust-owned sequence-numbered result snapshot plus a targeted
`desktop-state` event lets React subscribe then read current state without losing
native completions during bootstrap. Successful Store and native errors refresh slot metadata and thumbnails once
(errors can follow a committed filesystem change). Hidden Save and open/toggle do
not reread screenshot content. A request sequence rejects stale bootstrap responses.
Main-only event listen/unlisten permissions are added; emit, general filesystem,
global-shortcut plugin, tray, shell, and arbitrary-window grants are not added.

The two new Rust plugins provide OS hotkey registration and second-launch IPC;
Tauri's `tray-icon` feature uses its already-locked tray/menu libraries. Cargo.lock
adds their required transitive packages; npm runtime dependencies and lockfile are
unchanged. No app server, daemon, or additional persistent application process is
introduced. The embedded OS WebKit/WebView2 subprocesses remain normal webview
infrastructure.

Idle operation is event driven: no recurring timer, screenshot loop, clipboard
monitor, or retained inactive capture snapshot. The hidden picker intentionally
keeps its webview, React state, bounded thumbnails, and Rust metadata alive for fast
reopen. Dropping/recreating that webview could reduce memory but would lose local UI
state and require reloads; it is not implemented. RSS measurements must distinguish
the main process from shared OS WebKit helpers and are not peak-memory guarantees.

## Task 004 validation

On macOS 26.6.2, the canonical app was rebuilt through `npm run app:test:mac`
with the existing Apple Development identity, unchanged identifier/designated
requirement, strict signature verification, and a verified canonical executable.
The user confirmed physical Store from another app reveals the full-slot explanation,
the reassigned picker toggle works, and the menu bar icon is visible. Settings
recording/reassignment, duplicate rejection, Escape cancellation, and resetting
defaults were exercised; preferences persisted through quit/reopen and rebuilding.
A real 2712 × 1769 Save to Device export completed without changing any internal
slot or the manifest, and subsequent Escape closed overlays and restored the picker.
Screen Recording access worked without reauthorization. Cmd+Q terminated the PID;
close left it running. A forced second launch revealed the existing picker.

The final official-plugin build passed `npm ci`, frontend check/build (34 tests),
Rust formatting, locked Clippy with warnings denied, 32 Rust tests, and the locked
debug/no-bundle build. Tests cover parsing/defaults, persistence, independent startup
registration failures, transactional reassignment/rollback, repeat presses and the
operation lease, lifecycle decisions, first-empty/full-slot validation, replacement
confirmation/revision checks, export independence, and visibility restoration.
Policy/unit tests do not establish every native workflow.

Two `ps` snapshots of the final Debug main process observed 0.0% reported CPU in
both states, about 102 MiB RSS visible (104480 KiB) and 104 MiB hidden (106096 KiB).
These are approximate spot measurements, not an extended benchmark or a peak claim,
and exclude separate OS WebKit helper processes. Hiding retains the same webview,
React state and bounded thumbnails; it does not promise to reduce RSS. No persistent
capture buffer or polling loop is added.

Hashes of all ten pre-existing PNGs/thumbnails, the manifest and original export
settings remained unchanged. Defaults were restored to 1/2/3 after shortcut tests;
the independent test export was retained. macOS shared-key conflicts are not reliably
reported by the OS, as described above. Native background Store success/selection
with a free slot, hidden Save completion/cancellation, `--background` startup,
explicit tray Quit, long idle measurements, multiple monitors, receiving-app drag,
older macOS, and Windows runtime remain unverified. Existing five occupied slots
were preserved. No Windows runtime claim follows from CI compilation.

## Earlier validation status

Task 003B.4 used only the canonical `~/Applications/Slotshot.app` on macOS 26.6.2,
rebuilt by `npm run app:test:mac` with the existing Apple Development identity,
strict bundle verification, and verified executable path. A real 2698 × 1086 PNG
was captured into empty slot 4 without another permission grant; its full PNG,
thumbnail, and assignment survived normal quit/reopen. Original occupied assignments,
PNGs, thumbnails, and settings were unchanged; only the manifest gained the test
capture. The normal Settings sheet, enabled/disabled actions, native zoom/restore,
and geometry retention were observed. Regular produced a dark, readable surface
with Reduce Transparency on; the original off setting was restored. The user
confirmed enabled and disabled labels were readable over light and dark backgrounds
on the actual desktop, since isolated automation snapshots cannot establish that.

`npm ci`, frontend check/build (24 tests), Rust formatting, locked Clippy with
warnings denied, 21 Rust tests, and the locked debug native build passed. The
removed 22nd Rust test belonged only to the deleted experiment. Storage, capture,
export, native drag, and selection implementations retain their existing behavior
and test coverage. No source image or separately exported PNG was replaced/deleted.
Windows/older-macOS runtime, receiving-app multi-file drag compatibility, display
hot-plug/mixed-DPI behavior, and controlled performance measurements remain
unverified in this task. Compilation alone is not native runtime evidence.
