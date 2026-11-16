import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useEvents } from "../queries/useEvents";
import { HOME, type Route } from "../state/route";
import { useStore } from "../state/store";
import { account } from "../test/accounts";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { addPreview, documentView, otherGroup, otherView, statementView } from "../test/documents";
import { PdfCoversProbe } from "../test/PdfCoversProbe";
import { settings } from "../test/settings";
import { accountLines, coverage, importPreview } from "../test/statements";
import { Window } from "./Window";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], confirmation: null, paletteOpen: false, pdfCovers: [], menu: null, importPreview: null, addPreview: null, subscriptionSheet: null, statementsYear: null });
  mocked.runCommand.mockResolvedValue(undefined);
  mocked.listCommands.mockResolvedValue([]);
  mocked.listAccounts.mockResolvedValue([account()]);
  mocked.listTaxYears.mockResolvedValue([]);
  mocked.rulesTakeNotices.mockResolvedValue([]);
});

const row = (name: string) => screen.getByRole("button", { name });
const title = () => screen.getByRole("heading", { level: 1 }).textContent;

describe("sidebar", () => {
  it("lists Today, Subscriptions, Accounts and Documents with Today current", () => {
    render(<Window />);

    for (const name of ["Today", "Subscriptions", "Accounts", "Documents"]) expect(row(name)).toBeTruthy();
    expect(row("Today").getAttribute("aria-current")).toBe("page");
    expect(row("Documents").getAttribute("aria-current")).toBeNull();
  });

  it.each(["Subscriptions", "Accounts", "Documents"])("marks %s current and shows it as the header title when clicked", async (name) => {
    render(<Window />);

    await userEvent.click(row(name));

    expect(row(name).getAttribute("aria-current")).toBe("page");
    expect(row("Today").getAttribute("aria-current")).toBeNull();
    expect(title()).toBe(name);
  });

  it("keeps Accounts current on a page nested under it", () => {
    useStore.setState({ route: { page: "review" } });

    render(<Window />);

    expect(title()).toBe("Review");
    expect(row("Accounts").getAttribute("aria-current")).toBe("page");
  });

  it.each<[string, Route]>([
    ["a statement", { page: "statement", statementId: "st-1" }],
    ["an other document", { page: "other", otherId: "other-1" }],
    ["a payslip", { page: "payslip", payslipId: "ps-7", year: 2026 }],
  ])("keeps Documents current on the page of %s", (_name, route) => {
    useStore.setState({ route });

    render(<Window />);

    expect(row("Documents").getAttribute("aria-current")).toBe("page");
    expect(row("Accounts").getAttribute("aria-current")).toBeNull();
  });

  it("shows the PDF dropzone and switches its text while files are dragged over", () => {
    render(<Window />);
    const zone = screen.getByText("Drop a PDF");

    expect(screen.getByText("Statements are recognized; for anything else Hamster asks what it is")).toBeTruthy();
    fireEvent.dragOver(zone, { dataTransfer: { types: ["Files"] } });
    expect(screen.getByText("Release to add")).toBeTruthy();

    fireEvent.dragLeave(screen.getByText("Release to add"));
    expect(screen.getByText("Drop a PDF")).toBeTruthy();
  });

  it("ignores a drag that carries no files", () => {
    render(<Window />);

    fireEvent.dragOver(screen.getByText("Drop a PDF"), { dataTransfer: { types: ["text/plain"] } });

    expect(screen.queryByText("Release to add")).toBeNull();
  });
});

describe("settings button", () => {
  const settingsButton = () => screen.getByRole("button", { name: "Settings and rules (⌘,)" });

  it("asks Rust to open Settings", async () => {
    render(<Window />);

    await userEvent.click(settingsButton());

    expect(mocked.runCommand).toHaveBeenCalledWith("app:settings");
  });

  it("shows the error when Settings cannot be opened", async () => {
    mocked.runCommand.mockRejectedValue({ kind: "internal", message: "No window available", code: null });
    render(<Window />);

    await userEvent.click(settingsButton());

    expect((await screen.findByRole("alert")).textContent).toContain("No window available");
  });
});

