import type { ComponentType, ReactNode } from "react";
import type { IconName } from "../components/icons";
import { ranked } from "../lib/rank";

/** A group or row the search finds: `id` is its `anchor` on the page, `title` its label as shown. */
export interface SettingsRow {
  id: string;
  title: string;
  /** More words the search finds the row by. */
  keywords?: string[];
}

export interface SettingsPage {
  id: string;
  /** The heading it is listed under; pages with the same group stand together. */
  group: string;
  label: string;
  /** More words the search finds the page by: its label and group in the other language. */
  aliases?: string[];
  icon: IconName;
  intro?: ReactNode;
  Body: ComponentType;
  rows: SettingsRow[];
  /** A page one level down, addressed as `<page id>/<detail id>` and opened from a `DrillRow`. */
  detail?: { title: (id: string) => string; Body: ComponentType<{ id: string }> };
}

/** A search hit; `row` is absent when the page itself matched. */
export interface SettingsMatch {
  page: SettingsPage;
  row?: SettingsRow;
}

/** The named page, the page holding the named row, or the first when the name is absent or unknown. */
export function resolvePage(pages: SettingsPage[], id: string | undefined): SettingsPage {
  return pages.find((p) => p.id === id) ?? pages.find((p) => p.rows.some((r) => r.id === id)) ?? pages[0];
}

/** The page a request names and, for `<page id>/<detail id>` on a page with details, the detail it opens. */
export function resolveTarget(pages: SettingsPage[], id: string | undefined): { page: SettingsPage; detail?: string } {
  const [pageId, detail] = id?.split(/\/(.*)/s) ?? [];
  const page = pages.find((p) => p.id === pageId && p.detail);
  return page && detail ? { page, detail } : { page: resolvePage(pages, id) };
}

/** Every page and row matching `query`, best first, ranked like the palette. */
export function searchSettings(pages: SettingsPage[], query: string): SettingsMatch[] {
  const entries = pages.flatMap((page): SettingsMatch[] => [{ page }, ...page.rows.map((row) => ({ page, row }))]);
  return ranked(entries, query, ({ page, row }) => (row ? [row.title, (row.keywords ?? []).join(" ")] : [page.label, [page.group, ...(page.aliases ?? [])].join(" ")]));
}

/** The pages in their order, in runs that share a group. */
export function groupsOf(pages: SettingsPage[]): { group: string; pages: SettingsPage[] }[] {
  const groups: { group: string; pages: SettingsPage[] }[] = [];
  for (const page of pages) {
    const last = groups.at(-1);
    if (last?.group === page.group) last.pages.push(page);
    else groups.push({ group: page.group, pages: [page] });
  }
  return groups;
}
