import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { categoryChoice } from "../test/year";
import { CategoriesSheet, CategorySwitches } from "./CategorySwitches";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => ({ kind: t.kind, message: t.message }));

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
  mocked.yearCategories.mockResolvedValue([
    categoryChoice(),
    categoryChoice({ key: "union", label: "Union dues", examples: null, shown: false }),
  ]);
  mocked.yearSetCategoryShown.mockResolvedValue("Hide category Union dues");
});

describe("CategorySwitches", () => {
  it("lists a switch per category of the year, on for shown ones, with the examples as description", async () => {
    renderWithQuery(<CategorySwitches year={2026} />);

    const group = within(await screen.findByRole("region", { name: "Categories" }));
    expect((await group.findByRole<HTMLInputElement>("switch", { name: "Work books" })).checked).toBe(true);
    expect(group.getByRole<HTMLInputElement>("switch", { name: "Union dues" }).checked).toBe(false);
    expect(group.getByText("Books, journals")).toBeTruthy();
    expect(mocked.yearCategories).toHaveBeenCalledWith(2026);
  });

  it("shows no switch until the categories are loaded", () => {
    mocked.yearCategories.mockReturnValue(new Promise(() => {}));
    renderWithQuery(<CategorySwitches year={2026} />);

    expect(screen.queryByRole("switch")).toBeNull();
  });

  describe("toggling", () => {
    it("hides a shown category and shows its label with Undo", async () => {
      mocked.yearSetCategoryShown.mockResolvedValue("Hide category Work books");
      renderWithQuery(<CategorySwitches year={2026} />);

      await userEvent.click(await screen.findByRole("switch", { name: "Work books" }));

      expect(mocked.yearSetCategoryShown).toHaveBeenCalledWith(2026, "720", false);
      await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Hide category Work books" }]));
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });

    it("shows a hidden category again", async () => {
      renderWithQuery(<CategorySwitches year={2026} />);

      await userEvent.click(await screen.findByRole("switch", { name: "Union dues" }));

      expect(mocked.yearSetCategoryShown).toHaveBeenCalledWith(2026, "union", true);
    });

    it("shows no toast when nothing changed", async () => {
      mocked.yearSetCategoryShown.mockResolvedValue(null);
      renderWithQuery(<CategorySwitches year={2026} />);

      await userEvent.click(await screen.findByRole("switch", { name: "Union dues" }));

      await waitFor(() => expect(mocked.yearSetCategoryShown).toHaveBeenCalled());
      expect(toasts()).toEqual([]);
    });

    it("reports a rejected change as an error toast", async () => {
      mocked.yearSetCategoryShown.mockRejectedValue({ kind: "conflict", message: "Year is closed", code: null });
      renderWithQuery(<CategorySwitches year={2026} />);

      await userEvent.click(await screen.findByRole("switch", { name: "Union dues" }));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "Year is closed" }]));
    });
  });
});

describe("CategoriesSheet", () => {
  it("is titled with the year, explains itself and lists the switches", async () => {
    renderWithQuery(<CategoriesSheet year={2026} onClose={vi.fn()} />);

    const sheet = within(await screen.findByRole("dialog", { name: "Categories for 2026" }));
    expect(sheet.getByText(/Hide the categories you never use/)).toBeTruthy();
    expect(await sheet.findByRole("switch", { name: "Work books" })).toBeTruthy();
  });

  it("calls onClose when Done is clicked", async () => {
    const onClose = vi.fn();
    renderWithQuery(<CategoriesSheet year={2026} onClose={onClose} />);

    await userEvent.click(await screen.findByRole("button", { name: "Done" }));

    expect(onClose).toHaveBeenCalled();
  });
});
