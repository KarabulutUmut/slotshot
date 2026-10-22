# Architecture

## Status and boundaries

Task 002's native region capture is implemented. Task 003A aligns documentation
only; all sections marked planned describe the approved target for subsequent
implementation, not existing capabilities. Keep one small React/TypeScript
frontend with plain CSS/local state and a Rust/Tauri 2 backend. No server,
accounts, database, analytics, application network client, or cloud storage.

## Current application (Task 002)

- `src/`: `App.tsx` is the Copy Region/Save Region test launcher;
  `RegionOverlay.tsx` is the temporary drag surface; `capture.ts` defines typed
  commands/results. Browser preview disables native buttons.
- `src-tauri/src/capture.rs`: on-demand snapshots, macOS permission checks,
  temporary selection windows, capture lifecycle, and arboard native image
  clipboard writes. The legacy clipboard implementation has not been removed.
- `region.rs`: pure, tested normalized-coordinate validation/pixel conversion.
- `export.rs`: PNG encoding and collision-safe exports to Pictures; `create_new`
  avoids overwrites, and failed write/sync attempts remove incomplete exports.
- Screenshots stay in capture-session memory except for explicit Pictures exports
  and the OS clipboard. No slot persistence, settings, thumbnails, native file
  drag-out, or final picker is implemented. Node/Vite are development tools only.

## Existing region capture boundary

Only the main window can invoke `capture_region(copy | save)`. It resolves with
cancelled/copied/saved metadata or an actionable error. Rust serializes captures
with a busy guard. Capture, PNG work, and clipboard/file operations use Tauri's
blocking pool during explicit capture requests. This destination contract is
legacy and must change for Store in Slot / Save to Device.

Rust snapshots all displays before overlays exist. Each display gets one opaque
borderless webview showing its frozen PNG. `region_snapshot` returns binary bytes
for the caller's own active display; JavaScript uses a temporary Blob URL, never
base64 or persistent browser storage. `region_ready` reports image load; overlays
appear only once every image is ready. A 15-second initialization deadline recovers
from renderer/bootstrap failure; user selection has no timeout. `finish_region`
receives normalized endpoints; `cancel_region` handles Escape/Cancel/display errors.
Closing an overlay cancels capture. Rust validates coordinates and crops the
original snapshot; the frontend cannot supply arbitrary output paths.

The current ACL grants the main capture command and four overlay commands only,
scoped to their windows; there are no general clipboard, filesystem, shell,
window-control, or network plugin grants. `build.rs` generates the command ACL
manifest; generated permission files are ignored. CSP permits local Blob images.

macOS uses desktop points and platform-gated AppKit window levels above the menu
bar. Windows uses physical positions/sizes, including negative origins. Moving,
then sizing the Windows window on the main thread avoids DPI transition errors;
final bounds are verified. Selection fractions map to actual snapshot dimensions,
independent of browser `devicePixelRatio`. No cross-monitor stitching is implemented.
See [Task 002 decisions and validation](CAPTURE.md).

One capture session accepts at most 16 displays and 64 megapixels total (256 MiB
RGBA before encoding; not a peak-memory claim). Snapshots are PNG bytes in Rust
and decoded images in temporary webviews. Cropping decodes the selected display;
windows/snapshots are released on success, cancellation, and error. Copy uses
arboard's image API without temporary files; OS clipboard content can outlive the
session. These are current limits and behavior, not a future retention policy.

## Persistent slot storage (planned)

Rust owns slot lifecycle, paths, PNG encoding, and filesystem operations. Resolve
an application-owned root using Tauri's Rust `app_local_data_dir` path API
(macOS Application Support; Windows LocalAppData), not the working directory or
export directory. Use Rust path APIs throughout; keep platform-specific code gated.
Never store screenshot bytes in localStorage or a database.

Maintain a small versioned metadata file for slot count/assignments and managed
PNG/thumbnail paths. Count is 1–10, default 5. Each occupied numbered slot references
one full-resolution PNG; thumbnails are bounded, regenerable derivatives. Load
metadata across restarts without decoding every full-resolution image. Validate
slot IDs/counts in Rust as well as the frontend; resolve file paths from trusted
metadata under the managed root, not frontend-supplied arbitrary paths. Validate
persisted paths too and avoid following paths/symlinks outside managed storage.

Serialize mutations. Stage new files in the same filesystem, flush/sync as
appropriate, and atomically commit files/metadata using platform-correct replacement.
Do not delete a good old image before its replacement is durable. After commit,
permanently remove the old PNG and obsolete thumbnail. Clear removes both files
and empties the assignment. Increasing count creates empty slots; decreasing it
removes every PNG and thumbnail above the new count. Before destructive reduction,
require explicit confirmation if affected slots are occupied, and revalidate the
current affected set before committing. Cancelling leaves files/count unchanged.

