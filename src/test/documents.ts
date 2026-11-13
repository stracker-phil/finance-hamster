import type { AddPreview, DocumentGroup, DocumentRowView, DocumentView, Estimate, LineChoice, VendorView, VendorsView } from "../api";

/** A demo draft document with a payment date, one linked bank line and an estimate, with `patch` applied. */
export function documentView(patch: Partial<DocumentView> = {}): DocumentView {
  return {
    id: "doc-1",
    title: "Demo Cloud Ltd · DEM-2026-1109",
    state: "draft",
    file: "Documents/Inbox/demo-cloud-1109.pdf",
    payDate: "2026-11-10",
    year: 2026,
    lines: [lineChoice()],
    paid: "€121.23",
    paidFromLines: true,
    paidText: null,
    vendor: "Demo Cloud Ltd",
    vendors: [
      { name: "Demo Cloud Ltd", detail: "2 documents" },
      { name: "Example Market", detail: "1 document" },
    ],
    items: [
      { key: "kz-169", label: "KZ 169 · Digital work equipment", kz: "169" },
      { key: "kz-720", label: "KZ 720 · Training", kz: "720" },
    ],
    itemKey: "kz-169",
    workShare: 100,
    estimate: estimate({ workShare: 100, shareHint: null, anv: "€121.23", formula: "€121.23 × 100 %" }),
    note: "",
    invoiceAmount: null,
    invoiceCurrency: "EUR",
    missing: [],
    ...patch,
  };
}

/** A demo row of the Documents list, with `patch` applied. */
export function documentRow(patch: Partial<DocumentRowView> = {}): DocumentRowView {
  return { id: "doc-1", title: "Demo Cloud Ltd", sub: "10 Nov · KZ 169 Digital work equipment", state: "finished", anv: "€121.23", ...patch };
}

/** A demo group of the Documents list (Tax year 2026), with `patch` applied. */
export function documentGroup(patch: Partial<DocumentGroup> = {}): DocumentGroup {
  return { title: "Tax year 2026", year: 2026, count: "1 document", total: "€121.23", documents: [documentRow()], ...patch };
}

/** A demo vendor of Settings → Vendors, with `patch` applied. */
export function vendorView(patch: Partial<VendorView> = {}): VendorView {
  return { id: "ven-1", name: "Demo Cloud Ltd", detail: "2 documents · KZ 169", defaultItemKey: null, unused: false, ...patch };
}

/** A demo vendors view with two vendors, with `patch` applied. */
export function vendorsView(patch: Partial<VendorsView> = {}): VendorsView {
  return {
    vendors: [vendorView(), vendorView({ id: "ven-2", name: "Example Market", detail: "1 document · KZ 720", unused: false })],
    items: [
      { key: "kz-169", label: "KZ 169 · Digital work equipment", kz: "169" },
      { key: "kz-720", label: "KZ 720 · Training", kz: "720" },
    ],
    ...patch,
  };
}

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
