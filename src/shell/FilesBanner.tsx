import { TriangleAlert } from "lucide-react";
import { api, toAppError } from "../api";
import { useT } from "../i18n";
import { useLastFileCheck } from "../queries/integrity";
import { useStore } from "../state/store";

const showVerify = () => api.settingsShow("verify-files").catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));

/** A red line over the page while stored PDFs are missing or changed. */
export function FilesBanner() {
  const { data: check } = useLastFileCheck();
  const count = check?.problems.length ?? 0;
  const text = useT().shell.filesBanner;
  if (count === 0) return null;
  return (
    <div className="files-banner" role="alert">
      <TriangleAlert size={15} />
      <span>{text.problems(count)}</span>
      <button className="link-btn" onClick={() => void showVerify()}>
        {text.show}
      </button>
    </div>
  );
}
