# Slotshot

Slotshot is an open-source screenshot utility being built for macOS and Windows.
The approved direction is region capture to persistent local screenshot slots or
independent PNG exports, with native file drag-out into other applications.
The planned picker has 1–10 configurable slots (default 5), each holding one
full-resolution PNG.
It uses Tauri 2, Rust, React, TypeScript, Vite, and plain CSS. The application is
local-only: no server, accounts, database, analytics, or cloud services.

## Development status

**Implemented — Task 002: native region capture.** The desktop app offers **Copy Region** and
**Save Region** test buttons. Drag a rectangle on any connected display; Escape
cancels. Copy writes an actual image to the system clipboard. Save exports a PNG
to your Pictures folder without overwriting existing files. Regions stay on one
display; cross-display stitching is not implemented.

The selection shows a frozen snapshot taken before the overlay opens. Screenshots
stay in memory until completion or cancellation; only Save writes an image file.
macOS requires Screen Recording permission. Slots, hotkeys, settings, and the final
picker UI remain planned. There are no downloadable releases.

**Approved target — Task 003A specifies; Task 003B must implement.** Capture will
offer **Store in Slot** and **Save to Device**. Legacy Copy Region and its native
clipboard operation still exist and must be removed/replaced. The final product
has no system clipboard Copy, Paste, paste simulation, clipboard monitoring or
restoration, or interception of standard Cmd/Ctrl+V. Persistent slots, native file
drag-and-drop, export settings, and the approved picker are not implemented yet.

See [capture design, limitations, and manual test instructions](docs/CAPTURE.md).
Windows CI compilation does not establish Windows runtime behavior.

## Approved planned functionality

- Persist slots in the OS-standard application data location across restarts.
  Replace/Clear removes the old PNG and thumbnail. Reducing the count deletes all
  removed slots' files; confirm first if any affected slot is occupied. Increasing
  the count adds empty slots; never retain hidden screenshots in removed slots.
- Select one image, Shift ranges, or Cmd (macOS) / Ctrl (Windows) individual images,
  then drag all selected PNGs in one native file operation that preserves sources.
- Save to Device and Save Selected create independent PNGs in an export directory
  chosen during initial setup (Pictures is the sensible default), changeable in
  Settings. Slot changes never delete these exports.
- Preserve the compact, dark, translucent Template 1 picker, thumbnail numbers,
  blue selection highlights, full window resizing, and Save/Clear toolbar actions.
  Native drag-out is the primary transfer interaction; no thumbnail context menus.
- Configurable global capture/picker hotkeys, consistent macOS/Windows behavior,
  and no interception of standard paste shortcuts.
- Low idle resource use and bounded image memory.

CrabNebula drag-rs / its Tauri 2 drag plugin is the intended native drag direction,
subject to compatibility testing. Test single/multiple file drops on both platforms,
including ChatGPT and common file-accepting apps; not every app accepts multiple
dropped images. No drag dependencies are installed yet.

See [product requirements](docs/PRODUCT.md) and [architecture](docs/ARCHITECTURE.md)
for lifecycle and implementation requirements, and [approved design](docs/DESIGN.md)
for the existing Template 1 reference and interaction changes.

## Prerequisites

- Node.js 24 or newer and npm 11 or newer. `.nvmrc` selects Node 24 for development
  and CI; use `nvm use` if you have nvm installed.
- Stable Rust via [rustup](https://rustup.rs/), including Cargo, rustfmt, and Clippy.
- **macOS:** Xcode Command Line Tools (`xcode-select --install`) or full Xcode,
  with its initial setup and license acceptance completed.
- **Windows:** Visual Studio Build Tools with **Desktop development with C++**,
  a Windows SDK, and the Microsoft Edge WebView2 Runtime. Use the Rust MSVC
  toolchain (`stable-x86_64-pc-windows-msvc` on x64 Windows).

Follow the official [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)
for installation and platform details. These tools can be substantial downloads
and are not installed by this repository. Minimum supported OS versions will be
confirmed through native runtime validation; capture APIs are now selected. Linux and mobile
are outside the current product scope.

## Install and develop

```sh
git clone https://github.com/KarabulutUmut/slotshot.git
cd slotshot
npm ci
npm run tauri -- dev
```

From an existing checkout, run only the last two commands. Tauri starts Vite and
opens the native desktop window. The initial Rust build downloads and compiles
native dependencies and may take several minutes. Missing `cargo` or `rustc`
means Rust must be installed and available on your terminal's PATH.

For the **frontend preview only**, without Rust:

```sh
npm ci
npm run dev
```

Open `http://127.0.0.1:1420`. Capture buttons are disabled in a browser; this
preview does not validate native APIs.
Vite binds to loopback and uses a fixed port; stop another process using port
1420 before starting it. There is no production web server.

## Checks and builds

```sh
npm run check
npm run build
```

`check` runs TypeScript on the frontend and Vite configuration, ESLint with zero
warnings allowed, and Prettier verification. `build` checks types and creates
production frontend assets in `dist/`. Use `npm run format` to format supported
files; Rust uses rustfmt separately.

With native prerequisites installed, also run:

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri -- build --debug --no-bundle -- --locked
```

`cargo test` covers coordinate validation, PNG roundtrips, filename collisions,
save errors, and overlay initialization recovery. The final command
compiles a debug desktop executable without packaging or signing. For a local
release build, use `npm run tauri -- build --no-bundle -- --locked`; signed distribution
and installers are future work. Build output is under `src-tauri/target/`.

`package-lock.json` and `src-tauri/Cargo.lock` pin npm and Rust dependencies.
Use `npm ci` and `--locked` for native checks/builds so validation cannot silently
change dependency resolution. Regenerate lockfiles only for intentional dependency
updates; do not handwrite them.

For documentation-only changes, validate changed Markdown formatting, local links,
specification consistency, and `git diff --check`; dependency installs, application
builds, and native smoke tests are not required. See [AGENTS.md](AGENTS.md).

The [CI workflow](.github/workflows/ci.yml) runs on pushes, pull requests, and manual
dispatches. Both macOS and Windows jobs check frontend code, Rust formatting,
Clippy, Rust tests, and compile the desktop app. GitHub-hosted runners supply
Xcode or MSVC/Windows SDK/WebView2. CI does not launch the GUI, validate native
workflows, publish a release, or install an app on your machine.

## Roadmap

1. **Foundation (Task 001):** scaffold, placeholder, documentation, developer
   checks, and CI configuration. Task 001.5 validates the existing foundation and
   commits the native dependency lockfile; macOS launch has been manually confirmed.
2. **Capture (Task 002):** implemented single-region copy/save engine; real OS
   permissions, scaling, multi-monitor capture, and Windows runtime need validation.
3. **Architecture alignment (Task 003A):** document the file-based direction and
   preserve the approved visual reference; no application changes.
4. **File-based workflow (Task 003B):** replace legacy Copy Region, implement slot
   persistence/cleanup, setup/settings for independent exports, native file drag,
   and the Template 1 picker with tested selection. See the
   [implementation handoff](docs/ARCHITECTURE.md#task-003b-handoff).
5. **Validation and distribution:** test native capture and drag destination
   compatibility on macOS and Windows, configure hotkeys, measure idle resources,
   and prepare signed releases once ready.

## Contributing and license

Read [AGENTS.md](AGENTS.md) for project conventions. Keep changes small, maintain
cross-platform behavior, add tests with behavior, and document anything that
could not be verified.

Slotshot is licensed under the [MIT License](LICENSE).
