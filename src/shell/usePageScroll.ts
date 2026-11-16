import { useEffect, useLayoutEffect, type RefObject } from "react";
import { useStore } from "../state/store";

/** The element each page scrolls its content in; a page with a side panel lists its main scroller first. */
const SCROLLER = ".page-scroll, .year-layout__main";

/** How long a returning page may take to load enough content to reach its scroll position. */
const RESTORE_MS = 1500;

/** Tells the store where the page's list is scrolled to, and scrolls a page that Back or Forward returned to
 *  where it was left once its content is tall enough. */
export function usePageScroll(page: RefObject<HTMLElement | null>) {
  const route = useStore((s) => s.route);

  useEffect(() => {
    const onScroll = (e: Event) => {
      if (e.target instanceof Element && e.target === page.current?.querySelector(SCROLLER)) useStore.getState().noteScroll(e.target.scrollTop);
    };
    document.addEventListener("scroll", onScroll, true);
    return () => document.removeEventListener("scroll", onScroll, true);
  }, [page]);

  useLayoutEffect(() => {
    const target = useStore.getState().scroll;
    const scroller = page.current?.querySelector(SCROLLER);
    if (!target || !scroller) return;
    const reach = () => {
      scroller.scrollTop = target;
      return scroller.scrollTop >= target - 1;
    };
    if (reach()) return;
    // The owner's own scrolling wins over a list that is still loading.
    const giveUp = () => stop();
    const grown = new MutationObserver(() => reach() && stop());
    const timer = window.setTimeout(() => stop(), RESTORE_MS);
    function stop() {
      grown.disconnect();
      window.clearTimeout(timer);
      scroller?.removeEventListener("wheel", giveUp);
      scroller?.removeEventListener("keydown", giveUp);
    }
    grown.observe(scroller, { childList: true, subtree: true });
    scroller.addEventListener("wheel", giveUp, { passive: true });
    scroller.addEventListener("keydown", giveUp);
    return stop;
  }, [page, route]);
}
