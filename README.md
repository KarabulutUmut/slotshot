# Slotshot

A local screenshot utility for macOS and Windows, built with Tauri 2, Rust,
React, TypeScript, Vite, and plain CSS. No server, accounts, database, analytics,
or cloud services. There are no downloadable releases yet.

## Current application — Task 003B

- **Store in Slot:** select a numbered slot, then capture a region. With no single
  destination selected, use the first empty slot. Replacing an occupied slot
  requires confirmation; cancelling capture keeps the previous image.
- **Persistent picker:** 1–10 slots (default 5), full-resolution PNGs and bounded
  thumbnails in OS application data. Restart restores assignments and settings.
- **Native file drag:** click to select, Shift for a range, Cmd on macOS / Ctrl on
  Windows to toggle individuals. Drag an occupied thumbnail to offer all selected
  occupied PNGs together with copy semantics. Empty slots offer no files.
- **Independent exports:** choose Pictures or another folder on first run. Change
  it in Settings. **Save to Device** captures directly there; toolbar **Save**
  exports selected slots individually with unique filenames and per-file results.
- **Clear / slot count:** permanently delete affected internal PNGs and thumbnails.
  Count reductions involving occupied slots require confirmation. Exported files
  survive replacement, Clear, and count reduction. Cleanup errors remain visible
  and can be retried with Reload or restart.
- A compact, dark, resizable Template 1 picker with numbered thumbnails and blue
  selection. Tab, arrow keys, Home/End, Space/Enter, and modifier selection provide
  basic keyboard access. Open Slotshot normally to open the picker.

There are no Copy/Paste controls, system clipboard integration, standard paste
shortcut interception, thumbnail context menus, or reordering. Global hotkeys,
tray/background operation, editing, signing, and distribution remain out of scope.
See [product rules](docs/PRODUCT.md), [architecture and limitations](docs/ARCHITECTURE.md),
and [approved design](docs/DESIGN.md).

## Native validation and limitations

Capture uses a frozen snapshot before selection overlays open; Escape or Cancel
abandons the capture. Regions stay on their originating display. macOS requires
Screen Recording permission. Multi-monitor/mixed-DPI fidelity, Windows runtime,
and receiving-app drag compatibility require native testing; compilation alone
is not runtime evidence. Destination apps may accept only one file or reject drops.

The macOS translucent webview requires Tauri’s `macos-private-api` feature; this
precludes Mac App Store submission without revisiting transparency. Windows uses
Acrylic where supported; appearance depends on OS and accessibility settings.
The application keeps normal window controls and OS resizing.

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

Open `http://127.0.0.1:1420`. Native actions are disabled in a browser; this
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

`npm run check` also runs the selection tests using Node’s built-in test runner.
`cargo test` covers capture coordinates/readiness, slot lifecycle/recovery, exports,
trusted drag inputs, symlinks, and operation locking. The final command
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

## Exact macOS manual testing

1. From `~/Developer/Screenshot`, run `npm ci`, then
   `npm run tauri -- build --debug --bundles app -- --locked`, then
   `open ./src-tauri/target/debug/bundle/macos/Slotshot.app`.
   Alternatively use `npm run tauri -- dev`. Use the exact new bundle path to
   avoid opening an older installed build. The bundle tests production assets/CSP.
   Quit other Slotshot
   instances first; the storage lock deliberately rejects a second instance.
2. On first run, choose **Use Pictures** or **Choose Folder…**. Cancel the folder
   dialog once and verify setup stays open. Open Settings, change folder, and
   change the slot count from 5 to 10 and back while empty.
3. Prepare non-private colored shapes/text in another window. Select empty slot 1,
   click **Store in Slot**, and grant permission in **System Settings → Privacy &
   Security → Screen & System Audio Recording** if requested. The entry may be
   Slotshot or the terminal/development host. Fully quit/reopen after changing it.
   Drag a region and release; inspect the thumbnail and reported pixel dimensions.
4. Select occupied slot 1 and choose Store again. Cancel replacement confirmation
   and verify no capture begins. Confirm replacement, then cancel capture with
   Escape; the old image must remain. Repeat using the overlay Cancel button.
   Complete replacement and verify the old internal PNG/thumbnail are removed.
5. Populate slots 2 and 5. Test plain click, Shift ranges in both directions,
   Cmd individual toggles, dragging an already selected group, and keyboard access.
   Resize narrow/wide; verify every configured slot and both toolbar buttons remain
   accessible. No thumbnail context menu or reordering should appear.
6. Save selected slots twice. Open the reported export PNGs in Preview and check
   content, sharpness, colors, dimensions, and distinct filenames. Capture with
   Save to Device; it must create an independent export without populating a slot.
7. Drag one screenshot, then a group, to Finder, ChatGPT, and another file-accepting
   app available locally. Record app/OS versions and single/multiple-file outcomes.
   Escape a drag and try a rejected destination. Internal sources must remain
   present; Clear/replacement/count changes must be blocked until drag ends.
8. Reduce the count to 3 while slot 5 is occupied. Cancel first: count and files
   stay intact. Confirm next: slot 5’s PNG and thumbnail must disappear. Increase
   to 5 again: slot 5 must be empty. Clear selected occupied slots after confirming.
   Verify all earlier exports remain unchanged.
9. Quit/reopen with occupied slots and check count, assignments, thumbnails, and
   export folder persistence. Inspect the managed location described in
   [architecture](docs/ARCHITECTURE.md#storage-and-recovery) if testing cleanup.
   Never use personal/private captures for destructive failure tests.
10. Repeat reversed drags and edge captures on Retina/external displays, including
    displays above/left of primary and mixed scaling. A 100-point region on a 2×
    snapshot should be 200 pixels. Inspect for clipping or captured overlay UI.
    Reconfigure displays only between captures. Check recovery after closing an overlay.
11. In a disposable account/folder, test unavailable/unwritable exports, disk/write
    errors, interrupted writes, and cleanup retry. Errors must be visible, existing
    slots/exports preserved, and partial exports removed. Revoke screen permission,
    quit/reopen, and verify an actionable error and usable picker, then restore it.
12. Quit Slotshot when done. Repeat relevant capture, storage, export, selection,
    and drag tests on Windows; a Windows CI build does not validate them.

## Contributing and license

Use regular feature branches in the primary local checkout; see [AGENTS.md](AGENTS.md).
Keep changes small and document unverified behavior. Historical Task 002 details
remain in Git history and [PR #1](https://github.com/KarabulutUmut/slotshot/pull/1).
Slotshot is licensed under the [MIT License](LICENSE).
