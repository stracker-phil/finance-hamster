import { describe, expect, it } from "vitest";
import { groupsOf, resolvePage, resolveTarget, searchSettings, type SettingsPage } from "./registry";

const page = (id: string, label: string, group: SettingsPage["group"], rows: SettingsPage["rows"] = []): SettingsPage => ({ id, group, label, icon: "settings", Body: () => null, rows });

const PAGES = [
  page("rules", "Tax Rules", "Finance", [{ id: "reload", title: "Reload rules", keywords: ["refresh"] }]),
  page("look", "Appearance", "App", [{ id: "mode", title: "Mode", keywords: ["dark", "light"] }]),
  page("keys", "Shortcuts", "App"),
];

describe("resolvePage", () => {
  it("returns the page with the id", () => {
    expect(resolvePage(PAGES, "keys").id).toBe("keys");
  });

  it("returns the page that holds the row with the id", () => {
    expect(resolvePage(PAGES, "mode").id).toBe("look");
  });

  it.each([undefined, "missing"])("falls back to the first page for %s", (id) => {
    expect(resolvePage(PAGES, id).id).toBe("rules");
  });
});

describe("resolveTarget", () => {
  const withDetail = (p: SettingsPage): SettingsPage => ({ ...p, detail: { title: (id) => `Rules ${id}`, Body: () => null } });
  const pages = [withDetail(PAGES[0]), PAGES[1], PAGES[2]];

  it("returns the page and the detail for <page id>/<detail id> on a page with details", () => {
    const target = resolveTarget(pages, "rules/2026");

    expect(target.page.id).toBe("rules");
    expect(target.detail).toBe("2026");
  });

  it("keeps slashes inside the detail id", () => {
    expect(resolveTarget(pages, "rules/a/b").detail).toBe("a/b");
  });

  it("returns the page without a detail for a plain page id", () => {
    expect(resolveTarget(pages, "look")).toEqual({ page: PAGES[1] });
  });

  it("returns the page that holds a row id, without a detail", () => {
    const target = resolveTarget(pages, "mode");

    expect(target.page.id).toBe("look");
    expect(target.detail).toBeUndefined();
  });

  it("falls back for a detail on a page without details", () => {
    const target = resolveTarget(pages, "keys/x");

    expect(target.page.id).toBe("rules");
    expect(target.detail).toBeUndefined();
  });

  it.each([undefined, "missing", "missing/x"])("falls back to the first page for %s", (id) => {
    const target = resolveTarget(pages, id);

    expect(target.page.id).toBe("rules");
    expect(target.detail).toBeUndefined();
  });
});

describe("searchSettings", () => {
  const found = (query: string) => searchSettings(PAGES, query).map((match) => (match.row ? `${match.page.id}/${match.row.id}` : match.page.id));

  it("finds a page by its label", () => {
    expect(found("short")).toEqual(["keys"]);
  });

  it("finds a row by its title and by its keywords", () => {
    expect(found("mode")).toEqual(["look/mode"]);
    expect(found("dark")).toEqual(["look/mode"]);
  });

  it("finds a page by its group name", () => {
    expect(found("finance")).toEqual(["rules"]);
  });

  it("lists the best match first", () => {
    expect(found("re")).toEqual(["rules/reload", "rules", "look"]);
  });

  it("finds nothing for a query that matches no page or row", () => {
    expect(found("zzzz")).toEqual([]);
  });
});

describe("groupsOf", () => {
  it("collects pages in runs of the same group in order", () => {
    const groups = groupsOf(PAGES).map(({ group, pages }) => [group, pages.map((p) => p.id)]);

    expect(groups).toEqual([
      ["Finance", ["rules"]],
      ["App", ["look", "keys"]],
    ]);
  });

  it("starts a new group when the group name repeats after another", () => {
    const pages = [page("a", "A", "App"), page("b", "B", "Finance"), page("c", "C", "App")];

    expect(groupsOf(pages).map((g) => g.group)).toEqual(["App", "Finance", "App"]);
  });

  it("returns no groups for no pages", () => {
    expect(groupsOf([])).toEqual([]);
  });
});
