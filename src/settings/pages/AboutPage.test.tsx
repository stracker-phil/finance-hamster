import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { renderWithQuery } from "../../test/renderWithQuery";
import { AboutPage } from "./AboutPage";

const mocked = vi.mocked(api, true);

describe("AboutPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.appInfo.mockResolvedValue({ name: "Hamster", version: "2.4.1" });
  });

  it("shows the app version", async () => {
    renderWithQuery(<AboutPage />);

    expect(await screen.findByText("2.4.1")).toBeTruthy();
    expect(screen.getByText("Version")).toBeTruthy();
  });
});
