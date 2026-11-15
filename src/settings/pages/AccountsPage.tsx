import { useState } from "react";
import type { Account } from "../../api";
import { AccountSheet } from "../../accounts/AccountSheet";
import { useT } from "../../i18n";
import { useAccounts } from "../../queries/accounts";
import { useSettings, useUpdateSettings } from "../../queries/settings";
import { useCoverage } from "../../queries/statements";
import { Group, Row, Switch } from "../controls";

/** Settings → Accounts (mockup Settings, page accounts). */
export function AccountsPage() {
  const { data: accounts } = useAccounts();
  const { data: coverage } = useCoverage();
  const lastOf = (id: string) => coverage?.accounts.find((a) => a.id === id)?.last;
  const { data: settings } = useSettings();
  const { mutate } = useUpdateSettings();
  const { accounts: text, common } = useT().settings;
  // `null` adds an account; an account edits it.
  const [editing, setEditing] = useState<Account | null | undefined>(undefined);
  return (
    <>
      <Group title={text.bankAccounts} anchor="bank-accounts">
        {accounts?.length === 0 && <Row label={text.none} description={text.noneDescription} />}
        {accounts?.map((account) => (
          <Row key={account.id} label={account.name} description={[account.bank, account.ibanMasked, lastOf(account.id) ?? text.noStatement].filter(Boolean).join(" · ")}>
            <button className="btn btn--small" onClick={() => setEditing(account)}>
              {common.edit}
            </button>
          </Row>
        ))}
      </Group>
      <Group title={text.importing} anchor="importing">
        <Switch
          label={text.ownTransfers}
          description={text.ownTransfersDescription}
          anchor="own-transfers"
          checked={settings?.hideOwnTransfers ?? true}
          onChange={(hideOwnTransfers) => mutate({ hideOwnTransfers })}
        />
        <Row label={text.format} description={text.formatDescription}>
          <span className="settings-value">PDF</span>
        </Row>
      </Group>
      <Group title={common.add} anchor="add-account">
        <Row label={text.another}>
          <button className="btn btn--small" onClick={() => setEditing(null)}>
            {text.addButton}
          </button>
        </Row>
      </Group>
      {editing !== undefined && <AccountSheet account={editing ?? undefined} onClose={() => setEditing(undefined)} />}
    </>
  );
}
