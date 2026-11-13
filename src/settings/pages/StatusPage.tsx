import { copyText } from "../../lib/clipboard";
import { useStatus } from "../../queries/integrity";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

const copy = (value: string) =>
  copyText(value)
    .then(() => useStore.getState().showToast("success", `Copied ${value}`))
    .catch(() => {});

/** A health check computed on this Mac; a value copies on click. */
export function StatusPage() {
  const { data: groups } = useStatus();
  return (
    <>
      {groups?.map((group) => (
        <Group key={group.title} title={group.title} anchor={`status-${group.title.toLowerCase()}`}>
          {group.rows.length === 0 && <Row label="Nothing to check yet" />}
          {group.rows.map((row) => (
            <Row key={row.label} label={row.label} description={row.detail}>
              <button className={row.ok ? "status-value status-value--ok" : "status-value"} title="Copy" onClick={() => void copy(row.value)}>
                {row.value}
              </button>
            </Row>
          ))}
        </Group>
      ))}
    </>
  );
}
