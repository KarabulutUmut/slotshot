# Architecture

## Current application

- `src/`: one React/TypeScript frontend, plain CSS and local state. `App.tsx` is
  the launcher; `RegionOverlay.tsx` is the temporary drag surface. `capture.ts`
  defines the typed command boundary. Browser preview disables native buttons.
- `src-tauri/src/capture.rs`: on-demand snapshots, macOS permission checks,
  temporary selection windows, lifecycle, and native image clipboard writes.
- `region.rs`: pure, tested normalized-coordinate validation and pixel conversion.
- `export.rs`: PNG encoding and atomic collision-safe exports to Pictures.
- Production uses the OS webview. Node and Vite are development tools. No server,
  accounts, database, analytics, application network client, or cloud storage.

## Region capture boundary

Only the main window can invoke `capture_region(copy | save)`. It resolves with
cancelled/copied/saved metadata or an actionable error after completion. Rust
serializes captures with a busy guard. Capture, PNG work, and clipboard/file
operations run on Tauri's blocking pool only during an explicit capture request.

Rust snapshots all displays before creating any overlays. Each display gets one
opaque borderless webview displaying its frozen PNG. `region_snapshot` returns
binary bytes for the caller's own active display; JavaScript uses a temporary Blob
URL, never base64 or persistent storage. `region_ready` reports image load. Rust
shows the overlays only when every image is ready. A 15-second initialization
deadline recovers from renderer/bootstrap failures; user selection has no timeout.
The frontend sends normalized endpoints through `finish_region`; `cancel_region`
handles Escape, Cancel, and display errors. Closing an overlay cancels capture.

Rust derives the display from the calling window, validates coordinates, and crops
the original snapshot. The frontend cannot supply an output path or access arbitrary
files. Only the main capture command and four overlay commands are granted, scoped
to their respective windows. There are no general clipboard, filesystem, shell,
window-control, or network plugin grants. `build.rs` generates the command ACL
manifest; generated permission files are ignored. CSP permits local Blob images.

macOS geometry uses desktop points and platform-gated AppKit window levels above
the menu bar. Windows uses physical positions/sizes, including negative origins.
Moving and then sizing the Windows window on the main thread avoids preserving
logical dimensions across a DPI transition; final bounds are verified. Selection
fractions map directly to actual snapshot dimensions, independent of browser
`devicePixelRatio`. Regions stay within one monitor; there is no mixed-DPI stitching.
See [capture decision and limitations](CAPTURE.md).

## Image lifetime and storage

Only one session exists at a time. Up to 16 displays and 64 megapixels of total
image area are accepted (256 MiB of RGBA pixels before encoding; not a peak RAM
claim). Snapshots are PNG bytes in Rust and decoded images in temporary webviews.
Cropping decodes only the selected display. Snapshots and overlay windows are
released on success, cancellation, or errors. There is no screenshot cache. Copy
uses arboard's image API and writes no temporary image file. OS clipboard content
can outlive the session.

Save resolves Pictures through Tauri's Rust path API. Timestamp/suffix filenames
are opened with `create_new` to avoid races and overwrites. Incomplete exports are
removed after write/sync failure. Save is the only feature that persists images.
No settings exist yet; future settings should use a versioned file in the app
config directory and safe replacement, without a database.

## Resource goals and validation

No permanent capture windows, polling, hotkey listeners, or capture workers exist.
A single 150 ms settling delay follows hiding the launcher. Snapshots are sequential,
not synchronized video frames. Idle CPU, startup, capture latency, and peak RAM still
need measurement. Slots, shortcuts, and clipboard monitoring are not implemented.
Multi-image paste remains an open technical question.

TypeScript, ESLint, Prettier, rustfmt, and Clippy check the application. Rust tests
cover reverse drags, image scaling, fractional edges, invalid/empty selections,
PNG pixels, collisions, save errors, and initialization recovery. GitHub Actions
runs locked checks/tests and builds on macOS and Windows. Compilation does not
prove permissions, GUI behavior, clipboard compatibility, mixed-monitor placement,
or capture fidelity. Record runtime results separately in [CAPTURE.md](CAPTURE.md).
