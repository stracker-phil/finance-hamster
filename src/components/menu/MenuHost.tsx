import { useEffect, useRef, useState, type RefObject } from "react";
import { createPortal } from "react-dom";
import { useCoversPdf } from "../../lib/pdfCover";
import { useStore } from "../../state/store";
import { MenuPanel } from "./MenuPanel";
import type { Placement } from "./placement";
import { isEnabled, isItem, type MenuItem, type OpenMenu } from "./types";

/** Shows the menu `openMenu` asked for; mounted once per window. */
export function MenuHost() {
  const menu = useStore((s) => s.menu);
  const close = useStore((s) => s.closeMenu);
  return menu ? <OpenMenuLayer key={menu.id} menu={menu} onClose={close} /> : null;
}

function placementOf(at: OpenMenu["at"]): Placement {
  if (!(at instanceof HTMLElement)) return { point: at };
  const { left, top, right, bottom } = at.getBoundingClientRect();
  return { below: { left, top, right, bottom } };
}

/** A button's menu starts on its checked row, else its first; a right-click's on none. */
function initialRow(menu: OpenMenu): number {
  if (!(menu.at instanceof HTMLElement)) return -1;
  const checked = menu.entries.findIndex((e) => isItem(e) && e.checked === true && isEnabled(e));
  return checked >= 0 ? checked : menu.entries.findIndex(isEnabled);
}

function returnFocus(opener: HTMLElement | null) {
  if (opener?.isConnected) opener.focus({ preventScroll: true });
}

/** Closes on Escape, a press outside, a scroll, a resize or the window losing focus; the focus then goes back to
 *  the opener unless a row moved it on. */
function useDismiss(layer: RefObject<HTMLDivElement | null>, menu: OpenMenu, onClose: () => void) {
  useEffect(() => {
    const inside = (target: EventTarget | null) => target instanceof Node && !!layer.current?.contains(target);
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      onClose();
    };
    // A press on the menu's own button is left to the button, which closes it.
    const onPress = (e: MouseEvent) => {
      const onAnchor = menu.at instanceof HTMLElement && e.target instanceof Node && menu.at.contains(e.target);
      if (!inside(e.target) && !onAnchor) onClose();
    };
    const onScroll = (e: Event) => {
      if (!inside(e.target)) onClose();
    };
    window.addEventListener("keydown", onKey, true);
    document.addEventListener("mousedown", onPress, true);
    document.addEventListener("scroll", onScroll, true);
    window.addEventListener("blur", onClose);
    window.addEventListener("resize", onClose);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      document.removeEventListener("mousedown", onPress, true);
      document.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("resize", onClose);
      const focused = document.activeElement;
      if (!focused || focused === document.body || inside(focused)) returnFocus(menu.opener);
    };
  }, [layer, menu, onClose]);
}

/** The open menu, portaled to `<body>`; it is cut out of the native PDF preview (ADR 0057). */
function OpenMenuLayer({ menu, onClose }: { menu: OpenMenu; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [placement] = useState(() => placementOf(menu.at));
  const [initial] = useState(() => initialRow(menu));
  useDismiss(ref, menu, onClose);
  useCoversPdf("popover", ref);

  // The focus goes back first, so a sheet the row opens returns it there too.
  const choose = (item: MenuItem) => {
    returnFocus(menu.opener);
    onClose();
    item.run?.();
  };

  return createPortal(
    <div ref={ref} className="menu-layer" onContextMenu={(e) => e.preventDefault()}>
      <MenuPanel entries={menu.entries} label={menu.label} placement={placement} initial={initial} onChoose={choose} />
    </div>,
    document.body,
  );
}
