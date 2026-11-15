import { useId } from "react";
import type { Account } from "../api";
import { Modal } from "../components/Modal";
import { useT } from "../i18n";
import { useStore } from "../state/store";
import { AccountForm } from "./AccountForm";

/** Adds an account, or edits `account`. */
export function AccountSheet({ account, onClose }: { account?: Account; onClose: () => void }) {
  const id = useId();
  const text = useT().statements.account;
  const saved = (name: string) => {
    useStore.getState().showToast("success", account ? text.saved(name) : text.added(name));
    onClose();
  };
  return (
    <Modal labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>{account ? text.edit : text.add}</h2>
      <AccountForm
        account={account}
        submitLabel={account ? text.save : text.add}
        onSaved={saved}
        onCancel={onClose}
      />
    </Modal>
  );
}
