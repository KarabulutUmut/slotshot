# Slotshot product requirements

Task 004 extends the file-based workflow with native global shortcuts and background operation.
Native compatibility still requires the runtime validation in [README](../README.md).
Slotshot stays local-only: no server, accounts, database, analytics, or cloud storage.

## Capture and reuse

1. Choose an export directory during first-run setup, offering OS Pictures as the
   default. Settings changes the directory and slot count (1–10, default 5).
2. **Store in Slot** captures into the selected single slot, or the first empty
   slot if there is no single selection. Explicitly confirm occupied replacement.
   Each slot stores one full-resolution PNG and a generated thumbnail.
3. **Save to Device** captures directly to the configured export folder without
   changing slots. Both destinations reuse frozen native region capture; Escape
   or Cancel preserves existing files. No cross-display stitching.
4. Select with click, Shift ranges, or Cmd (macOS) / Ctrl (Windows) individual
   toggles. Dragging an already selected occupied thumbnail preserves the group
   and starts one native file drag for all selected occupied PNGs. Empty slots
   offer no files. Copy semantics preserve internal sources.
5. Toolbar **Save** exports selected occupied slots as independent PNG files,
   without overwriting earlier exports. Report successes and per-file failures.
   **Clear** permanently removes selected internal images and thumbnails after
   confirmation; separately exported files remain independent.

## Binding slot lifecycle

| Action          | Result                                                                                                                                |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Restart         | Restore slot count, assignments, full-resolution PNGs, and export setting.                                                            |
| Replace         | Commit the new PNG/thumbnail and metadata safely before deleting previous files.                                                      |
| Clear           | Permanently remove selected PNGs/thumbnails; leave numbered empty slots.                                                              |
| Increase count  | Keep existing images; add empty slots.                                                                                                |
| Decrease count  | Confirm if affected slots are occupied; delete all images/thumbnails above the count. Cancel leaves count/files unchanged.            |
| Cleanup failure | Show an error, retain retry information on disk, retry with Reload/restart. Never claim files were removed while deletion is failing. |
| Export          | Independent PNG copies; later slot changes never delete or modify them.                                                               |

Do not silently retain screenshots in hidden/removed slots. Thumbnails are
regenerable derivatives, never the only content copy. Deletion means ordinary
filesystem deletion, without a secure-erasure guarantee.

## Picker and boundaries

Preserve [Template 1](DESIGN.md): a compact dark translucent resizable picker,
numbered thumbnails, blue selection, Save/Clear toolbar, keyboard access, and
native drag-out as the primary transfer action. No thumbnail context menus or
reordering. Native OS effects and receiving-app support differ by platform;
never promise universal multiple-file drops.

No system clipboard Copy/Paste, paste simulation, clipboard monitoring/restoration,
or interception of standard Cmd/Ctrl+V. No screenshot bytes in browser persistence.
Use thumbnails, bounded capture memory, and event-driven idle behavior; measure
resource usage before making performance claims.

## Global shortcuts and background lifecycle

Store in Slot, Save to Device, and Show/Hide Picker have independently configurable
shortcuts. Initial defaults are Control+Option+Shift+1/2/3 on macOS and
Ctrl+Alt+Shift+1/2/3 on Windows. Settings records combinations, cancels with Escape,
disables individual actions, and resets defaults. Rust validates duplicates,
modifier-only/invalid input, and protected editing/system combinations before
registration. Failed reassignment retains the previous preference and restores
registrations; exceptional OS rollback errors are reported. One unavailable default
must not disable the other actions. Preferences live locally across restarts.

Native captures use an empty slot only, including when the picker is visible.
Full slots reveal instructions to select and explicitly confirm replacement in the
picker. Successful Store reveals/selects the new slot; background Save leaves the
picker hidden and changes no slots. Cancellation/failure restores prior visibility.
First-run export setup precedes either capture destination. All capture entry points
share one engine and operation lease. Repeated actions cannot queue captures or
raise the picker over overlays.

Closing hides the picker; explicit Quit terminates Slotshot. Native minimize stays
unchanged. Ordinary launch shows the picker; `--background` starts hidden. A second
launch focuses the existing instance, with the existing storage lock retained.
No autostart, hidden Dock policy, keyboard monitoring, or Accessibility grant is added.
The native menu bar/notification menu contains Open Slotshot, Store in Slot, Save to
Device, Settings, and Quit Slotshot. macOS icon click opens its menu; Windows left
click opens the picker and right click opens the menu.

Editing, annotations, installers, signed releases, and updates remain future work.
