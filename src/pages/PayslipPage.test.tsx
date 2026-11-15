import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { payslipField, payslipView } from "../test/payslips";
import { renderWithQuery } from "../test/renderWithQuery";
import { PayslipPage } from "./PayslipPage";

const mocked = vi.mocked(api, true);

const show = async (patch: Parameters<typeof payslipView>[0] = {}) => {
  mocked.payslipView.mockResolvedValue(payslipView(patch));
  renderWithQuery(<PayslipPage payslipId="ps-7" />);
  await screen.findByRole("heading", { name: "Payslip July 2026" });
};

const gross = () => screen.getByLabelText<HTMLInputElement>(/^Gross pay/);
const toastMessages = () => useStore.getState().toasts.map((t) => t.message);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: { page: "payslip", payslipId: "ps-7", year: 2026 }, back: [], forward: [], toasts: [], pdfCovers: 0 });
  mocked.payslipEdit.mockResolvedValue(null);
  mocked.payslipAttach.mockResolvedValue(null);
  mocked.payslipDelete.mockResolvedValue("Delete Payslip July 2026");
});

describe("PayslipPage", () => {
  describe("the form", () => {
    it("shows the title and each amount under its label with the hint and the value", async () => {
      await show({ amounts: [payslipField({ label: "Gross pay", hint: "Line 210", value: "3.200,00" }), payslipField({ name: "tax", label: "Income tax", hint: "Line 260", value: "480,00" })] });

      expect(mocked.payslipView).toHaveBeenCalledWith("ps-7");
      expect(gross().value).toBe("3.200,00");
      expect(screen.getByText("Line 210")).toBeTruthy();
      expect(screen.getByLabelText<HTMLInputElement>(/^Income tax/).value).toBe("480,00");
    });

    it("shows the home office days and the note", async () => {
      await show({ teleworkDays: "12", note: "Bonus included" });

      expect(screen.getByLabelText<HTMLInputElement>(/^Home office days/).value).toBe("12");
      expect(screen.getByRole<HTMLTextAreaElement>("textbox", { name: "Note" }).value).toBe("Bonus included");
    });

    it("saves a typed amount on blur", async () => {
      await show();

      await userEvent.type(gross(), "3200,00");
      await userEvent.tab();

      expect(mocked.payslipEdit).toHaveBeenCalledWith("ps-7", { field: "amount", name: "gross", value: "3200,00" });
    });

    it("saves a typed amount on Enter", async () => {
      await show();

      await userEvent.type(gross(), "3200,00{Enter}");

      expect(mocked.payslipEdit).toHaveBeenCalledWith("ps-7", { field: "amount", name: "gross", value: "3200,00" });
    });

    it("saves the home office days on blur", async () => {
      await show();

      await userEvent.type(screen.getByLabelText(/^Home office days/), "12");
      await userEvent.tab();

      expect(mocked.payslipEdit).toHaveBeenCalledWith("ps-7", { field: "teleworkDays", value: "12" });
    });

    it("saves the note on blur", async () => {
      await show();

      await userEvent.type(screen.getByRole("textbox", { name: "Note" }), "Bonus included");
      await userEvent.tab();

      expect(mocked.payslipEdit).toHaveBeenCalledWith("ps-7", { field: "note", text: "Bonus included" });
    });

    it("saves nothing when a field is left unchanged", async () => {
      await show({ amounts: [payslipField({ value: "3200,00" })] });

      await userEvent.click(gross());
      await userEvent.tab();

      expect(mocked.payslipEdit).not.toHaveBeenCalled();
    });

    it("says what changed with Undo when the save returns a label", async () => {
      mocked.payslipEdit.mockResolvedValue("Edit Payslip July 2026");
      await show();

      await userEvent.type(gross(), "3200,00");
      await userEvent.tab();

      await vi.waitFor(() => expect(toastMessages()).toEqual(["Edit Payslip July 2026"]));
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });

    it("shows a refused field's message under that field only", async () => {
      mocked.payslipEdit.mockRejectedValue({ kind: "invalid_input", message: "payslip.gross: Enter an amount", code: null, field: { path: "payslip.gross", message: "Enter an amount" } });
      await show();

      await userEvent.type(gross(), "abc");
      await userEvent.tab();

      expect((await screen.findByRole("alert")).textContent).toBe("Enter an amount");
      expect(gross().closest("div")?.contains(screen.getByRole("alert"))).toBe(true);
      expect(toastMessages()).toEqual([]);
    });

    it("clears the field message once a later save succeeds", async () => {
      mocked.payslipEdit.mockRejectedValueOnce({ kind: "invalid_input", message: "Enter an amount", code: null, field: { path: "payslip.gross", message: "Enter an amount" } });
      await show();
      await userEvent.type(gross(), "abc");
      await userEvent.tab();
      await screen.findByRole("alert");

      await userEvent.clear(gross());
      await userEvent.type(gross(), "3200,00");
      await userEvent.tab();

      await vi.waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
    });

    it("reports a failure that names no field as an error toast", async () => {
      mocked.payslipEdit.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await show();

      await userEvent.type(gross(), "3200,00");
      await userEvent.tab();

      await vi.waitFor(() => expect(toastMessages()).toEqual(["Disk full"]));
      expect(screen.queryByRole("alert")).toBeNull();
    });
  });

  describe("a closed year", () => {
    it("shows the frozen banner, disables every field and offers no Delete or PDF button", async () => {
      await show({ open: false });

      expect(screen.getByRole("status").textContent).toContain("The tax year is closed; its payslips are frozen.");
      expect(gross()).toBeDisabled();
      expect(screen.getByLabelText(/^Home office days/)).toBeDisabled();
      expect(screen.getByRole("textbox", { name: "Note" })).toBeDisabled();
      expect(screen.queryByRole("button", { name: "Delete Payslip" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Replace PDF…" })).toBeNull();
    });

    it("shows no banner and enabled fields in an open year", async () => {
      await show();

      expect(screen.queryByRole("status")).toBeNull();
      expect(gross()).toBeEnabled();
    });
  });

  describe("the PDF", () => {
    it("shows the PDF preview and offers Replace PDF… when there is one", async () => {
      await show({ hasPdf: true });

      await vi.waitFor(() => expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "payslip", id: "ps-7" }, expect.anything()));
      expect(screen.getByRole("button", { name: "Replace PDF…" })).toBeTruthy();
      expect(screen.queryByText(/No PDF yet/)).toBeNull();
    });

    it("says there is no PDF yet and offers Attach PDF… when there is none", async () => {
      await show({ hasPdf: false });

      expect(screen.getByText("No PDF yet. The numbers count without it.")).toBeTruthy();
      expect(screen.getByRole("button", { name: "Attach PDF…" })).toBeTruthy();
      expect(mocked.pdfViewShow).not.toHaveBeenCalled();
    });

    it("attaches a PDF and says so with Undo", async () => {
      mocked.payslipAttach.mockResolvedValue("Attach PDF to Payslip July 2026");
      await show({ hasPdf: false });

      await userEvent.click(screen.getByRole("button", { name: "Attach PDF…" }));

      expect(mocked.payslipAttach).toHaveBeenCalledWith("ps-7");
      await vi.waitFor(() => expect(toastMessages()).toEqual(["Attach PDF to Payslip July 2026"]));
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });

    it("says nothing when the file dialog is cancelled", async () => {
      await show({ hasPdf: false });

      await userEvent.click(screen.getByRole("button", { name: "Attach PDF…" }));

      await vi.waitFor(() => expect(mocked.payslipAttach).toHaveBeenCalled());
      expect(toastMessages()).toEqual([]);
    });

    it("reports a PDF that cannot be attached as an error toast", async () => {
      mocked.payslipAttach.mockRejectedValue({ kind: "io", message: "Not a PDF", code: null });
      await show({ hasPdf: false });

      await userEvent.click(screen.getByRole("button", { name: "Attach PDF…" }));

      await vi.waitFor(() => expect(toastMessages()).toEqual(["Not a PDF"]));
    });
  });

  describe("Delete Payslip", () => {
    it("deletes the payslip, says so with Undo and goes back to its tax year", async () => {
      await show();

      await userEvent.click(screen.getByRole("button", { name: "Delete Payslip" }));

      expect(mocked.payslipDelete).toHaveBeenCalledWith("ps-7");
      await vi.waitFor(() => expect(useStore.getState().route).toEqual({ page: "year", year: 2026 }));
      expect(toastMessages()).toEqual(["Delete Payslip July 2026"]);
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });

    it("stays on the payslip and reports an error toast when the delete is refused", async () => {
      mocked.payslipDelete.mockRejectedValue({ kind: "conflict", message: "The year is closed", code: null });
      await show();

      await userEvent.click(screen.getByRole("button", { name: "Delete Payslip" }));

      await vi.waitFor(() => expect(toastMessages()).toEqual(["The year is closed"]));
      expect(useStore.getState().route).toEqual({ page: "payslip", payslipId: "ps-7", year: 2026 });
    });
  });

  it("says the payslip was not found, with no form and no error toast, when it cannot be read", async () => {
    mocked.payslipView.mockRejectedValue({ kind: "not_found", message: "No such payslip", code: null });
    renderWithQuery(<PayslipPage payslipId="ps-gone" />);

    expect(await screen.findByText("Payslip not found")).toBeTruthy();
    expect(screen.getByText("It may have been deleted.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Delete Payslip" })).toBeNull();
    expect(toastMessages()).toEqual([]);
  });
});
