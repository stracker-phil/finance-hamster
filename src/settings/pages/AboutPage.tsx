import { useQuery } from "@tanstack/react-query";
import { api } from "../../api";
import { Group, Row } from "../controls";

export function AboutPage() {
  const { data } = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });
  return (
    <Group title="Hamster" anchor="version">
      <Row label="Version">
        <span className="settings-value">{data?.version ?? ""}</span>
      </Row>
      <Row label="Purpose" description="Collects ANV documents with the real amount paid, keeps bank statements and tracks subscriptions. Private, offline, for one person." />
    </Group>
  );
}
