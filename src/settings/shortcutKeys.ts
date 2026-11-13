/** A key press as an accelerator (`CmdOrCtrl+Shift+K`); null while only modifiers are held. */
export function acceleratorOf(e: { key: string; code: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }): string | null {
  if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return null;
  const key = e.code.startsWith("Key") ? e.code.slice(3) : e.code.startsWith("Digit") ? e.code.slice(5) : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const parts = [e.metaKey && "CmdOrCtrl", e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", key].filter(Boolean);
  return parts.join("+");
}