describe("tax rules notices", () => {
  const messages = () => useStore.getState().toasts.map((t) => ({ kind: t.kind, message: t.message }));

  it("tells each notice about the rules as a success toast", async () => {
    mocked.rulesTakeNotices.mockResolvedValue(["Tax Rules 2026 now use the built-in 2026 v2", "Tax Year 2027 started"]);

    render(<Window />);

    await waitFor(() =>
      expect(messages()).toEqual([
        { kind: "success", message: "Tax Rules 2026 now use the built-in 2026 v2" },
        { kind: "success", message: "Tax Year 2027 started" },
      ]),
    );
  });

  it("shows no toast when there is nothing to tell", async () => {
    render(<Window />);

    await screen.findByRole("complementary", { name: "Sidebar" });
    expect(messages()).toEqual([]);
  });

  it("shows no toast and keeps working when the notices cannot be read", async () => {
    mocked.rulesTakeNotices.mockRejectedValue({ kind: "internal", message: "No notices", code: null });

    render(<Window />);

    await screen.findByRole("complementary", { name: "Sidebar" });
    expect(messages()).toEqual([]);
  });
});

describe("tax years in the sidebar", () => {
  it("lists each year with Open for an open one and a lock for a closed one", async () => {
    mocked.listTaxYears.mockResolvedValue([
      { year: 2026, state: "open", ruleset: "2026 v1" },
      { year: 2025, state: "closed", ruleset: "2025 v1" },
    ]);
    render(<Window />);

    const open = await screen.findByRole("button", { name: /Tax year 2026/ });
    const closed = screen.getByRole("button", { name: /Tax year 2025/ });
    expect(within(open).getByText("Open")).toBeTruthy();
    expect(within(closed).queryByText("Open")).toBeNull();
  });

  it("opens the year page when a year is clicked", async () => {
    mocked.listTaxYears.mockResolvedValue([{ year: 2026, state: "open", ruleset: "2026 v1" }]);
    render(<Window />);

    await userEvent.click(await screen.findByRole("button", { name: /Tax year 2026/ }));

    expect(title()).toBe("Tax year 2026");
    expect(screen.getByRole("button", { name: /Tax year 2026/ }).getAttribute("aria-current")).toBe("page");
  });

  it("shows no years caption when there are none", () => {
    render(<Window />);

    expect(screen.queryByText("Tax years")).toBeNull();
  });
});

describe("first run", () => {
  function WithEvents() {
    useEvents();
    return <Window />;
  }

  beforeEach(() => {
    mocked.listAccounts.mockResolvedValue([]);
    mocked.listTaxYears.mockResolvedValue([{ year: 2026, state: "open", ruleset: "2026 v1" }]);
    mocked.listen.mockResolvedValue(() => {});
    mocked.addAccount.mockResolvedValue(account());
  });

  const addFirstAccount = async () => {
    await userEvent.type(await screen.findByLabelText("Name"), "Private");
    await userEvent.type(screen.getByLabelText("IBAN"), "AT61 1904 3002 3457 3201");
    await userEvent.click(screen.getByRole("button", { name: "Continue" }));
  };

  it("asks for the first account without the sidebar while no account exists", async () => {
    render(<Window />);

    expect(await screen.findByRole("heading", { name: "Add your first account" })).toBeTruthy();
    expect(screen.queryByRole("complementary", { name: "Sidebar" })).toBeNull();
    expect(screen.getByText("Step 1 of 2")).toBeTruthy();
  });

  it("moves to the statement step with the open tax year once the account is added", async () => {
    render(<Window />);

    await addFirstAccount();

    expect(await screen.findByRole("heading", { name: "Import your first statement" })).toBeTruthy();
    expect(screen.getByText("Step 2 of 2")).toBeTruthy();
    expect(screen.getByText("Tax year 2026 is ready, with its tax rules built in.")).toBeTruthy();
  });

  it("leads to the main window when the statement step is skipped", async () => {
    mocked.addAccount.mockImplementation(async () => {
      mocked.listAccounts.mockResolvedValue([account()]);
      return account();
    });
    render(<WithEvents />);
    await addFirstAccount();
    await screen.findByRole("heading", { name: "Import your first statement" });
    const handler = mocked.listen.mock.calls.find(([name]) => name === "data_changed")![1] as (payload: unknown) => void;
    await act(async () => handler({ tables: ["accounts"] }));

    await userEvent.click(screen.getByRole("button", { name: "Skip for Now" }));

    expect(await screen.findByRole("complementary", { name: "Sidebar" })).toBeTruthy();
    expect(title()).toBe("Today");
  });

  it("asks for a statement file when Import Statement… is chosen on the statement step", async () => {
    const preview = importPreview();
    mocked.statementImportChoose.mockResolvedValue(preview);
    render(<Window />);
    await addFirstAccount();

    await userEvent.click(await screen.findByRole("button", { name: "Import Statement…" }));

    expect(await screen.findByRole("dialog", { name: "Import statement" })).toBeTruthy();
    expect(useStore.getState().importPreview).toEqual(preview);
  });

  it("shows the Import sheet over the first run, and leads to the main window once imported", async () => {
    mocked.addAccount.mockImplementation(async () => {
      mocked.listAccounts.mockResolvedValue([account()]);
      return account();
    });
    mocked.statementImportCommit.mockResolvedValue("Imported Nr. 010/2026");
    render(<WithEvents />);
    await addFirstAccount();
    await screen.findByRole("heading", { name: "Import your first statement" });
    const handler = mocked.listen.mock.calls.find(([name]) => name === "data_changed")![1] as (payload: unknown) => void;
    await act(async () => handler({ tables: ["accounts"] }));
    act(() => useStore.getState().setImportPreview(importPreview()));

    await userEvent.click(await screen.findByRole("button", { name: "Import" }));

    expect(await screen.findByRole("complementary", { name: "Sidebar" })).toBeTruthy();
    expect(screen.queryByRole("dialog", { name: "Import statement" })).toBeNull();
  });

  it("shows the Import sheet of a dropped statement while no account exists", async () => {
    useStore.setState({ importPreview: importPreview() });

    render(<Window />);

    expect(await screen.findByRole("dialog", { name: "Import statement" })).toBeTruthy();
    expect(await screen.findByRole("heading", { name: "Add your first account", hidden: true })).toBeTruthy();
    expect(screen.queryByRole("complementary", { name: "Sidebar", hidden: true })).toBeNull();
  });
});

