import { useEffect, useRef, type RefObject } from "react";
import { api, toAppError, type Covers, type Frame } from "../api";
import { useStore, type PdfCover } from "../state/store";

/** Room around an overlay's box for its shadow. */
const SHADOW = 8;

const NOTHING: Covers = { hidden: false, holes: [], clickThrough: false };

/** Registers an overlay above the native PDF preview while `active` (ADR 0057). A sheet hides the preview; a
 *  popover or a float is cut out of it, where `ref` shows on the page. */
export function useCoversPdf(kind: PdfCover["kind"], ref?: RefObject<HTMLElement | null>, active = true) {
  useEffect(() => {
    if (!active) return;
    const cover: PdfCover = { kind, element: ref?.current ?? null };
    useStore.getState().coverPdf(cover);
    return () => useStore.getState().uncoverPdf(cover);
  }, [kind, ref, active]);
}

/** The boxes an element draws; one with `display: contents` draws only its children. */
function boxesOf(element: Element): DOMRect[] {
  if (getComputedStyle(element).display === "contents") return [...element.children].flatMap(boxesOf);
  const box = element.getBoundingClientRect();
  return box.width > 0 && box.height > 0 ? [box] : [];
}

const hole = (box: DOMRect): Frame => ({
  x: box.left - SHADOW,
  y: box.top - SHADOW,
  width: box.width + 2 * SHADOW,
  height: box.height + 2 * SHADOW,
});

/** The elements cut out of the preview. */
const cutOut = (covers: PdfCover[]) => covers.flatMap((c) => (c.kind !== "sheet" && c.element ? [c.element] : []));

/** What the open overlays ask of the preview. */
export function coversOf(covers: PdfCover[]): Covers {
  return {
    hidden: covers.some((c) => c.kind === "sheet"),
    holes: cutOut(covers).flatMap(boxesOf).map(hole),
    clickThrough: covers.some((c) => c.kind === "popover"),
  };
}

const LAYOUT_EVENTS = ["resize", "animationend", "transitionend"] as const;

/** Calls `onChange` when any of `elements` moves or changes size; returns the function that stops watching. */
function watchLayout(elements: HTMLElement[], onChange: () => void): () => void {
  // jsdom has no ResizeObserver.
  const resized = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(onChange);
  const changed = new MutationObserver(onChange);
  for (const element of elements) {
    resized?.observe(element);
    changed.observe(element, { attributes: true, childList: true, subtree: true });
  }
  for (const name of LAYOUT_EVENTS) window.addEventListener(name, onChange, true);
  return () => {
    resized?.disconnect();
    changed.disconnect();
    for (const name of LAYOUT_EVENTS) window.removeEventListener(name, onChange, true);
  };
}

/** Tells Rust what covers the preview whenever an overlay opens, closes, moves or changes size. */
export function usePdfCovers() {
  const covers = useStore((s) => s.pdfCovers);
  const sent = useRef(JSON.stringify(NOTHING));
  useEffect(() => {
    const send = () => {
      const payload = coversOf(covers);
      const key = JSON.stringify(payload);
      if (key === sent.current) return;
      sent.current = key;
      api.pdfViewCover(payload).catch((e: unknown) => api.logFrontendError(`PDF preview: ${toAppError(e).message}`).catch(() => {}));
    };
    let frame = 0;
    const sendNextFrame = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(send);
    };
    send();
    const stop = watchLayout(cutOut(covers), sendNextFrame);
    return () => {
      cancelAnimationFrame(frame);
      stop();
    };
  }, [covers]);
}
