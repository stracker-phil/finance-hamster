import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { archivedDocument } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { settings } from "../test/settings";
import { ArchivePage } from "./ArchivePage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], pdfCovers: 0 });
  mocked.getSettings.mockResolvedValue(settings());
  mocked.archiveList.mockResolvedValue([archivedDocument()]);
  mocked.documentStep.mockResolvedValue("Restored Sample Books");
});

describe("ArchivePage", () => {
  it("lists each excluded document by title with its reason and exclusion date, the file only as tooltip", async () => {
    renderWithQuery(<ArchivePage year={2026} />);

    const title = await screen.findByText("Sample Books");
    expect(title.getAttribute("title")).toBe("Documents/2026/sample-books.pdf");
    expect(screen.queryByText("Documents/2026/sample-books.pdf")).toBeNull();
    expect(screen.getByText("Duplicate of another invoice · Excluded 2 Nov 2026")).toBeTruthy();
    expect(mocked.archiveList).toHaveBeenCalledWith(2026);
  });

  it("names the configured days in the intro", async () => {
    mocked.getSettings.mockResolvedValue(settings({ archiveDays: 14 }));
    renderWithQuery(<ArchivePage year={2026} />);

    expect(await screen.findByText(/Deleting is possible 14 days after excluding/)).toBeTruthy();
  });

  it("says day, not days, for a single day", async () => {
    mocked.getSettings.mockResolvedValue(settings({ archiveDays: 1 }));
    renderWithQuery(<ArchivePage year={2026} />);

    expect(await screen.findByText(/Deleting is possible 1 day after excluding/)).toBeTruthy();
  });

  it("restores a document", async () => {
    renderWithQuery(<ArchivePage year={2026} />);

    await userEvent.click(await screen.findByRole("button", { name: "Restore" }));

    expect(mocked.documentStep).toHaveBeenCalledWith("doc-3", { kind: "restore" });
  });

  it("disables Delete… and says from when it is possible while the wait is not over", async () => {
    mocked.archiveList.mockResolvedValue([archivedDocument({ deletable: false, deletableNote: "Can be deleted from 9 Nov 2026" })]);
    renderWithQuery(<ArchivePage year={2026} />);

    expect(await screen.findByText("Can be deleted from 9 Nov 2026")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Delete…" })).toBeDisabled();
  });

  it("opens the Delete sheet for a deletable document and closes it on Cancel", async () => {
    renderWithQuery(<ArchivePage year={2026} />);

    await userEvent.click(await screen.findByRole("button", { name: "Delete…" }));
    expect(screen.getByRole("alertdialog", { name: "Delete Sample Books?" })).toBeTruthy();

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("says nothing is excluded when the year has no excluded documents", async () => {
    mocked.archiveList.mockResolvedValue([]);
    renderWithQuery(<ArchivePage year={2026} />);

    expect(await screen.findByText("Nothing excluded")).toBeTruthy();
    expect(screen.getByText("Excluded documents of 2026 show here.")).toBeTruthy();
  });
});
