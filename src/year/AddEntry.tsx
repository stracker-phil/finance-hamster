import { useState } from "react";
import { FileText } from "lucide-react";
import { api, toAppError, type KennzahlView, type NewEntry } from "../api";
import { useStore } from "../state/store";
import { showError, undoToast } from "../lib/undoToast";
import { today } from "../lib/today";
import { useT } from "../i18n";

/** A date in `year`: today while it is that year, else the year's last day. */
const startDate = (year: number) => (today().startsWith(String(year)) ? today() : `${year}-12-31`);

const fileName = (path: string) => path.split("/").pop() ?? path;

type EntryTarget = Pick<KennzahlView, "key" | "kz" | "label" | "add"> & { rate?: string | null };

/** The inline row that adds an amount or a trip to a Kennzahl; `onAdded` gets what it added: its text, else its label. */
export function AddEntry({ year, item, onDone, onAdded }: { year: number; item: EntryTarget; onDone: () => void; onAdded?: (label: string) => void }) {
  const trip = item.add === "trip";
  const [date, setDate] = useState(() => startDate(year));
  const [text, setText] = useState("");
  const [value, setValue] = useState("");
  const [evidence, setEvidence] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const words = useT().year.entry;

  const entry = (): NewEntry =>
    trip ? { kind: "trip", itemKey: item.key, date, km: value, text } : { kind: "amount", itemKey: item.key, date, amount: value, text, evidence };

  const add = () => {
    api
      .yearEntryAdd(year, entry())
      .then((label) => {
        undoToast(label);
        onAdded?.(text.trim() || label);
        onDone();
      })
      .catch((e: unknown) => {
        const failure = toAppError(e);
        setError(failure.field?.message ?? failure.message);
      });
  };
  const choose = () => {
    api
      .yearEvidenceChoose()
      .then((path) => {
        if (path) setEvidence(path);
      })
      .catch(showError);
  };
  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") add();
    if (e.key === "Escape") onDone();
  };

  return (
    <div className="entry-add" role="group" aria-label={trip ? words.addTrip(item.label) : words.addAmount(item.label)}>
      <div className="entry-add__row" onKeyDown={onKey}>
        <span className="year-dot year-dot--accent" />
        <input type="date" className="field entry-add__date" aria-label={words.date} value={date} onChange={(e) => setDate(e.target.value)} />
        <input
          className="field entry-add__text"
          aria-label={trip ? words.destination : words.note}
          placeholder={trip ? words.destinationPlaceholder : words.notePlaceholder}
          value={text}
          autoFocus
          onChange={(e) => setText(e.target.value)}
        />
        <input className="field entry-add__value" aria-label={trip ? words.km : words.amount} placeholder={trip ? words.kmPlaceholder : "€"} inputMode="decimal" value={value} onChange={(e) => setValue(e.target.value)} />
        {trip ? (
          <span className="add-doc__hint entry-add__rate">{item.rate}</span>
        ) : (
          <button className="btn btn--small entry-add__pdf" onClick={choose} title={evidence ?? undefined}>
            <FileText size={13} />
            {evidence ? fileName(evidence) : words.pdf}
          </button>
        )}
        <button className="btn btn--small btn--primary" onClick={add}>
          {words.add}
        </button>
      </div>
      <div className={error ? "add-doc__hint entry-add__note entry-add__note--error" : "add-doc__hint entry-add__note"} role={error ? "alert" : undefined}>
        {error ?? (trip ? words.tripHint : words.amountHint)}
      </div>
    </div>
  );
}

/** The "+ Add" button in a Kennzahl's head: a document opens the add flow, an amount or a trip the inline row. */
export function AddButton({ item, adding, onToggle }: { item: KennzahlView; adding: boolean; onToggle: () => void }) {
  const text = useT().year.entry;
  if (!item.add || item.reported) return null;
  const label = { document: text.document, amount: text.amountKind, trip: text.trip }[item.add];
  const click = () => {
    if (item.add === "document") void startDocument(item.key);
    else onToggle();
  };
  return (
    <button className="link-btn kennzahl__add" aria-expanded={item.add === "document" ? undefined : adding} onClick={click}>
      + {label}
    </button>
  );
}

/** Chooses a PDF and opens Add Document preset to the item; `onAdded` gets the label once it is added. */
export async function startDocument(itemKey: string, onAdded?: (label: string) => void) {
  try {
    const preview = await api.documentAddChoose();
    if (preview) useStore.getState().setAddPreview({ ...preview, itemKey, itemWhy: null }, onAdded);
  } catch (e) {
    showError(e);
  }
}
