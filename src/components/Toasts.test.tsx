import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../state/store";
import { Toasts } from "./Toasts";

beforeEach(() => {
  useStore.setState({ toasts: [] });
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

  it("covers the PDF preview while a toast shows and uncovers it once dismissed", async () => {
    useStore.setState({ pdfCovers: 0 });
    useStore.getState().showToast("error", "Could not save");
    render(<Toasts />);
    expect(useStore.getState().pdfCovers).toBe(1);

    await userEvent.click(screen.getByRole("button", { name: "Dismiss" }));

    expect(useStore.getState().pdfCovers).toBe(0);
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
