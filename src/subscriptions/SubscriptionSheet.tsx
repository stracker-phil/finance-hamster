import { useId, useState, type FormEvent } from "react";
import { api, toAppError, type Interval, type Reminder, type ReviewLine, type SubscriptionForm } from "../api";
import { Modal } from "../components/Modal";
import { undoToast } from "../lib/undoToast";
import { useSettings } from "../queries/settings";
import { useMatchCount, useSubscriptions } from "../queries/subscriptions";
import { Segmented } from "../settings/controls";
import { useStore } from "../state/store";
import { accountTone } from "../lib/accountTone";
import { deleteSubscription } from "./flow";
import { today } from "../lib/today";
import { SourceLine } from "./SourceLine";

const TONES = 6;

const empty = (): SubscriptionForm => ({
  name: "",
  tone: 0,
  category: "",
  active: true,
  endedOn: null,
  amount: "",
  currency: "EUR",
  interval: "monthly",
  start: today(),
  reminder: "off",
  itemKey: null,
  matchText: "",
  tolerancePercent: 0,
});

export const INTERVALS: { value: Interval; label: string }[] = [
  { value: "monthly", label: "Monthly" },
  { value: "quarterly", label: "Quarterly" },
  { value: "yearly", label: "Yearly" },
];

const STATES = [
  { value: "active" as const, label: "Active" },
  { value: "cancelled" as const, label: "Cancelled" },
];

const REMINDERS: { value: Reminder; label: string }[] = [
  { value: "off", label: "Off" },
  { value: "one_day", label: "1 day before" },
  { value: "three_days", label: "3 days before" },
  { value: "one_week", label: "1 week before" },
];

/** How far a charge may differ from the plan, in percent; the select spells "any amount" (null) as `any`. */
const ANY = "any";
const TOLERANCES = [
  { value: "0", label: "Exact amount" },
  { value: "1", label: "± 1 %" },
  { value: "2", label: "± 2 %" },
  { value: "5", label: "± 5 %" },
  { value: "10", label: "± 10 %" },
];
const toChoice = (percent: number | null) => (percent === null ? ANY : String(percent));
const fromChoice = (value: string) => (value === ANY ? null : Number(value));

/** The choices, plus a stored value none of them offers. */
function toleranceOptions(percent: number | null) {
  const offered = percent === null || TOLERANCES.some((t) => t.value === String(percent));
  const own = offered ? [] : [{ value: String(percent), label: `± ${percent} %` }];
  return [...TOLERANCES, ...own, { value: ANY, label: "Any amount" }];
}

const letterOf = (name: string) => name.match(/[\p{L}\p{N}]/u)?.[0]?.toUpperCase() ?? "?";

