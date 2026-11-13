import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { ExcludeSheet } from "./ExcludeSheet";

const mocked = vi.mocked(api, true);
const onClose = vi.fn();

const open = () => renderWithQuery(<ExcludeSheet documentId="doc-1" title="Demo Cloud Ltd" onClose={onClose} />);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], pdfCovers: 0 });
  mocked.documentStep.mockResolvedValue("Excluded Demo Cloud Ltd");
});

describe("ExcludeSheet", () => {
  it("names the document in its title", () => {
    open();

    expect(screen.getByRole("heading", { name: "Exclude Demo Cloud Ltd?" })).toBeTruthy();
  });

  it("keeps Exclude disabled until a reason is typed", async () => {
    open();
    const exclude = screen.getByRole("button", { name: "Exclude" });
    expect(exclude).toBeDisabled();

    await userEvent.type(screen.getByLabelText("Reason"), "   ");
    expect(exclude).toBeDisabled();

    await userEvent.type(screen.getByLabelText("Reason"), "Duplicate");
    expect(exclude).toBeEnabled();
  });

  it("excludes with the reason, toasts the result and closes", async () => {
    open();
    await userEvent.type(screen.getByLabelText("Reason"), "Duplicate of another invoice");

    await userEvent.click(screen.getByRole("button", { name: "Exclude" }));

    expect(mocked.documentStep).toHaveBeenCalledWith("doc-1", { kind: "exclude", reason: "Duplicate of another invoice" });
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["success", "Excluded Demo Cloud Ltd"]]);
  });

  it("shows a field error as an alert and stays open", async () => {
    mocked.documentStep.mockRejectedValue({ kind: "invalid_input", message: "reason: Too long", code: null, field: { path: "reason", message: "Too long" } });
    open();
    await userEvent.type(screen.getByLabelText("Reason"), "Duplicate");

    await userEvent.click(screen.getByRole("button", { name: "Exclude" }));

    expect((await screen.findByRole("alert")).textContent).toBe("Too long");
    expect(onClose).not.toHaveBeenCalled();
  });

  it("shows an error without a field as an alert", async () => {
    mocked.documentStep.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    open();
    await userEvent.type(screen.getByLabelText("Reason"), "Duplicate");

    await userEvent.click(screen.getByRole("button", { name: "Exclude" }));

    expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
  });

  it("closes on Cancel without excluding", async () => {
    open();

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onClose).toHaveBeenCalled();
    expect(mocked.documentStep).not.toHaveBeenCalled();
  });
});
