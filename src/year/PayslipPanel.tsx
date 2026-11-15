import { FileText } from "lucide-react";
import { useT } from "../i18n";
import { addPayslipPdf, enterPayslip } from "../payslips/flow";
import { usePayslipsYear } from "../queries/payslips";
import { useStore } from "../state/store";

/** Beside the tax year page: the year's twelve payslips in month order, then the year's sums. */
export function PayslipPanel({ year }: { year: number }) {
  const { data } = usePayslipsYear(year);
  const text = useT().payslips;
  const navigate = useStore((s) => s.navigate);
  return (
    <aside className="payslip-panel" aria-label={text.panel}>
      <h3 className="payslip-panel__title">{text.panel}</h3>
      <div className="payslip-panel__months" role="list" aria-label={text.months}>
        {data?.months.map((m) =>
          m.id ? (
            <div key={m.month} role="listitem">
              <button type="button" className="payslip-month" onClick={() => navigate({ page: "payslip", payslipId: m.id ?? "", year })}>
                <span className="payslip-month__name">{m.name}</span>
                {!m.hasPdf && <span className="add-doc__hint">{text.noPdf}</span>}
                <span className="payslip-month__amount">{m.payout ?? ""}</span>
                {m.hasPdf && <FileText size={13} className="payslip-month__icon" aria-hidden="true" />}
              </button>
            </div>
          ) : (
            <div key={m.month} role="listitem" className="payslip-month payslip-month--missing">
              <span className="payslip-month__name">{m.name}</span>
              {data.open ? (
                <span className="payslip-month__actions">
                  <button type="button" className="link-btn" onClick={() => enterPayslip(year, m.month)}>
                    {text.enter}
                  </button>
                  <button type="button" className="link-btn" onClick={() => addPayslipPdf(year, m.month)}>
                    {text.addPdf}
                  </button>
                </span>
              ) : (
                <span className="add-doc__hint">{text.missing}</span>
              )}
            </div>
          ),
        )}
      </div>
      {data && (
        <dl className="payslip-panel__totals" aria-label={text.year}>
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
      )}
    </aside>
  );
}
