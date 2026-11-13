import { useId } from "react";
import type { Account } from "../api";
import { Modal } from "../components/Modal";
import { useStore } from "../state/store";
import { AccountForm } from "./AccountForm";

/** Adds an account, or edits `account`. */
export function AccountSheet({ account, onClose }: { account?: Account; onClose: () => void }) {
  const id = useId();
  const saved = (name: string) => {
    useStore.getState().showToast("success", account ? `Saved ${name}` : `Added ${name}`);
    onClose();
  };
  return (
    <Modal labelledBy={id} className="sheet--narrow" onClose={onClose}>
      <h2 id={id}>{account ? "Edit Account" : "Add Account"}</h2>
      <AccountForm
        account={account}
        submitLabel={account ? "Save" : "Add Account"}
        onSaved={saved}
        onCancel={onClose}
      />
    </Modal>
  );
}
