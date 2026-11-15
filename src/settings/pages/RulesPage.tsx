import { useState } from "react";
import { api, toAppError, type RuleView } from "../../api";
import { t, useT } from "../../i18n";
import { useLineRules } from "../../queries/lineRules";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";
import { RuleSheet } from "./RuleSheet";

const rerun = async () => {
  const { confirm, showToast } = useStore.getState();
  const text = t().settings.rules;
  if (!(await confirm({ title: text.rerunTitle, text: text.rerunText, action: text.rerunAction }))) return;
  try {
    showToast("success", await api.rulesRerun());
  } catch (e) {
    showToast("error", toAppError(e).message);
  }
};

export function RulesPage() {
  const { data } = useLineRules();
  const { rules: text, common } = useT().settings;
  const [sheet, setSheet] = useState<{ rule?: RuleView } | null>(null);
  return (
    <>
      {data?.groups.map((group) => (
        <Group key={group.title} title={group.title}>
          {group.rules.map((rule) => (
            <Row key={rule.id} label={rule.label} description={rule.detail}>
              <button className="btn btn--small" onClick={() => setSheet({ rule })}>
                {common.edit}
              </button>
            </Row>
          ))}
        </Group>
      ))}
      <Group title={common.add} anchor="add-rule">
        <Row label={text.newRule} description={text.newRuleDescription}>
          <button className="btn btn--small" onClick={() => setSheet({})}>
            {text.newRuleButton}
          </button>
        </Row>
        <Row label={text.rerun} description={text.rerunDescription} anchor="rerun">
          <button className="btn btn--small" onClick={() => void rerun()}>
            {text.rerunButton}
          </button>
        </Row>
      </Group>
      {sheet && <RuleSheet rule={sheet.rule} items={data?.items ?? []} onClose={() => setSheet(null)} />}
    </>
  );
}
