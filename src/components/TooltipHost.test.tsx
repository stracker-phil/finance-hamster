import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { PdfCoversProbe } from "../test/PdfCoversProbe";
import { TooltipHost } from "./TooltipHost";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ pdfCovers: [] });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("TooltipHost", () => {
  it("shows the title of a hovered control as a tooltip", async () => {
    render(
      <>
        <TooltipHost />
        <button title="Back (⌘[)">←</button>
      </>,
    );

    await userEvent.hover(screen.getByRole("button"));

    expect((await screen.findByRole("tooltip")).textContent).toBe("Back (⌘[)");
  });

  it("cuts a hole for the tooltip in the PDF preview without hiding it, and sends the uncovered state when it hides", async () => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ left: 20, top: 30, width: 90, height: 24, right: 110, bottom: 54, x: 20, y: 30, toJSON: () => ({}) });
    render(
      <>
        <PdfCoversProbe />
        <TooltipHost />
        <button title="Back (⌘[)">←</button>
      </>,
    );
    expect(mocked.pdfViewCover).not.toHaveBeenCalled();

    await userEvent.hover(screen.getByRole("button"));
    await screen.findByRole("tooltip");
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [{ frame: { x: 19, y: 29, width: 92, height: 26 }, radius: 1 }], clickThrough: false }));

    await userEvent.keyboard("a");

    expect(screen.queryByRole("tooltip")).toBeNull();
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: false }));
  });
});
