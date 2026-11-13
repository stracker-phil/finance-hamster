import { describe, expect, it } from "vitest";
import { groupsOf, resolvePage, searchSettings, type SettingsPage } from "./registry";

const page = (id: string, label: string, group: SettingsPage["group"], rows: SettingsPage["rows"] = []): SettingsPage => ({
  id,
  group,
  label,
  icon: "settings",
  Body: () => null,
  rows,
});

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
