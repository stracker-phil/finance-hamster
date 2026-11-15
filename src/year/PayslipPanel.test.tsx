import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { setLanguage } from "../i18n";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { payslipsYear } from "../test/payslips";
import { renderWithQuery } from "../test/renderWithQuery";
import { PayslipPanel } from "./PayslipPanel";

const mocked = vi.mocked(api, true);

const show = async (year = payslipsYear()) => {
  mocked.payslipsYear.mockResolvedValue(year);
  renderWithQuery(<PayslipPanel year={2026} />);
  await screen.findAllByRole("listitem");
};

const month = (name: string) => {
  const found = screen.getAllByRole("listitem").find((li) => li.textContent?.startsWith(name));
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

describe("PayslipPanel", () => {
  it("lists the twelve months of the year in order", async () => {
    await show(payslipsYear({ open: false }));

    expect(screen.getByRole("complementary", { name: "Payslips" })).toBeTruthy();
    expect(screen.getAllByRole("listitem").map((li) => li.textContent)).toEqual([
      "JanuaryMissing", "FebruaryMissing", "MarchMissing", "AprilMissing", "MayMissing", "JuneMissing",
      "JulyMissing", "AugustMissing", "SeptemberMissing", "OctoberMissing", "NovemberMissing", "DecemberMissing",
    ]);
    expect(mocked.payslipsYear).toHaveBeenCalledWith(2026);
  });

  describe("a month with a payslip", () => {
    it("shows its payout and opens the payslip of the year when clicked", async () => {
      await show(payslipsYear({ filled: { 7: { id: "ps-7", payout: "€2,431.50" } } }));

      expect(month("July").getByText("€2,431.50")).toBeTruthy();
      await userEvent.click(screen.getByRole("button", { name: /^July/ }));

      expect(useStore.getState().route).toEqual({ page: "payslip", payslipId: "ps-7", year: 2026 });
    });

    it("offers neither Enter nor Add PDF", async () => {
      await show(payslipsYear({ filled: { 7: {} } }));

      expect(month("July").queryByRole("button", { name: "Enter" })).toBeNull();
      expect(month("July").queryByRole("button", { name: "Add PDF…" })).toBeNull();
    });

    it("says No PDF when the payslip has none", async () => {
      await show(payslipsYear({ filled: { 6: { hasPdf: false }, 7: { hasPdf: true } } }));

      expect(month("June").getByText("No PDF")).toBeTruthy();
      expect(month("July").queryByText("No PDF")).toBeNull();
    });
  });

  describe("a missing month of an open year", () => {
    it("offers Enter and Add PDF…", async () => {
      await show();

      expect(month("March").getByRole("button", { name: "Enter" })).toBeTruthy();
      expect(month("March").getByRole("button", { name: "Add PDF…" })).toBeTruthy();
      expect(month("March").queryByText("Missing")).toBeNull();
    });

    it("adds the payslip on Enter, says so with Undo and opens it", async () => {
      await show();

      await userEvent.click(month("March").getByRole("button", { name: "Enter" }));

      expect(mocked.payslipAdd).toHaveBeenCalledWith(2026, 3);
      await vi.waitFor(() => expect(useStore.getState().route).toEqual({ page: "payslip", payslipId: "ps-new", year: 2026 }));
      expect(toastMessages()).toEqual(["Add Payslip March 2026"]);
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });

    it("reports a payslip that cannot be added as an error toast and stays on the page", async () => {
      mocked.payslipAdd.mockRejectedValue({ kind: "conflict", message: "March already has a payslip", code: null });
      await show();

      await userEvent.click(month("March").getByRole("button", { name: "Enter" }));

      await vi.waitFor(() => expect(toastMessages()).toEqual(["March already has a payslip"]));
      expect(useStore.getState().route).toEqual(HOME);
    });

    it("adds the payslip with the chosen PDF on Add PDF… and opens it", async () => {
      mocked.payslipAddPdf.mockResolvedValue({ id: "ps-pdf", label: "Add Payslip with PDF March 2026" });
      await show();

      await userEvent.click(month("March").getByRole("button", { name: "Add PDF…" }));

      expect(mocked.payslipAddPdf).toHaveBeenCalledWith(2026, 3);
      await vi.waitFor(() => expect(useStore.getState().route).toEqual({ page: "payslip", payslipId: "ps-pdf", year: 2026 }));
      expect(toastMessages()).toEqual(["Add Payslip with PDF March 2026"]);
    });

    it("does nothing when the file dialog is cancelled", async () => {
      await show();

      await userEvent.click(month("March").getByRole("button", { name: "Add PDF…" }));

      await vi.waitFor(() => expect(mocked.payslipAddPdf).toHaveBeenCalled());
      expect(useStore.getState().route).toEqual(HOME);
      expect(toastMessages()).toEqual([]);
    });
  });

  describe("a missing month of a closed year", () => {
    it("says Missing and offers no action", async () => {
      await show(payslipsYear({ open: false, filled: { 7: {} } }));

      expect(month("March").getByText("Missing")).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Enter" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Add PDF…" })).toBeNull();
    });
  });

  describe("the year's sums", () => {
    it("lists each total with its label and the home office days of the year", async () => {
      await show(payslipsYear({ totals: [{ label: "Gross", value: "€30,000.00" }, { label: "Income tax", value: "€6,000.00" }], telework: "122 days · as of July" }));

      expect(screen.getByText("Gross").nextElementSibling?.textContent).toBe("€30,000.00");
      expect(screen.getByText("Income tax").nextElementSibling?.textContent).toBe("€6,000.00");
      expect(screen.getByText("Home office days").nextElementSibling?.textContent).toBe("122 days · as of July");
    });

    it("shows a dash for the home office days while no payslip prints them", async () => {
      await show(payslipsYear({ telework: null }));

      expect(screen.getByText("Home office days").nextElementSibling?.textContent).toBe("–");
    });
  });

  describe("in German", () => {
    it("labels the panel, its months and their actions in German", async () => {
      setLanguage("de");
      mocked.payslipsYear.mockResolvedValue(payslipsYear({ filled: { 7: {} } }));
      renderWithQuery(<PayslipPanel year={2026} />);
      await screen.findAllByRole("listitem");

      expect(screen.getByRole("complementary", { name: "Lohnzettel" })).toBeTruthy();
      expect(screen.getByRole("heading", { name: "Lohnzettel" })).toBeTruthy();
      expect(screen.getAllByRole("button", { name: "Eintragen" })).toHaveLength(11);
      expect(screen.getAllByRole("button", { name: "PDF hinzufügen …" })).toHaveLength(11);
      expect(screen.getByText("Homeoffice-Tage")).toBeTruthy();
    });
  });
});
