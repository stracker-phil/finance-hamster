import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { usePageCommands } from "./commands/pageCommands";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { queryClient as defaultClient } from "./queries/queryClient";
import { useEvents } from "./queries/useEvents";
import { usePdfCovers } from "./lib/pdfCover";
import { useLook } from "./queries/useLook";
import { Window } from "./shell/Window";
import { useAppEvents } from "./state/useAppEvents";

function MainWindow() {
  useAppEvents();
  useEvents();
  useLook();
  usePageCommands();
  usePdfCovers();
  return <Window />;
}

/** The main window's page; tests pass a fresh query client. */
export default function App({ client = defaultClient }: { client?: QueryClient }) {
  return (
    <ErrorBoundary>
      <QueryClientProvider client={client}>
        <MainWindow />
      </QueryClientProvider>
    </ErrorBoundary>
  );
}
