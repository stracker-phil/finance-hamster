import { useId, useState } from "react";
import { api, toAppError, type ReloadPreview } from "../../api";
import { Modal } from "../../components/Modal";
import { useStore } from "../../state/store";

/** What Reload changes in each open year; Apply stays disabled while a file has a problem. */
export function ReloadSheet({ previews, onClose }: { previews: ReloadPreview[]; onClose: () => void }) {
  const id = useId();
  const [busy, setBusy] = useState(false);
  const blocked = previews.some((p) => p.problems.length > 0);
  const apply = () => {
    setBusy(true);
    api
      .rulesReloadApply()
      .then((label) => {
        useStore.getState().showToast("success", label ? "Reloaded the tax rules" : "Nothing changed");
        onClose();
      })
      .catch((e: unknown) => {
        useStore.getState().showToast("error", toAppError(e).message);
        setBusy(false);
      });
  };
  return (
    <Modal labelledBy={id} onClose={onClose}>
      <h2 id={id}>Reload Tax Rules</h2>
      {previews.map((p) => (
        <section key={p.year} className="reload-year" aria-label={`Tax year ${p.year}`}>
          <h3>
            Tax year {p.year}: {p.from}
            {p.to && p.to !== p.from ? ` → ${p.to}` : ""}
          </h3>
          {p.problems.length > 0 ? (
            <>
              <p>The file can&apos;t be loaded. Fix it in your editor, then reload again.</p>
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
          Cancel
        </button>
        <button type="button" className="btn btn--primary" disabled={blocked || busy} onClick={apply}>
          Apply
        </button>
      </div>
    </Modal>
  );
}
