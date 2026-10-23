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
  and can be retried with Reload in Activity details or restart.
- A compact, dark, resizable Template 1 picker with numbered thumbnails and blue
  selection. Thumbnail sizes and wrapping respond to both window dimensions;
  normal quit remembers window size/position. Settings opens as a compact dialog. Tab, arrow keys, Home/End, Space/Enter, and modifier selection provide
  basic keyboard access. Open Slotshot normally to open the picker.

There are no Copy/Paste controls, system clipboard integration, standard paste
shortcut interception, thumbnail context menus, or reordering. Global hotkeys,
tray/background operation, editing, release signing, and distribution remain out of scope.
See [product rules](docs/PRODUCT.md), [architecture and limitations](docs/ARCHITECTURE.md),
and [approved design](docs/DESIGN.md).

## Native validation and limitations

Capture uses a frozen snapshot before selection overlays open; Escape or Cancel
abandons the capture. Regions stay on their originating display. macOS requires
Screen Recording permission. Multi-monitor/mixed-DPI fidelity, Windows runtime,
and receiving-app drag compatibility require native testing; compilation alone
is not runtime evidence. Destination apps may accept only one file or reject drops.

The macOS translucent webview in the locked Tauri/Wry stack uses a private
WebKit transparency key. The `macos-private-api` feature/configuration is retained,
but is a no-op in Tauri 2.12.1 and newer; removing that flag alone does not remove
the private API use. Mac App Store distribution needs a separate API audit and
potential transparency changes. Windows uses Acrylic where supported; appearance
depends on OS and accessibility settings. Window controls and resizing stay native.

Task 003B.4 finalizes the user-approved **Liquid Glass Regular** material on
macOS 26+, using Tauri's built-in effects, a subtle dark native tint, and a light
charcoal CSS tint. Older macOS versions request HUD vibrancy; Windows keeps its
existing Acrylic appearance. Reduce Transparency is respected by native materials
and an opaque CSS fallback where the media query is supported. The temporary
material selector, synthetic previews, and their commands/permissions are removed;
the picker always displays real stored thumbnails. No new dependencies are needed.

