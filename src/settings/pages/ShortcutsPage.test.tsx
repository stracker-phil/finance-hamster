import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { COMMAND_LIST } from "../../test/commands";
import { renderWithQuery } from "../../test/renderWithQuery";
import { ShortcutsPage } from "./ShortcutsPage";

const mocked = vi.mocked(api, true);

const shortcutButton = (title: string) => screen.getByRole("button", { name: `Shortcut for ${title}` });

const startRecording = async (title: string) => {
  await screen.findByText(title);
  await userEvent.click(shortcutButton(title));
};

describe("ShortcutsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.listCommands.mockResolvedValue(COMMAND_LIST);
    mocked.keybindingSet.mockResolvedValue(undefined);
    mocked.keybindingReset.mockResolvedValue(undefined);
  });

  it("lists each command with its id and shortcut", async () => {
    renderWithQuery(<ShortcutsPage />);

    expect(await screen.findByText("Go to Subscriptions")).toBeTruthy();
    expect(screen.getByText("go.subscriptions")).toBeTruthy();
    expect(screen.getByText("⌘2")).toBeTruthy();
    expect(screen.getByText("⌘,")).toBeTruthy();
  });

  it("shows None for a command without a shortcut", async () => {
    renderWithQuery(<ShortcutsPage />);

    expect(await screen.findByText("Import Statement…")).toBeTruthy();
    expect(screen.getAllByText("None")).toHaveLength(1);
  });

  it("lists Review's single keys as read-only", async () => {
    renderWithQuery(<ShortcutsPage />);

    expect(await screen.findByText("ANV / Subscription / Ignore")).toBeTruthy();
    expect(screen.getByText(/^A\s+S\s+I$/)).toBeTruthy();
    expect(screen.getByText("Skip")).toBeTruthy();
    expect(screen.getByText("J")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Shortcut for Skip" })).toBeNull();
  });

  describe("search", () => {
    it.each([
      ["title", "subscriptions", "Go to Subscriptions"],
      ["id", "import.statement", "Import Statement…"],
      ["shortcut", "⌘2", "Go to Subscriptions"],
    ])("filters the commands by their %s", async (_by, query, kept) => {
      renderWithQuery(<ShortcutsPage />);
      await screen.findByText("Go to Today");

      await userEvent.type(screen.getByRole("searchbox", { name: "Search shortcuts" }), query);

      expect(screen.getByText(kept)).toBeTruthy();
      expect(screen.queryByText("Go to Today")).toBeNull();
      expect(screen.queryByText("Command Palette")).toBeNull();
    });

    it("says no command matches when nothing fits", async () => {
      renderWithQuery(<ShortcutsPage />);
      await screen.findByText("Go to Today");

      await userEvent.type(screen.getByRole("searchbox", { name: "Search shortcuts" }), "zzz");

      expect(screen.getByText("No command matches “zzz”")).toBeTruthy();
      expect(screen.queryByText("Go to Today")).toBeNull();
    });
  });

  describe("recording a shortcut", () => {
    it("waits for a key after the shortcut button is clicked, and stops when it is clicked again", async () => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      expect(shortcutButton("Go to Today").getAttribute("aria-pressed")).toBe("true");
      expect(screen.getByText("Type shortcut…")).toBeTruthy();

      await userEvent.click(shortcutButton("Go to Today"));

      expect(shortcutButton("Go to Today").getAttribute("aria-pressed")).toBe("false");
      expect(screen.getByText("⌘1")).toBeTruthy();
    });

    it("sends the pressed shortcut as an accelerator", async () => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Meta>}{Shift>}k{/Shift}{/Meta}");

      expect(mocked.keybindingSet).toHaveBeenCalledWith("go.today", "CmdOrCtrl+Shift+K");
      expect(shortcutButton("Go to Today").getAttribute("aria-pressed")).toBe("false");
    });

    it("reads a digit from its key, not from the character the modifiers produce", async () => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Alt>}3{/Alt}");

      expect(mocked.keybindingSet).toHaveBeenCalledWith("go.today", "Alt+3");
    });

    it("keeps waiting while only a modifier is held", async () => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Meta>}");

      expect(mocked.keybindingSet).not.toHaveBeenCalled();
      expect(shortcutButton("Go to Today").getAttribute("aria-pressed")).toBe("true");
    });

    it.each(["{Backspace}", "{Delete}"])("removes the shortcut on %s", async (key) => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard(key);

      expect(mocked.keybindingSet).toHaveBeenCalledWith("go.today", null);
    });

    it("cancels on Escape without changing anything", async () => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Escape}");

      expect(mocked.keybindingSet).not.toHaveBeenCalled();
      expect(shortcutButton("Go to Today").getAttribute("aria-pressed")).toBe("false");
    });

    it("shows the message of a rejected shortcut in its row", async () => {
      mocked.keybindingSet.mockRejectedValue({ kind: "invalid_input", message: "⌘K is already used by Command Palette", code: null });
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Meta>}k{/Meta}");

      expect((await screen.findByRole("alert")).textContent).toBe("⌘K is already used by Command Palette");
    });
  });

  describe("Reset", () => {
    it("is offered only for commands with a custom shortcut", async () => {
      mocked.listCommands.mockResolvedValue([...COMMAND_LIST.slice(0, 2), { ...COMMAND_LIST[2], custom: true }]);
      renderWithQuery(<ShortcutsPage />);
      await screen.findByText("Open Settings");

      expect(screen.getAllByRole("button", { name: "Reset" })).toHaveLength(1);
    });

    it("is not offered when no shortcut was changed", async () => {
      renderWithQuery(<ShortcutsPage />);
      await screen.findByText("Open Settings");

      expect(screen.queryByRole("button", { name: "Reset" })).toBeNull();
    });

    it("restores the default shortcut of that command", async () => {
      mocked.listCommands.mockResolvedValue([{ ...COMMAND_LIST[0], custom: true }]);
      renderWithQuery(<ShortcutsPage />);

      await userEvent.click(await screen.findByRole("button", { name: "Reset" }));

      expect(mocked.keybindingReset).toHaveBeenCalledWith("go.today");
    });

    it("shows the message when the reset is rejected", async () => {
      mocked.listCommands.mockResolvedValue([{ ...COMMAND_LIST[0], custom: true }]);
      mocked.keybindingReset.mockRejectedValue({ kind: "io", message: "Keybindings file is read-only", code: null });
      renderWithQuery(<ShortcutsPage />);

      await userEvent.click(await screen.findByRole("button", { name: "Reset" }));

      expect((await screen.findByRole("alert")).textContent).toBe("Keybindings file is read-only");
    });
  });
});
