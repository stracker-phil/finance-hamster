import type { AddPreview, ImportPreview } from "../api";
import { create } from "zustand";
import { HOME, sameRoute, type Route } from "./route";

export type ToastKind = "success" | "info" | "error";

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
  setAddPreview: (preview: AddPreview | null) => void;
  setImportPreview: (preview: ImportPreview | null) => void;

  /** How many overlays (sheets, menus, toasts, tooltips) are open; the native PDF preview hides while any is. */
  pdfCovers: number;
  coverPdf: () => void;
  uncoverPdf: () => void;

  confirmation: Confirmation | null;
  /** Shows the sheet; resolves true when the owner confirms, false on Cancel or Escape. A new question
   *  replaces an open one, which resolves false. */
  confirm: (question: Omit<Confirmation, "answer">) => Promise<boolean>;
}

let nextToastId = 1;

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
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),

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
  setAddPreview: (addPreview) => set({ addPreview }),
  setImportPreview: (importPreview) => set({ importPreview }),

  pdfCovers: 0,
  coverPdf: () => set({ pdfCovers: get().pdfCovers + 1 }),
  uncoverPdf: () => set({ pdfCovers: Math.max(0, get().pdfCovers - 1) }),

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
