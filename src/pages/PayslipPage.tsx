import { Lock } from "lucide-react";
import { api, type PayslipView } from "../api";
import { PopUpButton } from "../components/menu";
import { DetailLayout } from "../documents/DetailLayout";
import { useT } from "../i18n";
import { showError, undoToast } from "../lib/undoToast";
import { PayslipForm } from "../payslips/PayslipForm";
import { usePayslip } from "../queries/payslips";
import { useTaxYears } from "../queries/taxYears";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";

function remove(id: string) {
  api
    .payslipDelete(id)
    .then((label) => {
      undoToast(label);
      useStore.getState().navigate({ page: "documents", tab: "payslips" });
    })
    .catch(showError);
}

function move(payslip: PayslipView, year: number, month: number) {
  api
    .payslipEdit(payslip.id, { field: "period", year, month })
    .then((label) => label && undoToast(label))
    .catch(showError);
}

/** Which tax year and month the payslip is for; moving it is one undoable change. */
function PeriodPickers({ payslip }: { payslip: PayslipView }) {
  const text = useT().payslips;
  const open = (useTaxYears().data ?? []).filter((y) => y.state === "open").map((y) => y.year);
  const years = open.includes(payslip.year) ? open : [payslip.year, ...open];
  return (
    <div className="payslip-period">
      <PopUpButton aria-label={text.taxYear} value={payslip.year} disabled={!payslip.open} options={years.map((y) => ({ value: y, label: String(y) }))} onChange={(year) => move(payslip, year, payslip.month)} />
      <PopUpButton aria-label={text.period} value={payslip.month} disabled={!payslip.open} options={payslip.monthNames.map((name, i) => ({ value: i + 1, label: name }))} onChange={(month) => move(payslip, payslip.year, month)} />
    </div>
  );
}

/** A payslip: its PDF over most of the page, the numbers typed from it in the sidebar. */
export function PayslipPage({ payslipId }: { payslipId: string }) {
  const { data: payslip, isError } = usePayslip(payslipId);
  const text = useT().payslips;
  if (isError) return <EmptyPage title={text.missingTitle} text={text.missingText} />;
  if (!payslip) return null;
  const attach = () => {
    api
      .payslipAttach(payslip.id)
      .then((label) => label && undoToast(label))
      .catch(showError);
  };
  const banners = !payslip.open && (
    <div className="doc-banner" role="status">
      <Lock size={15} />
      <span>{text.closed}</span>
    </div>
  );
  return (
    <DetailLayout
      source={payslip.hasPdf ? { kind: "payslip", id: payslip.id } : null}
      banners={banners}
      label={payslip.title}
      empty={
        <>
          <span className="add-doc__hint">{text.noPdfYet}</span>
          {payslip.open && (
            <button type="button" className="btn btn--small" onClick={attach}>
              {text.attachPdf}
            </button>
          )}
        </>
      }
    >
      <h2 className="detail__title">{payslip.title}</h2>
      <PeriodPickers payslip={payslip} />
      <PayslipForm payslip={payslip} />
      {payslip.open && (
        <div className="detail__actions">
          {payslip.hasPdf && (
            <button type="button" className="link-btn" onClick={attach}>
              {text.replacePdf}
            </button>
          )}
          <button type="button" className="link-btn" onClick={() => remove(payslip.id)}>
            {text.delete}
          </button>
        </div>
      )}
    </DetailLayout>
  );
}
