import { useEffect } from "react";
import { setLanguage } from "../i18n";
import { useAppearance } from "../lib/theme";
import { useSettings } from "./settings";

/** Applies the saved appearance, accent and language to this window. */
export function useLook() {
  const { data } = useSettings();
  useAppearance(data?.appearance, data?.accent);
  const language = data?.language;
  useEffect(() => {
    if (language) setLanguage(language);
  }, [language]);
}
