import { api, type RulesYear } from "../../api";
import { t, useT } from "../../i18n";
import { showError, undoToast } from "../../lib/undoToast";
import { useRulesOverview } from "../../queries/taxYears";
import { useStore } from "../../state/store";
import { DrillRow, Group, Row } from "../controls";

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

/** Asks for a rules file, says what importing it would do, and imports it once confirmed. */
export async function importRules() {
  const text = t().settings.taxRules;
  const toast = useStore.getState().showToast;
  try {
    const found = await api.rulesImportChoose();
    if (!found) return;
    if (found.year === null || found.problems.length > 0) return toast("error", text.importProblems(found.problems));
    if (found.replaces && found.changes.length === 0) return toast("info", text.importSame(found.year));
    const message = found.replaces ? text.importReplaces(found.year, found.changes) : text.importStarts(found.year);
    const confirmed = await useStore.getState().confirm({ title: text.importConfirm(found.year), text: message, action: text.importAction });
    if (confirmed) undoToast(await api.rulesImport(found.text));
  } catch (e) {
    showError(e);
  }
}

/** Settings → Tax Rules: Start for the next year above the years, newest first, so they read in order; each year
 *  opens one level down. Import takes a rules file for any year. */
export function TaxRulesPage() {
  const { data } = useRulesOverview();
  const text = useT().settings.taxRules;
  const next = data?.nextYear;
  return (
    <>
      <Group title={text.years} anchor="tax-years">
        {next !== undefined && next !== null && (
          <Row label={text.start(next)} description={data?.nextIsBuiltin ? text.startBuiltin : text.startCopy(next - 1)} anchor="start-year">
            <button className="btn btn--small" onClick={() => void startNext(next, data?.nextIsBuiltin ?? false)}>
              {text.startButton}
            </button>
          </Row>
        )}
        {data?.years.map((year) => (
          <DrillRow key={year.year} label={String(year.year)} description={describeYear(year)} target={`taxrules/${year.year}`} />
        ))}
      </Group>
      <Group>
        <Row label={text.importTitle} description={text.importDescription} anchor="import-rules">
          <button className="btn btn--small" onClick={() => void importRules()}>
            {text.importButton}
          </button>
        </Row>
      </Group>
    </>
  );
}
