import { useId, useState } from "react";
import { Check, FileText, TriangleAlert } from "lucide-react";
import { api, toAppError, type ImportPreview } from "../api";
import { Modal } from "../components/Modal";
import { useStore } from "../state/store";

function CheckRow({ ok, title, detail, mono }: { ok: boolean; title: string; detail: string; mono?: boolean }) {
  return (
    <div className="import-check">
      <span className={ok ? "import-check__dot" : "import-check__dot import-check__dot--warn"}>{ok ? <Check size={11} strokeWidth={3.5} /> : <TriangleAlert size={11} strokeWidth={2.6} />}</span>
      <div>
        <div>{title}</div>
        <div className={mono ? "import-check__detail mono" : "import-check__detail"}>{detail}</div>
      </div>
    </div>
  );
}

/** What was read and checked; nothing is stored until Import. */
export function ImportSheet({ preview, onImported }: { preview: ImportPreview; onImported?: () => void }) {
  const id = useId();
  const [busy, setBusy] = useState(false);
  const close = () => {
    void api.statementImportCancel(preview.token).catch(() => {});
    useStore.getState().setImportPreview(null);
  };
  const confirm = () => {
    setBusy(true);
    api
      .statementImportCommit(preview.token)
      .then((label) => {
        useStore.getState().setImportPreview(null);
        useStore.getState().showToast("success", label);
        onImported?.();
      })
      .catch((e: unknown) => {
        setBusy(false);
        useStore.getState().showToast("error", toAppError(e).message);
      });
  };
  return (
    <Modal labelledBy={id} onClose={close}>
      <h2 id={id}>Import statement</h2>
      <div className="import-file">
        <FileText size={28} strokeWidth={1.5} />
        <div className="import-file__text">
          <span className="mono">{preview.fileName}</span>
          <span className="import-file__summary">{preview.summary}</span>
        </div>
        <span className="import-file__account">{preview.accountName}</span>
      </div>
      <div className="import-checks">
        <CheckRow ok title={`Read ${preview.lines} transactions`} detail="Account recognized from the IBAN on page 1." />
        <CheckRow ok title="Balance adds up" detail={`${preview.balance} · matches the printed new balance`} mono />
        <CheckRow ok={preview.chain !== "gap"} title={preview.chainTitle} detail={preview.chainDetail} />
      </div>
      <p>
        The PDF is copied unchanged to <span className="mono">{preview.file}</span>.
      </p>
      <div className="actions">
        <button type="button" className="btn" onClick={close}>
          Cancel
        </button>
        <button type="button" className="btn btn--primary" disabled={busy} onClick={confirm}>
          Import
        </button>
      </div>
    </Modal>
  );
}
