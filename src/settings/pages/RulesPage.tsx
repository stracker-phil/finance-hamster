import { useState } from "react";
import { api, toAppError, type RuleView } from "../../api";
import { useLineRules } from "../../queries/lineRules";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";
import { RuleSheet } from "./RuleSheet";

const rerun = async () => {
  const { confirm, showToast } = useStore.getState();
  if (!(await confirm({ title: "Apply the rules again?", text: "Every line is checked again. Your own choices stay untouched.", action: "Re-run" }))) return;
  try {
    showToast("success", await api.rulesRerun());
  } catch (e) {
    showToast("error", toAppError(e).message);
  }
};

export function RulesPage() {
  const { data } = useLineRules();
  const [sheet, setSheet] = useState<{ rule?: RuleView } | null>(null);
  return (
    <>
      {data?.groups.map((group) => (
        <Group key={group.title} title={group.title}>
          {group.rules.map((rule) => (
            <Row key={rule.id} label={rule.label} description={rule.detail}>
              <button className="btn btn--small" onClick={() => setSheet({ rule })}>
                Edit…
              </button>
            </Row>
          ))}
        </Group>
      ))}
      <Group title="Add" anchor="add-rule">
        <Row label="New rule" description="Match bank text, amount or account. Then: assign an item or ignore.">
          <button className="btn btn--small" onClick={() => setSheet({})}>
            New Rule…
          </button>
        </Row>
        <Row label="Apply rules again" description="Re-checks all lines. Your own choices stay untouched." anchor="rerun">
          <button className="btn btn--small" onClick={() => void rerun()}>
            Re-run…
          </button>
        </Row>
      </Group>
      {sheet && <RuleSheet rule={sheet.rule} items={data?.items ?? []} onClose={() => setSheet(null)} />}
    </>
  );
}
