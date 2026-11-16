import { useId, useState } from "react";
import { api, toAppError, type PayslipEdit, type PayslipView } from "../api";
import { SavedInput, useDraftText } from "../components/SavedInput";
import { useT } from "../i18n";
import { undoToast } from "../lib/undoToast";
import { useStore } from "../state/store";

/** Sends one change; a field error shows under its field, anything else as a toast. */
function useSave(id: string) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const save = (change: PayslipEdit) => {
    api
      .payslipEdit(id, change)
      .then((label) => {
        setErrors({});
        if (label) undoToast(label);
      })
      .catch((e: unknown) => {
        const failure = toAppError(e);
        if (failure.field) setErrors({ [failure.field.path]: failure.field.message });
        else useStore.getState().showToast("error", failure.message);
      });
  };
  return { errors, save };
}

function Field({ id, label, hint, error, children }: { id: string; label: string; hint: string; error?: string; children: React.ReactNode }) {
  return (
    <>
      <label htmlFor={id} className="payslip-form__label">
        {label}
        {hint && <span className="add-doc__hint">{hint}</span>}
      </label>
      <div>
        {children}
        <div className="form-field__error" role={error ? "alert" : undefined}>
          {error ?? ""}
        </div>
      </div>
    </>
  );
}

function NoteField({ value, label, onSave }: { value: string; label: string; onSave: (text: string) => void }) {
  const { text, setText, commit } = useDraftText(value, onSave);
  return <textarea className="field note-field" rows={2} aria-label={label} value={text} onChange={(e) => setText(e.target.value)} onBlur={commit} />;
}

const FIELD_PATHS: Record<string, string> = { gross: "payslip.gross", social: "payslip.social", tax: "payslip.tax", net: "payslip.net", payout: "payslip.payout", homeOfficePay: "payslip.homeOfficePay" };

/** A payslip's numbers as printed on it; each field saves on its own as one undoable change. */
export function PayslipForm({ payslip }: { payslip: PayslipView }) {
  const id = useId();
  const { errors, save } = useSave(payslip.id);
  const text = useT().payslips;
  return (
    <fieldset className="payslip-form" disabled={!payslip.open}>
      <h3 className="doc-form__heading">{text.thisMonth}</h3>
      <div className="payslip-form__grid">
        {payslip.amounts.map((field) => (
          <Field key={field.name} id={`${id}-${field.name}`} label={field.label} hint={field.hint} error={errors[FIELD_PATHS[field.name]]}>
            <SavedInput id={`${id}-${field.name}`} value={field.value} inputMode="decimal" className="doc-form__money" onSave={(value) => save({ field: "amount", name: field.name, value })} />
          </Field>
        ))}
      </div>
      <h3 className="doc-form__heading">{text.yearToDate}</h3>
      <div className="payslip-form__grid">
        <Field id={`${id}-days`} label={text.teleworkDays} hint={text.teleworkHint} error={errors["payslip.teleworkDays"]}>
          <SavedInput id={`${id}-days`} value={payslip.teleworkDays} inputMode="numeric" className="doc-form__money" onSave={(value) => save({ field: "teleworkDays", value })} />
        </Field>
      </div>
      <h3 className="doc-form__heading">{text.note}</h3>
      <NoteField value={payslip.note} label={text.note} onSave={(value) => save({ field: "note", text: value })} />
    </fieldset>
  );
}
