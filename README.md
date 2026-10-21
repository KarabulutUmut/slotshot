# Slotshot

Slotshot is an open-source screenshot utility being built for macOS and Windows.
The goal is quick region capture with a visual screenshot clipboard containing
1–10 configurable slots. It uses Tauri 2, Rust, React, TypeScript, Vite, and plain
CSS. The application is local-only: no server, accounts, database, analytics, or
cloud services.

## Development status

**Task 001: foundation only.** The repository contains the desktop scaffold,
a static placeholder UI, TypeScript checks, linting, formatting, documentation,
and a GitHub Actions workflow for macOS and Windows. Product functionality is
not implemented, and there are no downloadable releases. The app icons use a simple
temporary rectangle symbol, not final Slotshot branding.

The frontend is independently runnable in a browser. A native desktop build and
launch require Rust and the platform prerequisites below. Native builds and GUI
behavior have not been verified locally because the foundation environment has
no Rust/Cargo installation. CI configuration is not evidence of a successful run.
There are no behavior tests yet; tests will be introduced with product logic.

## Planned functionality

- Region screenshots copied to the system clipboard or saved to disk.
- A visual clipboard with 1–10 slots and thumbnail previews.
- Assign, replace, delete, and select images, including range and multi-selection.
- Configurable global hotkeys and default save location.
- A resizable screenshot picker with consistent macOS and Windows behavior.
- Low idle resource use and bounded image memory.

**Multi-image pasting requires technical validation.** Multiple selection does
not imply that another application can accept several images in one paste.
Native clipboard formats and receiving-app behavior must be tested on both OSes.

See [product requirements](docs/PRODUCT.md) and [architecture](docs/ARCHITECTURE.md)
for planned workflows, technical boundaries, and open questions.

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
confirmed when native capture APIs are selected and validated. Linux and mobile
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

For the **frontend placeholder only**, without Rust:

```sh
npm ci
npm run dev
```

Open `http://127.0.0.1:1420`. This browser preview does not validate native APIs.
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
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- build --debug --no-bundle
```

`cargo test` currently has no application behavior tests. The final command
compiles a debug desktop executable without packaging or signing. For a local
release build, use `npm run tauri -- build --no-bundle`; signed distribution
and installers are future work. Build output is under `src-tauri/target/`.

`package-lock.json` is committed for reproducible npm installs. Cargo.lock is
not yet generated because Rust is absent in the foundation environment. Generate
and commit it on the first Rust-enabled checkout, then use `--locked` for native
CI checks. Do not handwrite it. Initial CI resolves Rust dependencies normally.

The [CI workflow](.github/workflows/ci.yml) runs on pushes, pull requests, and manual
dispatches. Both macOS and Windows jobs check frontend code, Rust formatting,
Clippy, Rust tests, and compile the desktop app. GitHub-hosted runners supply
Xcode or MSVC/Windows SDK/WebView2. CI does not launch the GUI, validate native
workflows, publish a release, or install an app on your machine.

## Roadmap

1. **Foundation (Task 001):** scaffold, placeholder, documentation, developer
   checks, and CI configuration. Native build/launch validation remains pending.
2. **Capture validation:** test OS permissions, region selection, scaling, and
   single-image clipboard/save behavior on macOS and Windows.
3. **Visual clipboard:** define image lifecycle, implement 1–10 slots, thumbnails,
   assignment/replacement/deletion, and tested selection behavior.
4. **Preferences and picker:** configurable shortcuts and save location, conflict
   handling, persistence rules, and picker resizing/keyboard access.
5. **Validation and distribution:** evaluate multi-image paste compatibility,
   measure idle resources, test real workflows on both OSes, and prepare signed
   releases once ready.

Only Task 001 is in scope for the initial foundation.

## Contributing and license

Read [AGENTS.md](AGENTS.md) for project conventions. Keep changes small, maintain
cross-platform behavior, add tests with behavior, and document anything that
could not be verified.

Slotshot is licensed under the [MIT License](LICENSE).
