import { CircleAlert, CircleCheck, CircleX, Copy, type LucideIcon } from "lucide-react";
import type { StatusRow, Tone } from "../../api";
import { copyText } from "../../lib/clipboard";
import { useStatus } from "../../queries/integrity";
import { t, useT } from "../../i18n";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";

/** Only tones that tell something get an icon; an info value has none. */
const TONES: Partial<Record<Tone, { icon: LucideIcon; label: "ok" | "warning" | "error" }>> = {
  ok: { icon: CircleCheck, label: "ok" },
  warning: { icon: CircleAlert, label: "warning" },
  error: { icon: CircleX, label: "error" },
};

const copy = (row: StatusRow) =>
  copyText(row.copyText ?? row.value)
    .then(() => useStore.getState().showToast("success", t().settings.status.copied(row.label)))
    .catch(() => {});

/** A click copies the value; the clipboard icon shows on hover or focus, in the tone icon's place. */
function StatusValue({ row }: { row: StatusRow }) {
  const text = useT().settings.status;
  const tone = TONES[row.tone];
  return (
    <button type="button" className="status-value" data-tone={row.tone} title={text.clickToCopy} onClick={() => void copy(row)}>
      {tone && <tone.icon className="status-value__tone" size={14} role="img" aria-label={text[tone.label]} />}
      <Copy className="status-value__copy" size={14} aria-hidden="true" />
      <span className="settings-value">{row.value}</span>
    </button>
  );
}

/** A health check computed on this Mac. */
export function StatusPage() {
  const { data: groups } = useStatus();
  const text = useT().settings.status;
  return (
    <>
      {groups?.map((group) => (
        <Group key={group.title} title={group.title} anchor={`status-${group.title.toLowerCase()}`}>
          {group.rows.length === 0 && <Row label={text.nothing} />}
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
