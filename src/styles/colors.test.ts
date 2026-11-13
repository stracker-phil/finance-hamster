/// <reference types="node" />
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { describe, expect, it } from "vitest";

// Read from disk: Vitest blanks every CSS module unless CSS processing is on.
const srcDir = resolve(process.cwd(), "src");
const palettesDir = join(srcDir, "styles", "palettes") + sep;

const all: [string, string][] = readdirSync(srcDir, { recursive: true, encoding: "utf8" })
  .filter((rel: string) => /\.(css|ts|tsx)$/.test(rel))
  .map((rel: string): [string, string] => [join(srcDir, rel), readFileSync(join(srcDir, rel), "utf8")]);

const paletteFiles = all.filter(([path]) => path.startsWith(palettesDir) && path.endsWith(".css"));
const scanned = all.filter(([path]) => !path.startsWith(palettesDir) && !/\.test\.tsx?$/.test(path) && !path.endsWith("types.gen.ts"));

const stripComments = (text: string) => text.replace(/\/\*[\s\S]*?\*\//g, "");

// CSS: any hex color. TS(X): only a hex inside a string literal, so "#3229" in prose passes.
const CSS_HEX = /#[0-9a-fA-F]{3,8}\b/g;
const TS_HEX = /["'`]#[0-9a-fA-F]{3,8}["'`]/g;
const FUNCTIONAL = /\b(?:rgba?|hsla?)\(/g;
const NAMED =
  /\b(?:white|black|red|green|blue|gray|grey|orange|yellow|purple|pink|silver|navy|teal|lime|maroon|olive|aqua|fuchsia|brown|gold|cyan|magenta|indigo|violet)\b/i;

/** Named colors in declaration values; selectors, strings and `var(--x-red)` names are left out. */
function namedColors(css: string): string[] {
  const text = stripComments(css)
    .replace(/var\([^)]*\)/g, "")
    .replace(/(["'])(?:(?!\1).)*\1/g, "");
  return [...text.matchAll(/[\w-]+\s*:\s*([^;{}]+)[;}]/g)].map((m) => m[1]).filter((value) => NAMED.test(value));
}

function colorLiterals(path: string, source: string): string[] {
  if (!path.endsWith(".css")) return [TS_HEX, FUNCTIONAL].flatMap((re) => source.match(re) ?? []);
  const text = stripComments(source);
  return [...[CSS_HEX, FUNCTIONAL].flatMap((re) => text.match(re) ?? []), ...namedColors(source)];
}

function customProperties(source: string): string[] {
  return [...stripComments(source).matchAll(/^\s*(--[\w-]+)\s*:/gm)].map((m) => m[1]).sort();
}

describe("color literals", () => {
  it("live only in the palette files", () => {
    const offenders = scanned
      .map(([path, source]) => ({ path, found: colorLiterals(path, source) }))
      .filter((o) => o.found.length > 0)
      .map((o) => `${o.path}: ${o.found.join(", ")}`);
    expect(offenders).toEqual([]);
  });

  it("scan sees the stylesheets and flags every kind of literal", () => {
    expect(scanned.filter(([path]) => path.endsWith(".css")).length).toBeGreaterThan(1);
    expect(scanned.length).toBeGreaterThan(10);
    expect(colorLiterals("a.css", "color: #fa8d3e;")).toHaveLength(1);
    expect(colorLiterals("a.tsx", 'const c = "#fa8d3e";')).toHaveLength(1);
    expect(colorLiterals("a.css", "box-shadow: 0 0 4px rgba(0, 0, 0, 0.2)")).toHaveLength(1);
    expect(colorLiterals("a.css", "color: hsl(10 20% 30%);")).toHaveLength(1);
    expect(colorLiterals("a.css", "background: white;")).toHaveLength(1);
    expect(colorLiterals("a.css", "border: 1px solid Red }")).toHaveLength(1);
  });

  it("scan lets tokens, keywords and prose through", () => {
    expect(colorLiterals("a.css", "background: transparent; color: currentColor")).toEqual([]);
    expect(colorLiterals("a.css", "color: var(--pill-red-text);")).toEqual([]);
    expect(colorLiterals("a.css", ':root[data-accent="red"] { --accent: var(--accent-red); }')).toEqual([]);
    expect(colorLiterals("a.tsx", 'label: "Order #4711"')).toEqual([]);
  });
});

describe("palette files", () => {
  it("include the light and the dark palette", () => {
    const names = paletteFiles.map(([path]) => path.slice(palettesDir.length));
    expect(names).toEqual(expect.arrayContaining(["light.css", "dark.css"]));
  });

  it("all define the same custom properties", () => {
    const light = paletteFiles.find(([path]) => path.endsWith(`${sep}light.css`));
    const expected = customProperties(light![1]);
    expect(expected.length).toBeGreaterThan(0);
    for (const [path, source] of paletteFiles) {
      expect({ path, tokens: customProperties(source) }).toEqual({ path, tokens: expected });
    }
  });
});