Handle interrupted writes and deletion failures with startup reconciliation of
managed files against committed metadata, removing obsolete images/thumbnails
and abandoned staging files. Report failed cleanup and retry it; do not silently
retain screenshots in hidden/removed slots or report deletion complete while
files remain. Restrict cleanup to Slotshot-owned files; never sweep export folders.
Document/test crash recovery and Windows open-file/rename failures during
implementation. Do not promise secure erasure beyond normal filesystem deletion.

## Independent exports and settings (planned)

Save to Device writes a new independent PNG to the configured export directory.
Choose that directory during initial setup, offering OS Pictures as the sensible
default; allow changes in Settings. Persist settings in a small versioned local
configuration file using the Rust app-config path API and safe replacement.
Validate writable destinations and reject internal slot-storage locations for
exports so cleanup cannot remove exports. No database is needed.

Save Selected exports each selected occupied slot as a separate full-resolution
PNG with collision-safe names. Export failure must preserve slots and existing
exports, remove only incomplete files from that attempt, and identify successful
and failed files. Replacing/clearing/reducing slots must never delete independent
exports. Changing the export directory does not move or delete earlier exports.
Adapt the current Pictures-only save path/errors rather than claiming this
configuration already exists.

## Native file drag-out (planned)

Use [CrabNebula drag-rs and its Tauri 2 drag plugin](https://github.com/crabnebula-dev/drag-rs)
as the intended technical direction, subject to compatibility testing. Upstream
documents native file drags and a Tauri 2 plugin; Slotshot has not installed or
validated them. Choose minimal dependencies/permissions only during implementation.
Keep OS operations in Rust; grant only the necessary window-scoped capabilities.

The frontend sends selected slot IDs. Rust validates occupied slots and resolves
their PNG paths, then starts **one** native drag offering all selected files.
Use source-preserving copy semantics, never move files out of internal storage.
Keep sources available throughout the native drag; coordinate/defer replacement,
Clear, and count reduction while affected files are in use. Validate cancellation,
completion, destination-requested move behavior, and Windows file locks; if the
integration cannot preserve sources, it does not meet the requirement.

Do not substitute HTML-only dragging, base64 payloads, clipboard writes, simulated
pasting, clipboard monitoring/restoration, or interception of standard Cmd/Ctrl+V.
The final product has no system clipboard Copy/Paste action or multi-image
clipboard feature. Remove the legacy arboard path/dependency when implementation
no longer uses it; do not change dependencies in Task 003A.

## Picker, resources, and platform validation (planned)

Follow [Template 1](DESIGN.md), including Save/Clear, no thumbnail context menus,
full resizing, and primary native drag-out. Shift selects ranges; Cmd (macOS) /
Ctrl (Windows) toggles individuals. Dragging a selected thumbnail keeps the group.
Keep UI selection state local and file operations in Rust; use thumbnails and
bounded decoded-image memory instead of keeping every full PNG decoded.

Preserve event-driven idle behavior. Currently there are no permanent capture
windows, polling, hotkey listeners, or capture workers; capture uses a single
150 ms settling delay after hiding the launcher and sequential snapshots, not
synchronized video frames. Measure idle CPU, startup, capture/drag latency, and
peak memory before making resource claims. Validate transparency and native
window differences without changing the approved visual direction.

| Area        | Required macOS and Windows runtime validation                                                                                                                                  |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Capture     | Permissions, cancellation, frozen image fidelity, mixed-DPI/multi-monitor positioning.                                                                                         |
| Storage     | Restart persistence, full PNG fidelity, replacement/Clear cleanup, occupied-count confirmation/cancellation, interrupted writes, recovery, permission and disk errors.         |
| Export      | Setup/Settings directory selection, collision-safe single/batch exports, partial failure reporting, exported files surviving slot deletion/replacement/reduction.              |
| Native drag | Single/multiple files in one drag; ChatGPT, Finder/Explorer, and common file-accepting apps; accepted/rejected/cancelled drops; source preservation and mutation coordination. |
| Picker      | Shift/Cmd/Ctrl selection, dragging a selected group, empty slots, resizing, keyboard access, visual consistency, idle overhead.                                                |

Record actual app/OS versions and observed single/multiple-drop outcomes. Do not
promise every receiving application accepts multiple image files. These workflows
are unverified; compilation is not runtime compatibility evidence. Existing Rust
tests cover coordinates, PNG pixels, export collisions/errors, and overlay
initialization recovery; new behavior needs focused lifecycle/selection/error tests.

## Task 003B handoff

- Replace Copy Region, `copy`/`copied` command/result handling, arboard writes,
  clipboard-specific errors, and the launcher's “visual screenshot clipboard”
  wording with the approved capture destinations and file-based picker.
- The React launcher and Rust/TypeScript capture implementation intentionally
  remain unchanged in 003A; only the static HTML product description is aligned.
- Implement slot persistence/cleanup/recovery, setup/settings and independent
  exports, native file drag, and Template 1 selection/interactions. Reuse the
  existing capture engine; no slots or file drag are implemented by this document.
- Validate the platform/receiving-app cases above and record limitations. Keep
  Task 002's historical clipboard validation clearly labeled rather than rewriting
  prior commits, PRs, or releases.
