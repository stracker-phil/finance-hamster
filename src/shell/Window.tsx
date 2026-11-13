import { ConfirmHost } from "../components/ConfirmSheet";
import { Toasts } from "../components/Toasts";
import { TooltipHost } from "../components/TooltipHost";
import { PageBody } from "../pages/PageBody";
import { useStore } from "../state/store";
import { Header } from "./Header";
import { Sidebar } from "./Sidebar";
import { titleFor } from "./titles";

/** The main window: sidebar, and the panel with the header over the current page. */
export function Window() {
  const route = useStore((s) => s.route);
  return (
    <div className="window">
      <Sidebar years={[]} />
      <main className="main">
        <div className="panel">
          <Header route={route} title={titleFor(route)} />
          <section className="page" aria-label={titleFor(route).title}>
            <PageBody route={route} />
          </section>
        </div>
      </main>
      <Toasts />
      <ConfirmHost />
      <TooltipHost />
    </div>
  );
}
