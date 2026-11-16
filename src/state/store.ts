import type { AddPreview, DocumentsTab, ImportPreview, PlanForm, ReviewLine, SubscriptionForm } from "../api";
import { create } from "zustand";
import type { MenuAt, MenuEntry, OpenMenu } from "../components/menu/types";
import { t } from "../i18n";
import { HOME, sameRoute, type Route } from "./route";

export type ToastKind = "success" | "info" | "error";

/** What a PDF in the Add sheet becomes. */
export type AddKind = "invoice" | "payslip" | "other";

/** New starts empty or from a bank line; Edit and Change Plan start from the subscription. */
export type SubscriptionSheet =
  | { kind: "new"; form: SubscriptionForm | null; source?: ReviewLine }
  | { kind: "edit"; id: string; form: SubscriptionForm }
  | { kind: "plan"; id: string; name: string; currentPlan: string; form: PlanForm; source?: ReviewLine };

/** An overlay above the native PDF preview: a sheet hides it, a popover or float is cut out of it. While a
 *  popover (a menu) is open, clicks on the preview go to the page, so they close it. */
export interface PdfCover {
  kind: "sheet" | "popover" | "float";
  element: HTMLElement | null;
}

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  /** How often the same message came in a row; shown as ×N instead of a second toast. */
  count: number;
  action?: { label: string; run: () => void };
}

/** A question that interrupts before something is lost. */
export interface Confirmation {
  title: string;
  text: string;
  /** The destructive button's label, e.g. "Delete". */
  action: string;
  answer: (confirmed: boolean) => void;
}

/** UI-only state of the main window; data reads go through TanStack Query (ADR 0014). */
export interface AppStore {
  route: Route;
  back: Visit[];
  forward: Visit[];
  /** Where the current page's list starts scrolled: 0 on a new visit, the left position on Back or Forward. */
  scroll: number;
  /** The page's list scrolled to `top`; kept for the history without a render. */
  noteScroll: (top: number) => void;
  navigate: (route: Route) => void;
  goBack: () => void;
  goForward: () => void;

  paletteOpen: boolean;
  /** The text the palette opens with: what the filter field held when ⌘↩ searched everywhere. */
  paletteQuery: string;
  searchEverywhere: (query: string) => void;
  setPaletteOpen: (open: boolean) => void;

  /** The header's filter text; a new visit starts without one, Back and Forward bring it back. */
  filter: string;
  setFilter: (filter: string) => void;

  toasts: Toast[];
  showToast: (kind: ToastKind, message: string, action?: Toast["action"]) => void;
  dismissToast: (id: number) => void;

  /** A read statement waiting in the Import sheet. */
  importPreview: ImportPreview | null;
  addPreview: AddPreview | null;
  /** Told the new document's label once the open Add Document sheet adds it; never on cancel. */
  addOnAdded: ((label: string) => void) | null;
  /** The kind the open Add sheet starts on. */
  addKind: AddKind;
  setAddPreview: (preview: AddPreview | null, onAdded?: (label: string) => void, kind?: AddKind) => void;
  /** The tab Dokumente opened last in this session. */
  documentsTab: DocumentsTab;
  setDocumentsTab: (tab: DocumentsTab) => void;
  /** The year Accounts showed last in this session; null for this year. */
  statementsYear: number | null;
  setStatementsYear: (year: number) => void;
  /** The open New, Edit or Change Plan sheet of a subscription. */
  subscriptionSheet: SubscriptionSheet | null;
  setSubscriptionSheet: (sheet: SubscriptionSheet | null) => void;
  setImportPreview: (preview: ImportPreview | null) => void;

  /** The open overlays above the native PDF preview (ADR 0057). */
  pdfCovers: PdfCover[];
  coverPdf: (cover: PdfCover) => void;
  uncoverPdf: (cover: PdfCover) => void;

  /** The one open menu (ADR 0030). */
  menu: OpenMenu | null;
  /** Shows a menu, replacing an open one; opening it again on the same element closes it. */
  openMenu: (entries: MenuEntry[], at: MenuAt, label?: string) => void;
  closeMenu: () => void;

  /** Back to Today with nothing open and nothing remembered from the profile before (ADR 0061). */
  leaveProfile: () => void;

  confirmation: Confirmation | null;
  /** Shows the sheet; resolves true when the owner confirms, false on Cancel or Escape. A new question
   *  replaces an open one, which resolves false. */
  confirm: (question: Omit<Confirmation, "answer">) => Promise<boolean>;
}

