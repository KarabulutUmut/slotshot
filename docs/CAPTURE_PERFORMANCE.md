# Task 004.1 capture validation

Measured on macOS 26.6.2 (25G83), one built-in Retina display, 1512 × 982 desktop
points / 3024 × 1964 capture pixels. Both builds used production frontend assets
in the Debug app, the same Apple Development identity, strict bundle verification,
and only `~/Applications/Slotshot.app`. Measurements are local observations, not
cross-platform or release-build guarantees.

## Method and results

Rust `Instant` measures elapsed time from accepted native command/button receipt.
Global shortcuts start their clock at the native callback. Timings include picker
hiding, geometry enumeration, acquisition, encoding, window creation/reuse,
show/focus, painted pointer-handler readiness, selection, final region acquisition/
encoding and storage. The overlay acknowledges readiness after two animation frames;
Rust checks that its window is visible and focused. Human selection/slot-choice
time is excluded from startup latency and remains visible in total elapsed timings.

Each build completed one first capture after application launch and 20 warm
Save to Device captures with the picker initially visible. Automated native drags
selected approximately 150 × 100 physical pixels. The foreground desktop scene
contained normal text/windows. Median and nearest-rank p95 use only those 20 warm
captures, excluding other smoke tests. The after benchmark also recorded two
physical global shortcut invocations; synthesized automation keys did not reach
the Carbon global callback. No shortcut or tray p95 is claimed from two samples.

| Input-ready latency                             | Before   | After      |
| ----------------------------------------------- | -------- | ---------- |
| Warm median, 20 captures                        | 2,424 ms | 67 ms      |
| Warm p95, nearest rank                          | 2,441 ms | 92 ms      |
| Warm maximum                                    | 2,465 ms | 101 ms     |
| First capture after launch                      | 2,452 ms | 56 ms      |
| Physical global shortcut, separate observations | —        | 78 / 47 ms |

First capture means the first user capture after normal launch, **not** OS cold
boot or the entire application launch duration. After startup intentionally
prewarms hidden overlays; its startup cost is outside the first-capture clock.
True immediate-during-launch behavior has not been benchmarked. Warm measurements
support the 300 ms median / 500 ms p95 targets for this Mac’s shared selector path;
20 physical hotkey trials and a separately timed tray series remain unmeasured.
The user confirmed that physical shortcut/tray selection feels immediate and that
slot clicking works. Hidden-picker tray behavior is user-confirmed, not a separate
20-sample instrumented series.

Median cumulative stages for the same warm samples:

| Stage                               | Before   | After |
| ----------------------------------- | -------- | ----- |
| Picker hidden                       | 1 ms     | 1 ms  |
| Display enumeration completed       | 156 ms   | 2 ms  |
| Full-display acquisition completed  | 449 ms   | —     |
| Full-display PNG encoding completed | 2,291 ms | —     |
| Overlay creation/reuse completed    | 2,322 ms | 2 ms  |
| Native show/focus calls completed   | 2,377 ms | 2 ms  |
| Painted, focused input-ready ACK    | 2,424 ms | 67 ms |

The reported approximately four-second delay reproduced here as approximately
2.4 seconds. The cause is structural: a fixed 150 ms settling wait, sequential
full-display capture, approximately 1.84 seconds of full-display PNG encoding,
then new WebView initialization/PNG IPC/decoding, all **before selection**. No
single four-second stage was observed. The new selector acquires no image before
selection and reuses its hidden WebView. After release, the selected small region
acquired in a median 26 ms and encoded in 7 ms; export then took approximately
5 ms. Those post-selection numbers depend on region size/content and filesystem.

## Native and data checks

- Signed app capture/export succeeded without another Screen Recording grant.
- Actual physical shortcuts recorded input-ready timestamps; user confirmed fast
  shortcut and hidden-picker tray behavior and destination slot clicking.
- Empty slot 1 received the user’s 606 × 520 PNG in a single destination click.
  With all five slots occupied, Store still opened region selection, then offered
  every occupied destination immediately without an overwrite confirmation.
