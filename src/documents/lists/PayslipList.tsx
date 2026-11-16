import { FileText } from "lucide-react";
import { useT } from "../../i18n";
import { addPayslipPdf, enterPayslip } from "../../payslips/flow";
import { usePayslipsList } from "../../queries/payslips";
import { useStore } from "../../state/store";

/** Every year with payslips or still open, newest first, each with its twelve months in their order; `query`
 *  keeps the matching months. */
export function PayslipList({ query, noMatch }: { query: string; noMatch: React.ReactNode }) {
  const { data } = usePayslipsList(query);
  const years = data ?? [];
  const navigate = useStore((s) => s.navigate);
  const text = useT().payslips;
  if (data && data.length === 0) return query.trim() ? <>{noMatch}</> : <p className="docs-page__none">{text.noYears}</p>;
  return (
    <>
      {years.map((y) => (
        <section key={y.year} className="docs-group" aria-label={String(y.year)}>
          <div className="docs-group__head">
            <button className="link-btn docs-group__title" onClick={() => navigate({ page: "year", year: y.year })}>
              {y.year}
            </button>
            <span className="add-doc__hint">{y.count}</span>
          </div>
          <div role="list" aria-label={text.months}>
            {y.months.map((m) =>
              m.id ? (
                <div key={m.month} role="listitem">
                  <button className="docs-row" onClick={() => navigate({ page: "payslip", payslipId: m.id ?? "", year: y.year })}>
                    <FileText size={17} strokeWidth={1.8} className="docs-row__icon" />
                    <span className="docs-row__text">
                      <span className="docs-row__title">{m.name}</span>
                      {!m.hasPdf && <span className="add-doc__hint">{text.noPdf}</span>}
                    </span>
                    <span className="docs-row__anv">{m.payout ?? ""}</span>
                  </button>
                </div>
              ) : (
                <div key={m.month} role="listitem" className="docs-row docs-row--missing">
                  <span className="docs-row__icon" />
                  <span className="docs-row__text">
                    <span className="docs-row__title">{m.name}</span>
                    <span className="add-doc__hint">{text.missing}</span>
                  </span>
                  {y.open && (
                    <span className="payslip-month__actions">
                      <button type="button" className="link-btn" onClick={() => enterPayslip(y.year, m.month)}>
                        {text.enter}
                      </button>
                      <button type="button" className="link-btn" onClick={() => addPayslipPdf(y.year, m.month)}>
                        {text.addPdf}
                      </button>
                    </span>
                  )}
                </div>
              ),
            )}
          </div>
        </section>
      ))}
    </>
  );
}
