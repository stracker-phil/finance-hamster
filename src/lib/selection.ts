/** The text selected inside `root`, on one line; empty when the selection is elsewhere. */
export function selectedIn(root: HTMLElement): string {
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || !selection.anchorNode || !root.contains(selection.anchorNode)) return "";
  return selection.toString().split(/\s+/).join(" ").trim();
}
