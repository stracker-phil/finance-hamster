import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { setLanguage } from "../i18n";
import { renderWithQuery } from "../test/renderWithQuery";
import { SettingsWindow } from "./SettingsWindow";

beforeEach(() => {
  vi.clearAllMocks();
});

const open = () => renderWithQuery(<SettingsWindow request={{ id: "about" }} onOpen={() => {}} />);

describe("SettingsWindow in German", () => {
  beforeEach(() => setLanguage("de"));

  it("lists the pages under German group and page names", () => {
    open();

    expect(screen.getByRole("navigation", { name: "Einstellungsseiten" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Finanzen" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Darstellung" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Händler" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Appearance" })).toBeNull();
  });

  it.each(["Sprache", "language"])("finds the language row when searching %s", async (query) => {
    open();

    await userEvent.type(screen.getByRole("searchbox", { name: "Einstellungen durchsuchen" }), query);

    expect(screen.getByRole("button", { name: "Sprache, Darstellung" })).toBeTruthy();
  });

  it("still finds a page by its English name", async () => {
    open();

    await userEvent.type(screen.getByRole("searchbox", { name: "Einstellungen durchsuchen" }), "Appearance");

    expect(screen.getByRole("button", { name: "Darstellung" })).toBeTruthy();
  });

  it("says so in German when nothing matches", async () => {
    open();

    await userEvent.type(screen.getByRole("searchbox", { name: "Einstellungen durchsuchen" }), "zzzzzz");

    expect(screen.getByText("Keine Treffer")).toBeTruthy();
  });
});
