import { describe, expect, it } from "vitest";
import { de } from "./de";
import { en } from "./en";

interface Leaf {
  path: string;
  value: unknown;
}

function leaves(node: unknown, path = ""): Leaf[] {
  if (node !== null && typeof node === "object") return Object.entries(node).flatMap(([key, child]) => leaves(child, path ? `${path}.${key}` : key));
  return [{ path, value: node }];
}

const english = leaves(en);
const german = new Map(leaves(de).map((leaf) => [leaf.path, leaf.value]));

describe("message catalogs", () => {
  it("gives every English text a German one of the same kind", () => {
    const wrong = english.filter(({ path, value }) => !german.has(path) || typeof german.get(path) !== typeof value).map(({ path }) => path);

    expect(wrong).toEqual([]);
  });

  it("has no German text that the English catalog lacks", () => {
    const known = new Set(english.map(({ path }) => path));

    expect([...german.keys()].filter((path) => !known.has(path))).toEqual([]);
  });

  it("has no empty German text", () => {
    const empty = [...german].filter(([, value]) => typeof value === "string" && value.trim() === "").map(([path]) => path);

    expect(empty).toEqual([]);
  });

  it("translates most texts instead of copying the English ones", () => {
    const texts = english.filter(({ value }) => typeof value === "string");
    const copied = texts.filter(({ path, value }) => german.get(path) === value);

    expect(copied.length / texts.length).toBeLessThan(0.2);
  });
});
