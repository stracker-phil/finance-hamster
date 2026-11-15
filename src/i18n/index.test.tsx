import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { setLanguage, t, useT } from "./index";

function Heading() {
  return <h1>{useT().appearance.languageGroup}</h1>;
}

describe("i18n", () => {
  it("speaks English until a language is set", () => {
    expect(t().appearance.languageGroup).toBe("Language");
    expect(document.documentElement.lang).not.toBe("de");
  });

  it("switches t() to German and marks the document as German", () => {
    setLanguage("de");

    expect(t().appearance.languageGroup).toBe("Sprache");
    expect(document.documentElement.lang).toBe("de");
  });

  it("renders a component that uses useT again when the language changes", () => {
    render(<Heading />);
    expect(screen.getByRole("heading", { name: "Language" })).toBeTruthy();

    act(() => setLanguage("de"));

    expect(screen.getByRole("heading", { name: "Sprache" })).toBeTruthy();
  });

  it("switches back to English", () => {
    setLanguage("de");

    setLanguage("en");

    expect(t().appearance.languageGroup).toBe("Language");
    expect(document.documentElement.lang).toBe("en");
  });
});
