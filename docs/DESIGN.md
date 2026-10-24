# Picker design — Template 1

The [Slotshot Mac Utility Showcase](design/Slotshot%20Mac%20Utility%20Showcase.png)
is the approved visual reference, preserved unchanged. The application implements
the picker itself, not the surrounding marketing poster.

Keep the compact dark translucent floating surface, rounded thumbnails, restrained
borders/spacing, numbered slots, blue selection, and fully resizable native window.
The grid adapts to 1–10 slots and wraps in narrow windows. The user approved
**A — Liquid Glass Regular** in Task 003B.4 as the final macOS material. macOS 26+
uses native Regular glass with tint RGBA(20,24,32,28), a 12-point radius, and the
approved light charcoal shell gradient (13–19% alpha). Earlier macOS versions
request native HUD vibrancy. Windows retains its existing Acrylic and charcoal/navy
CSS appearance. No image saturation, content, opacity, layout, or spacing changes
accompany the material decision. Native materials respect Reduce Transparency;
the CSS media query also supplies an opaque surface where supported. OS window
chrome, transparency settings, and material support differ.

The initial content size is 660 × 252 logical pixels, with native limits of
400 × 200 through 1280 × 720. macOS overlays its native traffic lights on the
header and hides the title; Windows retains its native title bar. The header’s
unused area drags the window (the only additional frontend permission).
Normal close/quit remembers logical content size and physical desktop position in
`picker-window.json` beside settings. Startup clamps them to the current monitor
work area, falling back to the primary monitor if the previous display is absent.
Maximized/minimized/fullscreen geometry is not saved. Abnormal termination can
lose the latest resize. No resize handler changes native dimensions during use.

A ResizeObserver measures the thumbnail viewport, excluding the header and fixed
footer. A small fit function chooses columns and the largest consistent 0.88:1
width/height frames that fit both dimensions, capped at 220 pixels wide. Ten slots
can form one wide row or multiple rows; a narrow/tall picker uses more rows.
At the smallest sizes, only the grid scrolls, with the toolbar still visible.
One or few slots remain centered rather than stretching into wide rectangles;
deliberately large windows can therefore retain some space around those frames.

Written [product rules](PRODUCT.md) supersede obsolete reference interactions:

- Header: Store in Slot, Save to Device, and Settings. First-run settings offers
  Pictures or another export folder. Settings also chooses 1–10 slots.
- Bottom toolbar: **Save** and **Clear** only. Native file drag-out is primary;
  no Copy/Paste controls or paste shortcuts.
- Click selects; Shift selects a contiguous range; Cmd (macOS) / Ctrl (Windows)
  toggles individuals. Dragging a selected occupied thumbnail preserves the group
  and offers selected occupied PNGs together, with source-preserving copy semantics.
- Empty numbered slots use a dashed frame and centered plus, without a text label.
  Select one and use Store in Slot to populate it. No thumbnail context menus or
  reordering. Preview images crop with object-fit: cover; storage, exports, and
  drag retain original resolution/aspect ratio. Small top-left badges label slots;
  original dimensions are available in the hover tooltip and accessible label.
- Visible focus, descriptive button labels, Tab, arrows/Home/End, and Space/Enter
  support keyboard use. Confirmation dialogs handle Escape/cancellation and keep
  destructive actions explicit. Counts/revisions are also validated in Rust.
- Settings is a modal sheet, so changing preferences never displaces the grid.
  Capture controls stay quiet in the header; Save/Clear sit immediately under the
  grid. A compact selection count replaces permanent gesture instructions.
- Activity details report capture, export, drag, and filesystem results without
  reserving a permanent status panel. Errors/cleanup warnings leave a visible
  Needs attention control. Its Activity sheet offers Reload only for recovery;
  Reload retries metadata loading and obsolete-file cleanup.

Native fidelity and cross-platform behavior require runtime validation; neither
the reference image nor a successful build establishes receiving-app compatibility.
Task 004 adds General and Shortcuts sections inside the existing scrollable Settings
sheet. Shortcut rows show native modifier labels, click-to-record state, individual
Off controls, errors, and Reset shortcuts. Escape cancels recording before closing
the sheet. The picker dimensions, grid, materials, toolbar, native resizing, and
selection rules are unchanged. Closing hides the same picker; opening restores its
selection and geometry. Native menus remain minimal: Open Slotshot, Store in Slot,
Save to Device, Settings, Quit Slotshot. The macOS crop-corner template icon follows
system appearance; Windows uses the existing color application icon.

Task 003B.2 native validation used only `~/Applications/Slotshot.app`, rebuilt via
`npm run app:test:mac` with Apple Development signing and strict bundle verification.
Temporary in-memory synthetic previews exercised 1/5/10 slots, empty/occupied/mixed
content, selection, default/minimum/enlarged/wide-short/narrow-tall windows, and
keyboard access to scrolled slots. They were removed before the final build; no
fixture mode ships. The final app restored a 900 × 400 resize after Cmd+Q/reopen,
opened the real capture overlay without reauthorization, and recovered after Escape.
Occupied-count reduction was cancelled safely. All existing slot PNGs, thumbnails,
and manifest remained byte-for-byte unchanged. Automated native pointer drags did
not establish file-drag initiation/transfer; repeat the existing manual receiving-app
checks. Windows GUI, display hot-plug/mixed-DPI restore, and reduced-transparency
appearance remain unverified. Native materials/chrome differ from the reference;
small windows intentionally scroll only the grid, and few slots stay centered.

The Task 003B.3 development material selector, synthetic thumbnail previews, and
associated native commands/permissions were removed in Task 003B.4. The picker
always loads real screenshot thumbnails. There is no material preference or test
mode to persist, and no migration or deletion of existing screenshot data.

Final Task 003B.4 native checks on macOS 26.6.2 confirmed the real-thumbnail picker,
normal Settings sheet, native zoom/restore and geometry retention, a real capture
stored in empty slot 4 and restored after restart, and readable Regular glass with
Reduce Transparency on (restored off afterward). The user confirmed enabled and
disabled labels over light and dark desktop backgrounds. Windows/older-macOS runtime
and receiving-app drag compatibility remain unverified; the automated layout and
selection tests continue to pass.
