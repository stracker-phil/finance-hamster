import { useState } from "react";
import { api, type RulesYear } from "../../api";
import { t, useT } from "../../i18n";
import { showError, undoToast } from "../../lib/undoToast";
import { useRulesOverview } from "../../queries/taxYears";
import { useStore } from "../../state/store";
import { Group, Row } from "../controls";
import { RulesYearSheet } from "./RulesYearSheet";

/** The line under a year: where its rules come from, its state, and anything built-in it doesn't use. */
export function describeYear(year: RulesYear): string {
  const text = t().settings.taxRules;
  const parts = [year.source === "own" ? text.own(year.basedOn) : text.builtin(year.ruleset), year.state === "open" ? text.open : text.closed];
  if (year.builtinNewer && year.problems.length > 0) parts.push(text.cantTakeOn(year.builtinNewer, year.problems[0]));
  else if (year.builtinNewer) parts.push(text.newer(year.builtinNewer));
  if (year.unverified > 0) parts.push(text.unverified(year.unverified));
  return parts.join(" · ");
}

async function startNext(year: number, builtin: boolean) {
  const text = t().settings.taxRules;
  const from = year - 1;
  const confirmed = await useStore.getState().confirm({ title: text.start(year), text: builtin ? text.startBuiltin : text.startCopy(from), action: text.startAction });
  if (confirmed) api.rulesStartNext().then(undoToast).catch(showError);
}

/** Settings → Tax Rules: each year's rules, opened to read or change them, and Start for the next year. */
export function TaxRulesPage() {
  const { data } = useRulesOverview();
  const [open, setOpen] = useState<number | null>(null);
  const text = useT().settings.taxRules;
  const next = data?.nextYear;
  return (
    <>
      <Group title={text.years} anchor="tax-years">
        {data?.years.map((year) => (
          <Row key={year.year} label={String(year.year)} description={describeYear(year)}>
            <button className="btn btn--small" onClick={() => setOpen(year.year)}>
              {year.state === "open" ? text.edit : text.show}
            </button>
          </Row>
        ))}
        {next !== undefined && next !== null && (
          <Row label={text.start(next)} description={data?.nextIsBuiltin ? text.startBuiltin : text.startCopy(next - 1)} anchor="start-year">
            <button className="btn btn--small" onClick={() => void startNext(next, data?.nextIsBuiltin ?? false)}>
              {text.startButton}
            </button>
          </Row>
        )}
      </Group>
      {open !== null && <RulesYearSheet year={open} onClose={() => setOpen(null)} />}
    </>
  );
}
