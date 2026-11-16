import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore, type PdfCover } from "../state/store";
import { coversOf, useCoversPdf, usePdfCovers } from "./pdfCover";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ pdfCovers: [] });
});

const covers = () => useStore.getState().pdfCovers;

function boxed(left: number, top: number, width: number, height: number, display?: string, radius?: string) {
  const element = document.createElement("div");
  if (display) element.style.display = display;
  if (radius) element.style.borderTopLeftRadius = radius;
  document.body.appendChild(element);
  vi.spyOn(element, "getBoundingClientRect").mockReturnValue({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) });
  return element;
}

function Overlay({ kind = "sheet", active }: { kind?: PdfCover["kind"]; active?: boolean }) {
  useCoversPdf(kind, undefined, active);
  return null;
}

function Overlays({ kinds }: { kinds: PdfCover["kind"][] }) {
  usePdfCovers();
  return (
    <>
      {kinds.map((kind, i) => (
        <Overlay key={i} kind={kind} />
      ))}
    </>
  );
}

describe("useCoversPdf", () => {
  it("registers the overlay with its kind while it is mounted", () => {
    const { unmount } = render(<Overlay kind="popover" />);
    expect(covers().map((c) => c.kind)).toEqual(["popover"]);

    unmount();

    expect(covers()).toEqual([]);
  });

  it("registers the overlay only while it is active", () => {
    const { rerender } = render(<Overlay active={false} />);
    expect(covers()).toEqual([]);

    rerender(<Overlay active />);
    expect(covers()).toHaveLength(1);

    rerender(<Overlay active={false} />);
    expect(covers()).toEqual([]);
  });

  it("registers every open overlay", () => {
    render(
      <>
        <Overlay kind="sheet" />
        <Overlay kind="float" />
      </>,
    );

    expect(covers().map((c) => c.kind)).toEqual(["sheet", "float"]);
  });
});

describe("coversOf", () => {
  it("asks for nothing when no overlay is open", () => {
    expect(coversOf([])).toEqual({ hidden: false, holes: [], clickThrough: false });
  });

  it("hides the preview for a sheet and cuts no hole for it", () => {
    expect(coversOf([{ kind: "sheet", element: boxed(10, 10, 100, 100) }])).toEqual({ hidden: true, holes: [], clickThrough: false });
  });

  it("cuts a popover out, grown by 1px on every side, and lets clicks through", () => {
    const result = coversOf([{ kind: "popover", element: boxed(100, 50, 200, 80) }]);

    expect(result).toEqual({ hidden: false, holes: [{ frame: { x: 99, y: 49, width: 202, height: 82 }, radius: 1 }], clickThrough: true });
  });

  it("cuts a float out without letting clicks through", () => {
    const result = coversOf([{ kind: "float", element: boxed(0, 0, 40, 20) }]);

    expect(result).toEqual({ hidden: false, holes: [{ frame: { x: -1, y: -1, width: 42, height: 22 }, radius: 1 }], clickThrough: false });
  });

  it("rounds a hole one pixel wider than the corner radius of its element", () => {
    const result = coversOf([{ kind: "popover", element: boxed(10, 10, 100, 40, undefined, "8px") }]);

    expect(result.holes).toEqual([{ frame: { x: 9, y: 9, width: 102, height: 42 }, radius: 9 }]);
  });

  it("combines a sheet, a popover and a float", () => {
    const result = coversOf([
      { kind: "sheet", element: null },
      { kind: "popover", element: boxed(0, 0, 10, 10) },
      { kind: "float", element: boxed(50, 50, 10, 10) },
    ]);

    expect(result.hidden).toBe(true);
    expect(result.clickThrough).toBe(true);
    expect(result.holes).toHaveLength(2);
  });

  it.each([
    ["no width", boxed(0, 0, 0, 30)],
    ["no height", boxed(0, 0, 30, 0)],
  ])("leaves out an element with %s", (_name, element) => {
    expect(coversOf([{ kind: "float", element }]).holes).toEqual([]);
  });

  it("leaves out an overlay that has no element", () => {
    expect(coversOf([{ kind: "popover", element: null }])).toEqual({ hidden: false, holes: [], clickThrough: true });
  });

  it("uses the boxes of the children for an element with display: contents", () => {
    const wrapper = boxed(0, 0, 0, 0, "contents");
    wrapper.append(boxed(20, 20, 30, 30), boxed(100, 100, 40, 40), boxed(0, 0, 0, 0));

    const result = coversOf([{ kind: "float", element: wrapper }]);

    expect(result.holes).toEqual([
      { frame: { x: 19, y: 19, width: 32, height: 32 }, radius: 1 },
      { frame: { x: 99, y: 99, width: 42, height: 42 }, radius: 1 },
    ]);
  });
});

describe("usePdfCovers", () => {
  it("leaves the preview alone while no overlay is open", () => {
    render(<Overlays kinds={[]} />);

    expect(mocked.pdfViewCover).not.toHaveBeenCalled();
  });

  it("hides the preview when a sheet opens and sends the uncovered state when it closes", async () => {
    const { rerender } = render(<Overlays kinds={[]} />);

    rerender(<Overlays kinds={["sheet"]} />);
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: true, holes: [], clickThrough: false }));

    rerender(<Overlays kinds={[]} />);
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: false }));
  });

  it("lets clicks through while a popover is open", async () => {
    render(<Overlays kinds={["popover"]} />);

    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: true }));
  });

  it("does not resend a value that did not change", async () => {
    const { rerender } = render(<Overlays kinds={["sheet"]} />);
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenCalledTimes(1));

    rerender(<Overlays kinds={["sheet", "sheet"]} />);
    rerender(<Overlays kinds={["sheet"]} />);

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(mocked.pdfViewCover).toHaveBeenCalledTimes(1);
  });

  it("logs the error when the preview cannot be covered", async () => {
    mocked.pdfViewCover.mockRejectedValue({ kind: "internal", message: "No view", code: null });

    render(<Overlays kinds={["sheet"]} />);

    await waitFor(() => expect(mocked.logFrontendError).toHaveBeenCalledWith("PDF preview: No view"));
  });
});
