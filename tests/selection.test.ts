import assert from "node:assert/strict";
import test from "node:test";
import { selectSlot } from "../src/selection.ts";

const plain = { shift: false, toggle: false };
const toggle = { shift: false, toggle: true };
const shift = { shift: true, toggle: false };

test("plain selection, individual toggles, and reverse ranges", () => {
  const first = selectSlot({ ids: [], anchor: null }, 5, plain);
  assert.deepEqual(selectSlot(first, 2, shift), {
    ids: [2, 3, 4, 5],
    anchor: 5,
  });
  const added = selectSlot(first, 2, toggle);
  assert.deepEqual(added, { ids: [2, 5], anchor: 2 });
  assert.deepEqual(selectSlot(added, 5, toggle).ids, [2]);
  assert.deepEqual(selectSlot(added, 3, plain).ids, [3]);
});

test("dragging a selected thumbnail preserves the group; an unselected one replaces it", () => {
  const group = { ids: [1, 3, 4], anchor: 1 };
  assert.deepEqual(selectSlot(group, 3, plain, true), group);
  assert.deepEqual(selectSlot(group, 2, plain, true).ids, [2]);
  assert.deepEqual(selectSlot(group, 1, plain).ids, [1]);
});

test("range without an anchor selects one slot; ranges include empty positions", () => {
  assert.deepEqual(selectSlot({ ids: [], anchor: null }, 3, shift).ids, [3]);
  assert.deepEqual(
    selectSlot({ ids: [1], anchor: 1 }, 4, shift).ids,
    [1, 2, 3, 4],
  );
});
