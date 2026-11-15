import type { ReactNode } from "react";

/** A row of a menu, run when chosen. */
export interface MenuItem {
  text: string;
  run?: () => void;
  /** False greys the row out; it can't be chosen and the keys skip it. */
  enabled?: boolean;
  /** Makes the row a choice; true shows the check. */
  checked?: boolean;
  /** A destructive action, in red. */
  danger?: boolean;
  /** At the trailing end: a count, a hint or a key. */
  hint?: string;
  /** The tooltip, e.g. why the row is disabled. */
  title?: string;
  icon?: ReactNode;
}

/** A row, a separator line, or a caption that starts a group of rows. */
export type MenuEntry = MenuItem | "separator" | { caption: string };

/** Where a menu opens: at a point (a right-click) or under an element (a button, which also gets the focus back). */
export type MenuAt = { x: number; y: number } | HTMLElement;

export interface OpenMenu {
  /** Tells menus apart, so a new one starts fresh. */
  id: number;
  entries: MenuEntry[];
  at: MenuAt;
  /** Its accessible name. */
  label: string;
  /** Where the focus goes back to when it closes. */
  opener: HTMLElement | null;
}

export const isItem = (entry: MenuEntry): entry is MenuItem => entry !== "separator" && !("caption" in entry);
export const isEnabled = (entry: MenuEntry | undefined): entry is MenuItem => !!entry && isItem(entry) && entry.enabled !== false;
