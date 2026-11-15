import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { choose, popUpValue, withMenus } from "../test/popup";
import { renderWithQuery } from "../test/renderWithQuery";
import { reviewQueue } from "../test/review";
import { lineView } from "../test/statements";
import { ChangeLineSheet } from "./ChangeLineSheet";

const mocked = vi.mocked(api, true);

const SUBSCRIPTIONS = [
  { id: "sub-1", name: "Lumen AI", letter: "L", tone: 0 },
  { id: "sub-2", name: "Pixel Cloud", letter: "P", tone: 1 },
];

function renderSheet(patch: Parameters<typeof lineView>[0] = {}, onClose = vi.fn()) {
  renderWithQuery(withMenus(<ChangeLineSheet line={lineView(patch)} onClose={onClose} />));
  return onClose;
}

const pressed = (name: string) => screen.getByRole("button", { name }).getAttribute("aria-pressed");
const save = () => screen.getByRole<HTMLButtonElement>("button", { name: "Save" });

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
  mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [], subscriptions: SUBSCRIPTIONS }));
  mocked.reviewDecide.mockResolvedValue("Change Example Market");
});

describe("ChangeLineSheet", () => {
  it("shows the line's title and amount", () => {
    renderSheet();

    expect(screen.getByRole("dialog", { name: "What is this line?" })).toBeTruthy();
    expect(screen.getByText("Example Market · −€22.48")).toBeTruthy();
  });

  describe("preselection", () => {
    it.each([
      ["anv", "ANV"],
      ["subscription", "Subscription"],
      ["ignored", "Ignore"],
    ] as const)("presses the action of a %s line", (status, pressedName) => {
      renderSheet({ status });

      for (const name of ["ANV", "Subscription", "Ignore"]) expect(pressed(name)).toBe(String(name === pressedName));
    });

    it("shows the item of an ANV line", async () => {
      renderSheet({ status: "anv", itemKey: "kz-720" });

      await waitFor(() => expect(popUpValue("Counts as")).toBe("KZ 720 · Professional literature"));
    });

    it("shows the subscription of a subscription line", async () => {
      renderSheet({ status: "subscription", subscriptionId: "sub-2" });

      await waitFor(() => expect(popUpValue("Choose a subscription")).toBe("Pixel Cloud"));
    });
  });

  describe("the Save button", () => {
    it("is disabled for an ANV line without an item until one is chosen", async () => {
      renderSheet({ status: "anv" });
      expect(save().disabled).toBe(true);

      await choose(userEvent, "Counts as", "KZ 169 · Digital work equipment");

      expect(save().disabled).toBe(false);
    });

    it("is disabled after switching to Subscription until a subscription is chosen", async () => {
      renderSheet({ status: "anv", itemKey: "kz-169" });
      expect(save().disabled).toBe(false);

      await userEvent.click(screen.getByRole("button", { name: "Subscription" }));
      expect(save().disabled).toBe(true);

      await choose(userEvent, "Choose a subscription", "Lumen AI");
      expect(save().disabled).toBe(false);
    });

    it("is always ready for Ignore", () => {
      renderSheet({ status: "ignored" });

      expect(save().disabled).toBe(false);
    });
  });

  describe("saving", () => {
    it("sends the chosen item for ANV, then closes with an Undo toast", async () => {
      const onClose = renderSheet({ id: "line-4", status: "ignored" });
      await userEvent.click(screen.getByRole("button", { name: "ANV" }));
      await choose(userEvent, "Counts as", "KZ 720 · Professional literature");

      await userEvent.click(save());

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-4", "anv", "kz-720", null, false);
      await waitFor(() => expect(onClose).toHaveBeenCalled());
      expect(useStore.getState().toasts.at(-1)).toMatchObject({ kind: "success", message: "Change Example Market", action: { label: "Undo" } });
    });

    it("sends the chosen subscription", async () => {
      renderSheet({ id: "line-4", status: "ignored" });
      await userEvent.click(screen.getByRole("button", { name: "Subscription" }));
      await choose(userEvent, "Choose a subscription", "Lumen AI");

      await userEvent.click(save());

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-4", "subscription", null, "sub-1", false);
    });

    it("sends Ignore without an item or subscription even for a line that had one", async () => {
      renderSheet({ id: "line-4", status: "anv", itemKey: "kz-169" });
      await userEvent.click(screen.getByRole("button", { name: "Ignore" }));

      await userEvent.click(save());

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-4", "ignore", null, null, false);
    });

    it("keeps the sheet open and shows the error when saving fails", async () => {
      mocked.reviewDecide.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const onClose = renderSheet({ status: "ignored" });

      await userEvent.click(save());

      await waitFor(() => expect(useStore.getState().toasts.at(-1)).toMatchObject({ kind: "error", message: "Disk full" }));
      expect(onClose).not.toHaveBeenCalled();
    });
  });

  it("closes without deciding with Cancel", async () => {
    const onClose = renderSheet({ status: "anv", itemKey: "kz-169" });

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onClose).toHaveBeenCalled();
    expect(mocked.reviewDecide).not.toHaveBeenCalled();
  });
});
