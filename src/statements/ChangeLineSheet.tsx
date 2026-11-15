import { useId, useState } from "react";
import { api, type Action, type LineView } from "../api";
import { Modal } from "../components/Modal";
import { PopUpButton } from "../components/menu";
import { useT } from "../i18n";
import { showError, undoToast } from "../lib/undoToast";
import { useReviewQueue } from "../queries/review";
import { Segmented } from "../settings/controls";

const ACTIONS: Action[] = ["anv", "subscription", "ignore"];

/** Decides an already decided line again: ANV with its item, a subscription, or Ignore. */
export function ChangeLineSheet({ line, onClose }: { line: LineView; onClose: () => void }) {
  const id = useId();
  const { data } = useReviewQueue();
  const { pages, review } = useT();
  const text = pages.account;
  const { choices, card } = review;
  const [action, setAction] = useState<Action>(line.status === "anv" ? "anv" : line.status === "subscription" ? "subscription" : "ignore");
  const [itemKey, setItemKey] = useState(line.itemKey);
  const [subscriptionId, setSubscriptionId] = useState(line.subscriptionId);
  const items = data?.items ?? [];
  const subscriptions = data?.subscriptions ?? [];
  const ready = action === "ignore" || (action === "anv" ? itemKey !== null : subscriptionId !== null);
  const save = () => {
    api
      .reviewDecide(line.id, action, action === "anv" ? itemKey : null, action === "subscription" ? subscriptionId : null, false)
      .then((label) => {
        undoToast(label);
        onClose();
      })
      .catch(showError);
  };
  return (
    <Modal labelledBy={id} className="sheet--narrow change-line" onClose={onClose}>
      <h2 id={id}>{text.changeTitle}</h2>
      <p>
        {line.title} · {line.amount}
      </p>
      <Segmented label={text.changeTitle} options={ACTIONS.map((a) => ({ value: a, label: choices[a].label }))} value={action} onChange={setAction} />
      <div className="change-line__detail">
        {action === "anv" && (
          <PopUpButton
            aria-label={text.item}
            value={itemKey ?? ""}
            options={[{ value: "", label: card.chooseItem }, ...items.map((i) => ({ value: i.key, label: i.label }))]}
            onChange={(key) => setItemKey(key || null)}
          />
        )}
        {action === "subscription" && (
          <PopUpButton
            aria-label={text.chooseSubscription}
            value={subscriptionId ?? ""}
            options={[{ value: "", label: text.chooseSubscription }, ...subscriptions.map((s) => ({ value: s.id, label: s.name }))]}
            onChange={(sub) => setSubscriptionId(sub || null)}
          />
        )}
      </div>
      <p className="add-doc__hint">{text.changeHint}</p>
      <div className="actions">
        <button type="button" className="btn" onClick={onClose}>
          {text.cancel}
        </button>
        <button type="button" className="btn btn--primary" disabled={!ready} onClick={save}>
          {text.save}
        </button>
      </div>
    </Modal>
  );
}
