import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { historyEntry } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { LogPage } from "./LogPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "" });
  mocked.yearLog.mockResolvedValue([]);
});

describe("LogPage", () => {
  it("lists the year's entries with their document names", async () => {
    mocked.yearLog.mockResolvedValue([historyEntry({ text: "Finished", document: "Demo Cloud Ltd" }), historyEntry({ text: "Added", document: "Example Market", documentId: "doc-2" })]);
    renderWithQuery(<LogPage year={2026} />);

    expect(await screen.findByRole("button", { name: "Demo Cloud Ltd" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Example Market" })).toBeTruthy();
    expect(screen.getByText("Finished")).toBeTruthy();
    expect(mocked.yearLog).toHaveBeenCalledWith(2026, "");
  });

  it("asks for the entries matching the header filter", async () => {
    useStore.setState({ filter: "market" });
    renderWithQuery(<LogPage year={2026} />);

    await screen.findByText("Nothing logged yet");
    expect(mocked.yearLog).toHaveBeenCalledWith(2026, "market");
  });

  it("opens the document of an entry when its name is clicked", async () => {
    mocked.yearLog.mockResolvedValue([historyEntry({ document: "Example Market", documentId: "doc-2" })]);
    renderWithQuery(<LogPage year={2026} />);

    await userEvent.click(await screen.findByRole("button", { name: "Example Market" }));

    expect(useStore.getState().route).toEqual({ page: "document", documentId: "doc-2", year: 2026 });
  });

  it("says nothing is logged yet when the year has no entries", async () => {
    renderWithQuery(<LogPage year={2026} />);

    expect(await screen.findByText("Nothing logged yet")).toBeTruthy();
    expect(screen.getByText("Changes to documents paid in 2026 show here.")).toBeTruthy();
  });
});
