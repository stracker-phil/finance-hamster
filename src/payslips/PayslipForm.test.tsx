import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { useStore } from "../state/store";
import { payslipField, payslipView } from "../test/payslips";
import { PayslipForm } from "./PayslipForm";

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
});

describe("PayslipForm", () => {
  it("shows a field's hint next to its label", () => {
    render(<PayslipForm payslip={payslipView({ amounts: [payslipField({ name: "gross", label: "Gross pay", hint: "Line 210" })] })} />);

    expect(screen.getByText("Line 210")).toBeTruthy();
    expect(screen.getByLabelText(/Gross pay/)).toBeTruthy();
  });

  it("leaves out the hint of a field whose hint is empty, so the label stands alone", () => {
    render(<PayslipForm payslip={payslipView({ amounts: [payslipField({ name: "net", label: "Net pay (netto)", hint: "" })] })} />);

    const label = screen.getByText("Net pay (netto)");
    expect(label.tagName).toBe("LABEL");
    expect(label.childElementCount).toBe(0);
  });
});
