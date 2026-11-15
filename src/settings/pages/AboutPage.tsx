import { useQuery } from "@tanstack/react-query";
import { api, toAppError } from "../../api";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

const run = (id: string) => () => void api.runCommand(id).catch((e: unknown) => useStore.getState().showToast("error", toAppError(e).message));

export function AboutPage() {
  const { data } = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });
  return (
    <Group title="Finance Hamster" anchor="version">
      <Row label="Version">
        <span className="settings-value">{data?.version ?? ""}</span>
      </Row>
      <Row label="Purpose" description="Collects ANV documents with the real amount paid, keeps bank statements and tracks subscriptions. Private, offline, for one person." />
      <Row label="Data folder" anchor="about-data-folder" description="The database, the journal and every PDF.">
        <button type="button" className="btn" onClick={run("app:data")}>
          Show in Finder
        </button>
      </Row>
      <Row label="Log folder" anchor="about-log-folder">
        <button type="button" className="btn" onClick={run("app:logs")}>
          Open
        </button>
      </Row>
    </Group>
  );
}
