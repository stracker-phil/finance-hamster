import { act, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { MenuHost } from "../components/menu";
import { HOME, type Route } from "../state/route";
import { useStore } from "../state/store";
import { addPreview } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { elsewhere } from "../test/year";
import { Header } from "./Header";

const mocked = vi.mocked(api, true);

const YEAR: Route = { page: "year", year: 2026 };

const field = (name: string) => screen.getByRole("textbox", { name });

function renderHeader(route: Route = YEAR) {
  renderWithQuery(
    <>
      <Header route={route} title={{ title: "Header" }} />
      <MenuHost />
    </>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", paletteOpen: false, paletteQuery: "", toasts: [], pdfCovers: 0, menu: null, subscriptionSheet: null });
  mocked.searchElsewhere.mockResolvedValue([]);
  mocked.listCommands.mockResolvedValue([]);
  useStore.setState({ addPreview: null, addKind: "invoice" });
});

describe("Header New menu", () => {
  const newButton = () => screen.getByRole("button", { name: /New/ });
  const openNew = async () => {
    await act(() => new Promise((resolve) => setTimeout(resolve, 0)));
    await userEvent.click(newButton());
  };

  it("lists the creation commands with the shortcut the command has now", async () => {
    mocked.listCommands.mockResolvedValue([
      { id: "statement:import", title: "Import Statement…", shortcut: "⌘I", custom: false },
      { id: "document:add", title: "Add Invoice PDF…", shortcut: "⌥⌘O", custom: true },
      { id: "payslip:add", title: "Add Payslip…", shortcut: "⌥⌘P", custom: true },
      { id: "other:add", title: "Add Other Document…", shortcut: null, custom: false },
      { id: "subscription:new", title: "New Subscription", shortcut: null, custom: false },
    ]);
    renderHeader();

    await openNew();

    const menu = screen.getByRole("menu", { name: "New" });
    expect(within(menu).getAllByRole("menuitem").map((i) => i.textContent)).toEqual(["Import Statement…⌘I", "Add Invoice…⌥⌘O", "Add Payslip…⌥⌘P", "Add Other Document…", "New Subscription"]);
    expect(newButton().getAttribute("aria-expanded")).toBe("true");
  });

  it("shows no hints while the commands are unknown", async () => {
    renderHeader();

    await openNew();

    expect(within(screen.getByRole("menu", { name: "New" })).getAllByRole("menuitem").map((i) => i.textContent)).toEqual(["Import Statement…", "Add Invoice…", "Add Payslip…", "Add Other Document…", "New Subscription"]);
  });

  it.each([
    ["Add Invoice…", "invoice"],
    ["Add Payslip…", "payslip"],
    ["Add Other Document…", "other"],
  ] as const)("asks for a PDF and opens the Add sheet on %s's kind when its row is chosen", async (name, kind) => {
    const preview = addPreview();
    mocked.documentAddChoose.mockResolvedValue(preview);
    renderHeader();
    await openNew();

    await userEvent.click(screen.getByRole("menuitem", { name }));

    expect(screen.queryByRole("menu")).toBeNull();
    await vi.waitFor(() => expect(useStore.getState().addPreview).toEqual(preview));
    expect(useStore.getState().addKind).toBe(kind);
  });

  it("asks for a new subscription when its row is chosen, and closes the menu", async () => {
    renderHeader();
    await openNew();

    await userEvent.click(screen.getByRole("menuitem", { name: "New Subscription" }));

    expect(screen.queryByRole("menu")).toBeNull();
    expect(useStore.getState().subscriptionSheet).not.toBeNull();
  });

  it("closes on Escape and returns the focus to the New button", async () => {
    renderHeader();
    await openNew();

    await userEvent.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(newButton());
    expect(newButton().getAttribute("aria-expanded")).toBe("false");
  });

  it("closes on a press outside the menu", async () => {
    renderHeader();
    await openNew();

    await userEvent.click(screen.getByRole("heading", { level: 1 }));

    expect(screen.queryByRole("menu")).toBeNull();
  });
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
      ["the invoices tab", elsewhere({ label: "3 documents", place: "All documents", target: { page: "documents", tab: "invoices" } }), { page: "documents", tab: "invoices" }],
      ["the statements tab", elsewhere({ label: "3 statements", place: "All statements", target: { page: "documents", tab: "statements" } }), { page: "documents", tab: "statements" }],
      ["a statement", elsewhere({ label: "Nr. 010/2026", place: "Private", target: { page: "statement", statementId: "st-10" } }), { page: "statement", statementId: "st-10" }],
      ["a payslip", elsewhere({ label: "Payslip July 2026", place: "Payslips", target: { page: "payslip", payslipId: "ps-7", year: 2026 } }), { page: "payslip", payslipId: "ps-7", year: 2026 }],
      ["an other document", elsewhere({ label: "Demo Lease", place: "Other", target: { page: "other", otherId: "other-1" } }), { page: "other", otherId: "other-1" }],
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
