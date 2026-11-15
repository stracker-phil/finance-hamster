import { useId, useState, type FormEvent } from "react";
import { api, toAppError, type PlanForm, type ReviewLine } from "../api";
import { Modal } from "../components/Modal";
import { dayMonthYear } from "../lib/date";
import { undoToast } from "../lib/undoToast";
import { Segmented } from "../settings/controls";
import { useStore } from "../state/store";
import { SourceLine } from "./SourceLine";
import { intervals } from "./SubscriptionSheet";
import { useT } from "../i18n";

const dayBefore = (iso: string) => {
  const day = new Date(`${iso}T12:00:00`);
  if (Number.isNaN(day.getTime())) return "";
  day.setDate(day.getDate() - 1);
  return dayMonthYear(day);
};

/** Change Plan: the current plan ends the day before the new one starts; its charges stay with it. */
export function PlanSheet({ id, name, currentPlan, initial, source, onClose }: { id: string; name: string; currentPlan: string; initial: PlanForm; source?: ReviewLine; onClose: () => void }) {
  const titleId = useId();
  const field = (key: string) => `${titleId}-${key}`;
  const [form, setForm] = useState(initial);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const all = useT().subscriptions;
  const text = all.plan;
  const set = (patch: Partial<PlanForm>) => setForm((f) => ({ ...f, ...patch }));
  const submit = (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    api
      .subscriptionChangePlan(id, form)
      .then((label) => {
        undoToast(label);
        onClose();
      })
      .catch((err: unknown) => {
        const error = toAppError(err);
        setErrors(error.field ? { [error.field.path]: error.field.message } : { "subscription.form": error.message });
        setBusy(false);
      });
  };
  const message = (key: string) => errors[`subscription.${key}`];
  const fieldError = (...keys: string[]) => {
    const text = keys.map(message).find(Boolean);
    return (
      <div className="form-field__error" role={text ? "alert" : undefined}>
        {text ?? ""}
      </div>
    );
  };
  const ends = dayBefore(form.start);
  return (
    <Modal labelledBy={titleId} onClose={onClose}>
      <form onSubmit={submit} className="plan-sheet">
        <div>
          <h2 id={titleId}>{text.title(name)}</h2>
          <span className="add-doc__hint">{text.current(currentPlan)}</span>
        </div>
        {source && <SourceLine line={source} />}
        <div className="sub-grid">
          <label htmlFor={field("name")}>{text.newPlan}</label>
          <div>
            <input id={field("name")} className="field" value={form.name} placeholder={text.namePlaceholder} autoFocus aria-invalid={!!message("planName")} onChange={(e) => set({ name: e.target.value })} />
            {fieldError("planName")}
          </div>
          <label htmlFor={field("amount")}>{text.amount}</label>
          <div>
            <div className="sub-grid__pair">
              <input id={field("amount")} className="field sub-sheet__amount" inputMode="decimal" value={form.amount} aria-invalid={!!message("amount")} onChange={(e) => set({ amount: e.target.value })} />
              <input className="field sub-sheet__currency" aria-label={text.currency} maxLength={3} value={form.currency} onChange={(e) => set({ currency: e.target.value.toUpperCase() })} />
            </div>
            {fieldError("amount", "currency")}
          </div>
          <span>{text.repeats}</span>
          <div>
              <Segmented label={text.repeats} options={intervals(all.sheet)} value={form.interval} onChange={(interval) => set({ interval })} />
              <div className="form-field__error" />
            </div>
          <label htmlFor={field("start")}>{text.starts}</label>
          <div>
            <input id={field("start")} type="date" className="field" value={form.start} aria-invalid={!!message("start")} onChange={(e) => set({ start: e.target.value })} />
            {fieldError("start")}
          </div>
        </div>
        <p className="plan-sheet__box">
          {ends ? text.ends(ends) : text.chooseStart}
        </p>
        {fieldError("form")}
        <div className="actions">
          <button type="button" className="btn" onClick={onClose}>
            {text.cancel}
          </button>
          <button type="submit" className="btn btn--primary" disabled={busy}>
            {text.change}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** The open Change Plan sheet from the store. */
export function PlanSheetHost() {
  const sheet = useStore((s) => s.subscriptionSheet);
  if (sheet?.kind !== "plan") return null;
  return <PlanSheet key={sheet.id} id={sheet.id} name={sheet.name} currentPlan={sheet.currentPlan} initial={sheet.form} source={sheet.source} onClose={() => useStore.getState().setSubscriptionSheet(null)} />;
}
