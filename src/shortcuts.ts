export type ShortcutConfig = { version: number; keys: (string | null)[] };
export type ShortcutView = {
  config: ShortcutConfig;
  errors: (string | null)[];
};
export const defaultShortcuts = (): ShortcutConfig => ({
  version: 1,
  keys: [1, 2, 3].map((key) => `Control+Alt+Shift+${key}`),
});
export function displayShortcut(value: string | null, mac: boolean): string {
  if (!value) return "Disabled";
  const parts = value
    .toLowerCase()
    .split("+")
    .map((part) => {
      if (part === "control" || part === "ctrl") return mac ? "⌃" : "Ctrl";
      if (part === "alt") return mac ? "⌥" : "Alt";
      if (part === "shift") return mac ? "⇧" : "Shift";
      if (["super", "cmd", "command"].includes(part)) return mac ? "⌘" : "Win";
      return part.replace(/^(key|digit)/, "").toUpperCase();
    });
  return parts.join(mac ? "" : "+");
}
// Physical KeyboardEvent.code avoids Option-produced glyphs and keyboard-layout ambiguity.
export function recordedShortcut(event: {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  repeat: boolean;
}): string | null {
  if (
    event.repeat ||
    !/^(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-9]|2[0-4]))$/.test(event.code)
  )
    return null;
  return [
    event.ctrlKey && "Control",
    event.altKey && "Alt",
    event.shiftKey && "Shift",
    event.metaKey && "Super",
    event.code,
  ]
    .filter(Boolean)
    .join("+");
}
