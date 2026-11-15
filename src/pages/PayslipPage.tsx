import { Lock } from "lucide-react";
import { api } from "../api";
import { PdfPreview } from "../components/PdfPreview";
import { useT } from "../i18n";
import { showError, undoToast } from "../lib/undoToast";
import { PayslipForm } from "../payslips/PayslipForm";
import { usePayslip } from "../queries/payslips";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";

function remove(id: string, year: number) {
  api
    .payslipDelete(id)
    .then((label) => {
      undoToast(label);
      useStore.getState().navigate({ page: "year", year });
    })
    .catch(showError);
}

/** A payslip: its PDF beside the numbers typed from it. */
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
  return (
    <div className="page-scroll">
      {!payslip.open && (
        <div className="doc-banner" role="status">
          <Lock size={15} />
          <span>{text.closed}</span>
        </div>
      )}
      <div className="doc-page payslip-page">
        <div className="doc-page__file">
          {payslip.hasPdf ? (
            <div className="doc-page__preview">
              <PdfPreview source={{ kind: "payslip", id: payslip.id }} />
            </div>
          ) : (
            <div className="doc-page__preview payslip-page__empty">
              <span className="add-doc__hint">{text.noPdfYet}</span>
            </div>
          )}
          {payslip.open && (
            <button type="button" className="btn btn--small" onClick={attach}>
              {payslip.hasPdf ? text.replacePdf : text.attachPdf}
            </button>
          )}
        </div>
        <div className="doc-form">
          <h2 className="payslip-page__title">{payslip.title}</h2>
          <PayslipForm payslip={payslip} />
          {payslip.open && (
            <div className="doc-form__exclude">
              <button type="button" className="link-btn" onClick={() => remove(payslip.id, payslip.year)}>
                {text.delete}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