describe("account page", () => {
  beforeEach(() => {
    useStore.setState({ route: { page: "account", accountId: "acc-2" } });
    mocked.listAccounts.mockResolvedValue([account(), account({ id: "acc-2", name: "Savings" })]);
    mocked.accountLines.mockResolvedValue(accountLines({ id: "acc-2", name: "Savings" }));
  });

  it("titles the header with the account's name and offers the crumb back to Accounts", async () => {
    render(<Window />);

    await waitFor(() => expect(title()).toBe("Savings"));
    expect(screen.getByRole("button", { name: "Back to Accounts" })).toBeTruthy();
  });

  it("shows the account's statements", async () => {
    render(<Window />);

    expect(await screen.findByRole("region", { name: "Nr. 010/2026" })).toBeTruthy();
    expect(mocked.accountLines).toHaveBeenCalledWith("acc-2", "");
  });
});

describe("Accounts year stepper", () => {
  beforeEach(() => {
    useStore.setState({ route: { page: "statements" }, statementsYear: null });
    mocked.statementsCoverage.mockImplementation((_query, year) => Promise.resolve(coverage({ year: year ?? 2026, years: [2024, 2025, 2026] })));
  });

  afterEach(() => {
    mocked.statementsCoverage.mockResolvedValue({ year: 2026, years: [2026], months: [], accounts: [] });
  });

  const stepper = async () => within(await screen.findByRole("group", { name: "Year" }));

  it("shows the year of the accounts in the toolbar", async () => {
    render(<Window />);

    const group = await stepper();
    expect(group.getByText("2026")).toBeTruthy();
    expect(group.getByRole<HTMLButtonElement>("button", { name: "Next Year" }).disabled).toBe(true);
  });

  it("asks for the previous year and remembers it when its arrow is clicked", async () => {
    render(<Window />);

    await userEvent.click((await stepper()).getByRole("button", { name: "Previous Year" }));

    await waitFor(() => expect(mocked.statementsCoverage).toHaveBeenCalledWith("", 2025));
    expect((await stepper()).getByText("2025")).toBeTruthy();
    expect(useStore.getState().route).toEqual({ page: "statements", year: 2025 });
    expect(useStore.getState().statementsYear).toBe(2025);
  });

  it("keeps the chosen year when Accounts is opened again without one", async () => {
    render(<Window />);
    await userEvent.click((await stepper()).getByRole("button", { name: "Previous Year" }));
    await waitFor(() => expect(useStore.getState().statementsYear).toBe(2025));

    await userEvent.click(row("Today"));
    await userEvent.click(row("Accounts"));

    expect(useStore.getState().route).toEqual({ page: "statements" });
    expect((await stepper()).getByText("2025")).toBeTruthy();
  });

  it("shows no stepper on other pages", () => {
    useStore.setState({ route: HOME });

    render(<Window />);

    expect(screen.queryByRole("group", { name: "Year" })).toBeNull();
  });
});

