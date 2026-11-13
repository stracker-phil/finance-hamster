import { useEffect, useId, useRef, useState } from "react";
import { Check, Link2, X } from "lucide-react";
import { api, toAppError, type DocumentEdit, type DocumentView } from "../api";
import { useStore } from "../state/store";
import { LinePicker } from "./LinePicker";

type Errors = Record<string, string>;

/** Sends one field change; a field error shows under its field, anything else as a toast. */
function useSave(documentId: string) {
  const [errors, setErrors] = useState<Errors>({});
  const save = (change: DocumentEdit) => {
    api
      .documentEdit(documentId, change)
      .then(() => setErrors({}))
      .catch((e: unknown) => {
        const failure = toAppError(e);
        if (failure.field) setErrors({ [failure.field.path]: failure.field.message });
        else useStore.getState().showToast("error", failure.message);
      });
  };
  return { errors, save };
}

/** Keeps typing local so each keystroke doesn't save; follows the stored value when it changes. */
function useDraftText(value: string, onSave: (text: string) => void) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    if (text !== value) onSave(text);
  };
  return { text, setText, commit };
}

/** A text field that saves on leaving it or on Return. */
function SavedInput({ id, value, onSave, className, ...props }: { id: string; value: string; onSave: (text: string) => void } & Omit<React.InputHTMLAttributes<HTMLInputElement>, "value" | "onChange">) {
  const { text, setText, commit } = useDraftText(value, onSave);
  return (
    <input
      id={id}
      className={className ? `field ${className}` : "field"}
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
      }}
      {...props}
    />
  );
}

function NoteField({ value, onSave }: { value: string; onSave: (text: string) => void }) {
  const { text, setText, commit } = useDraftText(value, onSave);
  const ref = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    const area = ref.current;
    if (!area) return;
    area.style.height = "auto";
    area.style.height = `${area.scrollHeight}px`;
  }, [text]);
  return <textarea ref={ref} className="field note-field" rows={2} aria-label="Note" value={text} onChange={(e) => setText(e.target.value)} onBlur={commit} />;
}

function FieldError({ message }: { message?: string }) {
  return (
    <div className="form-field__error" role={message ? "alert" : undefined}>
      {message ?? ""}
    </div>
  );
}

