/// <reference types="node" />
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { contrastRatio, hexToRgb, over, type Rgb } from "./contrast";

const SCHEMES = ["light", "dark"] as const;

const sheets = Object.fromEntries(
  SCHEMES.map((scheme) => [scheme, readFileSync(join(resolve(process.cwd(), "src"), "styles", "palettes", `${scheme}.css`), "utf8")]),
);

describe("contrastRatio", () => {
  it("is 21 for black on white and 1 for identical colors", () => {
    expect(contrastRatio(hexToRgb("#000000"), hexToRgb("#ffffff"))).toBeCloseTo(21, 5);
    expect(contrastRatio(hexToRgb("#777777"), hexToRgb("#777777"))).toBe(1);
  });
});

// "system" is left out: its colors come from the CSS `AccentColor` keyword, which only the browser resolves.
const ACCENTS = ["orange", "gold", "red", "magenta", "purple", "indigo", "blue", "teal", "graphite"] as const;

/** Resolves a palette token (hex, rgba(), var(--x)) to a color; a translucent color lands on `bg`. */
function paletteColors(css: string) {
  function token(name: string): string {
    const m = css.match(new RegExp(`^\\s*${name}:\\s*([^;]+);`, "m"));
    if (!m) throw new Error(`missing token ${name}`);
    return m[1].trim();
  }

  function resolveValue(value: string, bg: Rgb): Rgb {
    if (value.startsWith("#")) return hexToRgb(value);
    const variable = value.match(/^var\((--[\w-]+)\)$/);
    if (variable) return resolveValue(token(variable[1]), bg);
    const rgba = value.match(/^rgba\(\s*(\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)$/);
    if (rgba) return over([+rgba[1] / 255, +rgba[2] / 255, +rgba[3] / 255], +rgba[4], bg);
    throw new Error(`cannot resolve ${value}`);
  }

  const panel = resolveValue(token("--panel-bg"), [1, 1, 1]);

  /** Contrast of `fg` on `bg`, both over the panel when translucent. */
  return (fg: string, bg: string) => {
    const background = resolveValue(token(bg), panel);
    return contrastRatio(resolveValue(token(fg), background), background);
  };
}

describe.each(SCHEMES)("%s palette contrast", (scheme) => {
  const ratio = paletteColors(sheets[scheme]);

  it.each(ACCENTS)("the %s accent's text shade reads at 4.5:1 on the panel", (accent) => {
    expect(ratio(`--accent-${accent}-text`, "--panel-bg")).toBeGreaterThanOrEqual(4.5);
  });

  it.each(["--text", "--text-secondary", "--text-tertiary", "--text-muted"])("%s reads at 4.5:1 on the panel", (fg) => {
    expect(ratio(fg, "--panel-bg")).toBeGreaterThanOrEqual(4.5);
  });

  it.each(["--text", "--text-secondary", "--text-tertiary"])("%s reads at 4.5:1 on the sidebar", (fg) => {
    expect(ratio(fg, "--window-bg")).toBeGreaterThanOrEqual(4.5);
  });

  it("muted text reads at 3:1 on the sidebar", () => {
    expect(ratio("--text-muted", "--window-bg")).toBeGreaterThanOrEqual(3);
  });

  it.each(["anv", "teal", "green", "blue", "red", "violet"])("the %s pill reads at 4.5:1", (name) => {
    expect(ratio(`--pill-${name}-text`, `--pill-${name}-bg`)).toBeGreaterThanOrEqual(4.5);
  });

  it.each(["sub", "doc", "miss"])("the %s chip reads at 4.5:1", (name) => {
    expect(ratio(`--chip-${name}-text`, `--chip-${name}-bg`)).toBeGreaterThanOrEqual(4.5);
  });

  it.each(["info", "error"])("the %s banner reads at 4.5:1", (name) => {
    expect(ratio(`--banner-${name}-text`, `--banner-${name}-bg`)).toBeGreaterThanOrEqual(4.5);
  });
});
