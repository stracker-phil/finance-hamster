import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useT } from "../i18n";
import { renderWithQuery } from "../test/renderWithQuery";
import { settings } from "../test/settings";
import { useLook } from "./useLook";

const mocked = vi.mocked(api, true);

function Probe() {
  useLook();
  return <p>{useT().settings.appearance.languageGroup}</p>;
}

describe("useLook", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows German once the saved language is German", async () => {
    mocked.getSettings.mockResolvedValue(settings({ language: "de" }));

    renderWithQuery(<Probe />);

    expect(await screen.findByText("Sprache")).toBeTruthy();
    expect(document.documentElement.lang).toBe("de");
  });

  it("stays English while the settings load", () => {
    mocked.getSettings.mockReturnValue(new Promise(() => {}));

    renderWithQuery(<Probe />);

    expect(screen.getByText("Language")).toBeTruthy();
  });

  it("stays English when the saved language is English", async () => {
    mocked.getSettings.mockResolvedValue(settings({ language: "en" }));

    renderWithQuery(<Probe />);

    expect(await screen.findByText("Language")).toBeTruthy();
    expect(document.documentElement.lang).toBe("en");
  });
});
