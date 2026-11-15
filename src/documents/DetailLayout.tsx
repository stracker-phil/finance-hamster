import type { ReactNode } from "react";
import type { PdfSource } from "../api";
import { PdfPreview } from "../components/PdfPreview";

/** Every kind of document opens alike (ADR 0056): its PDF over most of the page, its details in a sidebar on the
 *  right. `source` is null while there's no PDF; `empty` then takes its place. `fileTitle` (the stored path) shows on
 *  hover. */
export function DetailLayout({ source, empty, banners, label, fileTitle, children }: { source: PdfSource | null; empty?: ReactNode; banners?: ReactNode; label: string; fileTitle?: string; children: ReactNode }) {
  return (
    <div className="detail">
      {banners}
      <div className="detail__body">
        <div className="detail__pdf" title={fileTitle}>{source ? <PdfPreview source={source} /> : <div className="detail__empty">{empty}</div>}</div>
        <aside className="detail__side" aria-label={label}>
          {children}
        </aside>
      </div>
    </div>
  );
}