Task 003B.4 validation on macOS 26.6.2: the canonical app rebuilt with the same
Apple Development identity and passed strict signature verification. A real
2698 × 1086 PNG was stored in previously empty slot 4 without reauthorization,
then restored with its thumbnail after normal quit/reopen. Existing screenshot
PNGs, thumbnails, settings, and occupied assignments were unchanged; the manifest
only gained the test capture. Native zoom/restore and the normal Settings sheet
worked. Regular remained readable with Reduce Transparency enabled; that OS
setting was restored off. The user confirmed enabled and disabled labels were
readable over light and dark desktop backgrounds. Windows and older-macOS runtime,
receiving-app multi-file drag compatibility, and display edge cases remain unverified.

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
npm run app:test:mac
```

On macOS, use the canonical workflow below. On Windows, use `npm run tauri -- dev`
after `npm ci`; Tauri starts Vite and opens the native desktop window.
The initial Rust build downloads and compiles
native dependencies and may take several minutes. Missing `cargo` or `rustc`
means Rust must be installed and available on your terminal's PATH.

### One macOS testing app

After `npm ci`, run this from `~/Developer/Screenshot` for every native test:

```sh
npm run app:test:mac
```

Test **only `~/Applications/Slotshot.app`**, bundle identifier
`io.github.karabulutumut.slotshot`, executable `Contents/MacOS/slotshot`.
The command rebuilds the current source with production frontend assets in a
Debug `.app` bundle using locked Rust dependencies, signs/verifies the whole bundle,
installs at that exact path, opens it, and verifies the running executable path.
It builds no Release, DMG, installer, updater, or notarization package. The Swift
helper is development tooling using AppKit from the existing Xcode prerequisites;
it adds no application dependency or permission.

An existing canonical instance receives a normal quit request before replacement;
there is a ten-second timeout and no force kill. Finish/cancel active captures or
drags before running the command. Other running Slotshot builds cause an error:
quit them manually first. Do not launch the old Debug/Release bundles or use
`tauri dev` for macOS permission testing. Existing Release bundles are untouched;
previous Debug bundles and replaced canonical apps are retained in adjacent hidden
`.slotshot-previous-*/Slotshot.app` directories (their paths are printed). Remove
those backups manually only when no longer needed. An install rename failure
restores the prior canonical app; a build/signing failure leaves it installed.
Failed staging bundles are retained in `~/Applications/.slotshot-staging-*` for
inspection. The script never deletes screenshot slots, preferences, exports, or
TCC permissions. The unchanged identifier keeps the existing application-data path.

The script selects the sole valid **Apple Development** identity in your keychain.
Inspect identities with `security find-identity -v -p codesigning`. If there are
multiple, select the same certificate consistently using its SHA-1:

```sh
SLOTSHOT_SIGNING_IDENTITY="CERTIFICATE_SHA1" npm run app:test:mac
```

If none is available, the command clearly reports an **ad-hoc fallback**; you can
also select it explicitly with `SLOTSHOT_SIGNING_IDENTITY=-`. The complete bundle
is still sealed/verified, but its hash-based identity changes with code changes:
**Screen Recording permission persistence is not guaranteed with ad-hoc signing**.
For stable development signing, add your Apple account in Xcode → Settings →
Accounts → Manage Certificates → Apple Development. The certificate **and private
key** must be in an accessible keychain and appear as a valid codesigning identity.
Unlock that keychain and approve `codesign` key access yourself if macOS asks.
A free personal development account can be used for local testing; this is not
notarized distribution. See [Apple's certificate instructions](https://developer.apple.com/documentation/xcode/sharing-your-teams-signing-certificates)
and [Tauri's signing documentation](https://v2.tauri.app/distribute/sign/macos/).
An explicitly selected missing/invalid identity fails instead of silently falling
back. Missing toolchain/dependencies, ambiguous identities, an unrelated/symlink
destination, a running alternate build, or installation/signing failures stop the
command with an error. Do not use `sudo` to run it.

### macOS Screen Recording troubleshooting

The Task 003B.1 diagnosis on macOS 26.6.2 reproduced both Core Graphics preflight
and request returning false, before XCap was called. TCC logged **"Failed to match
existing code requirement"** for `io.github.karabulutumut.slotshot` and
`kTCCServiceScreenCapture`: the stored grant required a previous binary hash.
Both old Debug and Release bundles had linker-only ad-hoc signatures, different
hash-based designated requirements, an unbound Info.plist, no team identity, and
failed whole-bundle signature verification. Their plist bundle identifiers and
executable names were identical. The verified failure was a stale code-identity
grant, not evidence of a capture-engine failure or an incorrect bundle identifier.
A fixed path plus stable certificate signing addresses the changing identity;
switching from the old signature still needs a one-time grant refresh.
Apple explains how [designated requirements identify successive app versions](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements).

If capture says **"Screen Recording access was not granted"**, check the running
executable shown in the error. Use this one-time recovery for the old grant:

1. Build/install with `npm run app:test:mac`, then fully quit Slotshot with Cmd+Q.
2. Open **System Settings → Privacy & Security → Screen & System Audio Recording**
   (called **Screen Recording** on older macOS). Remove only the stale **Slotshot**
   entry using the minus button. Do not remove grants for other apps.
3. Add **`~/Applications/Slotshot.app`** with the plus button. In the file chooser,
   Cmd+Shift+G lets you enter that exact path. Enable its screen recording switch;
   do not select a bundle from `src-tauri/target`, a backup, or the raw executable.
   If macOS requires authentication, complete it yourself.
4. Reopen that exact app and retry **Store in Slot** into an empty slot. Fully
   quit/reopen again if macOS requests it. On a clean account, the capture request
   may first offer to open System Settings; authorize the same canonical app.

This is a user-controlled refresh of Slotshot's grant; the build command never
runs `tccutil reset` or reads/writes the TCC database. No Gatekeeper/SIP changes,
Full Disk Access, microphone, Accessibility, or terminal permission grants are
required by this workflow. Certificate renewal/team changes or OS privacy policy
can still require reauthorization even with stable signing.

**"Screen capture backend failed after the permission check passed"** is a
different failure: the permission gate succeeded and XCap returned an error.
Record that underlying error and macOS/display setup; refreshing a grant is not
proof of a backend fix. Real region capture and storage must be verified manually
after permission is granted; successful signing/building alone does not prove capture.

Task 003B.1 validation on this Mac: Apple Development signing and strict bundle
verification passed; the canonical command rebuilt, replaced, and launched the
correct path repeatedly. After the user refreshed the stale grant, a real
2715 × 1686 PNG was stored in slot 1. A further rebuild/relaunch preserved its PNG,
thumbnail, metadata, and preferences byte-for-byte, and capture succeeded again
(2355 × 1347 PNG in slot 2) without another permission refresh. The captures remain
local and are not committed. This verifies this Mac's workflow, not Windows,
other display configurations, ad-hoc permission retention, or certificate renewal.

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

`npm run check` also runs selection and macOS workflow safety tests using Node’s built-in test runner.
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

1. From `~/Developer/Screenshot`, run `npm ci` once after dependency changes, then
   `npm run app:test:mac`. Use only `~/Applications/Slotshot.app`; the bundle tests
   production assets/CSP. Quit alternate Slotshot instances first; the storage
   lock deliberately rejects a second instance.
2. On first run, choose **Use Pictures** or **Choose Folder…**. Cancel the folder
   dialog once and verify setup stays open. Open Settings, change folder, and
   change the slot count from 5 to 10 and back while empty.
3. Prepare non-private colored shapes/text in another window. Select empty slot 1,
   click **Store in Slot**, and grant permission in **System Settings → Privacy &
   Security → Screen & System Audio Recording** if requested. The entry may be
   **Slotshot at `~/Applications/Slotshot.app`**. Use the recovery procedure above
   if an older Slotshot entry is already enabled. Fully quit/reopen after changing it.
   Drag a region and release; inspect the thumbnail and its pixel-dimensions tooltip.
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
