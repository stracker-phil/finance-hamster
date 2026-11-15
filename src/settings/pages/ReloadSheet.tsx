import { useId, useState } from "react";
import { api, toAppError, type ReloadPreview } from "../../api";
import { Modal } from "../../components/Modal";
import { t, useT } from "../../i18n";
import { useStore } from "../../state/store";

/** What Reload changes in each open year; Apply stays disabled while a file has a problem. */
export function ReloadSheet({ previews, onClose }: { previews: ReloadPreview[]; onClose: () => void }) {
  const id = useId();
  const { reload: text, common } = useT().settings;
  const [busy, setBusy] = useState(false);
  const blocked = previews.some((p) => p.problems.length > 0);
  const apply = () => {
    setBusy(true);
    api
      .rulesReloadApply()
      .then((label) => {
        useStore.getState().showToast("success", label ? t().settings.reload.reloaded : t().settings.reload.nothing);
        onClose();
      })
      .catch((e: unknown) => {
        useStore.getState().showToast("error", toAppError(e).message);
        setBusy(false);
      });
  };
  return (
    <Modal labelledBy={id} onClose={onClose}>
      <h2 id={id}>{text.title}</h2>
      {previews.map((p) => (
        <section key={p.year} className="reload-year" aria-label={text.year(p.year)}>
          <h3>
            {text.year(p.year)}: {p.from}
            {p.to && p.to !== p.from ? ` → ${p.to}` : ""}
          </h3>
          {p.problems.length > 0 ? (
            <>
              <p>{text.cantLoad}</p>
              <ul className="reload-list reload-list--problems">
                {p.problems.map((line) => (
                  <li key={line}>{line}</li>
                ))}
              </ul>
            </>
          ) : (
            <ul className="reload-list">
              {p.changes.map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          )}
        </section>
      ))}
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          {common.cancel}
        </button>
        <button type="button" className="btn btn--primary" disabled={blocked || busy} onClick={apply}>
          {text.apply}
        </button>
      </div>
    </Modal>
  );
}
