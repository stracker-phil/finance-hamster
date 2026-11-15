import { useId, useState } from "react";
import { Check, FileText, TriangleAlert } from "lucide-react";
import { api, toAppError, type ImportPreview } from "../api";
import { Modal } from "../components/Modal";
import { useT } from "../i18n";
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
  const text = useT().statements.importSheet;
  const cancel = useT().components.cancel;
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
      <h2 id={id}>{text.title}</h2>
      <div className="import-file">
        <FileText size={28} strokeWidth={1.5} />
        <div className="import-file__text">
          <span className="mono">{preview.fileName}</span>
          <span className="import-file__summary">{preview.summary}</span>
        </div>
        <span className="import-file__account">{preview.accountName}</span>
      </div>
      <div className="import-checks">
        <CheckRow ok title={text.read(preview.lines)} detail={text.recognized} />
        <CheckRow ok title={text.balance} detail={text.balanceDetail(preview.balance)} mono />
        <CheckRow ok={preview.chain !== "gap"} title={preview.chainTitle} detail={preview.chainDetail} />
      </div>
      <p title={preview.file}>{text.copiedTo}</p>
      <div className="actions">
        <button type="button" className="btn" onClick={close}>
          {cancel}
        </button>
        <button type="button" className="btn btn--primary" disabled={busy} onClick={confirm}>
          {text.import}
        </button>
      </div>
    </Modal>
  );
}
