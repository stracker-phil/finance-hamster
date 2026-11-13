import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME, type Route } from "../state/route";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { elsewhere } from "../test/year";
import { Header } from "./Header";

const mocked = vi.mocked(api, true);

const YEAR: Route = { page: "year", year: 2026 };

const field = (name: string) => screen.getByRole("textbox", { name });

function renderHeader(route: Route = YEAR) {
  renderWithQuery(<Header route={route} title={{ title: "Header" }} />);
}

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", paletteOpen: false, paletteQuery: "", toasts: [], pdfCovers: 0 });
  mocked.searchElsewhere.mockResolvedValue([]);
});

describe("Header filter field", () => {
  describe("matches elsewhere", () => {
    it("lists each match with its place and the Search everywhere entry while typing", async () => {
      mocked.searchElsewhere.mockResolvedValue([elsewhere(), elsewhere({ label: "4 lines", place: "Private", target: { page: "account", accountId: "acc-1" } })]);
      renderHeader();

      await userEvent.type(field("Filter this tax year"), "pixel");

      const menu = await screen.findByRole("menu", { name: "Matches elsewhere" });
      expect(await screen.findByRole("menuitem", { name: /Pixelforge Ltd\. invoice/ })).toBeTruthy();
      expect(screen.getByRole("menuitem", { name: /4 lines/ })).toBeTruthy();
      expect(screen.getByText("Tax year 2026")).toBeTruthy();
      expect(screen.getByRole("menuitem", { name: /Search everywhere/ })).toBeTruthy();
      expect(menu.textContent).toContain("⌘↩");
      expect(mocked.searchElsewhere).toHaveBeenLastCalledWith("pixel");
    });

    it("offers only Search everywhere when nothing matches", async () => {
      renderHeader();

      await userEvent.type(field("Filter this tax year"), "zzz");

      expect(await screen.findByRole("menuitem", { name: /Search everywhere/ })).toBeTruthy();
      expect(screen.getAllByRole("menuitem")).toHaveLength(1);
      expect(screen.queryByText("Matches elsewhere")).toBeNull();
    });

    it("shows no menu while the field is empty", async () => {
      renderHeader();

      await userEvent.click(field("Filter this tax year"));

      expect(screen.queryByRole("menu")).toBeNull();
    });

    it("closes the menu when the field loses focus", async () => {
      renderHeader();
      await userEvent.type(field("Filter this tax year"), "pixel");
      await screen.findByRole("menu", { name: "Matches elsewhere" });

      await userEvent.tab();

      expect(screen.queryByRole("menu", { name: "Matches elsewhere" })).toBeNull();
    });

    it.each([
      ["a document", elsewhere({ label: "Pixelforge Ltd. invoice" }), { page: "document", documentId: "doc-9", year: 2026 }],
      ["the documents list", elsewhere({ label: "3 documents", place: "All documents", target: { page: "documents" } }), { page: "documents" }],
      ["an account", elsewhere({ label: "4 lines", place: "Private", target: { page: "account", accountId: "acc-1" } }), { page: "account", accountId: "acc-1" }],
    ])("opens %s when its match is clicked", async (_, match, route) => {
      mocked.searchElsewhere.mockResolvedValue([match]);
      renderHeader();
      await userEvent.type(field("Filter this tax year"), "pixel");

      await userEvent.click(await screen.findByRole("menuitem", { name: new RegExp(match.label.replace(".", "\\.")) }));

      expect(useStore.getState().route).toEqual(route);
    });

    it("opens the palette with the text when Search everywhere is clicked", async () => {
      renderHeader();
      await userEvent.type(field("Filter this tax year"), "pixel");

      await userEvent.click(await screen.findByRole("menuitem", { name: /Search everywhere/ }));

      expect(useStore.getState().paletteOpen).toBe(true);
      expect(useStore.getState().paletteQuery).toBe("pixel");
    });
  });

  describe("keyboard", () => {
    it("opens the palette with the text on ⌘Enter", async () => {
      renderHeader();
      await userEvent.type(field("Filter this tax year"), "pixel");

      await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

      expect(useStore.getState().paletteOpen).toBe(true);
      expect(useStore.getState().paletteQuery).toBe("pixel");
    });

    it("keeps Enter on a page with its own filter to filtering", async () => {
      renderHeader();

      await userEvent.type(field("Filter this tax year"), "pixel{Enter}");

      expect(useStore.getState().paletteOpen).toBe(false);
    });

    it("opens the palette on plain Enter where the field searches everywhere", async () => {
      renderHeader({ page: "today" });

      await userEvent.type(field("Search everywhere"), "pixel{Enter}");

      expect(useStore.getState().paletteOpen).toBe(true);
      expect(useStore.getState().paletteQuery).toBe("pixel");
    });

    it("does not open the palette on Enter without text", async () => {
      renderHeader({ page: "today" });

      await userEvent.type(field("Search everywhere"), "  {Enter}");

      expect(useStore.getState().paletteOpen).toBe(false);
    });

    it("clears the filter on Escape", async () => {
      renderHeader();
      await userEvent.type(field("Filter this tax year"), "pixel");

      await userEvent.keyboard("{Escape}");

      expect(useStore.getState().filter).toBe("");
      expect((field("Filter this tax year") as HTMLInputElement).value).toBe("");
    });
  });
});
