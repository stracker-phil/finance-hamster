import { MenuHost } from "../components/menu";
import { useEffect, useState } from "react";
import { api } from "../api";
import { ConfirmHost } from "../components/ConfirmSheet";
import { Toasts } from "../components/Toasts";
import { TooltipHost } from "../components/TooltipHost";
import { CommandPalette } from "../palette/CommandPalette";
import { FirstRun } from "../pages/FirstRun";
import { PageBody } from "../pages/PageBody";
import { useAccounts } from "../queries/accounts";
import { useDocument } from "../queries/documents";
import { usePayslip } from "../queries/payslips";
import { useOther } from "../queries/others";
import { useStatement } from "../queries/statements";
import { useTaxYears } from "../queries/taxYears";
import { AddDocumentSheet } from "../documents/AddDocumentSheet";
import { ImportSheet } from "../statements/ImportSheet";
import { PlanSheetHost } from "../subscriptions/PlanSheet";
import { SubscriptionSheetHost } from "../subscriptions/SubscriptionSheet";
import { useStore } from "../state/store";
import { FilesBanner } from "./FilesBanner";
import { Header } from "./Header";
import { YearSidebarToggle } from "../year/YearSidebarToggle";
import { DocumentsTabs } from "../documents/DocumentsTabs";
import { Sidebar } from "./Sidebar";
import { titleFor } from "./titles";

/** The main window: sidebar, and the panel with the header over the current page; the first run while no
 *  account exists. */
export function Window() {
  const route = useStore((s) => s.route);
  const { data: accounts } = useAccounts();
  const { data: years = [] } = useTaxYears();
  const importPreview = useStore((s) => s.importPreview);
  const addPreview = useStore((s) => s.addPreview);
  const { data: openDocument } = useDocument(route.page === "document" ? route.documentId : "", route.page === "document");
  const documentTitle = route.page === "document" ? openDocument?.title : undefined;
  const { data: openPayslip } = usePayslip(route.page === "payslip" ? route.payslipId : "", route.page === "payslip");
  const payslipTitle = route.page === "payslip" ? openPayslip?.title : undefined;
  const { data: openStatement } = useStatement(route.page === "statement" ? route.statementId : "", route.page === "statement");
  const { data: openOther } = useOther(route.page === "other" ? route.otherId : "", route.page === "other");
  const detailTitle = payslipTitle ?? (route.page === "statement" ? openStatement?.title : undefined) ?? (route.page === "other" ? openOther?.title : undefined);
  // Tax rules taken on before the window opened (a newer built-in version) are told once.
  useEffect(() => {
    api
      .rulesTakeNotices()
      .then((notices) => notices.forEach((n) => useStore.getState().showToast("success", n)))
      .catch(() => undefined);
  }, []);
  // Keeps the first run's second step showing once its account exists.
  const [finishingFirstRun, setFinishingFirstRun] = useState(false);
  const importSheet = importPreview && <ImportSheet preview={importPreview} onImported={() => setFinishingFirstRun(false)} />;
  if (accounts?.length === 0 || finishingFirstRun) {
    return (
      <div className="window window--first-run">
        <FirstRun onAccountAdded={() => setFinishingFirstRun(true)} onDone={() => setFinishingFirstRun(false)} />
        {importSheet}
        <Toasts />
      </div>
    );
  }
  const accountName = route.page === "account" ? accounts?.find((a) => a.id === route.accountId)?.name : undefined;
  const pageName = accountName ?? documentTitle ?? detailTitle;
  const title = pageName ? { ...titleFor(route), title: pageName } : titleFor(route);
  return (
    <div className="window">
      <Sidebar years={years.map((y) => ({ year: y.year, closed: y.state === "closed" }))} />
      <main className="main">
        <div className="panel">
          <Header route={route} title={title} controls={route.page === "documents" ? <DocumentsTabs tab={route.tab} /> : undefined} trailing={route.page === "year" ? <YearSidebarToggle /> : undefined} />
          <FilesBanner />
          <section className="page" aria-label={title.title}>
            <PageBody route={route} />
          </section>
        </div>
      </main>
      {importSheet}
      {addPreview && <AddDocumentSheet preview={addPreview} />}
      <SubscriptionSheetHost />
      <PlanSheetHost />
      <CommandPalette />
      <Toasts />
      <ConfirmHost />
      <MenuHost />
      <TooltipHost />
    </div>
  );
}
