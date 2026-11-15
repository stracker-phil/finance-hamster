import { useState } from "react";
import { Check } from "lucide-react";
import { api, toAppError, type CheckQuestion } from "../api";
import { AddEntry } from "../year/AddEntry";
import { useT } from "../i18n";

/** What a Yes added to a question during this run. */
export type Added = { count: number; label: string };

type Props = {
  year: number;
  question: CheckQuestion;
  focused: boolean;
  added: Added | undefined;
  /** The inline amount or trip row is open. */
  adding: boolean;
  onFocus: () => void;
  onYes: () => void;
  /** No, Done, or a saved number: the question leaves the run. */
  onAnswer: () => void;
  onLater: () => void;
  onAdded: (label: string) => void;
  onCloseInput: () => void;
};

/** The op label without its verb, which "Added 1 ·" already says: `Add Trip to KZ 721` → `Trip to KZ 721`,
 * `Fahrt zu KZ 721 hinzufügen` → `Fahrt zu KZ 721`. */
const shortLabel = (label: string) => label.replace(/^Add /, "").replace(/ hinzufügen$/, "");

/** One question of the year check. */
export function CheckRow(p: Props) {
  const { question: q } = p;
  const [value, setValue] = useState(q.value);
  const [error, setError] = useState<string | null>(null);
  const isNumber = q.add === null;
  const save = () => {
    setError(null);
    api
      .yearAnswer(p.year, q.key, value)
      .then(p.onAnswer)
      .catch((e: unknown) => {
        const failure = toAppError(e);
        setError(failure.field?.message ?? failure.message);
      });
  };
  const text = useT().year.check;
  const cls = ["check-row", p.focused && "check-row--focused", p.added && "check-row--added"].filter(Boolean).join(" ");
  return (
    <div className={cls} role="group" aria-label={q.question} onFocus={p.onFocus} onMouseDown={p.onFocus}>
      <div className="check-row__main">
        <span className="check-row__kz">{q.kz ? `KZ ${q.kz}` : text.question}</span>
        <div className="check-row__text">
          <div className="check-row__question">{q.question}</div>
          {q.hint && <div className="check-row__hint">{q.hint}</div>}
          {p.added && (
            <div className="check-row__added">
              <Check size={13} strokeWidth={2.6} />
              {text.added(p.added.count, shortLabel(p.added.label))}
            </div>
          )}
          {error && (
            <div className="check-row__error" role="alert">
              {error}
            </div>
          )}
        </div>
        <div className="check-row__buttons">
          {isNumber ? (
            <>
              <label className="check-row__number">
                <input className="field" aria-label={q.question} inputMode="decimal" value={value} onChange={(e) => setValue(e.target.value)} onKeyDown={(e) => e.key === "Enter" && save()} />
                {q.unit && <span>{q.unit}</span>}
              </label>
              <button className="btn btn--small" onClick={save}>
                {text.save}
              </button>
              <button className="btn btn--small btn--ghost" onClick={p.onLater}>
                {text.later}
              </button>
            </>
          ) : p.added ? (
            <>
              <button className="btn btn--small" onClick={p.onYes}>
                {text.another}
              </button>
              <button className="btn btn--small btn--primary" onClick={p.onAnswer}>
                {text.done}
              </button>
            </>
          ) : (
            <>
              <button className="btn btn--small" onClick={p.onYes}>
                {text.yes}
              </button>
              <button className="btn btn--small" onClick={p.onAnswer}>
                {text.no}
              </button>
              <button className="btn btn--small btn--ghost" onClick={p.onLater}>
                {text.later}
              </button>
            </>
          )}
        </div>
      </div>
      {p.adding && q.add && q.add !== "document" && (
        <AddEntry year={p.year} item={{ key: q.key, kz: q.kz ?? "", add: q.add }} onDone={p.onCloseInput} onAdded={p.onAdded} />
      )}
    </div>
  );
}
