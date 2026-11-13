import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { archivedDocument } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { DeleteSheet } from "./DeleteSheet";

const mocked = vi.mocked(api, true);
const onClose = vi.fn();

const open = () => renderWithQuery(<DeleteSheet doc={archivedDocument()} onClose={onClose} />);
const confirmDelete = async () => {
  await userEvent.type(screen.getByLabelText("Reason"), "Scanned twice");
  await userEvent.click(screen.getByRole("checkbox", { name: "I understand this can't be undone" }));
  await userEvent.click(screen.getByRole("button", { name: "Delete" }));
};

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], pdfCovers: 0 });
  mocked.documentDelete.mockResolvedValue("Deleted Sample Books");
});

describe("DeleteSheet", () => {
  it("names the document and the file that will be removed", () => {
    open();

    expect(screen.getByRole("alertdialog", { name: "Delete Sample Books?" })).toBeTruthy();
    expect(screen.getByText("Documents/2026/sample-books.pdf")).toBeTruthy();
  });

  it("keeps Delete disabled until there is a reason and the understanding is checked", async () => {
    open();
    const del = screen.getByRole("button", { name: "Delete" });
    expect(del).toBeDisabled();

    await userEvent.type(screen.getByLabelText("Reason"), "Scanned twice");
    expect(del).toBeDisabled();

    await userEvent.click(screen.getByRole("checkbox", { name: "I understand this can't be undone" }));
    expect(del).toBeEnabled();
  });

  it("stays disabled when only the checkbox is checked", async () => {
    open();

    await userEvent.click(screen.getByRole("checkbox", { name: "I understand this can't be undone" }));

    expect(screen.getByRole("button", { name: "Delete" })).toBeDisabled();
  });

  it("deletes with the reason and the understanding, toasts the result and closes", async () => {
    open();

    await confirmDelete();

    expect(mocked.documentDelete).toHaveBeenCalledWith("doc-3", "Scanned twice", true);
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["success", "Deleted Sample Books"]]);
  });

  it("shows the error as an alert and stays open", async () => {
    mocked.documentDelete.mockRejectedValue({ kind: "invalid_input", message: "Too early to delete", code: null });
    open();

    await confirmDelete();

    expect((await screen.findByRole("alert")).textContent).toBe("Too early to delete");
    expect(onClose).not.toHaveBeenCalled();
  });

  it("closes on Cancel without deleting", async () => {
    open();

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onClose).toHaveBeenCalled();
    expect(mocked.documentDelete).not.toHaveBeenCalled();
  });
});
