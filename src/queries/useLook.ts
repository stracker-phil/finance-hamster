import { useAppearance } from "../lib/theme";
import { useSettings } from "./settings";

/** Applies the saved appearance and accent to this window. */
export function useLook() {
  const { data } = useSettings();
  useAppearance(data?.appearance, data?.accent);
}
