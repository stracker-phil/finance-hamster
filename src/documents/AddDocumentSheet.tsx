import { useId, useState } from "react";
import { Check, FileText, TriangleAlert } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { api, toAppError, type AddPreview, type LineChoice } from "../api";
import { Modal } from "../components/Modal";
import { useStore } from "../state/store";

function AddDocumentFile({ preview }: { preview: AddPreview }) {
  return (
    <div className="import-file">
      <FileText size={26} strokeWidth={1.5} />
      <div className="import-file__text">
        <span className="mono">{preview.fileName}</span>
        {preview.nearDuplicate ? (
          <span className="add-doc__warn">
            <TriangleAlert size={12} /> {preview.nearDuplicate}
          </span>
        ) : (
          <span className="add-doc__ok">
            <Check size={12} /> Not added before · no similar document found
          </span>
        )}
      </div>
    </div>
  );
}

/** The bank lines that may have paid the invoice, and "link later" as `null`. */
function PaidWithPicker({ lines, value, onChange }: { lines: LineChoice[]; value: string | null; onChange: (id: string | null) => void }) {
  const group = useId();
  const options: { id: string | null; line?: LineChoice }[] = [...lines.map((line) => ({ id: line.id, line })), { id: null }];
  return (
    <div className="add-doc__lines" role="radiogroup" aria-label="Paid with">
      {options.map(({ id, line }) => (
        <label key={id ?? "later"} className={value === id ? "choice choice--on" : "choice"}>
          <input type="radio" name={group} checked={value === id} onChange={() => onChange(id)} aria-label={line ? `${line.title}, ${line.detail}, ${line.amount}` : "Not imported yet, link later"} />
          {line ? (
            <>
              <span className="choice__text">
                <b>{line.title}</b>
                <span className="add-doc__hint">{line.detail}</span>
              </span>
              <span className="choice__amount">{line.amount}</span>
            </>
          ) : (
            <span className="choice__text choice__text--muted">Not imported yet · link later</span>
          )}
        </label>
      ))}
    </div>
  );
}

/** Adds a read invoice as a draft: vendor, item and the bank line that paid it. */
export function AddDocumentSheet({ preview }: { preview: AddPreview }) {
  const id = useId();
  const [vendor, setVendor] = useState(preview.vendor ?? "");
  const [itemKey, setItemKey] = useState(preview.itemKey ?? "");
  const [lineId, setLineId] = useState<string | null>(preview.lines[0]?.id ?? null);
  const [error, setError] = useState<{ path?: string; message: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const { data: estimate } = useQuery({
    queryKey: ["document-estimate", lineId, itemKey],
    queryFn: () => api.documentEstimate(lineId, itemKey || null),
  });

  const close = () => {
    void api.documentAddCancel(preview.token).catch(() => {});
    useStore.getState().setAddPreview(null);
  };
  const add = () => {
    setBusy(true);
    api
      .documentAddCommit({ token: preview.token, vendor, itemKey: itemKey || null, lineId })
      .then((label) => {
        const { addOnAdded, setAddPreview, showToast } = useStore.getState();
        setAddPreview(null);
        showToast("success", label);
        addOnAdded?.(label);
      })
      .catch((e: unknown) => {
        const failure = toAppError(e);
        setError(failure.field ? { path: failure.field.path, message: failure.field.message } : { message: failure.message });
        setBusy(false);
      });
  };
  const fieldError = (path: string) => (error?.path === path ? error.message : undefined);

  return (
    <Modal labelledBy={id} onClose={close}>
      <h2 id={id}>Add document</h2>
      <AddDocumentFile preview={preview} />
      <div className="add-doc__grid">
        <label htmlFor={`${id}-vendor`}>Vendor</label>
        <div className="add-doc__field">
          <input id={`${id}-vendor`} className="field" value={vendor} placeholder="Who sent the invoice" aria-invalid={!!fieldError("document.vendor")} onChange={(e) => setVendor(e.target.value)} />
          {preview.vendorFromPdf && vendor === preview.vendor && <span className="add-doc__hint">read from PDF</span>}
        </div>
        {(preview.invoice || preview.invoiceAmount) && (
          <>
            <span>Invoice</span>
            <div className="add-doc__invoice">
              {preview.invoice && <span>{preview.invoice}</span>}
              {preview.invoiceAmount && <span className="add-doc__amount">{preview.invoiceAmount}</span>}
            </div>
          </>
        )}
        <label htmlFor={`${id}-item`}>Item</label>
        <div className="add-doc__field">
          <select id={`${id}-item`} className="field" value={itemKey} aria-invalid={!!fieldError("document.item")} onChange={(e) => setItemKey(e.target.value)}>
            <option value="">Choose later</option>
            {preview.items.map((i) => (
              <option key={i.key} value={i.key}>
                {i.label}
              </option>
            ))}
          </select>
          {preview.itemWhy && itemKey === preview.itemKey && <span className="add-doc__hint">{preview.itemWhy}</span>}
        </div>
        {estimate && (
          <>
            <span>Work share</span>
            <span className="add-doc__hint">
              {estimate.workShare} %{estimate.shareHint ? ` · ${estimate.shareHint}` : ""}
            </span>
          </>
        )}
      </div>
      <h3 className="add-doc__heading">Paid with</h3>
      <PaidWithPicker lines={preview.lines} value={lineId} onChange={setLineId} />
      {estimate?.gwg && (
        <div className="add-doc__ok add-doc__gwg">
          <Check size={12} /> {estimate.gwg}
        </div>
      )}
      <p className="add-doc__summary">
        {estimate ? `Counts as ${estimate.anv} (${estimate.formula}). ` : ""}Saved as a draft; finish it whenever you like.
      </p>
      <div className="form-field__error" role={error ? "alert" : undefined}>
        {error?.message ?? ""}
      </div>
      <div className="actions">
        <button type="button" className="btn" onClick={close}>
          Cancel
        </button>
        <button type="button" className="btn btn--primary" disabled={busy || !vendor.trim()} onClick={add}>
          Add Document
        </button>
      </div>
    </Modal>
  );
}
