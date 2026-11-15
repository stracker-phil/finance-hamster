import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { HOME } from "../state/route";
import { COMMAND_LIST } from "../test/commands";
import { renderWithQuery } from "../test/renderWithQuery";
import { elsewhere } from "../test/year";
import { CommandPalette } from "./CommandPalette";

const mocked = vi.mocked(api, true);

const option = (name: RegExp | string) => screen.findByRole("option", { name });
const search = () => screen.getByRole("textbox", { name: "Search everywhere" });
const optionNames = () => screen.getAllByRole("option").map((o) => o.textContent);
const selected = () => screen.getAllByRole("option").find((o) => o.getAttribute("aria-selected") === "true")?.textContent;

async function openPalette() {
  useStore.setState({ paletteOpen: true });
  renderWithQuery(<CommandPalette />);
  await option(/Go to Today/);
}

describe("CommandPalette", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ route: HOME, back: [], forward: [], paletteOpen: false, paletteQuery: "", toasts: [] });
    mocked.listCommands.mockResolvedValue(COMMAND_LIST);
    mocked.runCommand.mockResolvedValue(undefined);
    mocked.searchElsewhere.mockResolvedValue([]);
  });

  it("shows nothing while closed", () => {
    renderWithQuery(<CommandPalette />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("lists every command but itself, with its shortcut", async () => {
    await openPalette();

    expect(optionNames()).toEqual(["Go to Today⌘1", "Go to Subscriptions⌘2", "Open Settings⌘,", "Import Statement…"]);
  });

  it("selects the first command first", async () => {
    await openPalette();

    expect(selected()).toBe("Go to Today⌘1");
  });

  it("filters the commands as the query is typed", async () => {
    await openPalette();

    await userEvent.type(search(), "sub");

    expect(optionNames()).toEqual(["Go to Subscriptions⌘2"]);
  });

  it("finds a command by its id", async () => {
    await openPalette();

    await userEvent.type(search(), "statement:import");

    expect(optionNames()).toEqual(["Import Statement…"]);
  });

  it("puts a title prefix before a later word match", async () => {
    mocked.listCommands.mockResolvedValue([
      { id: "go:settings", title: "Go to Settings", shortcut: null, custom: false },
      { id: "app:settings", title: "Settings", shortcut: null, custom: false },
    ]);
    useStore.setState({ paletteOpen: true });
    renderWithQuery(<CommandPalette />);
    await option("Settings");

    await userEvent.type(search(), "sett");

    expect(optionNames()).toEqual(["Settings", "Go to Settings"]);
  });

  it("says so when no command matches", async () => {
    await openPalette();

    await userEvent.type(search(), "zzzz");

    expect(screen.getByText("Nothing matches")).toBeTruthy();
    expect(screen.queryAllByRole("option")).toHaveLength(0);
  });

  it("does nothing on Enter when no command matches", async () => {
    await openPalette();
    await userEvent.type(search(), "zzzz{Enter}");

    expect(mocked.runCommand).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("moves the selection with the arrow keys and wraps around", async () => {
    await openPalette();

    await userEvent.keyboard("{ArrowDown}");
    expect(selected()).toBe("Go to Subscriptions⌘2");

    await userEvent.keyboard("{ArrowUp}{ArrowUp}");
    expect(selected()).toBe("Import Statement…");

    await userEvent.keyboard("{ArrowDown}");
    expect(selected()).toBe("Go to Today⌘1");
  });

  it("selects the first match again after the query changes", async () => {
    await openPalette();
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");

    await userEvent.type(search(), "go");

    expect(selected()).toBe("Go to Today⌘1");
  });

  it("runs the selected command on Enter and closes", async () => {
    await openPalette();

    await userEvent.keyboard("{ArrowDown}{Enter}");

    expect(mocked.runCommand).toHaveBeenCalledWith("go:subscriptions");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(useStore.getState().paletteOpen).toBe(false);
  });

  it("runs the best match of the query on Enter", async () => {
    await openPalette();

    await userEvent.type(search(), "settings{Enter}");

    expect(mocked.runCommand).toHaveBeenCalledWith("app:settings");
  });

  it("runs a clicked command and closes", async () => {
    await openPalette();

    await userEvent.click(screen.getByRole("option", { name: /Import Statement/ }));

    expect(mocked.runCommand).toHaveBeenCalledWith("statement:import");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("closes on Escape without running anything", async () => {
    await openPalette();

    await userEvent.keyboard("{Escape}");

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(mocked.runCommand).not.toHaveBeenCalled();
  });

  describe("matches elsewhere", () => {
    const pixelforge = () => option(/Pixelforge Ltd\. invoice/);

    it("opens with the text the header searched everywhere for, and lists its matches", async () => {
      mocked.searchElsewhere.mockResolvedValue([elsewhere()]);
      useStore.setState({ paletteOpen: true, paletteQuery: "pixel" });
      renderWithQuery(<CommandPalette />);

      expect(await pixelforge()).toBeTruthy();
      expect((search() as HTMLInputElement).value).toBe("pixel");
      expect(mocked.searchElsewhere).toHaveBeenCalledWith("pixel");
    });

    it("lists the matches after the commands, each with its place", async () => {
      mocked.searchElsewhere.mockResolvedValue([elsewhere()]);
      await openPalette();

      await userEvent.type(search(), "go");

      await pixelforge();
      expect(optionNames()).toEqual(["Go to Today⌘1", "Go to Subscriptions⌘2", "Pixelforge Ltd. invoiceTax year 2026"]);
    });

    it("shows no matches for an empty query", async () => {
      mocked.searchElsewhere.mockResolvedValue([elsewhere()]);

      await openPalette();

      expect(optionNames()).not.toContain("Pixelforge Ltd. invoiceTax year 2026");
      expect(mocked.searchElsewhere).not.toHaveBeenCalled();
    });

    it("opens the match and closes on Enter", async () => {
      mocked.searchElsewhere.mockResolvedValue([elsewhere()]);
      await openPalette();
      await userEvent.type(search(), "pixelforge");
      await pixelforge();

      await userEvent.keyboard("{Enter}");

      expect(useStore.getState().route).toEqual({ page: "document", documentId: "doc-9", year: 2026 });
      expect(screen.queryByRole("dialog")).toBeNull();
      expect(mocked.runCommand).not.toHaveBeenCalled();
    });

    it("opens a clicked match and closes", async () => {
      mocked.searchElsewhere.mockResolvedValue([elsewhere({ label: "4 lines", place: "Private", target: { page: "account", accountId: "acc-1" } })]);
      await openPalette();
      await userEvent.type(search(), "zzzz");

      await userEvent.click(await option(/4 lines/));

      expect(useStore.getState().route).toEqual({ page: "account", accountId: "acc-1" });
      expect(screen.queryByRole("dialog")).toBeNull();
    });

    it("says nothing matches when neither a command nor a document does", async () => {
      await openPalette();

      await userEvent.type(search(), "zzzz");

      expect(screen.getByText("Nothing matches")).toBeTruthy();
    });
  });

  it("reports a command that fails as an error toast", async () => {
    mocked.runCommand.mockRejectedValue({ kind: "io", message: "Could not open the folder", code: null });
    await openPalette();

    await userEvent.keyboard("{Enter}");

    await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Could not open the folder"]));
  });
});
