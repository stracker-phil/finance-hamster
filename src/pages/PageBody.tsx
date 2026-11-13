import type { Route } from "../state/route";
import { AccountPage } from "./AccountPage";
import { DocumentPage } from "./DocumentPage";
import { EmptyPage } from "./EmptyPage";
import { ReviewPage } from "./ReviewPage";
import { StatementsPage } from "./StatementsPage";

/** The body of the current page. */
export function PageBody({ route }: { route: Route }) {
  switch (route.page) {
    case "today":
      return <EmptyPage title="You're up to date" text="Import a statement to get started." />;
    case "subscriptions":
      return <EmptyPage title="No subscriptions" text="Subscriptions show here once you add one." />;
    case "statements":
      return <StatementsPage />;
    case "documents":
      return <EmptyPage title="No documents" text="Drop an invoice PDF onto the sidebar to add it." />;
    case "review":
      return <ReviewPage />;
    case "account":
      return <AccountPage accountId={route.accountId} />;
    case "year":
      return <EmptyPage title={`Tax year ${route.year}`} text="Nothing is recorded for this year yet." />;
    case "document":
      return <DocumentPage documentId={route.documentId} />;
    case "log":
      return <EmptyPage title="Log" text="Every change to a document shows here." />;
    case "archive":
      return <EmptyPage title="Archive" text="Excluded documents show here." />;
  }
}
