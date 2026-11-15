import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, toAppError } from "../../api";
import { Icon } from "../../components/icons";
import { useLanguage, useT } from "../../i18n";
import { useDeletions } from "../../queries/documents";
import { useLastFileCheck } from "../../queries/integrity";
import { useDataFolder, useSettings, useUpdateSettings } from "../../queries/settings";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

const showDataFolder = () => {
  api.showDataFolder().catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));
};

const day = (ms: number) => new Date(ms).toLocaleDateString(useLanguage.getState().language === "de" ? "de-DE" : undefined, { day: "numeric", month: "short" });

/** Verify files: compares every stored PDF with its checksum; the last run shows under it. */
function VerifyFiles() {
  const { data: check } = useLastFileCheck();
  const [busy, setBusy] = useState(false);
  const text = useT().settings.privacy;
  const verify = () => {
    setBusy(true);
    api
      .filesVerify()
      .catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message))
      .finally(() => setBusy(false));
  };
  const last = !check
    ? text.notRun
    : check.problems.length === 0
      ? text.allMatch(day(check.at), check.checked)
      : text.someProblems(day(check.at), check.problems.length, check.checked);
  return (
    <Group title={text.checking} anchor="checking">
      <Row label={text.verify} anchor="verify-files" description={text.verifyDescription(last)}>
        <button className="btn btn--small" disabled={busy} onClick={verify}>
          {text.verifyButton}
        </button>
      </Row>
      {check?.problems.map((p) => (
        <Row key={p.file} label={p.what} description={<span className="mono">{p.file}</span>}>
          <span className="settings-value settings-value--problem">{p.trouble === "missing" ? text.missing : text.changed}</span>
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
  const { privacy: text, common } = useT().settings;
  return (
    <>
      <Group title={text.dataFolder} anchor="data-folder">
        <Row label={text.location} description={<span className="mono">{folder ?? ""}</span>}>
          <button className="btn btn--small" onClick={showDataFolder}>
            {common.showInFinder}
          </button>
        </Row>
        <Row label={text.documents} description={text.documentsDescription} />
        <Row label={text.decisions} description={text.decisionsDescription} />
        <Row label={text.taxRules} description={text.taxRulesDescription} />
      </Group>
      <VerifyFiles />
      <Group title={text.archive} anchor="archive">
        <Row label={text.archiveDays} description={text.archiveDaysDescription} htmlFor="archive-days">
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
      <Group title={text.deletionLog} anchor="deletion-log">
        {deletions?.length === 0 && <Row label={text.nothingDeleted} description={text.nothingDeletedDescription} />}
        {deletions?.map((d, i) => (
          <Row key={i} label={d.title} description={`${d.file} · ${d.dates} · ${d.reason}`} />
        ))}
      </Group>
      <Group title={text.backup} anchor="backup">
        <Row label={text.timeMachine} description={backup === true ? text.includedDescription : backup === false ? text.excludedDescription : text.unknownDescription}>
          <span className={backup ? "settings-value settings-value--ok" : "settings-value"}>
            {backup && <Icon name="check" size={14} strokeWidth={2.4} />}
            {backup === true ? text.included : backup === false ? text.excluded : text.unknown}
          </span>
        </Row>
      </Group>
      <Group title={text.network} anchor="network">
        <Row label={text.internet} description={text.internetDescription}>
          <span className="settings-value settings-value--ok">
            <Icon name="check" size={14} strokeWidth={2.4} />
            {text.none}
          </span>
        </Row>
      </Group>
    </>
  );
}
