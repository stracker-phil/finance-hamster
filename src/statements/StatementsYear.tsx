import { YearStepper } from "../components/YearStepper";
import { useT } from "../i18n";
import { useCoverage } from "../queries/statements";
import { useStore } from "../state/store";

/** The year Accounts asks for: the route's, else the one chosen last; undefined asks for this year. */
export function useStatementsYear(asked?: number): number | undefined {
  const remembered = useStore((s) => s.statementsYear);
  return asked ?? remembered ?? undefined;
}

/** In the toolbar beside the title: steps through the years with a tax year or any data. */
export function StatementsYear({ year: asked }: { year?: number }) {
  const filter = useStore((s) => s.filter);
  const { data } = useCoverage(filter, useStatementsYear(asked));
  const text = useT().pages.statements;
  if (!data) return null;
  const choose = (year: number) => {
    const { setStatementsYear, navigate } = useStore.getState();
    setStatementsYear(year);
    navigate({ page: "statements", year });
  };
  return <YearStepper year={data.year} years={data.years} label={text.year} previousLabel={text.previousYear} nextLabel={text.nextYear} onChange={choose} />;
}
