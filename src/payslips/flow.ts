import { api, type PayslipAdded } from "../api";
import { showError, undoToast } from "../lib/undoToast";
import { useStore } from "../state/store";

function opened(year: number, added: PayslipAdded | null) {
  if (!added) return;
  undoToast(added.label);
  useStore.getState().navigate({ page: "payslip", payslipId: added.id, year });
}

/** Adds the month's payslip to type by hand and opens it. */
export const enterPayslip = (year: number, month: number) => {
  api.payslipAdd(year, month).then((added) => opened(year, added)).catch(showError);
};

/** Asks for a PDF, adds the month's payslip with it and opens it. */
export const addPayslipPdf = (year: number, month: number) => {
  api.payslipAddPdf(year, month).then((added) => opened(year, added)).catch(showError);
};

/** Adds a PDF waiting in the Add sheet as the month's payslip and opens it. */
export const addWaitingPayslip = (token: string, year: number, month: number) =>
  api.payslipAddWaiting(token, year, month).then((added) => {
    useStore.getState().setAddPreview(null);
    opened(year, added);
  });

/** Shows or hides the payslip panel beside the tax year page. */
export const togglePayslipPanel = () => {
  api
    .getSettings()
    .then((s) => api.updateSettings({ payslipPanel: !s.payslipPanel }))
    .catch(showError);
};
