import test from "node:test";
import assert from "node:assert/strict";
import {
  defaultShortcuts,
  displayShortcut,
  recordedShortcut,
} from "../src/shortcuts.ts";

test("default shortcut labels use platform modifiers and preserve disabled assignments", () => {
  const config = defaultShortcuts();
  assert.deepEqual(config.keys, [
    "Control+Alt+Shift+1",
    "Control+Alt+Shift+2",
    "Control+Alt+Shift+3",
  ]);
  assert.equal(displayShortcut(config.keys[0], true), "⌃⌥⇧1");
  assert.equal(displayShortcut(config.keys[1], false), "Ctrl+Alt+Shift+2");
  assert.equal(displayShortcut("shift+alt+super+KeyK", true), "⇧⌥⌘K");
  assert.equal(displayShortcut(null, false), "Disabled");
});
test("record physical codes while ignoring repeat, modifier-only and Escape keys", () => {
  const event = {
    ctrlKey: true,
    altKey: true,
    shiftKey: false,
    metaKey: false,
    repeat: false,
    code: "KeyK",
  };
  assert.equal(recordedShortcut(event), "Control+Alt+KeyK");
  for (const code of [
    "AltLeft",
    "ControlRight",
    "Escape",
    "Tab",
    "PrintScreen",
    "F25",
  ]) {
    assert.equal(recordedShortcut({ ...event, code }), null);
  }
  assert.equal(recordedShortcut({ ...event, repeat: true }), null);
});
