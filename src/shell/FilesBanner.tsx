import { TriangleAlert } from "lucide-react";
import { api, toAppError } from "../api";
import { useLastFileCheck } from "../queries/integrity";
import { useStore } from "../state/store";

const showVerify = () => api.settingsShow("verify-files").catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));

/** A red line over the page while stored PDFs are missing or changed. */
export function FilesBanner() {
  const { data: check } = useLastFileCheck();
  const count = check?.problems.length ?? 0;
  if (count === 0) return null;
  return (
    <div className="files-banner" role="alert">
      <TriangleAlert size={15} />
      <span>{count === 1 ? "1 stored file is missing or changed." : `${count} stored files are missing or changed.`}</span>
      <button className="link-btn" onClick={() => void showVerify()}>
        Show Files
      </button>
    </div>
  );
}
