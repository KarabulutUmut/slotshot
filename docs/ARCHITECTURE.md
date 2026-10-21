# Architecture

## Current foundation

- `src/`: React and TypeScript render one static placeholder, with plain CSS.
  Vite serves local development assets and builds static production files into
  `dist/`. There is no router, state framework, or application network client.
- `src-tauri/`: a minimal Tauri 2 Rust entry point creates one resizable desktop
  window. There are no application commands, background workers, or plugins.
- The production app uses the OS webview; Node and Vite are development tools,
  not a bundled server. Runtime data stays on the device.
- The webview has a content security policy and no granted command permissions.
  Add narrow capabilities only alongside features that need them.

## Planned native integration

Keep screen capture, clipboard operations, global shortcuts, file access, and OS
permission handling in Rust. Expose a small typed command/event boundary to React
as each feature is implemented. Validate input and return actionable errors.

Prefer maintained Tauri 2 plugins or small existing crates after checking their
macOS/Windows behavior. Use platform-gated native modules where OS APIs differ;
do not build a generic plugin framework ahead of concrete needs. Screen capture
permissions, multi-monitor coordinates, and display scaling need testing on real
systems. No native feature dependencies are selected in Task 001.

Multi-image clipboard formats and receiving-app behavior are an open technical
question; see [PRODUCT.md](PRODUCT.md). Do not assume one cross-platform API makes
multi-image paste reliable.

## Planned local storage

No settings or images are persisted yet. When required, store a small versioned
settings file in the platform app-config directory using Rust/Tauri path APIs.
Use safe file replacement and validate settings on load. Do not use a database.

Keep full-resolution images out of React state and avoid duplicating them as
base64 strings. Prefer bounded native image storage and small frontend thumbnails.
Decide image persistence, cleanup, and count-reduction rules before writing a disk
cache. Save exported files only to the user-selected destination.

## Resource-efficiency goals

Use events instead of polling, capture only on explicit user action, and create
expensive resources only when needed. Bound image memory to the configured 1–10
slots, release replaced/deleted images, and perform expensive work off the UI
thread. Choose picker/background window lifecycle behavior when that feature is
implemented. Measure idle CPU, resident memory, startup time, and capture latency
on both platforms; the foundation has no measured performance claims.

## Validation

TypeScript checks both UI and Vite configuration. ESLint checks TypeScript/React;
Prettier formats supported source and documentation; rustfmt and Clippy cover Rust.
CI targets macOS and Windows and compiles a debug desktop binary without packaging.
There are no behavior tests yet because no product logic is implemented. Add focused
tests with the first behavior; native permission and clipboard workflows also need
manual integration checks. CI does not prove a native GUI launches or paste works.
