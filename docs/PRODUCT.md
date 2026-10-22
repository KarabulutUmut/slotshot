# Slotshot product requirements

## Current implementation and approved target

Task 002 implements native region capture through **Copy Region** and **Save
Region** test buttons. Copy writes an actual image to the system clipboard; Save
writes a unique PNG to Pictures. Screenshots otherwise stay in memory until the
capture completes or is cancelled. No internal slots, settings, file drag-out, or
final picker exist. See [Task 002 behavior and validation](CAPTURE.md).

Task 003A documents the approved target below; it changes no application behavior.
Task 003B must replace the legacy launcher/Copy Region path. Target platforms are
macOS and Windows; minimum OS versions still require native validation. Slotshot
remains local-only, with no server, accounts, database, analytics, or cloud storage.

## Capture and export workflows (planned)

1. **Initial setup:** choose an export directory, offering the OS Pictures folder
   as a sensible default. The directory can be changed later in Settings. Start
   with 5 slots; allow a configured count from 1–10.
2. **Store in Slot:** capture a selected screen region with the existing native
   engine and assign it to a numbered slot. An occupied destination deliberately
   replaces its previous image. Store one full-resolution PNG and show a thumbnail.
3. **Save to Device:** capture a region and write an independent PNG to the
   configured export directory. This does not populate or depend on a slot.
4. **Reuse screenshots:** open the picker and select one occupied slot, Shift for
   a contiguous range, or Cmd on macOS / Ctrl on Windows to toggle individuals.
   Dragging a selected thumbnail preserves the multi-selection and starts one
   native drag offering all selected PNG files. Empty slots offer no file.
5. **Save Selected:** export selected occupied slots as separate PNG files to the
   configured directory. These are independent copies, never references to slot
   files. Use unique filenames without overwriting existing exports; report any
   per-file failure and which files were successfully saved.
6. **Manage and personalize:** Clear removes selected occupied slots' images and
   thumbnails; change slot count, export directory, and planned capture/picker
   hotkeys in Settings. Show shortcut conflicts and unavailable/unwritable paths.

Capture shows a frozen snapshot from before selection overlays open. Escape or
Cancel abandons capture without changing slots or writing exports. Regions stay
on their originating display; cross-display stitching is not implemented. Explain
permission, capture, storage, export, and drag errors clearly and allow retry.

## Slot lifecycle (planned, binding)

| Action                | Required result                                                                                                                                                                                 |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Restart               | Retain slot count, assignments, and full-resolution PNGs in the OS-standard application data location.                                                                                          |
| Replace               | Safely commit the new PNG, then permanently remove the previous image and obsolete thumbnail.                                                                                                   |
| Clear                 | Remove the image and thumbnail; leave an empty numbered slot.                                                                                                                                   |
| Increase count        | Keep existing assignments and create empty slots up to the new count.                                                                                                                           |
| Decrease count        | Permanently delete every image and thumbnail belonging to slots above the new count. Confirm before proceeding if any affected slot is occupied; cancellation leaves count and files unchanged. |
| Delete/replace a slot | Never delete or modify separately exported screenshots.                                                                                                                                         |

Never silently retain hidden screenshots from removed slots. Use safe/atomic file
replacement and cleanup of incomplete writes and obsolete files. Report cleanup
failures and recover/retry them; do not claim deletion succeeded while files remain.
Thumbnails are disposable derivatives, never the only copy of screenshot content.

## Native file transfer (planned)

Native drag-out is the primary transfer interaction. Use CrabNebula drag-rs / the
Tauri 2 drag plugin as the intended direction, subject to compatibility testing.
One drag must offer all selected PNGs and preserve Slotshot's source files; no
move out of internal storage. Destination compatibility must be tested on macOS
and Windows with ChatGPT and common file-accepting applications. Do not promise
every application accepts multiple dropped image files.

The final product has no system clipboard Copy or Paste action, multi-image
clipboard transfer, automatic paste simulation, clipboard monitoring/restoration,
or hijacking of standard Cmd/Ctrl+V. This replaces the earlier clipboard product
direction; Task 002's still-present native clipboard path is legacy behavior.

## Picker and resource requirements (planned)

Preserve [Template 1](DESIGN.md): a compact, dark, elegant, translucent macOS-style
floating picker with screenshot thumbnails, slot numbers, blue selection
highlights, and a fully resizable window. Keep Save and Clear; remove Copy/Paste
from the planned toolbar. No right-click/context menus on thumbnails. Provide
keyboard access, visible selection, and consistent behavior on both platforms.

Keep idle CPU near zero through event-driven work, bounded decoded-image memory,
and thumbnails for previews. Measure idle CPU, memory, startup, capture, and drag
latency rather than claiming unverified budgets. Capture/picker hotkeys must not
intercept standard paste. Annotations, editing, background/tray behavior, signed
installers, and automatic updates are outside this documentation task.
