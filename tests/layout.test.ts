import assert from "node:assert/strict";
import test from "node:test";
import { fitSlots, nextSlotIndex } from "../src/layout.ts";

for (const count of [1, 5, 10]) {
  for (const [width, height] of [
    [616, 148],
    [356, 96],
    [856, 296],
    [1216, 96],
    [356, 596],
  ]) {
    test(`${count} slots fit or remain scrollable in ${width} × ${height}`, () => {
      const layout = fitSlots(count, width, height);
      const rows = Math.ceil(count / layout.columns);
      assert.ok(
        layout.width * layout.columns + 14 * (layout.columns - 1) <= width,
      );
      assert.ok(layout.width >= 48 && layout.width <= 220);
      assert.ok(Math.abs(layout.width / layout.height - 0.88) < 0.001);
      if (!layout.scroll)
        assert.ok(layout.height * rows + 14 * (rows - 1) <= height);
    });
  }
}
test("height changes size and wrapping, width can put ten slots in one row", () => {
  const short = fitSlots(10, 616, 148);
  const tall = fitSlots(10, 616, 600);
  assert.ok(tall.width > short.width);
  assert.ok(tall.columns < short.columns);
  assert.equal(fitSlots(10, 1216, 148).columns, 10);
  assert.equal(fitSlots(10, 616, 300).columns, 5);
  assert.equal(fitSlots(10, 616, 148).columns, 5);
  assert.equal(fitSlots(10, 616, 148).scroll, false);
});
test("extreme short heights scroll the grid instead of squeezing out the toolbar", () => {
  assert.equal(fitSlots(10, 350, 50).scroll, true);
});

test("keyboard navigation follows responsive columns and clamps incomplete rows", () => {
  assert.equal(nextSlotIndex(1, "ArrowDown", 5, 10), 6);
  assert.equal(nextSlotIndex(1, "ArrowDown", 3, 10), 4);
  assert.equal(nextSlotIndex(9, "ArrowUp", 3, 10), 6);
  assert.equal(nextSlotIndex(8, "ArrowDown", 3, 10), 9);
  assert.equal(nextSlotIndex(0, "ArrowLeft", 3, 10), 0);
  assert.equal(nextSlotIndex(9, "ArrowRight", 3, 10), 9);
  assert.equal(nextSlotIndex(5, "Home", 3, 10), 0);
  assert.equal(nextSlotIndex(2, "End", 3, 10), 9);
});
