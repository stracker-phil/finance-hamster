interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** A right-click's point, or under a button. */
export type Placement = { point: { x: number; y: number } } | { below: Box };

interface Size {
  width: number;
  height: number;
}

/** Kept clear of the window's edges. */
const MARGIN = 4;
/** Between a button and its menu. */
const GAP = 4;

/** Where a menu of `size` goes in a window of `view`: at its place when it fits, flipped to the other side when it
 *  doesn't, then shifted inside the window. */
export function place(placement: Placement, size: Size, view: Size): { left: number; top: number } {
  const fitsRight = (left: number) => left + size.width <= view.width - MARGIN;
  const fitsDown = (top: number) => top + size.height <= view.height - MARGIN;
  let left: number;
  let top: number;
  if ("point" in placement) {
    const { x, y } = placement.point;
    left = fitsRight(x) ? x : x - size.width;
    top = fitsDown(y) ? y : y - size.height;
  } else {
    const box = placement.below;
    left = fitsRight(box.left) ? box.left : box.right - size.width;
    top = fitsDown(box.bottom + GAP) ? box.bottom + GAP : box.top - GAP - size.height;
  }
  return { left: clamp(left, size.width, view.width), top: clamp(top, size.height, view.height) };
}

function clamp(start: number, length: number, room: number): number {
  return Math.max(MARGIN, Math.min(start, room - MARGIN - length));
}
