import type { AddPreview, Estimate, LineChoice } from "../api";

/** A demo bank line offered as the payment of an invoice, with `patch` applied. */
export function lineChoice(patch: Partial<LineChoice> = {}): LineChoice {
  return { id: "line-1", title: "Demo Cloud Ltd", detail: "Business · Tue, 10 Nov · USD 129.00", amount: "−€121.23", ...patch };
}

/** A demo invoice read and waiting in the Add document sheet, with `patch` applied. */
export function addPreview(patch: Partial<AddPreview> = {}): AddPreview {
  return {
    token: "add-1",
    fileName: "invoice-demo-1109.pdf",
    nearDuplicate: null,
    vendor: "Demo Cloud Ltd",
    vendorFromPdf: true,
    invoice: "DEM-2026-1109 · 9 Nov 2026",
    invoiceAmount: "USD 129.00",
    items: [
      { key: "kz-169", label: "KZ 169 · Digital work equipment", kz: "169" },
      { key: "kz-720", label: "KZ 720 · Training", kz: "720" },
    ],
    itemKey: "kz-169",
    itemWhy: "last used for Demo Cloud Ltd",
    lines: [lineChoice(), lineChoice({ id: "line-2", title: "Example Market", detail: "Business · Mon, 9 Nov · EUR 12.00", amount: "−€12.00" })],
    ...patch,
  };
}

/** A demo estimate of what an invoice counts as, with `patch` applied. */
export function estimate(patch: Partial<Estimate> = {}): Estimate {
  return { workShare: 60, shareHint: "from the item: 40 % private", anv: "€72.74", formula: "€121.23 × 60 %", gwg: null, ...patch };
}
