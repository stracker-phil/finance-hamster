import { useQuery } from "@tanstack/react-query";
import { api, toAppError } from "../../api";
import { useT } from "../../i18n";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

const run = (id: string) => () => void api.runCommand(id).catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));

/** The official name; the same in both languages. */
const FULL_NAME = "Finance Hamster";

export function AboutPage() {
  const { data } = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });
  const { about: text, common } = useT().settings;
  return (
    <Group title={FULL_NAME} anchor="version">
      <Row label={text.version}>
        <span className="settings-value">{data?.version ?? ""}</span>
      </Row>
      <Row label={text.purpose} description={text.purposeDescription} />
      <Row label={text.dataFolder} anchor="about-data-folder" description={text.dataFolderDescription}>
        <button type="button" className="btn" onClick={run("app:data")}>
          {common.showInFinder}
        </button>
      </Row>
      <Row label={text.logFolder} anchor="about-log-folder">
        <button type="button" className="btn" onClick={run("app:logs")}>
          {text.open}
        </button>
      </Row>
    </Group>
  );
}
