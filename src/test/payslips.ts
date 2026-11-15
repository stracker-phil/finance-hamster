import type { PayslipField, PayslipMonth, PayslipView, PayslipsYear } from "../api";

const NAMES = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/** A demo month of the payslip panel without a payslip, with `patch` applied. */
export function payslipMonth(month: number, patch: Partial<PayslipMonth> = {}): PayslipMonth {
  return { month, name: NAMES[month - 1], id: null, payout: null, hasPdf: false, ...patch };
}

/** A demo month that has a payslip with its PDF, with `patch` applied. */
export function payslipMonthWith(month: number, patch: Partial<PayslipMonth> = {}): PayslipMonth {
  return payslipMonth(month, { id: `ps-${month}`, payout: "€2,400.00", hasPdf: true, ...patch });
}

/** A demo payslip panel of an open year: twelve months, those in `filled` (by month number) holding payslips, with `patch` applied. */
export function payslipsYear(patch: Partial<PayslipsYear> & { filled?: Record<number, Partial<PayslipMonth>> } = {}): PayslipsYear {
  const { filled = {}, ...rest } = patch;
  const months = Array.from({ length: 12 }, (_, i) => (filled[i + 1] ? payslipMonthWith(i + 1, filled[i + 1]) : payslipMonth(i + 1)));
  return { year: 2026, open: true, months, totals: [{ label: "Gross", value: "€30,000.00" }, { label: "Tax", value: "€6,000.00" }], telework: null, ...rest };
}

/** A demo amount field of the payslip form, with `patch` applied. */
export function payslipField(patch: Partial<PayslipField> = {}): PayslipField {
  return { name: "gross", label: "Gross pay", hint: "Line 210", value: "", ...patch };
}

/** A demo payslip of July 2026 in an open year, with `patch` applied. */
export function payslipView(patch: Partial<PayslipView> = {}): PayslipView {
  return {
    id: "ps-7",
    year: 2026,
    month: 7,
    title: "Payslip July 2026",
    open: true,
    hasPdf: true,
    amounts: [payslipField(), payslipField({ name: "tax", label: "Income tax", hint: "Line 260", value: "480,00" })],
    teleworkDays: "",
    note: "",
    ...patch,
  };
}