describe("document page", () => {
  beforeEach(() => {
    useStore.setState({ route: { page: "document", documentId: "doc-1", year: null } });
    mocked.documentView.mockResolvedValue(documentView());
  });

  it("titles the header with the document's title", async () => {
    render(<Window />);

    await waitFor(() => expect(title()).toBe("Demo Cloud Ltd · DEM-2026-1109"));
  });

  it("shows the document's form", async () => {
    render(<Window />);

    expect(await screen.findByText("Every required fact is set")).toBeTruthy();
  });
});

describe("Documents tabs", () => {
  const tabs = () => screen.queryByRole("group", { name: "Kind of document" });

  it("sit in the header on Documents, with the remembered tab pressed", () => {
    useStore.setState({ route: { page: "documents" }, documentsTab: "statements" });

    render(<Window />);

    const header = screen.getByRole("banner");
    expect(within(header).getByRole("group", { name: "Kind of document" })).toBeTruthy();
    expect(within(header).getByRole("button", { name: "Statements" }).getAttribute("aria-pressed")).toBe("true");
  });

  it.each<[string, Route]>([
    ["Today", { page: "today" }],
    ["a statement", { page: "statement", statementId: "st-10" }],
    ["the tax year", { page: "year", year: 2026 }],
  ])("are not in the header on %s", (_name, route) => {
    useStore.setState({ route });

    render(<Window />);

    expect(tabs()).toBeNull();
  });

  it("switch the list shown below when a tab is clicked", async () => {
    useStore.setState({ route: { page: "documents" }, documentsTab: "invoices" });
    mocked.othersList.mockResolvedValue([otherGroup()]);
    render(<Window />);

    await userEvent.click(screen.getByRole("button", { name: "Other" }));

    expect(await screen.findByRole("button", { name: /Demo Lease/ })).toBeTruthy();
    expect(useStore.getState().route).toEqual({ page: "documents", tab: "other" });
    expect(screen.getByRole("button", { name: "Other" }).getAttribute("aria-pressed")).toBe("true");
  });
});

describe("statement and other document pages", () => {
  it("titles the header with the loaded statement's title and a crumb back to the Statements tab", async () => {
    useStore.setState({ route: { page: "statement", statementId: "st-10" } });
    mocked.statementView.mockResolvedValue(statementView());

    render(<Window />);

    await waitFor(() => expect(title()).toBe("Private · Nr. 010/2026"));
    expect(screen.getByRole("button", { name: "Back to Documents" })).toBeTruthy();
  });

  it("titles the header with the loaded other document's title", async () => {
    useStore.setState({ route: { page: "other", otherId: "other-1" } });
    mocked.otherView.mockResolvedValue(otherView());

    render(<Window />);

    await waitFor(() => expect(title()).toBe("Demo Lease"));
  });

  it("titles the header with the generic name until the page loads", () => {
    useStore.setState({ route: { page: "statement", statementId: "st-10" } });

    render(<Window />);

    expect(title()).toBe("Statement");
  });
});

describe("Add document sheet", () => {
  it("shows over the main window once an invoice was read", async () => {
    useStore.setState({ addPreview: addPreview() });

    render(<Window />);

    expect(await screen.findByRole("dialog", { name: "Add document" })).toBeTruthy();
    expect(screen.getByText("invoice-demo-1109.pdf")).toBeTruthy();
  });

  it("is absent while nothing was read", async () => {
    render(<Window />);

    await screen.findByRole("complementary", { name: "Sidebar" });
    expect(screen.queryByRole("dialog", { name: "Add document" })).toBeNull();
  });

  it("closes on Cancel", async () => {
    useStore.setState({ addPreview: addPreview() });
    render(<Window />);

    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("dialog", { name: "Add document" })).toBeNull();
  });
});

describe("Import sheet", () => {
  it("shows over the main window once a statement was read", async () => {
    useStore.setState({ importPreview: importPreview() });

    render(<Window />);

    expect(await screen.findByRole("dialog", { name: "Import statement" })).toBeTruthy();
    expect(screen.getByText("statement-010-2026.pdf")).toBeTruthy();
  });

  it("is absent while nothing was read", async () => {
    render(<Window />);

    await screen.findByRole("complementary", { name: "Sidebar" });
    expect(screen.queryByRole("dialog", { name: "Import statement" })).toBeNull();
  });

  it("closes on Cancel", async () => {
    useStore.setState({ importPreview: importPreview() });
    render(<Window />);

    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("dialog", { name: "Import statement" })).toBeNull();
  });
});

