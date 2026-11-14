import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { yearCheckView } from "../test/year";
import { CloseYearSheet } from "./CloseYearSheet";

const mocked = vi.mocked(api, true);

const PROMPT = "No year check ran after 31 Dec 2026. It asks what you might have forgotten to claim.";

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [] });
  mocked.yearCloseCheck.mockResolvedValue({ blockers: [], warnings: [] });
});

describe("CloseYearSheet", () => {
  describe("with a year check prompt", () => {
    const show = async (onClose = () => {}) => {
      mocked.yearCheckView.mockResolvedValue(yearCheckView({ closePrompt: PROMPT }));
      renderWithQuery(<CloseYearSheet year={2026} onClose={onClose} />);
      await screen.findByRole("dialog", { name: "Run the year check first?" });
    };

    it("asks to run the year check first with its text and three choices", async () => {
      await show();

      expect(screen.getByText(PROMPT)).toBeTruthy();
      for (const name of ["Cancel", "Close Without Check…", "Start Year Check"]) expect(screen.getByRole("button", { name })).toBeTruthy();
      expect(screen.queryByRole("dialog", { name: "Close Tax Year 2026?" })).toBeNull();
    });

    it("shows the close sheet after Close Without Check", async () => {
      await show();

      await userEvent.click(screen.getByRole("button", { name: "Close Without Check…" }));

      expect(await screen.findByRole("dialog", { name: "Close Tax Year 2026?" })).toBeTruthy();
      expect(screen.queryByRole("dialog", { name: "Run the year check first?" })).toBeNull();
    });

    it("opens the year check and closes the prompt on Start Year Check", async () => {
      const onClose = vi.fn();
      await show(onClose);

      await userEvent.click(screen.getByRole("button", { name: "Start Year Check" }));

      expect(useStore.getState().route).toEqual({ page: "yearCheck", year: 2026 });
      expect(onClose).toHaveBeenCalled();
    });

    it("closes without navigating on Cancel", async () => {
      const onClose = vi.fn();
      await show(onClose);

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(onClose).toHaveBeenCalled();
      expect(useStore.getState().route).toEqual(HOME);
    });
  });

  it("shows the close sheet directly when there is no prompt", async () => {
    mocked.yearCheckView.mockResolvedValue(yearCheckView({ closePrompt: null }));

    renderWithQuery(<CloseYearSheet year={2026} onClose={() => {}} />);

    expect(await screen.findByRole("dialog", { name: "Close Tax Year 2026?" })).toBeTruthy();
    expect(screen.queryByRole("dialog", { name: "Run the year check first?" })).toBeNull();
  });
});