/** A page left for another, as Back or Forward returns to it. */
export interface Visit {
  route: Route;
  filter: string;
  scroll: number;
}

let scrolledTo = 0;

const leaving = (): Visit => {
  const { route, filter } = useStore.getState();
  return { route, filter, scroll: scrolledTo };
};

const returnTo = ({ route, filter, scroll }: Visit) => {
  scrolledTo = scroll;
  return { route, filter, scroll };
};

let nextToastId = 1;
let nextMenuId = 1;

/** A plain toast equal to the last one adds to its count instead of stacking. */
const repeats = (last: Toast, kind: ToastKind, message: string, action?: Toast["action"]) =>
  !action && !last.action && last.kind === kind && last.message === message;

/** Where a session starts, and where a profile switch returns to (`leaveProfile`). */
const SESSION_START = {
  route: HOME,
  back: [],
  forward: [],
  scroll: 0,
  filter: "",
  paletteOpen: false,
  paletteQuery: "",
  importPreview: null,
  addPreview: null,
  addOnAdded: null,
  addKind: "invoice",
  documentsTab: "invoices",
  statementsYear: null,
  subscriptionSheet: null,
  menu: null,
} satisfies Partial<AppStore>;

export const useStore = create<AppStore>((set, get) => ({
  ...SESSION_START,
  navigate: (route) => {
    const { route: current, back } = get();
    if (sameRoute(route, current)) return;
    set({ route, back: [...back, leaving()], forward: [], filter: "", scroll: 0 });
    scrolledTo = 0;
  },
  goBack: () => {
    const { back, forward } = get();
    const previous = back.at(-1);
    if (!previous) return;
    const left = leaving();
    set({ ...returnTo(previous), back: back.slice(0, -1), forward: [left, ...forward] });
  },
  goForward: () => {
    const { back, forward } = get();
    const [next, ...rest] = forward;
    if (!next) return;
    const left = leaving();
    set({ ...returnTo(next), back: [...back, left], forward: rest });
  },
  noteScroll: (top) => {
    scrolledTo = top;
  },

  searchEverywhere: (paletteQuery) => set({ paletteOpen: true, paletteQuery }),
  setPaletteOpen: (paletteOpen) => set({ paletteOpen, paletteQuery: "" }),

  setFilter: (filter) => set({ filter }),

  toasts: [],
  showToast: (kind, message, action) => {
    const toasts = get().toasts;
    const last = toasts.at(-1);
    if (last && repeats(last, kind, message, action)) {
      set({ toasts: [...toasts.slice(0, -1), { ...last, id: nextToastId++, count: last.count + 1 }] });
      return;
    }
    set({ toasts: [...toasts, { id: nextToastId++, kind, message, count: 1, action }] });
  },
  dismissToast: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),

  setAddPreview: (addPreview, onAdded, kind = "invoice") => set({ addPreview, addOnAdded: addPreview ? (onAdded ?? null) : null, addKind: kind }),
  setDocumentsTab: (documentsTab) => set({ documentsTab }),
  setStatementsYear: (statementsYear) => set({ statementsYear }),
  setSubscriptionSheet: (subscriptionSheet) => set({ subscriptionSheet }),
  setImportPreview: (importPreview) => set({ importPreview }),

  pdfCovers: [],
  coverPdf: (cover) => set({ pdfCovers: [...get().pdfCovers, cover] }),
  uncoverPdf: (cover) => set({ pdfCovers: get().pdfCovers.filter((c) => c !== cover) }),

  openMenu: (entries, at, label = t().components.menu) => {
    if (at instanceof HTMLElement && get().menu?.at === at) return set({ menu: null });
    const focused = document.activeElement;
    const opener = at instanceof HTMLElement ? at : focused instanceof HTMLElement && focused !== document.body ? focused : null;
    set({ menu: { id: nextMenuId++, entries, at, label, opener } });
  },
  closeMenu: () => set({ menu: null }),

  leaveProfile: () => {
    get().confirmation?.answer(false);
    scrolledTo = 0;
    set(SESSION_START);
  },

  confirmation: null,
  confirm: (question) =>
    new Promise<boolean>((resolve) => {
      get().confirmation?.answer(false);
      set({
        confirmation: {
          ...question,
          answer: (confirmed) => {
            set({ confirmation: null });
            resolve(confirmed);
          },
        },
      });
    }),
}));
