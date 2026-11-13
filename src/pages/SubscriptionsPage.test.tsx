import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { endedRow, subscriptionDetail, subscriptionGroup, subscriptionRow, subscriptionsView } from "../test/subscriptions";
import { SubscriptionsPage } from "./SubscriptionsPage";

const mocked = vi.mocked(api, true);

const rowButton = (name: RegExp) => screen.findByRole("button", { name });

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ filter: "", toasts: [], subscriptionSheet: null, confirmation: null });
  mocked.subscriptionsView.mockResolvedValue(subscriptionsView());
  mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail());
  mocked.subscriptionSetEnded.mockResolvedValue("Resume Old Streaming");
});

describe("SubscriptionsPage", () => {
  describe("without any subscription", () => {
    beforeEach(() => {
      mocked.subscriptionsView.mockResolvedValue(subscriptionsView({ empty: true, groups: [], summary: "" }));
    });

    it("says there are none and offers New Subscription", async () => {
      renderWithQuery(<SubscriptionsPage />);

      expect(await screen.findByText("No subscriptions")).toBeTruthy();
      expect(screen.getByRole("button", { name: "New Subscription" })).toBeTruthy();
    });

    it("opens an empty New sheet when New Subscription is clicked", async () => {
      renderWithQuery(<SubscriptionsPage />);

      await userEvent.click(await screen.findByRole("button", { name: "New Subscription" }));

      expect(useStore.getState().subscriptionSheet).toEqual({ kind: "new", form: null });
    });
  });

  describe("the list", () => {
    beforeEach(() => {
      mocked.subscriptionsView.mockResolvedValue(
        subscriptionsView({
          groups: [
            subscriptionGroup({
              label: "Next 30 days",
              total: "€45.00",
              rows: [
                subscriptionRow({ anv: true, change: "€25.00 now" }),
                subscriptionRow({ id: "sub-2", name: "Pixel Cloud", letter: "P", interval: "Yearly", next: "2 Jan", amount: "€25.00" }),
              ],
            }),
            subscriptionGroup({ label: "Later", total: "€9.00", rows: [subscriptionRow({ id: "sub-3", name: "Daily Reader", letter: "D", amount: "€4.00" }), subscriptionRow({ id: "sub-4", name: "Night Radio", letter: "N", amount: "€5.00" })] }),
          ],
        }),
      );
    });

    it("shows the summary line", async () => {
      renderWithQuery(<SubscriptionsPage />);

      expect(await screen.findByText("1 active · €20.00 a month")).toBeTruthy();
    });

    it("shows each group with its label and total", async () => {
      renderWithQuery(<SubscriptionsPage />);

      const group = within(await screen.findByRole("region", { name: "Next 30 days" }));
      expect(group.getByText("€45.00")).toBeTruthy();
      const later = within(screen.getByRole("region", { name: "Later" }));
      expect(later.getByText("€9.00")).toBeTruthy();
      expect(later.getByRole("button", { name: /Daily Reader/ })).toBeTruthy();
    });

    it("shows a row's letter, name, ANV pill, interval, change chip, next charge and amount", async () => {
      renderWithQuery(<SubscriptionsPage />);

      const lumen = within(await rowButton(/Lumen AI/));
      for (const text of ["L", "Lumen AI", "ANV", "Monthly", "€25.00 now", "15 Dec", "€20.00"]) expect(lumen.getByText(text)).toBeTruthy();
    });

    it("shows a row without ANV or change without a pill or chip", async () => {
      renderWithQuery(<SubscriptionsPage />);

      const pixel = within(await rowButton(/Pixel Cloud/));
      expect(pixel.queryByText("ANV")).toBeNull();
      expect(pixel.getByText("Yearly")).toBeTruthy();
    });
  });

  describe("selecting a row", () => {
    it("marks the row pressed and shows its panel", async () => {
      renderWithQuery(<SubscriptionsPage />);
      const row = await rowButton(/Lumen AI/);
      expect(row.getAttribute("aria-pressed")).toBe("false");

      await userEvent.click(row);

      expect(row.getAttribute("aria-pressed")).toBe("true");
      const panel = within(await screen.findByRole("complementary", { name: "Subscription" }));
      expect(await panel.findByText("Next charge 15 Dec")).toBeTruthy();
      expect(mocked.subscriptionDetail).toHaveBeenCalledWith("sub-1");
    });

    it("closes the panel on Escape", async () => {
      renderWithQuery(<SubscriptionsPage />);
      await userEvent.click(await rowButton(/Lumen AI/));
      await screen.findByRole("complementary", { name: "Subscription" });

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("complementary", { name: "Subscription" })).toBeNull();
      expect(screen.getByRole("button", { name: /Lumen AI/ }).getAttribute("aria-pressed")).toBe("false");
    });

    it("closes the panel with its Close button", async () => {
      renderWithQuery(<SubscriptionsPage />);
      await userEvent.click(await rowButton(/Lumen AI/));

      await userEvent.click(await screen.findByRole("button", { name: "Close (Esc)" }));

      expect(screen.queryByRole("complementary", { name: "Subscription" })).toBeNull();
    });
  });

  describe("ended subscriptions", () => {
    beforeEach(() => {
      mocked.subscriptionsView.mockResolvedValue(subscriptionsView({ ended: [endedRow()] }));
    });

    it("folds them away behind an Ended toggle with their count", async () => {
      renderWithQuery(<SubscriptionsPage />);

      const toggle = await screen.findByRole("button", { name: "Ended · 1" });
      expect(toggle.getAttribute("aria-expanded")).toBe("false");
      expect(screen.queryByText("Old Streaming")).toBeNull();
    });

    it("shows their rows with the end date when the toggle is opened, and folds them again", async () => {
      renderWithQuery(<SubscriptionsPage />);
      const toggle = await screen.findByRole("button", { name: "Ended · 1" });

      await userEvent.click(toggle);

      expect(toggle.getAttribute("aria-expanded")).toBe("true");
      expect(screen.getByText("Old Streaming")).toBeTruthy();
      expect(screen.getByText("Ended 31 Mar")).toBeTruthy();

      await userEvent.click(toggle);
      expect(screen.queryByText("Old Streaming")).toBeNull();
    });

    it("resumes an ended subscription with Resume", async () => {
      renderWithQuery(<SubscriptionsPage />);
      await userEvent.click(await screen.findByRole("button", { name: "Ended · 1" }));

      await userEvent.click(screen.getByRole("button", { name: "Resume" }));

      expect(mocked.subscriptionSetEnded).toHaveBeenCalledWith("sub-9", null);
      await waitFor(() => expect(useStore.getState().toasts.at(-1)?.message).toBe("Resume Old Streaming"));
    });

    it("shows the panel of an ended subscription when its row is opened", async () => {
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail({ id: "sub-9", name: "Old Streaming", next: null, ended: "Ended 31 Mar" }));
      renderWithQuery(<SubscriptionsPage />);
      await userEvent.click(await screen.findByRole("button", { name: "Ended · 1" }));

      await userEvent.click(screen.getByRole("button", { name: "Old Streaming" }));

      const panel = within(await screen.findByRole("complementary", { name: "Subscription" }));
      expect(await panel.findByText("Old Streaming")).toBeTruthy();
      expect(mocked.subscriptionDetail).toHaveBeenCalledWith("sub-9");
    });
  });

  describe("filtering", () => {
    it("asks for the subscriptions matching the header filter", async () => {
      useStore.setState({ filter: "lumen" });

      renderWithQuery(<SubscriptionsPage />);

      await waitFor(() => expect(mocked.subscriptionsView).toHaveBeenCalledWith("lumen"));
    });

    it("says that nothing matches when the filter leaves no subscription", async () => {
      useStore.setState({ filter: "zzz" });
      mocked.subscriptionsView.mockResolvedValue(subscriptionsView({ groups: [], ended: [] }));

      renderWithQuery(<SubscriptionsPage />);

      expect(await screen.findByText("No subscription matches “zzz”.")).toBeTruthy();
    });
  });
});
