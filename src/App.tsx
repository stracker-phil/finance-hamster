import { ErrorBoundary } from "./components/ErrorBoundary";
import { useAppearance } from "./lib/theme";
import { Window } from "./shell/Window";
import { useAppEvents } from "./state/useAppEvents";

export default function App() {
  useAppEvents();
  useAppearance();
  return (
    <ErrorBoundary>
      <Window />
    </ErrorBoundary>
  );
}
