import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { Window } from "./Window";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], confirmation: null, paletteOpen: false });
  mocked.runCommand.mockResolvedValue(undefined);
  mocked.listCommands.mockResolvedValue([]);
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
