import { PopUpButton } from "../../components/menu";
import { useQuery } from "@tanstack/react-query";
import { useId, useState, type FormEvent } from "react";
import { api, toAppError, type ItemOption, type RuleForm, type RuleView } from "../../api";
import { Modal } from "../../components/Modal";
import { useAccounts } from "../../queries/accounts";
import { useStore } from "../../state/store";
import { Segmented } from "../controls";

const EMPTY: RuleForm = { contains: [], amount: null, tolerance: null, accountId: null, ownCounterparty: false, action: "ignore", itemKey: null };

const ACTIONS = [
  { value: "ignore" as const, label: "Ignore" },
  { value: "anv" as const, label: "Assign item" },
];

/** Adds a rule, or edits `rule`; field errors from Rust show under their field. */
export function RuleSheet({ rule, items, onClose }: { rule?: RuleView; items: ItemOption[]; onClose: () => void }) {
  const id = useId();
  const { data: accounts = [] } = useAccounts();
  const [form, setForm] = useState<RuleForm>(rule?.form ?? EMPTY);
  const [contains, setContains] = useState((rule?.form.contains ?? []).join(", "));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const draft = { ...form, contains: contains.split(",") };
  const { data: preview } = useQuery({ queryKey: ["rule-preview", draft], queryFn: () => api.rulePreview(draft), placeholderData: (previous) => previous });
  const set = (patch: Partial<RuleForm>) => setForm((f) => ({ ...f, ...patch }));
  const text = (value: string) => (value.trim() === "" ? null : value);

  /** Closes with a toast when `call` succeeds; shows its error under the field it names otherwise. */
  const run = (call: Promise<string>, done: string) => {
    setBusy(true);
    call
      .then(() => {
        useStore.getState().showToast("success", done);
        onClose();
      })
      .catch((err: unknown) => {
        const error = toAppError(err);
        setErrors(error.field ? { [error.field.path]: error.field.message } : { "rule.form": error.message });
        setBusy(false);
      });
  };

  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(api.ruleSave(rule?.id ?? null, { ...form, contains: contains.split(",") }), rule ? "Saved the rule" : "Added the rule");
  };

  const remove = async () => {
    if (!rule) return;
    const confirmed = await useStore.getState().confirm({ title: "Delete this rule?", text: "The lines it decided get the next matching rule, or go back to Review.", action: "Delete" });
    if (confirmed) run(api.ruleDelete(rule.id), "Deleted the rule");
  };

  const fieldError = (key: string): string | undefined => errors[`rule.${key}`];
  const error = (key: string) => (
    <div className="form-field__error" role={fieldError(key) ? "alert" : undefined}>
      {fieldError(key) ?? ""}
    </div>
  );

  return (
    <Modal labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>{rule ? "Edit Rule" : "New Rule"}</h2>
      <form className="rule-form" onSubmit={submit}>
        <div className="form-field">
          <label htmlFor={`${id}-contains`}>Bank text contains</label>
          <input id={`${id}-contains`} className="field" value={contains} placeholder="ALPENMARKT, BILLA" autoFocus onChange={(e) => setContains(e.target.value)} />
          <span className="form-check__desc">Any one of these, separated by commas, in the booking text, its detail lines, the merchant or the counterparty; upper or lower case doesn&apos;t matter.</span>
        </div>
        <div className="rule-form__pair">
          <div className="form-field">
            <label htmlFor={`${id}-amount`}>Amount in €</label>
            <div className="rule-form__unit">
              <input id={`${id}-amount`} className="field" value={form.amount ?? ""} placeholder="Any" inputMode="decimal" aria-invalid={!!fieldError("amount")} onChange={(e) => set({ amount: text(e.target.value) })} />
              <span>€</span>
            </div>
            {error("amount")}
          </div>
          <div className="form-field">
            <label htmlFor={`${id}-tolerance`}>Within ± €</label>
            <div className="rule-form__unit">
              <input id={`${id}-tolerance`} className="field" value={form.tolerance ?? ""} placeholder="0.00" inputMode="decimal" aria-invalid={!!fieldError("tolerance")} onChange={(e) => set({ tolerance: text(e.target.value) })} />
              <span>€</span>
            </div>
            {error("tolerance")}
          </div>
        </div>
        <div className="form-field">
          <label htmlFor={`${id}-account`}>Account</label>
          <PopUpButton
            id={`${id}-account`}
            aria-invalid={!!fieldError("account")}
            value={form.accountId ?? ""}
            options={[{ value: "", label: "Any account" }, ...accounts.map((a) => ({ value: a.id, label: a.name }))]}
            onChange={(v) => set({ accountId: text(v) })}
          />
          {error("account")}
        </div>
        <label className="form-check">
          <input type="checkbox" className="switch" role="switch" checked={form.ownCounterparty} onChange={(e) => set({ ownCounterparty: e.target.checked })} />
          <span>Counterparty is one of my accounts</span>
        </label>
        <span className="form-check__desc">Every condition you fill in must hold; an empty one is ignored. The amount is compared without its sign, in euros as booked.</span>
        {error("conditions")}
        <div className="rule-form__preview" aria-live="polite">
          <div>{preview?.text ?? " "}</div>
          {preview?.examples.map((line) => (
            <div key={line} className="rule-form__example">
              {line}
            </div>
          ))}
        </div>
        <div className="form-field">
          <span className="form-field__label">Then</span>
          <Segmented label="Then" options={ACTIONS} value={form.action === "subscription" ? undefined : form.action} onChange={(action) => set({ action })} />
        </div>
        <span className="form-check__desc">A subscription finds its own charges by its bank text and plan amount; set that on the subscription, not here.</span>
        {form.action === "anv" && (
          <div className="form-field">
            <label htmlFor={`${id}-item`}>Item</label>
            <PopUpButton
              id={`${id}-item`}
              aria-invalid={!!fieldError("item")}
              value={form.itemKey ?? ""}
              options={[{ value: "", label: "Choose…" }, ...items.map((i) => ({ value: i.key, label: i.label }))]}
              onChange={(v) => set({ itemKey: text(v) })}
            />
            {error("item")}
          </div>
        )}
        {error("form")}
        <div className="actions">
          {rule && !rule.builtin && (
            <button type="button" className="btn btn--danger-text rule-form__delete" disabled={busy} onClick={() => void remove()}>
              Delete…
            </button>
          )}
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="btn btn--primary" disabled={busy}>
            {rule ? "Save" : "Add Rule"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
