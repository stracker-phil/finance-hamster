import { useEffect, useRef, useState } from "react";
import { api, type Frame, type PdfSource } from "../api";
import { useT } from "../i18n";

function frameOf(element: HTMLElement): Frame {
  const r = element.getBoundingClientRect();
  return { x: r.left, y: r.top, width: r.width, height: r.height };
}

/** The place the native PDF preview shows in (ADR 0037): Rust lays a PDFView over this box and moves it along
 *  while the page scrolls or resizes. */
export function PdfPreview({ source }: { source: PdfSource }) {
  const ref = useRef<HTMLDivElement>(null);
  const [failed, setFailed] = useState(false);
  const text = useT().components;
  const { kind, id } = source;
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    let shown = false;
    let pending = 0;
    const report = () => {
      cancelAnimationFrame(pending);
      pending = requestAnimationFrame(() => {
        if (shown) void api.pdfViewMove(frameOf(element)).catch(() => {});
      });
    };
    setFailed(false);
    api
      .pdfViewShow({ kind, id }, frameOf(element))
      .then(() => {
        shown = true;
        report();
      })
      .catch(() => setFailed(true));
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(report);
    observer?.observe(element);
    window.addEventListener("resize", report);
    document.addEventListener("scroll", report, true);
    return () => {
      cancelAnimationFrame(pending);
      observer?.disconnect();
      window.removeEventListener("resize", report);
      document.removeEventListener("scroll", report, true);
      void api.pdfViewHide().catch(() => {});
    };
  }, [kind, id]);
  return (
    <div ref={ref} className="pdf-preview" data-pdf-preview>
      {failed && <div className="pdf-preview__error">{text.pdfFailed}</div>}
    </div>
  );
}
