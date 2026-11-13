import { describe, expect, it } from "vitest";
import { acceleratorOf } from "./shortcutKeys";

const press = (key: string, code: string, mods: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }> = {}) => ({
  key,
  code,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe("acceleratorOf", () => {
  it.each([
    ["⌘K", press("k", "KeyK", { metaKey: true }), "CmdOrCtrl+K"],
    ["⌘⇧K", press("K", "KeyK", { metaKey: true, shiftKey: true }), "CmdOrCtrl+Shift+K"],
    ["all four modifiers", press("j", "KeyJ", { shiftKey: true, altKey: true, ctrlKey: true, metaKey: true }), "CmdOrCtrl+Ctrl+Alt+Shift+J"],
    ["⌥ with a letter that types another character", press("å", "KeyA", { altKey: true }), "Alt+A"],
    ["a digit", press("1", "Digit1", { metaKey: true }), "CmdOrCtrl+1"],
    ["a shifted digit", press("!", "Digit1", { metaKey: true, shiftKey: true }), "CmdOrCtrl+Shift+1"],
    ["a named key", press("ArrowUp", "ArrowUp", { metaKey: true }), "CmdOrCtrl+ArrowUp"],
    ["a key without a code", press("k", "", { ctrlKey: true }), "Ctrl+K"],
    ["a plain key", press("F5", "F5"), "F5"],
  ])("turns %s into an accelerator", (_name, event, expected) => {
    expect(acceleratorOf(event)).toBe(expected);
  });

  it.each(["Meta", "Control", "Alt", "Shift"])("returns null while only %s is held", (key) => {
    expect(acceleratorOf(press(key, `${key}Left`, { metaKey: true, shiftKey: true }))).toBeNull();
  });
});
