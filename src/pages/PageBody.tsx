import type { Route } from "../state/route";
import { AccountPage } from "./AccountPage";
import { ArchivePage } from "./ArchivePage";
import { DocumentPage } from "./DocumentPage";
import { DocumentsPage } from "./DocumentsPage";
import { LogPage } from "./LogPage";
import { OtherPage } from "./OtherPage";
import { PayslipPage } from "./PayslipPage";
import { StatementPage } from "./StatementPage";
import { ReviewPage } from "./ReviewPage";
import { StatementsPage } from "./StatementsPage";
import { SubscriptionsPage } from "./SubscriptionsPage";
import { TodayPage } from "./TodayPage";
import { YearPage } from "./YearPage";

/** The body of the current page. */
export function PageBody({ route }: { route: Route }) {
  switch (route.page) {
    case "today":
      return <TodayPage />;
    case "subscriptions":
      return <SubscriptionsPage key={route.subscriptionId ?? ""} open={route.subscriptionId} />;
    case "statements":
      return <StatementsPage />;
    case "documents":
      return <DocumentsPage tab={route.tab} />;
    case "review":
      return <ReviewPage />;
    case "account":
      return <AccountPage accountId={route.accountId} statementId={route.statementId} />;
    case "year":
      return <YearPage year={route.year} attention={route.attention} />;
    case "document":
      return <DocumentPage documentId={route.documentId} />;
    case "statement":
      return <StatementPage statementId={route.statementId} />;
    case "other":
      return <OtherPage otherId={route.otherId} />;
    case "payslip":
      return <PayslipPage payslipId={route.payslipId} />;
    case "log":
      return <LogPage year={route.year} />;
    case "archive":
      return <ArchivePage year={route.year} />;
  }
}
