import { useEffect } from "react";
import { setLanguage, useLanguage } from "../i18n";
import { useAppearance } from "../lib/theme";
import { useSettings } from "./settings";

/** Applies the saved appearance, accent and language to this window. Called at each window's root, so a new
 * language renders the whole tree again, texts read with `t()` included. */
export function useLook() {
  useLanguage((s) => s.language);
  const { data } = useSettings();
  useAppearance(data?.appearance, data?.accent);
  const language = data?.language;
  useEffect(() => {
    if (language) setLanguage(language);
  }, [language]);
}
