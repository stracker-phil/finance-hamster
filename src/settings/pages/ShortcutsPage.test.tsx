import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { COMMAND_LIST } from "../../test/commands";
import { renderWithQuery } from "../../test/renderWithQuery";
import { ShortcutsPage } from "./ShortcutsPage";

const mocked = vi.mocked(api, true);

describe("ShortcutsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.listCommands.mockResolvedValue(COMMAND_LIST);
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
});
