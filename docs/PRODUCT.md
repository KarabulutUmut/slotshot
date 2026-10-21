# Slotshot product requirements

## Status and scope

Task 001 provides only the application foundation and a placeholder screen.
Every workflow and feature below is planned, not implemented. Target platforms
are macOS and Windows; exact minimum OS versions require native validation.

Slotshot is a lightweight, open-source region screenshot utility with a visual
screenshot clipboard. It runs locally without accounts, a server, a database,
analytics, or cloud services.

## Planned requirements

- Capture a user-selected screen region and copy the image or save it to disk.
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

## Out of scope for Task 001

Screen capture, slot state, selection logic, hotkeys, clipboard access, saved
settings, native permission requests, background/tray behavior, installers for
distribution, signing, and automatic updates. The initial window is resizable,
but the screenshot picker itself does not exist yet.
