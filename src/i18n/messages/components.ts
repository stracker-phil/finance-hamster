import { both } from "../both";

export const components = both(
  {
    cancel: "Cancel",
    menu: "Menu",
    dismiss: "Dismiss",
    somethingWrong: "Something went wrong",
    details: "Details",
    hideDetails: "Hide details",
    tryAgain: "Try Again",
    pdfFailed: "This PDF can't be shown.",
    undo: "Undo",
    undid: (label: string) => `Undid ${label}`,
    redid: (label: string) => `Redid ${label}`,
    nothingToUndo: "Nothing to undo",
    nothingToRedo: "Nothing to redo",
  },
  {
    cancel: "Abbrechen",
    menu: "Menü",
    dismiss: "Schließen",
    somethingWrong: "Etwas ist schiefgelaufen",
    details: "Details",
    hideDetails: "Details ausblenden",
    tryAgain: "Erneut versuchen",
    pdfFailed: "Dieses PDF kann nicht angezeigt werden.",
    undo: "Widerrufen",
    undid: (label: string) => `Widerrufen: ${label}`,
    redid: (label: string) => `Wiederholt: ${label}`,
    nothingToUndo: "Nichts zu widerrufen",
    nothingToRedo: "Nichts zu wiederholen",
  },
);
