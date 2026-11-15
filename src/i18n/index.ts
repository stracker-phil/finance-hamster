import { create } from "zustand";
import type { Language } from "../api";
import { de } from "./de";
import { en, type Messages } from "./en";

export type { Messages };

const CATALOGS: Record<Language, Messages> = { en, de };

/** The window's language, set from the settings by `useLook`; English until they load. */
export const useLanguage = create<{ language: Language }>(() => ({ language: "en" }));

export function setLanguage(language: Language) {
  useLanguage.setState({ language });
  document.documentElement.lang = language;
}

/** The texts in the current language, for code outside components. */
export const t = (): Messages => CATALOGS[useLanguage.getState().language];

/** The texts in the current language; the component renders again when it changes. */
export const useT = (): Messages => CATALOGS[useLanguage((s) => s.language)];