- Escape was exercised both during selection and while awaiting a destination.
  Existing PNG/thumbnail hashes remained unchanged across cancellation/export.
- An export deliberately covered the selector’s instruction/banner area. Inspection
  of its 638 × 128 PNG showed sharp underlying menu/window text with no selection
  border, dimming, instructions or Cancel button. The native hide barrier precedes
  acquisition; content protection is an additional safeguard.
- One pending screenshot lives only in Rust memory; no pending capture files or
  backup generations are created. Quit/crash therefore cannot retain a pending
  capture on disk. Committed slot reconciliation retains its existing recovery.
- Focused Rust tests cover instant occupied replacement, deletion of superseded
  PNG/thumbnail generations, invalid PNG/dimensions, stale revisions, failed
  metadata commits, cleanup retry, export independence, restart recovery, session
  IDs/phases and duplicate destinations. Frontend tests cover keys through slot 10.

Final validation passed `npm ci`, `npm run check` (35 frontend tests),
`npm run build`, Rust formatting, locked Clippy with warnings denied, 35 Rust
tests, and `npm run tauri -- build --debug --no-bundle -- --locked`. The final
canonical signed build relaunched with all five assignments/thumbnails intact,
no pending capture restored, and another successful export without reauthorization.

Clear and occupied count reduction retain their confirmation dialogs. They were
reviewed but left outside this capture-focused change.

## Resources and platform limits

Before: after the baseline series, main-process RSS was approximately 172 MiB;
the associated picker WebContent process was approximately 88 MiB. After: main
RSS was approximately 113–117 MiB, with picker/pooled-overlay WebContent processes
approximately 91–94 MiB and 46 MiB. Retaining the hidden selector costs a small
WebView even while idle, but removes retained full-display capture/encoding memory
from this path. These snapshots suggest comparable/slightly lower combined memory
for this one-display run; they are not peak-memory measurements or a guarantee
against allocator/OS variation. Other shared WebKit/GPU processes are excluded.

During a 45-second quiet interval, main CPU time increased 0.08 seconds and the
two WebContent processes increased 0.02 / 0.05 seconds (approximately 0.33% of one
CPU combined). RSS stayed in the ranges above. There is no background recording,
screen acquisition loop, image cache or timer. Longer idle profiling, multiple
displays and Windows resource measurements remain unverified.

Windows retains locked XCap’s physical-coordinate GDI region capture and native
DPI move/size verification; no Windows machine was available for runtime testing.
Older macOS retains the existing HUD appearance fallback. XCap’s Core Graphics
capture API is deprecated by Apple; ScreenCaptureKit screenshots were investigated,
but changing native backends is unnecessary for this measured preselection
bottleneck and would add availability/binding work. No runtime dependency,
lockfile, signing identity, bundle identifier, permission gate, glass material,
export setting or native file-drag API is changed.

Unverified: physical multi-display/mixed-DPI/rotated geometry, hot-plug during a
selection, older macOS, Windows GUI, protected/HDR/remote content, receiving-app
multi-file drag compatibility, native disk-full/fault injection, long idle/peak
memory, and 20-trial physical shortcut/tray distributions. Unit/build results do
not establish these native cases. Original occupied screenshots are preserved;
independent benchmark exports remain local and are never committed.

## Reproduce locally

After building/installing with `npm run app:test:mac`, quit the canonical app
normally and launch it with:

```sh
open -a "$HOME/Applications/Slotshot.app" --args --capture-timings
```

Only this opt-in flag writes stage-only JSON lines to stderr and the OS temporary
folder as `slotshot-capture-timings-<pid>.jsonl`, capped at approximately 512 KiB per
process. Lines contain only capture sequence, static stage name and monotonic
elapsed milliseconds: no screenshot bytes, coordinates, titles or application
content. Normal launches create no timing log. Group by capture ID, retain samples
with the input-ready ACK, and distinguish `button_received`, `hotkey_received` and
`native_action_received` (tray). Orphan IDs from ignored actions/releases have no
interactive sample. Preserve the first sample separately and calculate warm p95 as
the 19th sorted value of 20 samples. Do not commit local screenshots or raw logs.
