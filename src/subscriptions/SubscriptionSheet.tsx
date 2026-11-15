import { PopUpButton } from "../components/menu";
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
import { useT, type Messages } from "../i18n";

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
  exactOnly: false,
});

type SheetText = Messages["subscriptions"]["sheet"];

export const intervals = (text: SheetText): { value: Interval; label: string }[] => [
  { value: "monthly", label: text.monthly },
  { value: "quarterly", label: text.quarterly },
  { value: "yearly", label: text.yearly },
];

const states = (text: SheetText) => [
  { value: "active" as const, label: text.active },
  { value: "cancelled" as const, label: text.cancelled },
];

const reminders = (text: SheetText): { value: Reminder; label: string }[] => [
  { value: "off", label: text.reminderOff },
  { value: "one_day", label: text.oneDay },
  { value: "three_days", label: text.threeDays },
  { value: "one_week", label: text.oneWeek },
];

const otherAmounts = (text: SheetText) => [
  { value: "ask" as const, label: text.ask },
  { value: "exact" as const, label: text.exact },
];

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
  const text = useT().subscriptions.sheet;

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
  const startLabel = form.start < today() ? text.firstCharge : text.nextCharge;

  return (
    <Modal labelledBy={titleId} className="sub-sheet" onClose={onClose}>
      <form onSubmit={submit}>
        <div className="sub-sheet__head">
          <button type="button" className={`tile sub-sheet__tile ${accountTone(form.tone)}`} aria-label={text.changeColor} onClick={() => set({ tone: (form.tone + 1) % TONES })}>
            {letterOf(form.name)}
          </button>
          <div className="sub-sheet__title">
            <h2 id={titleId}>{editing ? text.editTitle(form.name) : text.newTitle}</h2>
            <span className="add-doc__hint">{editing ? text.editHint : text.newHint}</span>
          </div>
          <Segmented label={text.state} options={states(text)} value={form.active ? "active" : "cancelled"} onChange={(state) => set({ active: state === "active", endedOn: state === "active" ? null : (form.endedOn ?? today()) })} />
        </div>
        <div className="sub-sheet__body">
          {source && <SourceLine line={source} onSelect={(matchText) => set({ matchText })} />}
          <div className="sub-grid">
            <label htmlFor={field("name")}>{text.name}</label>
            <div>
              <input id={field("name")} className="field" value={form.name} autoFocus aria-invalid={!!message("name")} onChange={(e) => set({ name: e.target.value })} />
              {fieldError("name")}
            </div>
            <label htmlFor={field("amount")}>{text.amount}</label>
            <div>
              <div className="sub-grid__pair">
                <input id={field("amount")} className="field sub-sheet__amount" inputMode="decimal" value={form.amount} aria-invalid={!!message("amount")} onChange={(e) => set({ amount: e.target.value })} />
                <input className="field sub-sheet__currency" aria-label={text.currency} maxLength={3} value={form.currency} aria-invalid={!!message("currency")} onChange={(e) => set({ currency: e.target.value.toUpperCase() })} />
              </div>
              {fieldError("amount", "currency")}
            </div>
            <span>{text.repeats}</span>
            <div>
              <Segmented label={text.repeats} options={intervals(text)} value={form.interval} onChange={(interval) => set({ interval })} />
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
                <label htmlFor={field("ended")}>{text.endedOn}</label>
                <div>
                  <input id={field("ended")} type="date" className="field" value={form.endedOn ?? ""} onChange={(e) => set({ endedOn: e.target.value || null })} />
                  {fieldError("endedOn")}
                </div>
              </>
            )}
            <label htmlFor={field("category")}>{text.category}</label>
            <div>
              <input id={field("category")} className="field" list={field("categories")} value={form.category} placeholder={text.categoryPlaceholder} onChange={(e) => set({ category: e.target.value })} />
              <datalist id={field("categories")}>
                {view?.categories.map((c) => (
                  <option key={c} value={c} />
                ))}
              </datalist>
            </div>
            <label htmlFor={field("reminder")}>{text.reminder}</label>
            <PopUpButton id={field("reminder")} value={form.reminder} options={reminders(text)} onChange={(reminder: Reminder) => set({ reminder })} />
          </div>

          <h3 className="sub-sheet__heading">{text.anv}</h3>
          <div className="sub-group">
            <label className="sub-group__row">
              <span className="sub-group__text">
                {text.counts}
                <span className="add-doc__hint">{text.countsHint}</span>
              </span>
              <input type="checkbox" className="switch" role="switch" checked={form.itemKey !== null} onChange={(e) => set({ itemKey: e.target.checked ? (items[0]?.key ?? "") : null })} />
            </label>
            {form.itemKey !== null && (
              <div className="sub-group__row">
                <label htmlFor={field("item")} className="sub-group__text">
                  {text.item}
                </label>
                <PopUpButton
                  id={field("item")}
                  aria-invalid={!!message("item")}
                  value={form.itemKey}
                  options={items.map((i) => ({ value: i.key, label: i.label }))}
                  onChange={(itemKey) => set({ itemKey })}
                />
              </div>
            )}
          </div>
          {fieldError("item")}

          <h3 className="sub-sheet__heading">{text.find}</h3>
          <div className="sub-group">
            <div className="sub-group__row">
              <label htmlFor={field("match")} className="sub-group__text">
                {text.contains}
                {source && <span className="add-doc__hint">{text.containsHint}</span>}
              </label>
              <input id={field("match")} className="field mono sub-sheet__match" value={form.matchText} placeholder="LUMEN AI" aria-invalid={!!message("match")} onChange={(e) => set({ matchText: e.target.value })} />
            </div>
            <div className="sub-group__row">
              <span className="sub-group__text">
                {text.another}
                <span className="add-doc__hint">{form.exactOnly ? text.exactHint : text.askHint}</span>
              </span>
              <Segmented label={text.another} options={otherAmounts(text)} value={form.exactOnly ? "exact" : "ask"} onChange={(v) => set({ exactOnly: v === "exact" })} />
            </div>
          </div>
          {fieldError("match")}
          <p className="add-doc__hint sub-sheet__count" aria-live="polite">
            {count?.text ?? " "}
          </p>
          {fieldError("form")}
        </div>
        <div className="actions sub-sheet__actions">
          {editing && (
            <button type="button" className="link-btn sub-sheet__delete" onClick={() => void remove()}>
              {text.delete}
            </button>
          )}
          <button type="button" className="btn" onClick={onClose}>
            {text.cancel}
          </button>
          <button type="submit" className="btn btn--primary" disabled={busy}>
            {editing ? text.save : text.add}
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
