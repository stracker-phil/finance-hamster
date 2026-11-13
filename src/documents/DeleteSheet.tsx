import { useId, useState } from "react";
import { api, toAppError, type ArchivedDocument } from "../api";
import { Modal } from "../components/Modal";
import { useStore } from "../state/store";

/** Deletes an excluded document for good: names the file, asks for a reason and an explicit understanding. */
export function DeleteSheet({ doc, onClose }: { doc: ArchivedDocument; onClose: () => void }) {
  const id = useId();
  const [reason, setReason] = useState("");
  const [understood, setUnderstood] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const remove = () => {
    api
      .documentDelete(doc.id, reason, understood)
      .then((label) => {
        useStore.getState().showToast("success", label);
        onClose();
      })
      .catch((e: unknown) => {
        const failure = toAppError(e);
        setError(failure.field?.message ?? failure.message);
      });
  };
  return (
    <Modal role="alertdialog" labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>Delete {doc.title}?</h2>
      <p>
        The file <span className="mono">{doc.file}</span> is removed from this Mac. The deletion log keeps its name, dates and your reason.
      </p>
      <div className="form-field">
        <label htmlFor={`${id}-reason`}>Reason</label>
        <input id={`${id}-reason`} className="field" autoFocus value={reason} onChange={(e) => setReason(e.target.value)} />
      </div>
      <label className="form-check">
        <input type="checkbox" checked={understood} onChange={(e) => setUnderstood(e.target.checked)} />
        <span>I understand this can&apos;t be undone</span>
      </label>
      <div className="form-field__error" role={error ? "alert" : undefined}>
        {error ?? ""}
      </div>
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="btn btn--danger" disabled={!reason.trim() || !understood} onClick={remove}>
          Delete
        </button>
      </div>
    </Modal>
  );
}
