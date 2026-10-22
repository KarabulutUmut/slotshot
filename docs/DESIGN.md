# Picker design — Template 1

The [Slotshot Mac Utility Showcase](design/Slotshot%20Mac%20Utility%20Showcase.png)
is the approved visual reference, preserved unchanged. The application implements
the picker itself, not the surrounding marketing poster.

Keep the compact dark translucent floating surface, rounded thumbnails, restrained
borders/spacing, numbered slots, blue selection, and fully resizable native window.
The grid adapts to 1–10 slots and wraps in narrow windows. macOS uses HUD vibrancy;
Windows requests Acrylic. OS window chrome, transparency settings, and material
support differ; keep the same CSS surface and document observed differences.

Written [product rules](PRODUCT.md) supersede obsolete reference interactions:

- Header: Store in Slot, Save to Device, and Settings. First-run settings offers
  Pictures or another export folder. Settings also chooses 1–10 slots.
- Bottom toolbar: **Save** and **Clear** only. Native file drag-out is primary;
  no Copy/Paste controls or paste shortcuts.
- Click selects; Shift selects a contiguous range; Cmd (macOS) / Ctrl (Windows)
  toggles individuals. Dragging a selected occupied thumbnail preserves the group
  and offers selected occupied PNGs together, with source-preserving copy semantics.
- Empty numbered slots show a restrained plus/empty label. Select one and use
  Store in Slot to populate it. No thumbnail context menus or reordering.
- Visible focus, descriptive button labels, Tab, arrows/Home/End, and Space/Enter
  support keyboard use. Confirmation dialogs handle Escape/cancellation and keep
  destructive actions explicit. Counts/revisions are also validated in Rust.
- Status text reports capture, export, drag, and filesystem errors. Reload retries
  metadata loading and obsolete-file cleanup. Cleanup warnings remain visible.

Native fidelity and cross-platform behavior require runtime validation; neither
the reference image nor a successful build establishes receiving-app compatibility.
Global shortcuts and tray/background operation remain outside Task 003B.
