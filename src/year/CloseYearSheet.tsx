import { useEffect, useId, useState } from "react";
import { api, toAppError, type CloseCheck } from "../api";
import { Modal } from "../components/Modal";
import { showError, undoToast } from "../lib/undoToast";

/** Close Year: what blocks it, what only warns, and what closing does. */
export function CloseYearSheet({ year, onClose }: { year: number; onClose: () => void }) {
  const titleId = useId();
  const [check, setCheck] = useState<CloseCheck | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    api.yearCloseCheck(year).then(setCheck).catch(showError);
  }, [year]);
  const blocked = (check?.blockers.length ?? 0) > 0;
  const close = () => {
    setBusy(true);
    api
      .yearClose(year)
      .then((label) => {
        undoToast(label);
        onClose();
      })
      .catch((e: unknown) => {
        setError(toAppError(e).message);
        setBusy(false);
      });
  };
  return (
    <Modal labelledBy={titleId} className="sheet--narrow" onClose={onClose}>
      <h2 id={titleId}>Close Tax Year {year}?</h2>
      <p>Hamster keeps a snapshot of every Kennzahl and a summary PDF. The year is then read-only until you reopen it.</p>
      {check && (
        <ul className="close-list" aria-label="Before closing">
          {check.blockers.map((b) => (
            <li key={b} className="close-list__item close-list__item--blocker">
              <span className="year-dot year-dot--warn" />
              {b}
            </li>
          ))}
          {check.warnings.map((w) => (
            <li key={w} className="close-list__item">
              <span className="year-dot" />
              {w}
            </li>
          ))}
          {check.blockers.length === 0 && check.warnings.length === 0 && (
            <li className="close-list__item">
              <span className="year-dot year-dot--done" />
              Everything is in place
            </li>
          )}
        </ul>
      )}
      <div className="form-field__error" role={error ? "alert" : undefined}>
        {error ?? (blocked ? "Finish these first, then close the year." : "")}
      </div>
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="btn btn--primary" disabled={!check || blocked || busy} onClick={close}>
          Close Year
        </button>
      </div>
    </Modal>
  );
}
