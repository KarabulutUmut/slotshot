# Slotshot product requirements

## Status and scope

Task 002 implements region capture through two test buttons in the existing window.
Drag a rectangle on any connected display, then copy an actual image or save a PNG
to Pictures. Escape cancels without exporting. The overlay shows a frozen snapshot
and is never included in the captured image. A region stays on the display where
the drag starts; cross-display stitching is deferred. Errors return to the main
window with a retryable message. No screenshots are retained between captures.

The remaining requirements and shortcut/slot workflows below are planned. Target
platforms are macOS and Windows; exact minimum OS versions require native validation.
See [capture validation and limitations](CAPTURE.md).

Slotshot is a lightweight, open-source region screenshot utility with a visual
screenshot clipboard. It runs locally without accounts, a server, a database,
analytics, or cloud services.

## Planned requirements

- Offer a visual clipboard with a configurable total of 1–10 screenshot slots.
  Each occupied slot shows a thumbnail; users can assign, replace, and delete it.
- Select one thumbnail, a contiguous range, or multiple individual thumbnails.
  Support familiar keyboard modifiers on macOS and Windows, visible selection,
  and keyboard access. Exact bindings and selection behavior will be specified
  and tested during implementation.
- Configure global hotkeys and a default save directory. Report shortcut
  conflicts, permission denial, and unavailable or unwritable paths clearly.
- Provide a resizable picker with a consistent layout and behavior on both OSes.
- Keep idle CPU usage near zero through event-driven work and bounded image
  storage. Measure CPU, memory, startup, and capture latency before setting budgets.
- Keep all images and settings local. Persistence and retention rules must be
  explicit before storing screenshot contents between launches.

## Expected workflows

1. **Quick capture:** invoke a configured shortcut, drag a region, then copy or
   save the result. Escape cancels without changing clipboard contents or files.
2. **Keep a screenshot:** capture a region and assign it to an available slot.
   Choosing an occupied slot deliberately replaces its previous image.
3. **Reuse screenshots:** open the picker, inspect thumbnails, and select one,
   a range, or several individual images. Copy or save the supported selection.
4. **Manage slots:** remove unwanted images or change the slot count within 1–10.
   Define how occupied slots are handled when reducing the count before shipping.
5. **Personalize:** change shortcuts and save directory, validate the settings,
   and use the updated choices on subsequent captures.
6. **Recover from an error:** explain denied screen permissions, cancelled capture,
   failed saves, or shortcut conflicts and let the user retry or change settings.

## Multi-image pasting: technical validation required

Selecting multiple screenshots does **not** establish that multiple images can be
pasted into another application. Clipboard representations and receiving-app
support differ by OS and application. Validate native formats and actual paste
behavior on macOS and Windows with representative receiving apps before promising
multi-image paste. Do not silently substitute a collage, file list, or sequential
clipboard writes. Decide and document any fallback separately.

## Out of scope for Task 002

Screenshot slots, thumbnail selection, hotkeys, settings, annotations, image editing,
background/tray behavior, distribution installers, signing, and automatic updates.
The main window remains a development placeholder; there is no final picker UI.
