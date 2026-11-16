import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { PdfCoversProbe } from "../test/PdfCoversProbe";
import { Toasts } from "./Toasts";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], pdfCovers: [] });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Toasts", () => {
  it("shows an error toast with a title and its message", () => {
    useStore.getState().showToast("error", "Could not save");

    render(<Toasts />);

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Something went wrong");
    expect(alert.textContent).toContain("Could not save");
  });

  it("removes a toast when it is dismissed", async () => {
    useStore.getState().showToast("error", "Could not save");
    render(<Toasts />);

    await userEvent.click(screen.getByRole("button", { name: "Dismiss" }));

    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows a repeated error once with its count", () => {
    useStore.getState().showToast("error", "Disk full");
    useStore.getState().showToast("error", "Disk full");

    render(<Toasts />);

    expect(screen.getAllByRole("alert")).toHaveLength(1);
    expect(screen.getByText("Something went wrong (×2)")).toBeTruthy();
  });

  it("cuts a hole for the toast in the PDF preview without hiding it, and sends the uncovered state once dismissed", async () => {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ left: 300, top: 400, width: 200, height: 50, right: 500, bottom: 450, x: 300, y: 400, toJSON: () => ({}) });
    useStore.getState().showToast("error", "Could not save");
    render(
      <>
        <PdfCoversProbe />
        <Toasts />
      </>,
    );
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [{ x: 292, y: 392, width: 216, height: 66 }], clickThrough: false }));

    await userEvent.click(screen.getByRole("button", { name: "Dismiss" }));

    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: false }));
  });

  it("shows a toast action by its label and runs it on click", async () => {
    const run = vi.fn();
    useStore.getState().showToast("success", "Subscription deleted", { label: "Undo", run });
    render(<Toasts />);

    await userEvent.click(screen.getByRole("button", { name: "Undo" }));

    expect(run).toHaveBeenCalled();
    expect(screen.queryByText("Subscription deleted")).toBeNull();
  });
});
