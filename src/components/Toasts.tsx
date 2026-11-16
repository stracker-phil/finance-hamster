import { useEffect, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, Info, X } from "lucide-react";
import { useT } from "../i18n";
import { useStore, type Toast } from "../state/store";
import { useCoversPdf } from "../lib/pdfCover";

const AUTO_DISMISS_MS = { success: 2500, info: 4000, error: 8000 };
/** A toast with an action (Undo) stays long enough to reach it. */
const ACTION_DISMISS_MS = 10_000;

function ToastItem({ toast }: { toast: Toast }) {
  const dismiss = useStore((s) => s.dismissToast);
  const ref = useRef<HTMLDivElement>(null);
  useCoversPdf("float", ref);
  useEffect(() => {
    const ttl = toast.action ? ACTION_DISMISS_MS : AUTO_DISMISS_MS[toast.kind];
    const timer = setTimeout(() => dismiss(toast.id), ttl);
    return () => clearTimeout(timer);
  }, [toast, dismiss]);
  const [details, setDetails] = useState(false);
  const text = useT().components;
  const Icon = toast.kind === "error" ? AlertTriangle : toast.kind === "success" ? CheckCircle2 : Info;
  const [headline, ...rest] = toast.kind === "error" ? errorLines(toast.message) : [toast.message];
  return (
    <div ref={ref} className={`toast toast--${toast.kind}`} role={toast.kind === "error" ? "alert" : "status"}>
      <Icon size={16} />
      <span className="toast__text">
        {toast.kind === "error" && <strong className="toast__title">{text.somethingWrong}{toast.count > 1 ? ` (×${toast.count})` : ""}</strong>}
        {toast.kind !== "error" && toast.count > 1 && <span className="toast__count">×{toast.count} </span>}
        <span>{headline}</span>
        {rest.length > 0 && (
          <>
            {" "}
            <button className="link-btn" onClick={() => setDetails((d) => !d)}>
              {details ? text.hideDetails : text.details}
            </button>
            {details && <pre className="toast__details">{rest.join("\n")}</pre>}
          </>
        )}
      </span>
      {toast.action && (
        <button
          className="link-btn"
          onClick={() => {
            dismiss(toast.id);
            toast.action?.run();
          }}
        >
          {toast.action.label}
        </button>
      )}
      <button className="icon-btn icon-btn--tiny" aria-label={text.dismiss} onClick={() => dismiss(toast.id)}>
        <X size={13} />
      </button>
    </div>
  );
}

/** First line is the headline; the raw remainder hides behind "Details". */
function errorLines(message: string): string[] {
  const clean = message.replace(/^Unexpected error:\s*/, "").trim();
  const [first, ...rest] = clean.split(/\n+/);
  const headline = first.length > 160 ? `${first.slice(0, 157)}…` : first;
  const detail = [first.length > 160 ? first : null, ...rest].filter((l): l is string => !!l);
  return [headline, ...detail];
}

export function Toasts() {
  const toasts = useStore((s) => s.toasts);
  return (
    <div className="toasts">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} />
      ))}
    </div>
  );
}
