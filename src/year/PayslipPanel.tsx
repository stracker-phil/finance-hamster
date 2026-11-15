import { useT } from "../i18n";
import { usePayslipsYear } from "../queries/payslips";
import { useStore } from "../state/store";

/** The tax year's sidebar (ADR 0056): the year by its payslips, summed. Payslips are added and changed in
 *  Dokumente → Lohnzettel. */
export function PayslipPanel({ year }: { year: number }) {
  const { data } = usePayslipsYear(year);
  const text = useT().payslips;
  const navigate = useStore((s) => s.navigate);
  return (
    <aside className="year-inspector" aria-label={text.yourYear}>
      <h3 className="year-inspector__title">{text.yourYear}</h3>
      {data && (
        <>
          <dl className="year-inspector__totals">
            {data.totals.map((t) => (
              <div key={t.label} className="payslip-total">
                <dt>{t.label}</dt>
                <dd>{t.value}</dd>
              </div>
            ))}
            <div className="payslip-total">
              <dt>{text.homeOfficeDays}</dt>
              <dd>{data.telework ?? "–"}</dd>
            </div>
          </dl>
          <p className="add-doc__hint">{data.count}</p>
          <button type="button" className="link-btn" onClick={() => navigate({ page: "documents", tab: "payslips" })}>
            {text.allPayslips}
          </button>
        </>
      )}
    </aside>
  );
}
