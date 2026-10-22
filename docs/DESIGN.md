# Approved picker design — Template 1

## Reference and status

The existing [Slotshot Mac Utility Showcase](design/Slotshot%20Mac%20Utility%20Showcase.png)
is the approved Template 1 visual reference. Task 003A adds the previously
untracked, user-provided PNG unchanged; no replacement mockup is generated.
The current Task 002 window is a capture test launcher, not this picker.

Preserve the reference's compact, dark, elegant, translucent macOS-style floating
window, screenshot thumbnails, numbered slots, blue selection highlights, rounded
surfaces, and restrained toolbar. The window must be fully resizable. Maintain
visual consistency on macOS and Windows; validate transparency/window behavior on
each platform and document limitations without introducing a new design.

## Approved interaction changes

The written [product requirements](PRODUCT.md) supersede outdated interaction
labels in the reference image:

- Remove its Copy/Paste controls and paste-shortcut instructions. The final
  capture destinations are Store in Slot and Save to Device.
- Keep Save and Clear. Save Selected exports selected occupied slots as separate
  PNGs. Native drag-out is the primary way to transfer images to another app.
- Select one slot, Shift for ranges, Cmd (macOS) / Ctrl (Windows) for individual
  selection. Dragging an already selected thumbnail keeps the group selected and
  offers all selected PNG files in one native drag that preserves source files.
- No right-click/context menus on thumbnails, including the contextual action
  panel illustrated in the multiple-selection section. Keep actions in the toolbar.
- The image's “Drag to reorder” wording does not define the approved drag-out
  behavior or require implementing reordering. Do not confuse reordering with
  transferring PNG files to another application.
- Show empty numbered slots up to the configured count (1–10, default 5). Confirm
  count reductions that affect occupied slots; no hidden retained screenshots.

Provide visible selection and keyboard access. Use thumbnails for previews and
event-driven updates to keep idle overhead low. Native drag compatibility, full
resizing, and platform visual behavior need runtime testing; the image is not
evidence that these features are implemented.
