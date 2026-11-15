import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { HOME } from "../../state/route";
import { useStore } from "../../state/store";
import { payslipsYear } from "../../test/payslips";
import { renderWithQuery } from "../../test/renderWithQuery";
import { PayslipList } from "./PayslipList";

const mocked = vi.mocked(api, true);

const show = async (...years: ReturnType<typeof payslipsYear>[]) => {
  mocked.payslipsList.mockResolvedValue(years);
  renderWithQuery(<PayslipList />);
  await screen.findByRole("region", { name: String(years[0].year) });
};

const month = (year: number, name: string) => {
  const found = within(screen.getByRole("region", { name: String(year) })).getAllByRole("listitem").find((li) => li.textContent?.startsWith(name));
  if (!found) throw new Error(`No month ${name}`);
  return within(found);
};

const toastMessages = () => useStore.getState().toasts.map((t) => t.message);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [] });
  mocked.payslipAdd.mockResolvedValue({ id: "ps-new", label: "Add Payslip March 2026" });
  mocked.payslipAddPdf.mockResolvedValue(null);
});

describe("PayslipList", () => {
  it("shows each year with its count and all twelve months in order", async () => {
    await show(payslipsYear({ count: "1 of 12 payslips", filled: { 7: {} } }), payslipsYear({ year: 2025, open: false, count: "12 of 12 payslips" }));

    const year = screen.getByRole("region", { name: "2026" });
    expect(within(year).getByText("1 of 12 payslips")).toBeTruthy();
    expect(within(screen.getByRole("region", { name: "2025" })).getByText("12 of 12 payslips")).toBeTruthy();
    expect(within(year).getAllByRole("listitem").map((li) => li.textContent?.slice(0, 3))).toEqual(["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]);
  });

  it("says there are no payslips and no open tax year when there is no year at all", async () => {
    mocked.payslipsList.mockResolvedValue([]);
    renderWithQuery(<PayslipList />);

    expect(await screen.findByText("No payslips yet, and no open tax year to add them to.")).toBeTruthy();
    expect(screen.queryByRole("region")).toBeNull();
  });

  it("opens the tax year page from a year's title", async () => {
    await show(payslipsYear());

    await userEvent.click(screen.getByRole("button", { name: "2026" }));

    expect(useStore.getState().route).toEqual({ page: "year", year: 2026 });
  });

  describe("a month with a payslip", () => {
    it("shows its payout and opens the payslip of that year when clicked", async () => {
      await show(payslipsYear({ filled: { 7: { id: "ps-7", payout: "€2,431.50" } } }));

      expect(month(2026, "July").getByText("€2,431.50")).toBeTruthy();
      await userEvent.click(month(2026, "July").getByRole("button"));

      expect(useStore.getState().route).toEqual({ page: "payslip", payslipId: "ps-7", year: 2026 });
    });

    it("offers neither Enter nor Add PDF…", async () => {
      await show(payslipsYear({ filled: { 7: {} } }));

      expect(month(2026, "July").queryByRole("button", { name: "Enter" })).toBeNull();
      expect(month(2026, "July").queryByRole("button", { name: "Add PDF…" })).toBeNull();
    });

    it("says No PDF only for a payslip that has none", async () => {
      await show(payslipsYear({ filled: { 6: { hasPdf: false }, 7: { hasPdf: true } } }));

      expect(month(2026, "June").getByText("No PDF")).toBeTruthy();
      expect(month(2026, "July").queryByText("No PDF")).toBeNull();
    });
  });

  describe("a missing month of an open year", () => {
    it("says Missing and offers Enter and Add PDF…", async () => {
      await show(payslipsYear());

      expect(month(2026, "March").getByText("Missing")).toBeTruthy();
      expect(month(2026, "March").getByRole("button", { name: "Enter" })).toBeTruthy();
      expect(month(2026, "March").getByRole("button", { name: "Add PDF…" })).toBeTruthy();
    });

    it("adds the payslip on Enter, says so with Undo and opens it", async () => {
      await show(payslipsYear());

      await userEvent.click(month(2026, "March").getByRole("button", { name: "Enter" }));

      expect(mocked.payslipAdd).toHaveBeenCalledWith(2026, 3);
      await vi.waitFor(() => expect(useStore.getState().route).toEqual({ page: "payslip", payslipId: "ps-new", year: 2026 }));
      expect(toastMessages()).toEqual(["Add Payslip March 2026"]);
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });

    it("reports a payslip that cannot be added as an error toast and stays on the page", async () => {
      mocked.payslipAdd.mockRejectedValue({ kind: "conflict", message: "March already has a payslip", code: null });
      await show(payslipsYear());

      await userEvent.click(month(2026, "March").getByRole("button", { name: "Enter" }));

      await vi.waitFor(() => expect(toastMessages()).toEqual(["March already has a payslip"]));
      expect(useStore.getState().route).toEqual(HOME);
    });

    it("adds the payslip with the chosen PDF on Add PDF… and opens it", async () => {
      mocked.payslipAddPdf.mockResolvedValue({ id: "ps-pdf", label: "Add Payslip with PDF March 2026" });
      await show(payslipsYear());

      await userEvent.click(month(2026, "March").getByRole("button", { name: "Add PDF…" }));

      expect(mocked.payslipAddPdf).toHaveBeenCalledWith(2026, 3);
      await vi.waitFor(() => expect(useStore.getState().route).toEqual({ page: "payslip", payslipId: "ps-pdf", year: 2026 }));
      expect(toastMessages()).toEqual(["Add Payslip with PDF March 2026"]);
    });

    it("does nothing when the file dialog is cancelled", async () => {
      await show(payslipsYear());

      await userEvent.click(month(2026, "March").getByRole("button", { name: "Add PDF…" }));

      await vi.waitFor(() => expect(mocked.payslipAddPdf).toHaveBeenCalled());
      expect(useStore.getState().route).toEqual(HOME);
      expect(toastMessages()).toEqual([]);
    });

    it("adds for the year the month is listed in", async () => {
      await show(payslipsYear(), payslipsYear({ year: 2025 }));

      await userEvent.click(month(2025, "March").getByRole("button", { name: "Enter" }));

      expect(mocked.payslipAdd).toHaveBeenCalledWith(2025, 3);
    });
  });

  it("says Missing but offers no action for a missing month of a closed year", async () => {
    await show(payslipsYear({ open: false, filled: { 7: {} } }));

    expect(month(2026, "March").getByText("Missing")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Enter" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Add PDF…" })).toBeNull();
  });
});
