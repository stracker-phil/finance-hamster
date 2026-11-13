import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { schemeFor, useAppearance } from "./theme";

describe("schemeFor", () => {
  it.each([
    ["system", true, "dark"],
    ["system", false, "light"],
    ["light", true, "light"],
    ["dark", false, "dark"],
  ] as const)("resolves %s with a dark system %s to %s", (appearance, darkSystem, scheme) => {
    expect(schemeFor(appearance, darkSystem)).toBe(scheme);
  });
});

describe("useAppearance", () => {
  it("sets the scheme and accent on the root element", async () => {
    renderHook(() => useAppearance("dark", "teal"));

    await waitFor(() => {
      expect(document.documentElement.dataset.scheme).toBe("dark");
      expect(document.documentElement.dataset.accent).toBe("teal");
    });
  });

  it("follows the light system scheme on Automatic", async () => {
    renderHook(() => useAppearance("system", "blue"));

    await waitFor(() => expect(document.documentElement.dataset.scheme).toBe("light"));
  });
});
