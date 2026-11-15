import { screen } from "@testing-library/react";
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
  await screen.findByText(year.count);
};

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [] });
});

describe("PayslipPanel", () => {
  it("is the sidebar of the year's payslips, read for that year", async () => {
    await show();

    expect(screen.getByRole("complementary", { name: "Your year by the payslips" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Your year by the payslips" })).toBeTruthy();
    expect(mocked.payslipsYear).toHaveBeenCalledWith(2026);
  });

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

  it("says how many payslips there are", async () => {
    await show(payslipsYear({ count: "10 of 12 payslips" }));

    expect(screen.getByText("10 of 12 payslips")).toBeTruthy();
  });

  it("lists no months and offers no way to add a payslip", async () => {
    await show();

    expect(screen.queryByRole("listitem")).toBeNull();
    expect(screen.queryByRole("button", { name: "Enter" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Add PDF…" })).toBeNull();
  });

  it("opens the payslips tab of Documents from Show in Documents", async () => {
    await show();

    await userEvent.click(screen.getByRole("button", { name: "Show in Documents" }));

    expect(useStore.getState().route).toEqual({ page: "documents", tab: "payslips" });
  });

  it("shows only its title while the year cannot be read", async () => {
    mocked.payslipsYear.mockRejectedValue({ kind: "not_found", message: "No payslips", code: null });
    renderWithQuery(<PayslipPanel year={2026} />);

    await vi.waitFor(() => expect(mocked.payslipsYear).toHaveBeenCalled());
    expect(screen.getByRole("heading", { name: "Your year by the payslips" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Show in Documents" })).toBeNull();
  });

  describe("in German", () => {
    it("labels the sidebar, the totals and the way to the documents in German", async () => {
      setLanguage("de");
      await show();

      expect(screen.getByRole("complementary", { name: "Dein Jahr laut Lohnzetteln" })).toBeTruthy();
      expect(screen.getByText("Homeoffice-Tage")).toBeTruthy();
      expect(screen.getByRole("button", { name: "In Dokumente zeigen" })).toBeTruthy();
    });
  });
});
