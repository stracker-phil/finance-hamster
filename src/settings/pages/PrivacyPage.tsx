import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, toAppError } from "../../api";
import { Icon } from "../../components/icons";
import { useDeletions } from "../../queries/documents";
import { useLastFileCheck } from "../../queries/integrity";
import { useDataFolder, useSettings, useUpdateSettings } from "../../queries/settings";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

const showDataFolder = () => {
  api.showDataFolder().catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));
};

const day = (ms: number) => new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short" });

/** Verify files: compares every stored PDF with its checksum; the last run shows under it. */
function VerifyFiles() {
  const { data: check } = useLastFileCheck();
  const [busy, setBusy] = useState(false);
  const verify = () => {
    setBusy(true);
    api
      .filesVerify()
      .catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message))
      .finally(() => setBusy(false));
  };
  const last = !check
    ? "Not run since launch."
    : check.problems.length === 0
      ? `Last run: ${day(check.at)} · all ${check.checked} files match.`
      : `Last run: ${day(check.at)} · ${check.problems.length} of ${check.checked} files missing or changed.`;
  return (
    <Group title="Checking" anchor="checking">
      <Row label="Verify files" anchor="verify-files" description={`Compares every PDF with its stored checksum. ${last}`}>
        <button className="btn btn--small" disabled={busy} onClick={verify}>
          Verify Now
        </button>
      </Row>
      {check?.problems.map((p) => (
        <Row key={p.file} label={p.what} description={<span className="mono">{p.file}</span>}>
          <span className="settings-value settings-value--problem">{p.trouble === "missing" ? "Missing" : "Changed"}</span>
        </Row>
      ))}
    </Group>
  );
}

export function PrivacyPage() {
  const { data: folder } = useDataFolder();
  const { data: settings } = useSettings();
  const { mutate } = useUpdateSettings();
  const { data: deletions } = useDeletions();
  const { data: backup } = useQuery({ queryKey: ["backup-included"], queryFn: api.backupIncluded });
  return (
    <>
      <Group title="Data folder" anchor="data-folder">
        <Row label="Location" description={<span className="mono">{folder ?? ""}</span>}>
          <button className="btn btn--small" onClick={showDataFolder}>
            Show in Finder
          </button>
        </Row>
        <Row label="Documents and statements" description="Original PDFs, copied in once and never changed. docs/<year>/ and docs/inbox/." />
        <Row label="Your decisions" description="hamster.sqlite plus journal.jsonl, an append-only log readable without this app." />
        <Row label="Tax rules" description="rules/rules-<year>.json, plain JSON you can edit." />
      </Group>
      <VerifyFiles />
      <Group title="Archive" anchor="archive">
        <Row label="Days in Archive before delete" description="An excluded document can be deleted after this many days." htmlFor="archive-days">
          <input
            id="archive-days"
            className="field settings-number"
            type="number"
            min={1}
            value={settings?.archiveDays ?? 7}
            onChange={(e) => {
              const days = Number(e.target.value);
              if (days >= 1) mutate({ archiveDays: days });
            }}
          />
        </Row>
      </Group>
      <Group title="Deletion log" anchor="deletion-log">
        {deletions?.length === 0 && <Row label="Nothing deleted" description="Documents deleted from the Archive are listed here for good." />}
        {deletions?.map((d, i) => (
          <Row key={i} label={d.title} description={`${d.file} · ${d.dates} · ${d.reason}`} />
        ))}
      </Group>
      <Group title="Backup" anchor="backup">
        <Row label="Time Machine" description={backup === true ? "The data folder is included in your backups." : backup === false ? "The data folder is excluded from your backups." : "Time Machine isn't set up, or can't be asked."}>
          <span className={backup ? "settings-value settings-value--ok" : "settings-value"}>
            {backup && <Icon name="check" size={14} strokeWidth={2.4} />}
            {backup === true ? "Included" : backup === false ? "Excluded" : "Unknown"}
          </span>
        </Row>
      </Group>
      <Group title="Network" anchor="network">
        <Row label="Internet access" description="Hamster contains no network code. Invoice text is read on this Mac.">
          <span className="settings-value settings-value--ok">
            <Icon name="check" size={14} strokeWidth={2.4} />
            None
          </span>
        </Row>
      </Group>
    </>
  );
}
