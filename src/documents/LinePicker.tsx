import { useEffect, useRef, useState } from "react";
import { useDocumentLineOptions } from "../queries/documents";
import { useCoversPdf } from "../lib/pdfCover";

/** A popover listing likely bank lines, or those matching what is typed; choosing one links it. */
export function LinePicker({ documentId, onPick, onClose }: { documentId: string; onPick: (lineId: string) => void; onClose: () => void }) {
  const [query, setQuery] = useState("");
  const { data: options = [], isFetched } = useDocumentLineOptions(documentId, query, true);
  const ref = useRef<HTMLDivElement>(null);
  useCoversPdf(true);
  useEffect(() => {
    const away = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", away);
    return () => document.removeEventListener("mousedown", away);
  }, [onClose]);
  return (
    <div ref={ref} className="menu line-picker" role="dialog" aria-label="Link a bank line">
      <input
        className="field"
        autoFocus
        placeholder="Search bank lines"
        aria-label="Search bank lines"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") onClose();
        }}
      />
      <div className="line-picker__list" role="listbox" aria-label="Bank lines">
        {options.map((o) => (
          <button key={o.id} role="option" aria-selected={false} className="menu-row line-picker__row" onClick={() => onPick(o.id)}>
            <span className="line-picker__text">
              <span>{o.title}</span>
              <span className="add-doc__hint">{o.detail}</span>
            </span>
            <span className="choice__amount">{o.amount}</span>
          </button>
        ))}
        {isFetched && options.length === 0 && <div className="line-picker__empty">{query.trim() ? "No bank line matches" : "No likely bank line; search to find one"}</div>}
      </div>
    </div>
  );
}
