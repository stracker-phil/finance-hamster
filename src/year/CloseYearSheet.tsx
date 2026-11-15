import { useEffect, useId, useState } from "react";
import { api, toAppError, type CloseCheck } from "../api";
import { Modal } from "../components/Modal";
import { showError, undoToast } from "../lib/undoToast";
import { useYearCheck } from "../queries/yearCheck";
import { useStore } from "../state/store";
import { useT } from "../i18n";

/** Asked before Close Year while no year check ran after 31 Dec. */
function RunCheckFirst({ year, prompt, onSkip, onClose }: { year: number; prompt: string; onSkip: () => void; onClose: () => void }) {
  const titleId = useId();
  const text = useT().year.close;
  const start = () => {
    onClose();
    useStore.getState().navigate({ page: "yearCheck", year });
  };
  return (
    <Modal labelledBy={titleId} className="sheet--narrow" onClose={onClose}>
      <h2 id={titleId}>{text.checkFirst}</h2>
      <p>{prompt}</p>
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          {text.cancel}
        </button>
        <button type="button" className="btn" onClick={onSkip}>
          {text.withoutCheck}
        </button>
        <button type="button" className="btn btn--primary" autoFocus onClick={start}>
          {text.startCheck}
        </button>
      </div>
    </Modal>
  );
}

/** Close Year, behind the year check prompt while no run after 31 Dec exists. */
export function CloseYearSheet({ year, onClose }: { year: number; onClose: () => void }) {
  const { data: yearCheck } = useYearCheck(year);
  const [skipped, setSkipped] = useState(false);
  if (!yearCheck) return null;
  if (yearCheck.closePrompt && !skipped) return <RunCheckFirst year={year} prompt={yearCheck.closePrompt} onSkip={() => setSkipped(true)} onClose={onClose} />;
  return <CloseSheet year={year} onClose={onClose} />;
}

/** What blocks closing, what only warns, and what closing does. */
function CloseSheet({ year, onClose }: { year: number; onClose: () => void }) {
  const titleId = useId();
  const [check, setCheck] = useState<CloseCheck | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const text = useT().year.close;
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
      <h2 id={titleId}>{text.title(year)}</h2>
      <p>{text.text}</p>
      {check && (
        <ul className="close-list" aria-label={text.before}>
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
              {text.inPlace}
            </li>
          )}
        </ul>
      )}
      <div className="form-field__error" role={error ? "alert" : undefined}>
        {error ?? (blocked ? text.finishFirst : "")}
      </div>
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          {text.cancel}
        </button>
        <button type="button" className="btn btn--primary" disabled={!check || blocked || busy} onClick={close}>
          {text.close}
        </button>
      </div>
    </Modal>
  );
}
