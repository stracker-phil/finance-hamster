import { useState } from "react";
import type { Account } from "../../api";
import { AccountSheet } from "../../accounts/AccountSheet";
import { useAccounts } from "../../queries/accounts";
import { useSettings, useUpdateSettings } from "../../queries/settings";
import { Group, Row, Switch } from "../controls";

/** Settings → Accounts (mockup Settings, page accounts). */
export function AccountsPage() {
  const { data: accounts } = useAccounts();
  const { data: settings } = useSettings();
  const { mutate } = useUpdateSettings();
  // `null` adds an account; an account edits it.
  const [editing, setEditing] = useState<Account | null | undefined>(undefined);
  return (
    <>
      <Group title="Bank accounts" anchor="bank-accounts">
        {accounts?.length === 0 && <Row label="No accounts yet" description="Add the account whose statements you import." />}
        {accounts?.map((account) => (
          <Row key={account.id} label={account.name} description={[account.bank, account.ibanMasked].filter(Boolean).join(" · ")}>
            <button className="btn btn--small" onClick={() => setEditing(account)}>
              Edit…
            </button>
          </Row>
        ))}
      </Group>
      <Group title="Importing" anchor="importing">
        <Switch
          label="Hide transfers between my accounts"
          description="They show as Transfer and never count twice."
          anchor="own-transfers"
          checked={settings?.hideOwnTransfers ?? true}
          onChange={(hideOwnTransfers) => mutate({ hideOwnTransfers })}
        />
        <Row label="Statement format" description="Text-based PDF from the bank, checked by its printed balances.">
          <span className="settings-value">PDF</span>
        </Row>
      </Group>
      <Group title="Add" anchor="add-account">
        <Row label="Another bank account">
          <button className="btn btn--small" onClick={() => setEditing(null)}>
            Add Account…
          </button>
        </Row>
      </Group>
      {editing !== undefined && <AccountSheet account={editing ?? undefined} onClose={() => setEditing(undefined)} />}
    </>
  );
}
