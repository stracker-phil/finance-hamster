import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { historyEntry } from "../test/documents";
import { HistoryList } from "./HistoryList";

const entries = [
  historyEntry({ text: "Finished", when: "13 Nov 2026, 15:00", document: "Demo Cloud Ltd", documentId: "doc-1" }),
  historyEntry({ text: "Added", when: "12 Nov 2026, 09:10", document: "Example Market", documentId: "doc-2" }),
];

describe("HistoryList", () => {
  it("lists each entry's text and when, in the given order", () => {
    render(<HistoryList entries={entries} />);

    const items = within(screen.getByRole("list", { name: "Changes" })).getAllByRole("listitem");
    expect(items.map((li) => li.textContent)).toEqual(["Finished13 Nov 2026, 15:00", "Added12 Nov 2026, 09:10"]);
  });

  it("names no document by default", () => {
    render(<HistoryList entries={entries} />);

    expect(screen.queryByText("Demo Cloud Ltd")).toBeNull();
  });

  it("names the document of each entry with withDocument", () => {
    render(<HistoryList entries={entries} withDocument />);

    expect(screen.getByText("Demo Cloud Ltd")).toBeTruthy();
    expect(screen.getByText("Example Market")).toBeTruthy();
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("opens the document of an entry when its name is clicked", async () => {
    const onOpen = vi.fn();
    render(<HistoryList entries={entries} withDocument onOpen={onOpen} />);

    await userEvent.click(screen.getByRole("button", { name: "Example Market" }));

    expect(onOpen).toHaveBeenCalledWith("doc-2");
  });
});
