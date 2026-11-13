import { useState } from "react";
import { ConfirmHost } from "../components/ConfirmSheet";
import { Toasts } from "../components/Toasts";
import { TooltipHost } from "../components/TooltipHost";
import { CommandPalette } from "../palette/CommandPalette";
import { FirstRun } from "../pages/FirstRun";
import { PageBody } from "../pages/PageBody";
import { useAccounts } from "../queries/accounts";
import { useDocument } from "../queries/documents";
import { useTaxYears } from "../queries/taxYears";
import { AddDocumentSheet } from "../documents/AddDocumentSheet";
import { ImportSheet } from "../statements/ImportSheet";
import { PlanSheetHost } from "../subscriptions/PlanSheet";
import { SubscriptionSheetHost } from "../subscriptions/SubscriptionSheet";
import { useStore } from "../state/store";
import { Header } from "./Header";
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
  const pageName = accountName ?? documentTitle;
  const title = pageName ? { ...titleFor(route), title: pageName } : titleFor(route);
  return (
    <div className="window">
      <Sidebar years={years.map((y) => ({ year: y.year, closed: y.state === "closed" }))} />
      <main className="main">
        <div className="panel">
          <Header route={route} title={title} />
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
      <TooltipHost />
    </div>
  );
}
