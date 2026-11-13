import { useId, useState, type FormEvent } from "react";
import { api, toAppError, type Account } from "../api";

export interface AccountValues {
  name: string;
  bank: string;
  iban: string;
  own: boolean;
}

const EMPTY: AccountValues = { name: "", bank: "", iban: "", own: true };

interface Props {
  /** The account to edit; absent for a new one. Its IBAN can't change. */
  account?: Account;
  submitLabel: string;
  /** Shows a Cancel button before the submit button. */
  onCancel?: () => void;
  onSaved: (name: string) => void;
}

/** Name, bank, IBAN and the own-account switch; field errors from Rust show under their field. */
export function AccountForm({ account, submitLabel, onCancel, onSaved }: Props) {
  const id = useId();
  const [values, setValues] = useState<AccountValues>(account ? { name: account.name, bank: account.bank, iban: account.ibanMasked, own: account.own } : EMPTY);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const set = (patch: Partial<AccountValues>) => setValues((v) => ({ ...v, ...patch }));

  const submit = (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    const save = account ? api.editAccount(account.id, { name: values.name, bank: values.bank, own: values.own }) : api.addAccount(values);
    save
      .then(() => onSaved(values.name.trim()))
      .catch((err: unknown) => {
        const error = toAppError(err);
        setErrors(error.field ? { [error.field.path]: error.field.message } : { form: error.message });
      })
      .finally(() => setBusy(false));
  };

  const field = (key: "name" | "bank" | "iban", label: string, props: { placeholder?: string; disabled?: boolean; autoFocus?: boolean }) => {
    const error = errors[`account.${key}`];
    return (
      <div className="form-field">
        <label htmlFor={`${id}-${key}`}>{label}</label>
        <input id={`${id}-${key}`} className="field" value={values[key]} aria-invalid={!!error} aria-describedby={error ? `${id}-${key}-error` : undefined} onChange={(e) => set({ [key]: e.target.value })} {...props} />
        <div id={`${id}-${key}-error`} className="form-field__error" role={error ? "alert" : undefined}>
          {error ?? ""}
        </div>
      </div>
    );
  };

  return (
    <form className="account-form" onSubmit={submit}>
      {field("name", "Name", { placeholder: "Private", autoFocus: true })}
      {field("bank", "Bank", { placeholder: "Example Bank" })}
      {field("iban", "IBAN", { placeholder: "AT00 0000 0000 0000 0000", disabled: !!account })}
      <label className="form-check">
        <input type="checkbox" className="switch" role="switch" checked={values.own} onChange={(e) => set({ own: e.target.checked })} />
        <span>
          One of my accounts
          <span className="form-check__desc">Transfers between your own accounts show as Transfer.</span>
        </span>
      </label>
      <div className="form-field__error" role={errors.form ? "alert" : undefined}>
        {errors.form ?? ""}
      </div>
      <div className="actions">
        {onCancel && (
          <button type="button" className="btn" onClick={onCancel}>
            Cancel
          </button>
        )}
        <button type="submit" className="btn btn--primary" disabled={busy || !values.name.trim() || (!account && !values.iban.trim())}>
          {submitLabel}
        </button>
      </div>
    </form>
  );
}
