import { useId, useState } from "react";
import { api, toAppError } from "../api";
import { Modal } from "../components/Modal";
import { useStore } from "../state/store";

/** Excludes a document with a reason: it leaves every total and list and shows only in its year's Archive. */
export function ExcludeSheet({ documentId, title, onClose }: { documentId: string; title: string; onClose: () => void }) {
  const id = useId();
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const exclude = () => {
    api
      .documentStep(documentId, { kind: "exclude", reason })
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
    <Modal labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>Exclude {title}?</h2>
      <p>It leaves every total and list and shows only in its year&apos;s Archive. You can restore it anytime.</p>
      <div className="form-field">
        <label htmlFor={`${id}-reason`}>Reason</label>
        <input id={`${id}-reason`} className="field" autoFocus value={reason} placeholder="Duplicate of another invoice" onChange={(e) => setReason(e.target.value)} />
        <div className="form-field__error" role={error ? "alert" : undefined}>
          {error ?? ""}
        </div>
      </div>
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="btn btn--danger" disabled={!reason.trim()} onClick={exclude}>
          Exclude
        </button>
      </div>
    </Modal>
  );
}
