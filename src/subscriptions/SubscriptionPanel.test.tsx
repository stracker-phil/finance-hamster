import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { subscriptionDetail, subscriptionForm } from "../test/subscriptions";
import { SubscriptionPanel } from "./SubscriptionPanel";

const mocked = vi.mocked(api, true);

const render = (onClose = vi.fn()) => renderWithQuery(<SubscriptionPanel id="sub-1" onClose={onClose} />);

const answer = async (confirmed: boolean) => {
  await waitFor(() => expect(useStore.getState().confirmation).not.toBeNull());
  act(() => useStore.getState().confirmation!.answer(confirmed));
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.useFakeTimers({ shouldAdvanceTime: true, toFake: ["Date"] });
  vi.setSystemTime(new Date(2026, 10, 13, 12));
  useStore.setState({ toasts: [], subscriptionSheet: null, confirmation: null });
  mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail());
  mocked.subscriptionSetEnded.mockResolvedValue("End Lumen AI");
});

afterEach(() => {
  vi.useRealTimers();
});

describe("SubscriptionPanel", () => {
  describe("showing a subscription", () => {
    it("shows its name, interval, amount and next charge", async () => {
      render();

      expect(await screen.findByText("Lumen AI")).toBeTruthy();
      expect(screen.getByText("Monthly")).toBeTruthy();
      expect(screen.getByText("€20.00")).toBeTruthy();
      expect(screen.getByText("Next charge 15 Dec")).toBeTruthy();
    });

    it("lists the plans of the year with their price and start", async () => {
      render();

      const plans = within(await screen.findByRole("region", { name: "Plans in 2026" }));
      expect(plans.getByText("Plus")).toBeTruthy();
      expect(plans.getByText("€20.00 monthly")).toBeTruthy();
      expect(plans.getByText("since 15 Jan")).toBeTruthy();
    });

    it("shows no plans section when the subscription has no plan", async () => {
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail({ plans: [] }));

      render();

      await screen.findByText("Lumen AI");
      expect(screen.queryByRole("region", { name: "Plans in 2026" })).toBeNull();
    });

    it("shows the ANV, reminder and rule facts", async () => {
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail({ anv: "Digital work equipment", reminder: "3 days before", rule: "LUMEN AI" }));

      render();

      expect(await screen.findByText("Digital work equipment")).toBeTruthy();
      expect(screen.getByText("Counts for ANV")).toBeTruthy();
      expect(screen.getByText("Reminder")).toBeTruthy();
      expect(screen.getByText("3 days before")).toBeTruthy();
      expect(screen.getByText("Found in the bank text by")).toBeTruthy();
      expect(screen.getByText("LUMEN AI")).toBeTruthy();
    });

    it("lists the charges found in statements", async () => {
      render();

      const charges = within(await screen.findByRole("region", { name: "Charges found in statements" }));
      expect(charges.getByText("15 Nov")).toBeTruthy();
      expect(charges.getByText("Private")).toBeTruthy();
      expect(charges.getByText("−€20.00")).toBeTruthy();
      expect(charges.queryByText("No charge found yet.")).toBeNull();
    });

    it("says no charge was found yet when there is none", async () => {
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail({ charges: [] }));

      render();

      expect(await screen.findByText("No charge found yet.")).toBeTruthy();
    });

    it("calls onClose from its Close button", async () => {
      const onClose = vi.fn();
      render(onClose);

      await userEvent.click(screen.getByRole("button", { name: "Close (Esc)" }));

      expect(onClose).toHaveBeenCalled();
    });
  });

  describe("an active subscription", () => {
    it("opens Change Plan starting today with the current amount, currency and interval", async () => {
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail({ form: subscriptionForm({ amount: "12.50", currency: "USD", interval: "yearly" }) }));
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Change Plan…" }));

      expect(useStore.getState().subscriptionSheet).toEqual({
        kind: "plan",
        id: "sub-1",
        name: "Lumen AI",
        currentPlan: "Plus · €20.00 monthly",
        form: { name: "", amount: "12.50", currency: "USD", interval: "yearly", start: "2026-11-13" },
      });
    });

    it("opens Edit with the subscription's form", async () => {
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Edit…" }));

      expect(useStore.getState().subscriptionSheet).toEqual({ kind: "edit", id: "sub-1", form: subscriptionForm() });
    });

    it("asks before ending and then ends it today", async () => {
      render();
      await userEvent.click(await screen.findByRole("button", { name: "End Subscription…" }));

      expect(useStore.getState().confirmation?.title).toBe("End Lumen AI?");
      await answer(true);

      await waitFor(() => expect(mocked.subscriptionSetEnded).toHaveBeenCalledWith("sub-1", "2026-11-13"));
      await waitFor(() => expect(useStore.getState().toasts.at(-1)?.message).toBe("End Lumen AI"));
    });

    it("keeps the subscription when ending is cancelled", async () => {
      render();
      await userEvent.click(await screen.findByRole("button", { name: "End Subscription…" }));

      await answer(false);

      expect(mocked.subscriptionSetEnded).not.toHaveBeenCalled();
    });

    it("shows the error when ending fails", async () => {
      mocked.subscriptionSetEnded.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      render();
      await userEvent.click(await screen.findByRole("button", { name: "End Subscription…" }));

      await answer(true);

      await waitFor(() => expect(useStore.getState().toasts.at(-1)).toMatchObject({ kind: "error", message: "Disk full" }));
    });

    it("offers no Resume", async () => {
      render();

      await screen.findByRole("button", { name: "Change Plan…" });
      expect(screen.queryByRole("button", { name: "Resume" })).toBeNull();
    });
  });

  describe("an ended subscription", () => {
    beforeEach(() => {
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail({ next: null, ended: "Ended 31 Mar" }));
    });

    it("shows when it ended instead of the next charge", async () => {
      render();

      expect(await screen.findByText("Ended 31 Mar")).toBeTruthy();
      expect(screen.queryByText(/^Next charge/)).toBeNull();
    });

    it("offers Resume and Edit but neither Change Plan nor End Subscription", async () => {
      render();

      expect(await screen.findByRole("button", { name: "Resume" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Edit…" })).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Change Plan…" })).toBeNull();
      expect(screen.queryByRole("button", { name: "End Subscription…" })).toBeNull();
    });

    it("resumes it with Resume", async () => {
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Resume" }));

      expect(mocked.subscriptionSetEnded).toHaveBeenCalledWith("sub-1", null);
    });
  });
});