/** New and Edit share this sheet; Edit fixes mistakes in place, without plan history. */
export function SubscriptionSheet({ id, initial, source, onClose }: { id: string | null; initial: SubscriptionForm | null; source?: ReviewLine; onClose: () => void }) {
  const titleId = useId();
  const field = (name: string) => `${titleId}-${name}`;
  const { data: settings } = useSettings();
  const [form, setForm] = useState<SubscriptionForm>(() => initial ?? { ...empty(), reminder: settings?.defaultReminder ?? "three_days" });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const { data: view } = useSubscriptions("");
  const { data: count } = useMatchCount(form);
  const set = (patch: Partial<SubscriptionForm>) => setForm((f) => ({ ...f, ...patch }));
  const editing = id !== null;
  const items = view?.items ?? [];

  const submit = (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    api
      .subscriptionSave(id, form)
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
  const remove = async () => {
    if (id && (await deleteSubscription(id, form.name))) onClose();
  };
  const message = (key: string) => errors[`subscription.${key}`];
  /** The line under a field for its error, reserved so nothing moves when one appears. */
  const fieldError = (...keys: string[]) => {
    const text = keys.map(message).find(Boolean);
    return (
      <div className="form-field__error" role={text ? "alert" : undefined}>
        {text ?? ""}
      </div>
    );
  };
  const startLabel = form.start < today() ? "First charge" : "Next charge";

  return (
    <Modal labelledBy={titleId} className="sub-sheet" onClose={onClose}>
      <form onSubmit={submit}>
        <div className="sub-sheet__head">
          <button type="button" className={`tile sub-sheet__tile ${accountTone(form.tone)}`} aria-label="Change color" onClick={() => set({ tone: (form.tone + 1) % TONES })}>
            {letterOf(form.name)}
          </button>
          <div className="sub-sheet__title">
            <h2 id={titleId}>{editing ? `Edit ${form.name}` : "New Subscription"}</h2>
            <span className="add-doc__hint">{editing ? "Fixes mistakes; Change Plan keeps the history." : "Found in your statements by its bank text."}</span>
          </div>
          <Segmented label="State" options={STATES} value={form.active ? "active" : "cancelled"} onChange={(state) => set({ active: state === "active", endedOn: state === "active" ? null : (form.endedOn ?? today()) })} />
        </div>
        <div className="sub-sheet__body">
          {source && <SourceLine line={source} onSelect={(matchText) => set({ matchText })} />}
          <div className="sub-grid">
            <label htmlFor={field("name")}>Name</label>
            <div>
              <input id={field("name")} className="field" value={form.name} autoFocus aria-invalid={!!message("name")} onChange={(e) => set({ name: e.target.value })} />
              {fieldError("name")}
            </div>
            <label htmlFor={field("amount")}>Amount</label>
            <div>
              <div className="sub-grid__pair">
                <input id={field("amount")} className="field sub-sheet__amount" inputMode="decimal" value={form.amount} aria-invalid={!!message("amount")} onChange={(e) => set({ amount: e.target.value })} />
                <input className="field sub-sheet__currency" aria-label="Currency" maxLength={3} value={form.currency} aria-invalid={!!message("currency")} onChange={(e) => set({ currency: e.target.value.toUpperCase() })} />
              </div>
              {fieldError("amount", "currency")}
            </div>
            <span>Repeats</span>
            <div>
              <Segmented label="Repeats" options={INTERVALS} value={form.interval} onChange={(interval) => set({ interval })} />
              <div className="form-field__error" />
            </div>
            {!editing && (
              <>
                <label htmlFor={field("start")}>{startLabel}</label>
                <div>
                  <input id={field("start")} type="date" className="field" value={form.start} aria-invalid={!!message("start")} onChange={(e) => set({ start: e.target.value })} />
                  {fieldError("start")}
                </div>
              </>
            )}
            {!form.active && (
              <>
                <label htmlFor={field("ended")}>Ended on</label>
                <div>
                  <input id={field("ended")} type="date" className="field" value={form.endedOn ?? ""} onChange={(e) => set({ endedOn: e.target.value || null })} />
                  {fieldError("endedOn")}
                </div>
              </>
            )}
            <label htmlFor={field("category")}>Category</label>
            <div>
              <input id={field("category")} className="field" list={field("categories")} value={form.category} placeholder="Media, AI & Software, …" onChange={(e) => set({ category: e.target.value })} />
              <datalist id={field("categories")}>
                {view?.categories.map((c) => (
                  <option key={c} value={c} />
                ))}
              </datalist>
            </div>
            <label htmlFor={field("reminder")}>Reminder</label>
            <select id={field("reminder")} className="field" value={form.reminder} onChange={(e) => set({ reminder: e.target.value as Reminder })}>
              {REMINDERS.map((r) => (
                <option key={r.value} value={r.value}>
                  {r.label}
                </option>
              ))}
            </select>
          </div>

          <h3 className="sub-sheet__heading">ANV</h3>
          <div className="sub-group">
            <label className="sub-group__row">
              <span className="sub-group__text">
                Counts for the tax year
                <span className="add-doc__hint">Charges count for the item below. Each one still needs its invoice PDF.</span>
              </span>
              <input type="checkbox" className="switch" role="switch" checked={form.itemKey !== null} onChange={(e) => set({ itemKey: e.target.checked ? (items[0]?.key ?? "") : null })} />
            </label>
            {form.itemKey !== null && (
              <div className="sub-group__row">
                <label htmlFor={field("item")} className="sub-group__text">
                  Item
                </label>
                <select id={field("item")} className="field" value={form.itemKey} aria-invalid={!!message("item")} onChange={(e) => set({ itemKey: e.target.value })}>
                  {items.map((i) => (
                    <option key={i.key} value={i.key}>
                      {i.label}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </div>
          {fieldError("item")}

          <h3 className="sub-sheet__heading">Find it in statements</h3>
          <div className="sub-group">
            <div className="sub-group__row">
              <label htmlFor={field("match")} className="sub-group__text">
                Bank text contains
                {source && <span className="add-doc__hint">Or select text in the line above, such as a contract number.</span>}
              </label>
              <input id={field("match")} className="field mono sub-sheet__match" value={form.matchText} placeholder="LUMEN AI" aria-invalid={!!message("match")} onChange={(e) => set({ matchText: e.target.value })} />
            </div>
            <div className="sub-group__row">
              <label htmlFor={field("tolerance")} className="sub-group__text">
                Amount may differ by
                <span className="add-doc__hint">From the plan in effect on the charge&apos;s date. A charge beyond it asks “Plan changed?” in Review.</span>
              </label>
              <select
                id={field("tolerance")}
                className="field"
                value={toChoice(form.tolerancePercent)}
                onChange={(e) => set({ tolerancePercent: fromChoice(e.target.value) })}
              >
                {toleranceOptions(form.tolerancePercent).map((t) => (
                  <option key={t.value} value={t.value}>
                    {t.label}
                  </option>
                ))}
              </select>
            </div>
          </div>
          {fieldError("match", "tolerance")}
          <p className="add-doc__hint sub-sheet__count" aria-live="polite">
            {count?.text ?? " "}
          </p>
          {fieldError("form")}
        </div>
        <div className="actions sub-sheet__actions">
          {editing && (
            <button type="button" className="link-btn sub-sheet__delete" onClick={() => void remove()}>
              Delete Subscription…
            </button>
          )}
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="btn btn--primary" disabled={busy}>
            {editing ? "Save" : "Add Subscription"}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** The open subscription sheet from the store. */
export function SubscriptionSheetHost() {
  const sheet = useStore((s) => s.subscriptionSheet);
  const close = () => useStore.getState().setSubscriptionSheet(null);
  if (!sheet || sheet.kind === "plan") return null;
  return (
    <SubscriptionSheet
      key={sheet.kind === "edit" ? sheet.id : "new"}
      id={sheet.kind === "edit" ? sheet.id : null}
      initial={sheet.form}
      source={sheet.kind === "new" ? sheet.source : undefined}
      onClose={close}
    />
  );
}
