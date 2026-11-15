import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { COMMAND_LIST } from "../../test/commands";
import { renderWithQuery } from "../../test/renderWithQuery";
import { SETTINGS_PAGES } from "./index";
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
    mocked.keybindingConflict.mockResolvedValue(null);
    mocked.keybindingSet.mockResolvedValue(undefined);
    mocked.keybindingReassign.mockResolvedValue(undefined);
    mocked.keybindingReset.mockResolvedValue(undefined);
  });

  it("lists each command with its shortcut under In every window, without showing its id", async () => {
    renderWithQuery(<ShortcutsPage />);

    expect(await screen.findByText("Go to Subscriptions")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "In every window" })).toBeTruthy();
    expect(screen.getByText("⌘2")).toBeTruthy();
    expect(screen.getByText("⌘,")).toBeTruthy();
    expect(screen.queryByText("go:subscriptions")).toBeNull();
  });

  it("has a group heading for every shortcuts row of the Settings search index", async () => {
    const page = SETTINGS_PAGES.find((p) => p.id === "shortcuts");
    renderWithQuery(<ShortcutsPage />);
    await screen.findByText("Go to Today");

    expect(page?.rows?.length).toBeGreaterThan(0);
    for (const row of page?.rows ?? []) expect(screen.getByRole("heading", { name: row.title })).toBeTruthy();
  });

  it("shows None for a command without a shortcut", async () => {
    renderWithQuery(<ShortcutsPage />);

    expect(await screen.findByText("Import Statement…")).toBeTruthy();
    expect(screen.getAllByText("None")).toHaveLength(1);
  });

  it("lists the Review page's single keys as read-only", async () => {
    renderWithQuery(<ShortcutsPage />);

    expect(await screen.findByRole("heading", { name: "On the Review page" })).toBeTruthy();
    expect(screen.getByText("ANV / Subscription / Ignore")).toBeTruthy();
    expect(screen.getByText(/^A\s+S\s+I$/)).toBeTruthy();
    expect(screen.getByText("Skip")).toBeTruthy();
    expect(screen.getByText("J")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Shortcut for Skip" })).toBeNull();
  });

  it("lists no Year Check page keys any more", async () => {
    renderWithQuery(<ShortcutsPage />);
    await screen.findByRole("heading", { name: "On the Review page" });

    expect(screen.queryByRole("heading", { name: "On the Year Check page" })).toBeNull();
    expect(screen.queryByText("Yes / No / Later")).toBeNull();
  });

  describe("search", () => {
    it.each([
      ["title", "subscriptions", "Go to Subscriptions"],
      ["id", "statement:import", "Import Statement…"],
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

      expect(mocked.keybindingSet).toHaveBeenCalledWith("go:today", "CmdOrCtrl+Shift+K");
      expect(shortcutButton("Go to Today").getAttribute("aria-pressed")).toBe("false");
    });

    it("reads a digit from its key, not from the character the modifiers produce", async () => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Alt>}3{/Alt}");

      expect(mocked.keybindingSet).toHaveBeenCalledWith("go:today", "Alt+3");
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

      expect(mocked.keybindingSet).toHaveBeenCalledWith("go:today", null);
    });

    it("cancels on Escape without changing anything", async () => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Escape}");

      expect(mocked.keybindingSet).not.toHaveBeenCalled();
      expect(shortcutButton("Go to Today").getAttribute("aria-pressed")).toBe("false");
    });

    it("shows the message of a rejected shortcut in its row", async () => {
      mocked.keybindingSet.mockRejectedValue({ kind: "invalid_input", message: "Shortcut needs a modifier", code: null });
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Meta>}k{/Meta}");

      expect((await screen.findByRole("alert")).textContent).toBe("Shortcut needs a modifier");
    });

    it("shows the message when the conflict check is rejected, and sets nothing", async () => {
      mocked.keybindingConflict.mockRejectedValue({ kind: "io", message: "Keybindings unavailable", code: null });
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");

      await userEvent.keyboard("{Meta>}k{/Meta}");

      expect((await screen.findByRole("alert")).textContent).toBe("Keybindings unavailable");
      expect(mocked.keybindingSet).not.toHaveBeenCalled();
    });
  });

  describe("a shortcut another command already holds", () => {
    beforeEach(() => {
      mocked.keybindingConflict.mockResolvedValue({ shortcut: "⌘K", holder: "Command Palette…" });
    });

    const recordTaken = async () => {
      renderWithQuery(<ShortcutsPage />);
      await startRecording("Go to Today");
      await userEvent.keyboard("{Meta>}k{/Meta}");
      return screen.findByRole("alertdialog", { name: "Reassign ⌘K?" });
    };

    it("asks before reassigning and names the command that loses it", async () => {
      const dialog = await recordTaken();

      expect(within(dialog).getByText(/Command Palette…/)).toBeTruthy();
      expect(mocked.keybindingConflict).toHaveBeenCalledWith("go:today", "CmdOrCtrl+K");
      expect(mocked.keybindingSet).not.toHaveBeenCalled();
      expect(mocked.keybindingReassign).not.toHaveBeenCalled();
    });

    it("moves the shortcut to the command when Reassign is chosen", async () => {
      const dialog = await recordTaken();

      await userEvent.click(within(dialog).getByRole("button", { name: "Reassign" }));

      expect(mocked.keybindingReassign).toHaveBeenCalledWith("go:today", "CmdOrCtrl+K");
      expect(mocked.keybindingSet).not.toHaveBeenCalled();
      await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    });

    it("changes nothing when Cancel is chosen", async () => {
      const dialog = await recordTaken();

      await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("alertdialog")).toBeNull();
      expect(mocked.keybindingReassign).not.toHaveBeenCalled();
      expect(mocked.keybindingSet).not.toHaveBeenCalled();
    });

    it("changes nothing when Escape is pressed", async () => {
      await recordTaken();

      await userEvent.keyboard("{Escape}");

      await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
      expect(mocked.keybindingReassign).not.toHaveBeenCalled();
      expect(mocked.keybindingSet).not.toHaveBeenCalled();
    });

    it("shows the message when the reassignment is rejected", async () => {
      mocked.keybindingReassign.mockRejectedValue({ kind: "io", message: "Keybindings file is read-only", code: null });
      const dialog = await recordTaken();

      await userEvent.click(within(dialog).getByRole("button", { name: "Reassign" }));

      expect((await screen.findByRole("alert")).textContent).toBe("Keybindings file is read-only");
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

      expect(mocked.keybindingReset).toHaveBeenCalledWith("go:today");
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
