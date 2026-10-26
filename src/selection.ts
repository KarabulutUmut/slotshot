export type SelectionState = { ids: number[]; anchor: number | null };
export type Modifiers = { shift: boolean; toggle: boolean };

export function selectSlot(
  current: SelectionState,
  id: number,
  modifiers: Modifiers,
  preserveGroup = false,
): SelectionState {
  if (modifiers.shift && current.anchor !== null) {
    const start = Math.min(current.anchor, id);
    const end = Math.max(current.anchor, id);
    return {
      anchor: current.anchor,
      ids: Array.from({ length: end - start + 1 }, (_, i) => start + i),
    };
  }
  if (modifiers.toggle) {
    return {
      anchor: id,
      ids: current.ids.includes(id)
        ? current.ids.filter((item) => item !== id)
        : [...current.ids, id].sort((a, b) => a - b),
    };
  }
  if (preserveGroup && current.ids.includes(id)) return current;
  return { anchor: id, ids: [id] };
}

/** Destination shortcuts are active only while a pending capture owns the picker. */
export function destinationSlot(key: string, count: number): number | null {
  if (!/^[0-9]$/.test(key)) return null;
  const id = key === "0" ? 10 : Number(key);
  return id <= count ? id : null;
}
