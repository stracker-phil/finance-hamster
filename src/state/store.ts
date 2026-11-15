import type { AddPreview, ImportPreview, PlanForm, ReviewLine, SubscriptionForm } from "../api";
import { create } from "zustand";
import type { MenuAt, MenuEntry, OpenMenu } from "../components/menu/types";
import { HOME, sameRoute, type Route } from "./route";

export type ToastKind = "success" | "info" | "error";

/** New starts empty or from a bank line; Edit and Change Plan start from the subscription. */
export type SubscriptionSheet =
  | { kind: "new"; form: SubscriptionForm | null; source?: ReviewLine }
  | { kind: "edit"; id: string; form: SubscriptionForm }
  | { kind: "plan"; id: string; name: string; currentPlan: string; form: PlanForm; source?: ReviewLine };

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
  back: Route[];
  forward: Route[];
  navigate: (route: Route) => void;
  goBack: () => void;
  goForward: () => void;

  paletteOpen: boolean;
  /** The text the palette opens with: what the filter field held when ⌘↩ searched everywhere. */
  paletteQuery: string;
  searchEverywhere: (query: string) => void;
  setPaletteOpen: (open: boolean) => void;

  /** The header's filter text; each page clears it. */
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
  setAddPreview: (preview: AddPreview | null, onAdded?: (label: string) => void) => void;
  /** The open New, Edit or Change Plan sheet of a subscription. */
  subscriptionSheet: SubscriptionSheet | null;
  setSubscriptionSheet: (sheet: SubscriptionSheet | null) => void;
  setImportPreview: (preview: ImportPreview | null) => void;

  /** How many overlays (sheets, menus, toasts, tooltips) are open; the native PDF preview hides while any is. */
  pdfCovers: number;
  coverPdf: () => void;
  uncoverPdf: () => void;

  /** The one open menu (ADR 0030). */
  menu: OpenMenu | null;
  /** Shows a menu, replacing an open one; opening it again on the same element closes it. */
  openMenu: (entries: MenuEntry[], at: MenuAt, label?: string) => void;
  closeMenu: () => void;

  confirmation: Confirmation | null;
  /** Shows the sheet; resolves true when the owner confirms, false on Cancel or Escape. A new question
   *  replaces an open one, which resolves false. */
  confirm: (question: Omit<Confirmation, "answer">) => Promise<boolean>;
}

let nextToastId = 1;
let nextMenuId = 1;

/** A plain toast equal to the last one adds to its count instead of stacking. */
const repeats = (last: Toast, kind: ToastKind, message: string, action?: Toast["action"]) =>
  !action && !last.action && last.kind === kind && last.message === message;

export const useStore = create<AppStore>((set, get) => ({
  route: HOME,
  back: [],
  forward: [],
  navigate: (route) => {
    const { route: current, back } = get();
    if (sameRoute(route, current)) return;
    set({ route, back: [...back, current], forward: [], filter: "" });
  },
  goBack: () => {
    const { route, back, forward } = get();
    const previous = back.at(-1);
    if (!previous) return;
    set({ route: previous, back: back.slice(0, -1), forward: [route, ...forward], filter: "" });
  },
  goForward: () => {
    const { route, back, forward } = get();
    const [next, ...rest] = forward;
    if (!next) return;
    set({ route: next, back: [...back, route], forward: rest, filter: "" });
  },

  paletteOpen: false,
  paletteQuery: "",
  searchEverywhere: (paletteQuery) => set({ paletteOpen: true, paletteQuery }),
  setPaletteOpen: (paletteOpen) => set({ paletteOpen, paletteQuery: "" }),

  filter: "",
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

  importPreview: null,
  addPreview: null,
  addOnAdded: null,
  setAddPreview: (addPreview, onAdded) => set({ addPreview, addOnAdded: addPreview ? (onAdded ?? null) : null }),
  subscriptionSheet: null,
  setSubscriptionSheet: (subscriptionSheet) => set({ subscriptionSheet }),
  setImportPreview: (importPreview) => set({ importPreview }),

  pdfCovers: 0,
  coverPdf: () => set({ pdfCovers: get().pdfCovers + 1 }),
  uncoverPdf: () => set({ pdfCovers: Math.max(0, get().pdfCovers - 1) }),

  menu: null,
  openMenu: (entries, at, label = "Menu") => {
    if (at instanceof HTMLElement && get().menu?.at === at) return set({ menu: null });
    const focused = document.activeElement;
    const opener = at instanceof HTMLElement ? at : focused instanceof HTMLElement && focused !== document.body ? focused : null;
    set({ menu: { id: nextMenuId++, entries, at, label, opener } });
  },
  closeMenu: () => set({ menu: null }),

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
