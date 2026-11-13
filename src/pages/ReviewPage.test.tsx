import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useEvents } from "../queries/useEvents";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { reviewLine, reviewQueue } from "../test/review";
import { ReviewPage } from "./ReviewPage";

const mocked = vi.mocked(api, true);

function Page() {
  useEvents();
  return <ReviewPage />;
}

const render = () => renderWithQuery(<Page />);

async function emitDataChanged(tables: string[]) {
  await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "data_changed")).toBe(true));
  const handler = mocked.listen.mock.calls.find(([name]) => name === "data_changed")![1] as (payload: { tables: string[] }) => void;
  await act(async () => handler({ tables }));
}

const itemSelect = () => screen.getByLabelText<HTMLSelectElement>("If ANV, counts as");

beforeEach(() => {
  vi.clearAllMocks();
  mocked.listen.mockResolvedValue(() => {});
  useStore.setState({ route: { page: "review" }, back: [], forward: [], toasts: [] });
  mocked.reviewQueue.mockResolvedValue(reviewQueue());
  mocked.reviewDecide.mockResolvedValue("ANV: Example Bookshop");
});

describe("ReviewPage", () => {
  describe("the line card", () => {
    it("shows the line's account, date, title, amount and raw text", async () => {
      render();

      expect(await screen.findByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
      expect(screen.getByText("Private · Tue, 10 Nov")).toBeTruthy();
      expect(screen.getByText("−€34.90")).toBeTruthy();
      expect(screen.getByText(/Booking text 4711/)).toBeTruthy();
    });

    it("offers ANV, Subscription and Ignore with their keys", async () => {
      render();

      for (const name of ["ANV (A)", "Subscription (S)", "Ignore (I)"]) expect(await screen.findByRole("button", { name })).toBeTruthy();
    });

    it("says a single remaining line needs a decision in the singular", async () => {
      render();

      expect(await screen.findByText("1 line needs your decision")).toBeTruthy();
    });

    it("counts the lines still in the queue", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine(), reviewLine({ id: "line-2", title: "Example Cinema" })] }));

      render();

      expect(await screen.findByText("2 lines need your decision")).toBeTruthy();
    });
  });

  describe("the suggestion", () => {
    it("marks the suggested choice and names the key it was learned from", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "subscription" })] }));

      render();

      const suggested = await screen.findByRole("button", { name: "Subscription (S)" });
      expect(suggested.className).toContain("tbig--suggested");
      expect(screen.getByRole("button", { name: "ANV (A)" }).className).not.toContain("tbig--suggested");
      expect(screen.getByText("Suggested: Subscription, as before for “EXAMPLEBOOKS”")).toBeTruthy();
    });

    it("marks nothing when the key has no earlier decision", async () => {
      render();

      await screen.findByRole("heading", { name: "Example Bookshop" });
      for (const name of ["ANV (A)", "Subscription (S)", "Ignore (I)"]) expect(screen.getByRole("button", { name }).className).not.toContain("tbig--suggested");
      expect(screen.queryByText(/^Suggested:/)).toBeNull();
    });

    it("prefills the item last used for the key and says why", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "anv", itemKey: "kz-720", itemWhy: "last used for Example Bookshop" })] }));

      render();

      expect(await screen.findByText("last used for Example Bookshop")).toBeTruthy();
      expect(itemSelect().value).toBe("kz-720");
    });

    it("leaves the item unchosen when none was used before", async () => {
      render();

      await screen.findByRole("heading", { name: "Example Bookshop" });
      expect(itemSelect().value).toBe("");
    });
  });

  describe("deciding", () => {
    it("sends the clicked action with no item when none is chosen", async () => {
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Ignore (I)" }));

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "ignore", null, false);
    });

    it("sends the chosen item and the always flag", async () => {
      render();
      await screen.findByRole("heading", { name: "Example Bookshop" });

      await userEvent.selectOptions(itemSelect(), "kz-169");
      await userEvent.click(screen.getByRole("checkbox", { name: "Always do this for “EXAMPLEBOOKS”" }));
      await userEvent.click(screen.getByRole("button", { name: "ANV (A)" }));

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "anv", "kz-169", true);
    });

    it("sends the prefilled item when the owner keeps it", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "anv", itemKey: "kz-720", itemWhy: "last used for Example Bookshop" })] }));
      render();

      await userEvent.click(await screen.findByRole("button", { name: "ANV (A)" }));

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "anv", "kz-720", false);
    });

    it.each([
      ["a", "anv"],
      ["s", "subscription"],
      ["i", "ignore"],
    ])("decides %s as %s from the keyboard", async (key, action) => {
      render();
      await screen.findByRole("heading", { name: "Example Bookshop" });

      await userEvent.keyboard(key);

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", action, null, false);
    });

    it("toasts the returned label with an Undo that undoes the decision", async () => {
      mocked.undo.mockResolvedValue("ANV: Example Bookshop");
      render();

      await userEvent.click(await screen.findByRole("button", { name: "ANV (A)" }));

      const toast = await waitFor(() => {
        const t = useStore.getState().toasts.at(-1);
        expect(t?.message).toBe("ANV: Example Bookshop");
        return t!;
      });
      expect(toast.kind).toBe("success");
      expect(toast.action?.label).toBe("Undo");
      await act(async () => toast.action!.run());
      await waitFor(() => expect(mocked.undo).toHaveBeenCalled());
      await waitFor(() => expect(useStore.getState().toasts.at(-1)?.message).toBe("Undid ANV: Example Bookshop"));
    });

    it("shows the field message and focuses the item when ANV needs an item", async () => {
      mocked.reviewDecide.mockRejectedValue({ kind: "invalid_input", message: "Invalid input", code: null, field: { path: "review.item", message: "Choose an item for ANV." } });
      render();

      await userEvent.click(await screen.findByRole("button", { name: "ANV (A)" }));

      expect((await screen.findByRole("alert")).textContent).toBe("Choose an item for ANV.");
      expect(document.activeElement).toBe(itemSelect());
    });

    it("shows the message of any other error and keeps the line", async () => {
      mocked.reviewDecide.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Ignore (I)" }));

      expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
      expect(screen.getByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
    });
  });

  describe("moving between lines", () => {
    beforeEach(() => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine(), reviewLine({ id: "line-2", title: "Example Cinema" })] }));
    });

    it("disables Previous on the first line", async () => {
      render();

      const previous = await screen.findByRole<HTMLButtonElement>("button", { name: "← Previous" });
      expect(previous.disabled).toBe(true);
    });

    it("skips to the next line and back to the previous one", async () => {
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Skip →" }));
      expect(await screen.findByRole("heading", { name: "Example Cinema" })).toBeTruthy();
      expect(screen.getByText("2 lines need your decision")).toBeTruthy();

      await userEvent.click(screen.getByRole("button", { name: "← Previous" }));
      expect(await screen.findByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
    });

    it("skips with the J key", async () => {
      render();
      await screen.findByRole("heading", { name: "Example Bookshop" });

      await userEvent.keyboard("j");

      expect(await screen.findByRole("heading", { name: "Example Cinema" })).toBeTruthy();
    });

    it("decides the line that is showing after a skip", async () => {
      render();
      await userEvent.click(await screen.findByRole("button", { name: "Skip →" }));
      await screen.findByRole("heading", { name: "Example Cinema" });

      await userEvent.click(screen.getByRole("button", { name: "Ignore (I)" }));

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-2", "ignore", null, false);
    });

    it("disables Skip when only one line is left", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue());

      render();

      const skip = await screen.findByRole<HTMLButtonElement>("button", { name: "Skip →" });
      expect(skip.disabled).toBe(true);
    });
  });

  describe("when no line is left", () => {
    beforeEach(() => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));
    });

    it("says that no line is waiting when the queue starts empty", async () => {
      render();

      expect(await screen.findByText("All reviewed")).toBeTruthy();
      expect(screen.getByText("No line is waiting for a decision.")).toBeTruthy();
    });

    it("counts the decisions and new rules made in this session", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue());
      render();
      await userEvent.click(await screen.findByRole("checkbox", { name: "Always do this for “EXAMPLEBOOKS”" }));
      await userEvent.click(screen.getByRole("button", { name: "Ignore (I)" }));
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));

      await emitDataChanged(["decisions"]);

      expect(await screen.findByText("1 decided · 1 new rule")).toBeTruthy();
    });

    it("counts a decision without a rule as no new rules", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue());
      render();
      await userEvent.click(await screen.findByRole("button", { name: "Ignore (I)" }));
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));

      await emitDataChanged(["decisions"]);

      expect(await screen.findByText("1 decided · 0 new rules")).toBeTruthy();
    });

    it("goes back to Statements", async () => {
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Back to Statements" }));

      expect(useStore.getState().route).toEqual({ page: "statements" });
    });

    it("goes back to Today", async () => {
      useStore.setState({ route: HOME });
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Back to Today" }));

      expect(useStore.getState().route).toEqual({ page: "today" });
    });
  });
});
