import { CircleAlert, CircleCheck, CircleX, Copy, type LucideIcon } from "lucide-react";
import type { StatusRow, Tone } from "../../api";
import { copyText } from "../../lib/clipboard";
import { useStatus } from "../../queries/integrity";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

/** Only tones that tell something get an icon; an info value has none. */
const TONES: Partial<Record<Tone, { icon: LucideIcon; label: string }>> = {
  ok: { icon: CircleCheck, label: "OK" },
  warning: { icon: CircleAlert, label: "Warning" },
  error: { icon: CircleX, label: "Error" },
};

const copy = (row: StatusRow) =>
  copyText(row.copyText ?? row.value)
    .then(() => useStore.getState().showToast("success", `Copied ${row.label}`))
    .catch(() => {});

/** A click copies the value; the clipboard icon shows on hover or focus, in the tone icon's place. */
function StatusValue({ row }: { row: StatusRow }) {
  const tone = TONES[row.tone];
  return (
    <button type="button" className="status-value" data-tone={row.tone} title="Click to copy" onClick={() => void copy(row)}>
      {tone && <tone.icon className="status-value__tone" size={14} role="img" aria-label={tone.label} />}
      <Copy className="status-value__copy" size={14} aria-hidden="true" />
      <span className="settings-value">{row.value}</span>
    </button>
  );
}

/** A health check computed on this Mac. */
export function StatusPage() {
  const { data: groups } = useStatus();
  return (
    <>
      {groups?.map((group) => (
        <Group key={group.title} title={group.title} anchor={`status-${group.title.toLowerCase()}`}>
          {group.rows.length === 0 && <Row label="Nothing to check yet" />}
          {group.rows.map((row) => (
            <Row key={row.label} label={row.label} description={row.detail}>
              <StatusValue row={row} />
            </Row>
          ))}
        </Group>
      ))}
    </>
  );
}
