import type { ReviewLine } from "../api";
import { selectedIn } from "../lib/selection";
import { useT } from "../i18n";

/** The bank line a sheet was opened from; with `onSelect`, selected text is handed over upper-cased. */
export function SourceLine({ line, onSelect }: { line: ReviewLine; onSelect?: (text: string) => void }) {
  const pick = (e: React.MouseEvent<HTMLElement>) => {
    const text = selectedIn(e.currentTarget);
    if (text && onSelect) onSelect(text.toUpperCase());
  };
  const text = useT().subscriptions.source;
  return (
    <div className="sub-source" aria-label={text.bankLine} onMouseUp={pick}>
      <div className="sub-source__head">
        <span className="sub-source__meta">
          {line.accountName} · {line.date}
        </span>
        <span className="sub-source__title">{line.title}</span>
        <span className="sub-source__amount">{line.amount}</span>
      </div>
      <pre className="sub-source__raw">{line.raw}</pre>
    </div>
  );
}