/** The payment-first form of a draft: every field saves on its own as one undoable change. */
export function DocumentForm({ doc }: { doc: DocumentView }) {
  const id = useId();
  const { errors, save } = useSave(doc.id);
  const [picking, setPicking] = useState(false);
  const [moreOpen, setMoreOpen] = useState(doc.invoiceAmount !== null);
  const [shareOpen, setShareOpen] = useState(doc.workShare !== 100);
  const locked = doc.state !== "draft";
  return (
    <fieldset className="doc-form" disabled={locked}>
      <div className="doc-form__status">{doc.missing.length === 0 ? "Every required fact is set" : `Missing: ${doc.missing.join(", ")}`}</div>

      <h3 className="doc-form__heading">Payment</h3>
      <div className="doc-form__grid">
        <label htmlFor={`${id}-date`} className="doc-form__key">
          Date
        </label>
        <div>
          <div className="doc-form__row">
            <input id={`${id}-date`} type="date" className="field doc-form__date" value={doc.payDate ?? ""} onChange={(e) => save({ field: "payDate", value: e.target.value || null })} />
            {doc.year && <span className="add-doc__hint">Tax year {doc.year}</span>}
          </div>
          <FieldError message={errors["document.payDate"]} />
        </div>

        <span>Bank line</span>
        <div className="doc-form__lines">
          {doc.lines.map((line) => (
            <div key={line.id} className="doc-form__line">
              <span className="line-picker__text">
                <span>{line.title}</span>
                <span className="add-doc__hint">{line.detail}</span>
              </span>
              <span className="choice__amount">{line.amount}</span>
              <button type="button" className="icon-btn" aria-label={`Unlink ${line.title}`} onClick={() => save({ field: "unlink", line: line.id })}>
                <X size={14} />
              </button>
            </div>
          ))}
          <div className="doc-form__picker">
            <button type="button" className="btn btn--small" onClick={() => setPicking(true)}>
              <Link2 size={13} /> {doc.lines.length === 0 ? "Link a Bank Line…" : "Link Another Payment…"}
            </button>
            {picking && (
              <LinePicker
                documentId={doc.id}
                onClose={() => setPicking(false)}
                onPick={(line) => {
                  setPicking(false);
                  save({ field: "link", line });
                }}
              />
            )}
          </div>
        </div>

        <label htmlFor={`${id}-paid`}>Paid</label>
        <div>
          {doc.paidFromLines ? (
            <div className="doc-form__row">
              <span className="doc-form__amount">{doc.paid}</span>
              <span className="add-doc__hint">from the bank {doc.lines.length === 1 ? "line" : "lines"}</span>
            </div>
          ) : (
            <SavedInput id={`${id}-paid`} value={doc.paidText ?? ""} placeholder="0.00" inputMode="decimal" className="doc-form__money" onSave={(text) => save({ field: "paid", value: text || null })} />
          )}
          <FieldError message={errors["document.paid"]} />
        </div>
      </div>

      <h3 className="doc-form__heading">Classification</h3>
      <div className="doc-form__grid">
        <label htmlFor={`${id}-vendor`}>Vendor</label>
        <div>
          <SavedInput id={`${id}-vendor`} value={doc.vendor ?? ""} list={`${id}-vendors`} placeholder="Who sent the invoice" onSave={(name) => save({ field: "vendor", name })} />
          <datalist id={`${id}-vendors`}>
            {doc.vendors.map((v) => (
              <option key={v.name} value={v.name}>
                {v.detail}
              </option>
            ))}
          </datalist>
          <FieldError message={errors["document.vendor"]} />
        </div>

        <label htmlFor={`${id}-item`}>Item</label>
        <div>
          <select id={`${id}-item`} className="field" value={doc.itemKey ?? ""} disabled={locked || !doc.payDate} onChange={(e) => save({ field: "item", key: e.target.value || null })}>
            <option value="">{doc.payDate ? "Choose an item" : "Set the payment date first"}</option>
            {doc.items.map((i) => (
              <option key={i.key} value={i.key}>
                {i.label}
              </option>
            ))}
          </select>
          <FieldError message={errors["document.item"]} />
        </div>
      </div>

      {doc.estimate && (
        <div className="anv-result">
          <div className="anv-result__main">
            <div className="add-doc__hint">{doc.year ? `Counts in Tax year ${doc.year}` : "Counts as"}</div>
            <div className="anv-result__amount">{doc.estimate.anv}</div>
            <div className="add-doc__hint">{doc.estimate.formula}</div>
          </div>
          {shareOpen ? (
            <div className="anv-result__share">
              <div className="doc-form__row">
                <label htmlFor={`${id}-share`} className="add-doc__hint">
                  Work share
                </label>
                <SavedInput id={`${id}-share`} value={String(doc.workShare)} inputMode="numeric" className="anv-result__input" onSave={(text) => save({ field: "workShare", value: Number(text) })} />
                <span className="add-doc__hint">%</span>
              </div>
              {doc.estimate.shareHint && <span className="add-doc__hint">{doc.estimate.shareHint}</span>}
              <FieldError message={errors["document.workShare"]} />
            </div>
          ) : (
            <button type="button" className="link-btn" onClick={() => setShareOpen(true)}>
              Partly private use…
            </button>
          )}
        </div>
      )}
      {doc.estimate?.gwg && (
        <div className="add-doc__ok add-doc__gwg">
          <Check size={12} /> {doc.estimate.gwg}
        </div>
      )}

      <h3 className="doc-form__heading">Note</h3>
      <NoteField value={doc.note} onSave={(text) => save({ field: "note", text })} />

      <button type="button" className="link-btn doc-form__more" aria-expanded={moreOpen} onClick={() => setMoreOpen((open) => !open)}>
        More: invoice amount
      </button>
      {moreOpen && (
        <div className="doc-form__row doc-form__invoice">
          <label htmlFor={`${id}-invoice`} className="add-doc__hint">
            Invoice amount
          </label>
          <SavedInput id={`${id}-invoice`} value={doc.invoiceAmount ?? ""} inputMode="decimal" className="doc-form__money" onSave={(text) => save({ field: "invoice", amount: text || null, currency: doc.invoiceCurrency })} />
          <SavedInput id={`${id}-currency`} aria-label="Currency" value={doc.invoiceCurrency} className="doc-form__currency" onSave={(currency) => save({ field: "invoice", amount: doc.invoiceAmount, currency })} />
          <span className="add-doc__hint">optional, for reference</span>
          <FieldError message={errors["document.invoice"]} />
        </div>
      )}
    </fieldset>
  );
}
