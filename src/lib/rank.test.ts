import { describe, expect, it } from "vitest";
import { fold, rankMatch, ranked } from "./rank";

describe("fold", () => {
  it("drops case and accents", () => {
    expect(fold("Überweisung ÉCOLE")).toBe("uberweisung ecole");
  });
});

describe("rankMatch", () => {
  it("ranks a label prefix before a word start, all words and scattered letters", () => {
    const ranks = [
      rankMatch("Toggle Sidebar", "view.sidebar", "tog"),
      rankMatch("Show Sidebar", "view.sidebar", "sid"),
      rankMatch("Show Sidebar", "view.sidebar", "view sid"),
      rankMatch("Toggle Sidebar", "view.sidebar", "tsb"),
    ];

    expect(ranks).toEqual([0, 1, 2, 3]);
  });

  it("matches everything with an empty query", () => {
    expect(rankMatch("Anything", "", "")).toBe(0);
  });

  it("does not match when the letters are not in order", () => {
    expect(rankMatch("Toggle Sidebar", "view.sidebar", "zq")).toBeNull();
  });

  it("finds a word through the detail text", () => {
    expect(rankMatch("Open Settings", "settings.open", "settings open")).toBe(2);
  });

  it("ignores accents and case in the label", () => {
    expect(rankMatch("Überweisung", "", "uber")).toBe(0);
  });
});

describe("ranked", () => {
  const text = (name: string): [string, string] => [name, ""];

  it("puts the better match first and keeps the order of equal ones", () => {
    const items = ["Show Sidebar", "Sidebar Toggle", "Side Panel", "Inside"];

    expect(ranked(items, "side", text)).toEqual(["Sidebar Toggle", "Side Panel", "Show Sidebar", "Inside"]);
  });

  it("drops entries that do not match", () => {
    expect(ranked(["Alpha", "Beta"], "alp", text)).toEqual(["Alpha"]);
  });

  it("ignores surrounding spaces and case in the query", () => {
    expect(ranked(["Alpha", "Beta"], "  BET ", text)).toEqual(["Beta"]);
  });

  it("returns every item for an empty query", () => {
    expect(ranked(["Alpha", "Beta"], "", text)).toEqual(["Alpha", "Beta"]);
  });
});
