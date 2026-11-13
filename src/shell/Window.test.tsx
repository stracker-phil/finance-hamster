import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useEvents } from "../queries/useEvents";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { account } from "../test/accounts";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { accountLines, importPreview } from "../test/statements";
import { Window } from "./Window";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], confirmation: null, paletteOpen: false, pdfCovers: 0, importPreview: null });
  mocked.runCommand.mockResolvedValue(undefined);
  mocked.listCommands.mockResolvedValue([]);
  mocked.listAccounts.mockResolvedValue([account()]);
  mocked.listTaxYears.mockResolvedValue([]);
});

const row = (name: string) => screen.getByRole("button", { name });
const title = () => screen.getByRole("heading", { level: 1 }).textContent;

describe("sidebar", () => {
  it("lists Today, Subscriptions, Statements and Documents with Today current", () => {
    render(<Window />);

    for (const name of ["Today", "Subscriptions", "Statements", "Documents"]) expect(row(name)).toBeTruthy();
    expect(row("Today").getAttribute("aria-current")).toBe("page");
    expect(row("Documents").getAttribute("aria-current")).toBeNull();
  });

  it.each(["Subscriptions", "Statements", "Documents"])("marks %s current and shows it as the header title when clicked", async (name) => {
    render(<Window />);

    await userEvent.click(row(name));

    expect(row(name).getAttribute("aria-current")).toBe("page");
    expect(row("Today").getAttribute("aria-current")).toBeNull();
    expect(title()).toBe(name);
  });

  it("keeps Statements current on a page nested under it", () => {
    useStore.setState({ route: { page: "review" } });

    render(<Window />);

    expect(title()).toBe("Review");
    expect(row("Statements").getAttribute("aria-current")).toBe("page");
  });

  it("shows the invoice dropzone and switches its text while files are dragged over", () => {
    render(<Window />);
    const zone = screen.getByText("Drop an invoice PDF");

    fireEvent.dragOver(zone, { dataTransfer: { types: ["Files"] } });
    expect(screen.getByText("Release to add")).toBeTruthy();

    fireEvent.dragLeave(screen.getByText("Release to add"));
    expect(screen.getByText("Drop an invoice PDF")).toBeTruthy();
  });

  it("ignores a drag that carries no files", () => {
    render(<Window />);

    fireEvent.dragOver(screen.getByText("Drop an invoice PDF"), { dataTransfer: { types: ["text/plain"] } });

    expect(screen.queryByText("Release to add")).toBeNull();
  });
});

describe("settings button", () => {
  const settingsButton = () => screen.getByRole("button", { name: "Settings and rules (⌘,)" });

  it("asks Rust to open Settings", async () => {
    render(<Window />);

    await userEvent.click(settingsButton());

    expect(mocked.runCommand).toHaveBeenCalledWith("settings.open");
  });

  it("shows the error when Settings cannot be opened", async () => {
    mocked.runCommand.mockRejectedValue({ kind: "internal", message: "No window available", code: null });
    render(<Window />);

    await userEvent.click(settingsButton());

    expect((await screen.findByRole("alert")).textContent).toContain("No window available");
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
    expect(screen.getByText("Tax year 2026 is ready with the rules 2026 v1.")).toBeTruthy();
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

  it("titles the header with the account's name and offers the crumb back to Statements", async () => {
    render(<Window />);

    await waitFor(() => expect(title()).toBe("Savings"));
    expect(screen.getByRole("button", { name: "Back to Statements" })).toBeTruthy();
  });

  it("shows the account's statements", async () => {
    render(<Window />);

    expect(await screen.findByRole("region", { name: "Nr. 010/2026" })).toBeTruthy();
    expect(mocked.accountLines).toHaveBeenCalledWith("acc-2");
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
    mocked.listCommands.mockResolvedValue([{ id: "go.today", title: "Go to Today", shortcut: null }]);
    useStore.setState({ paletteOpen: true });

    render(<Window />);

    expect(await screen.findByRole("option", { name: "Go to Today" })).toBeTruthy();
  });

  it("covers the PDF preview while open", async () => {
    useStore.setState({ paletteOpen: true });

    render(<Window />);

    await screen.findByRole("dialog");
    expect(useStore.getState().pdfCovers).toBe(1);
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

    await userEvent.click(row("Statements"));

    expect(screen.getByPlaceholderText<HTMLInputElement>("Filter accounts").value).toBe("");
  });
});

describe("New menu", () => {
  const openMenu = () => userEvent.click(screen.getByRole("button", { name: /New/ }));

  it("lists the creation commands with their shortcuts", async () => {
    render(<Window />);

    await openMenu();

    const items = screen.getAllByRole("menuitem").map((i) => i.textContent);
    expect(items).toEqual(["Import Statement…⌘I", "Add Invoice PDF…⌘O", "New Subscription⌘N"]);
  });

  it("covers the PDF preview while open and uncovers it once closed", async () => {
    render(<Window />);
    expect(useStore.getState().pdfCovers).toBe(0);

    await openMenu();
    expect(useStore.getState().pdfCovers).toBe(1);

    await userEvent.keyboard("{Escape}");
    expect(useStore.getState().pdfCovers).toBe(0);
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

  it("keeps the items of later features disabled", async () => {
    render(<Window />);
    await openMenu();

    expect(screen.getByRole("menuitem", { name: /Add Invoice PDF…/ })).toBeDisabled();
    expect(screen.getByRole("menuitem", { name: /New Subscription/ })).toBeDisabled();
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
});