describe("command palette", () => {
  it("is hidden until it is opened", () => {
    render(<Window />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("shows over the window once opened", async () => {
    mocked.listCommands.mockResolvedValue([{ id: "go:today", title: "Go to Today", shortcut: null, custom: false }]);
    useStore.setState({ paletteOpen: true });

    render(<Window />);

    expect(await screen.findByRole("option", { name: "Go to Today" })).toBeTruthy();
  });

  it("hides the PDF preview while open", async () => {
    useStore.setState({ paletteOpen: true });

    render(
      <>
        <PdfCoversProbe />
        <Window />
      </>,
    );

    await screen.findByRole("dialog");
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: true, holes: [], clickThrough: false }));
  });
});

describe("back and forward", () => {
  it("disables both buttons until there is history", () => {
    render(<Window />);

    expect(screen.getByRole("button", { name: "Back (⌘[)" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Forward (⌘])" })).toBeDisabled();
  });

  it("goes back to the previous page and forward again", async () => {
    render(<Window />);
    await userEvent.click(row("Subscriptions"));

    await userEvent.click(screen.getByRole("button", { name: "Back (⌘[)" }));
    expect(title()).toBe("Today");
    expect(screen.getByRole("button", { name: "Back (⌘[)" })).toBeDisabled();

    await userEvent.click(screen.getByRole("button", { name: "Forward (⌘])" }));
    expect(title()).toBe("Subscriptions");
  });
});

describe("year sidebar toggle", () => {
  const toggle = () => screen.getByRole("button", { name: "Show or hide payslips" });

  beforeEach(() => {
    mocked.listTaxYears.mockResolvedValue([{ year: 2026, state: "open", ruleset: "2026 v1" }]);
    mocked.updateSettings.mockResolvedValue(settings());
  });

  it("is in the header on the tax year page, pressed while the sidebar is shown", async () => {
    useStore.setState({ route: { page: "year", year: 2026 } });
    mocked.getSettings.mockResolvedValue(settings({ payslipPanel: true }));

    render(<Window />);

    await waitFor(() => expect(toggle().getAttribute("aria-pressed")).toBe("true"));
  });

  it("is not pressed while the sidebar is hidden", async () => {
    useStore.setState({ route: { page: "year", year: 2026 } });
    mocked.getSettings.mockResolvedValue(settings({ payslipPanel: false }));

    render(<Window />);

    await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
    expect(toggle().getAttribute("aria-pressed")).toBe("false");
  });

  it("saves the opposite setting when clicked", async () => {
    useStore.setState({ route: { page: "year", year: 2026 } });
    mocked.getSettings.mockResolvedValue(settings({ payslipPanel: false }));
    render(<Window />);

    await userEvent.click(toggle());

    await waitFor(() => expect(mocked.updateSettings).toHaveBeenCalledWith({ payslipPanel: true }));
  });

  it.each<[string, Route]>([
    ["Today", { page: "today" }],
    ["Documents", { page: "documents" }],
    ["a payslip", { page: "payslip", payslipId: "ps-7", year: 2026 }],
  ])("is not offered on %s", async (_name, route) => {
    useStore.setState({ route });

    render(<Window />);

    await screen.findByRole("complementary", { name: "Sidebar" });
    expect(screen.queryByRole("button", { name: "Show or hide payslips" })).toBeNull();
  });
});

describe("filter field", () => {
  it("searches everywhere with ⌘K on Today", () => {
    render(<Window />);

    expect(screen.getByPlaceholderText("Search everywhere")).toBeTruthy();
    expect(screen.getByText("⌘K")).toBeTruthy();
  });

  it("filters subscriptions with ⌘F on Subscriptions", async () => {
    render(<Window />);

    await userEvent.click(row("Subscriptions"));

    expect(screen.getByPlaceholderText("Filter subscriptions")).toBeTruthy();
    expect(screen.getByText("⌘F")).toBeTruthy();
  });

  it("offers a Clear filter button once text is typed, which empties the field", async () => {
    render(<Window />);
    const field = screen.getByPlaceholderText<HTMLInputElement>("Search everywhere");

    await userEvent.type(field, "rent");
    expect(field.value).toBe("rent");
    expect(screen.queryByText("⌘K")).toBeNull();

    await userEvent.click(screen.getByRole("button", { name: "Clear filter (Esc)" }));

    expect(field.value).toBe("");
    expect(screen.getByText("⌘K")).toBeTruthy();
  });

  it("clears the field on Escape", async () => {
    render(<Window />);
    const field = screen.getByPlaceholderText<HTMLInputElement>("Search everywhere");
    await userEvent.type(field, "rent");

    await userEvent.keyboard("{Escape}");

    expect(field.value).toBe("");
  });

  it("is emptied when another page opens", async () => {
    render(<Window />);
    await userEvent.type(screen.getByPlaceholderText("Search everywhere"), "rent");

    await userEvent.click(row("Accounts"));

    expect(screen.getByPlaceholderText<HTMLInputElement>("Filter accounts").value).toBe("");
  });
});

describe("New menu", () => {
  const openMenu = async () => {
    await act(() => new Promise((resolve) => setTimeout(resolve, 0)));
    await userEvent.click(screen.getByRole("button", { name: /New/ }));
  };

  beforeEach(() => {
    mocked.listCommands.mockResolvedValue([
      { id: "statement:import", title: "Import Statement…", shortcut: "⌘I", custom: false },
      { id: "document:add", title: "Add Invoice PDF…", shortcut: "⌥⌘O", custom: true },
      { id: "payslip:add", title: "Add Payslip…", shortcut: null, custom: false },
      { id: "other:add", title: "Add Other Document…", shortcut: null, custom: false },
      { id: "subscription:new", title: "New Subscription", shortcut: null, custom: false },
    ]);
  });

  it("lists the creation commands with the shortcut each has now", async () => {
    render(<Window />);

    await openMenu();

    const menu = screen.getByRole("menu", { name: "New" });
    const items = within(menu).getAllByRole("menuitem").map((i) => i.textContent);
    expect(items).toEqual(["Import Statement…⌘I", "Add Invoice…⌥⌘O", "Add Payslip…", "Add Other Document…", "New Subscription"]);
  });

  it("lets clicks on the PDF preview through while open and sends the uncovered state once closed", async () => {
    render(
      <>
        <PdfCoversProbe />
        <Window />
      </>,
    );
    expect(mocked.pdfViewCover).not.toHaveBeenCalled();

    await openMenu();
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith(expect.objectContaining({ hidden: false, clickThrough: true })));

    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: false }));
  });

  it("starts the statement import when Import Statement… is chosen, and closes the menu", async () => {
    const preview = importPreview();
    mocked.statementImportChoose.mockResolvedValue(preview);
    render(<Window />);
    await openMenu();

    const item = screen.getByRole("menuitem", { name: /Import Statement…/ });
    expect(item).toBeEnabled();
    await userEvent.click(item);

    expect(screen.queryByRole("menu")).toBeNull();
    expect(await screen.findByRole("dialog", { name: "Import statement" })).toBeTruthy();
    expect(useStore.getState().importPreview).toEqual(preview);
  });

  it.each([
    ["Add Invoice…", "invoice"],
    ["Add Payslip…", "payslip"],
    ["Add Other Document…", "other"],
  ])("starts adding a PDF on the %s kind when it is chosen, and closes the menu", async (name, kind) => {
    const preview = addPreview();
    mocked.documentAddChoose.mockResolvedValue(preview);
    render(<Window />);
    await openMenu();

    const item = screen.getByRole("menuitem", { name: new RegExp(name) });
    expect(item).toBeEnabled();
    await userEvent.click(item);

    expect(screen.queryByRole("menu")).toBeNull();
    expect(await screen.findByRole("dialog", { name: "Add document" })).toBeTruthy();
    expect(useStore.getState().addPreview).toEqual(preview);
    expect(useStore.getState().addKind).toBe(kind);
  });

  it("opens the New Subscription sheet when New Subscription is chosen, and closes the menu", async () => {
    render(<Window />);
    await openMenu();

    const item = screen.getByRole("menuitem", { name: /New Subscription/ });
    expect(item).toBeEnabled();
    await userEvent.click(item);

    expect(screen.queryByRole("menu")).toBeNull();
    expect(await screen.findByRole("dialog", { name: "New Subscription" })).toBeTruthy();
  });

  it("closes on Escape", async () => {
    render(<Window />);
    await openMenu();

    await userEvent.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("closes on a pointer press outside the menu", async () => {
    render(<Window />);
    await openMenu();

    await userEvent.click(screen.getByRole("heading", { level: 1 }));

    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("closes when the New button is clicked again", async () => {
    render(<Window />);
    await openMenu();

    await userEvent.click(screen.getByRole("button", { name: /New/ }));

    expect(screen.queryByRole("menu")).toBeNull();
  });
});
