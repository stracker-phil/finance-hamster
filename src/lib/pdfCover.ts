import { useEffect } from "react";
import { api, toAppError } from "../api";
import { useStore } from "../state/store";

/** Marks an overlay as open while `active`: the native PDF preview sits above the page and would cover it
 *  (ADR 0037). */
export function useCoversPdf(active = true) {
  useEffect(() => {
    if (!active) return;
    useStore.getState().coverPdf();
    return () => useStore.getState().uncoverPdf();
  }, [active]);
}

/** Hides the native preview while any overlay is open and shows it again after the last one closes. */
export function usePdfCovers() {
  const covered = useStore((s) => s.pdfCovers > 0);
  useEffect(() => {
    if (!covered) return;
    const report = (e: unknown) => api.logFrontendError(`PDF preview: ${toAppError(e).message}`).catch(() => {});
    api.pdfViewSuspend().catch(report);
    return () => void api.pdfViewResume().catch(report);
  }, [covered]);
}
