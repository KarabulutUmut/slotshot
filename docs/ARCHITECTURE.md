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
  separate preference file, written only on normal close/quit. No screenshot paths
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
removed. HUD vibrancy on macOS and Acrylic on supported Windows versions remain
configured defaults. In locked Tauri 2.12.5, the retained `macos-private-api` flag
is a no-op; Wry 0.57.0 still uses the private WebKit `drawsBackground` KVC key for
transparency. Removing the flag alone is therefore insufficient for a Mac App
Store API audit. Native `NSGlassEffectView` and `NSVisualEffectView` themselves
are public AppKit APIs. OS settings may reduce transparency; window controls/resize
remain native. No hotkey or tray plugin is added.

`material.rs` temporarily exposes two main-window commands for the
[Task 003B.3 material evaluation](MATERIAL-EVALUATION.md). Native effect changes
are rejected outside macOS debug builds. The enum accepts only default/A/B/C;
Tauri's built-in `set_effects` handles main-thread replacement, without a new
library or generic frontend window-effect grant. A/B include HUD fallback for
older macOS. Effects and optional synthetic previews live only in memory; normal
capture windows and all filesystem commands retain their existing boundaries.
The selector should be removed after a visual choice is approved.

## Validation status

Frontend selection tests and Rust lifecycle/path/error tests use synthetic images.
On macOS 26.6.2, the rebuilt unsigned app was launched and the new picker,
translucent surface, native folder-dialog cancellation, Pictures setup, Shift/Cmd
selection, increasing to ten empty slots, returning to five, and restart retention of count/export
settings were observed. Capture returned the actionable Screen Recording permission
error and recovered to an enabled picker. Permission was not granted in this session,
so real captures, occupied-slot GUI workflows, and native file drops remain unverified.
Windows runtime, display edge cases, receiving-app compatibility, and resource
measurements also remain unverified. See README’s manual procedure; CI does not
launch the application.
