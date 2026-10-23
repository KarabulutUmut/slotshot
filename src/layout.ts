// Fit tall, consistent frames to the actual grid viewport, including its height.
// If even the smallest frames cannot fit, only the thumbnail area scrolls.
export function fitSlots(count: number, width: number, height: number) {
  const gap = 14;
  const ratio = 0.88;
  let best = { columns: 1, width: 56, height: 56 / ratio, scroll: true };
  let largest = 0;
  for (let columns = 1; columns <= count; columns++) {
    const rows = Math.ceil(count / columns);
    const size = Math.floor(
      Math.min(
        (width - gap * (columns - 1)) / columns,
        ((height - gap * (rows - 1)) / rows) * ratio,
        220,
      ),
    );
    if (size >= 56 && size > largest) {
      largest = size;
      best = { columns, width: size, height: size / ratio, scroll: false };
    }
  }
  if (!largest) {
    const columns = Math.max(
      1,
      Math.min(count, Math.floor((width + gap) / (72 + gap))),
    );
    const size = Math.max(
      48,
      Math.min(100, Math.floor((width - gap * (columns - 1)) / columns)),
    );
    best = { columns, width: size, height: size / ratio, scroll: true };
  }
  return best;
}

export function nextSlotIndex(
  current: number,
  key: string,
  columns: number,
  count: number,
) {
  if (key === "Home") return 0;
  if (key === "End") return count - 1;
  const offset =
    key === "ArrowUp"
      ? -columns
      : key === "ArrowDown"
        ? columns
        : key === "ArrowLeft"
          ? -1
          : 1;
  return Math.max(0, Math.min(count - 1, current + offset));
}
