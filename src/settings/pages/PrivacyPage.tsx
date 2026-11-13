import { api, toAppError } from "../../api";
import { Icon } from "../../components/icons";
import { useDataFolder } from "../../queries/settings";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

const showDataFolder = () => {
  api.showDataFolder().catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));
};

export function PrivacyPage() {
  const { data: folder } = useDataFolder();
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
