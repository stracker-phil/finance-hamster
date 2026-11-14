import type { Route } from "../state/route";
import { AccountPage } from "./AccountPage";
import { ArchivePage } from "./ArchivePage";
import { DocumentPage } from "./DocumentPage";
import { DocumentsPage } from "./DocumentsPage";
import { LogPage } from "./LogPage";
import { ReviewPage } from "./ReviewPage";
import { StatementsPage } from "./StatementsPage";
import { SubscriptionsPage } from "./SubscriptionsPage";
import { TodayPage } from "./TodayPage";
import { YearCheckPage } from "./YearCheckPage";
import { YearPage } from "./YearPage";

/** The body of the current page. */
export function PageBody({ route }: { route: Route }) {
  switch (route.page) {
    case "today":
      return <TodayPage />;
    case "subscriptions":
      return <SubscriptionsPage />;
    case "statements":
      return <StatementsPage />;
    case "documents":
      return <DocumentsPage />;
    case "review":
      return <ReviewPage />;
    case "account":
      return <AccountPage accountId={route.accountId} />;
    case "year":
      return <YearPage year={route.year} />;
    case "document":
      return <DocumentPage documentId={route.documentId} />;
    case "log":
      return <LogPage year={route.year} />;
    case "archive":
      return <ArchivePage year={route.year} />;
    case "yearCheck":
      return <YearCheckPage year={route.year} />;
  }
}
