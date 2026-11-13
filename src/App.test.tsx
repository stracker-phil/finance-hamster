import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({ ...(await orig<object>()), api: (await import("./test/apiMock")).createApiMock() }));

import { api, type EventName } from "./api";
import App from "./App";
import { createQueryClient } from "./queries/queryClient";
import { HOME } from "./state/route";
import { useStore } from "./state/store";
import { settings } from "./test/settings";
import { addPreview } from "./test/documents";
import { importPreview } from "./test/statements";

const mocked = vi.mocked(api, true);

const renderApp = () => render(<App client={createQueryClient()} />);

/** Fires the handler the app registered for `event`, as Rust would. */
async function emit(event: EventName, payload: unknown = null) {
  await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === event)).toBe(true));
  const handler = mocked.listen.mock.calls.find(([name]) => name === event)![1] as (payload: unknown) => void;
  act(() => handler(payload));
}

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], confirmation: null, paletteOpen: false, pdfCovers: 0, importPreview: null, addPreview: null, subscriptionSheet: null });
    mocked.takeLaunchEvents.mockResolvedValue([]);
    mocked.listen.mockResolvedValue(() => {});
    mocked.getSettings.mockResolvedValue(settings());
    mocked.listCommands.mockResolvedValue([]);
    delete document.documentElement.dataset.scheme;
    delete document.documentElement.dataset.accent;
  });

  it("shows the main window on the Today page", async () => {
    renderApp();

    expect(screen.getByRole("heading", { level: 1, name: "Today" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Today" }).getAttribute("aria-current")).toBe("page");
    await waitFor(() => expect(mocked.takeLaunchEvents).toHaveBeenCalled());
  });

  it("shows no alert when no launch event is queued", async () => {
    renderApp();

    await waitFor(() => expect(mocked.takeLaunchEvents).toHaveBeenCalled());
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows the message of an error launch event taken on mount as an error toast", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Could not open the database" }]);

    renderApp();

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Something went wrong");
    expect(alert.textContent).toContain("Could not open the database");
  });

  it("shows the error message when the launch events cannot be taken", async () => {
    mocked.takeLaunchEvents.mockRejectedValue({ kind: "io", message: "Socket closed", code: null });

    renderApp();

    expect((await screen.findByRole("alert")).textContent).toContain("Socket closed");
  });

  it("takes the launch events again when the launch event fires", async () => {
    renderApp();
    await waitFor(() => expect(mocked.listen).toHaveBeenCalled());
    expect(screen.queryByRole("alert")).toBeNull();

    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Second launch failed" }]);
    await emit("launch");

    expect((await screen.findByRole("alert")).textContent).toContain("Second launch failed");
  });

  it("unsubscribes from its events on unmount", async () => {
    const unlisten = vi.fn();
    mocked.listen.mockResolvedValue(unlisten);
    const { unmount } = renderApp();
    await waitFor(() => expect(mocked.listen).toHaveBeenCalled());

    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalled());
  });

  describe("look", () => {
    it("applies the saved appearance and accent to the window", async () => {
      mocked.getSettings.mockResolvedValue(settings({ appearance: "dark", accent: "blue" }));

      renderApp();

      await waitFor(() => expect(document.documentElement.dataset.scheme).toBe("dark"));
      expect(document.documentElement.dataset.accent).toBe("blue");
    });

    it("applies the new appearance when the settings changed event fires", async () => {
      mocked.getSettings.mockResolvedValue(settings({ appearance: "light" }));
      renderApp();
      await waitFor(() => expect(document.documentElement.dataset.scheme).toBe("light"));

      mocked.getSettings.mockResolvedValue(settings({ appearance: "dark" }));
      await emit("settings_changed");

      await waitFor(() => expect(document.documentElement.dataset.scheme).toBe("dark"));
    });
  });

  describe("page commands from Rust", () => {
    it.each([
      ["go.subscriptions", "Subscriptions"],
      ["go.statements", "Statements"],
      ["go.documents", "Documents"],
    ])("opens the page for %s", async (id, title) => {
      renderApp();

      await emit("run_command", id);

      expect(screen.getByRole("heading", { level: 1, name: title })).toBeTruthy();
    });

    it("returns to Today for go.today", async () => {
      renderApp();
      await emit("run_command", "go.documents");

      await emit("run_command", "go.today");

      expect(screen.getByRole("heading", { level: 1, name: "Today" })).toBeTruthy();
    });

    it("goes back and forward through the visited pages", async () => {
      renderApp();
      await emit("run_command", "go.subscriptions");

      await emit("run_command", "go.back");
      expect(screen.getByRole("heading", { level: 1, name: "Today" })).toBeTruthy();

      await emit("run_command", "go.forward");
      expect(screen.getByRole("heading", { level: 1, name: "Subscriptions" })).toBeTruthy();
    });

    it("opens the command palette for palette.open", async () => {
      mocked.listCommands.mockResolvedValue([{ id: "go.today", title: "Go to Today", shortcut: "⌘1" }]);
      renderApp();

      await emit("run_command", "palette.open");

      expect(await screen.findByRole("dialog")).toBeTruthy();
      expect(await screen.findByRole("option", { name: /Go to Today/ })).toBeTruthy();
    });

    it("suspends the native PDF preview while the palette is open and resumes it once closed", async () => {
      renderApp();
      await emit("run_command", "palette.open");
      await screen.findByRole("dialog");
      await waitFor(() => expect(mocked.pdfViewSuspend).toHaveBeenCalled());
      expect(mocked.pdfViewResume).not.toHaveBeenCalled();

      await userEvent.keyboard("{Escape}");

      await waitFor(() => expect(mocked.pdfViewResume).toHaveBeenCalled());
    });

    it("focuses the header filter for filter.focus", async () => {
      renderApp();

      await emit("run_command", "filter.focus");

      expect(screen.getByPlaceholderText("Search everywhere")).toHaveFocus();
    });

    it("opens the New Subscription sheet for subscription.new", async () => {
      renderApp();

      await emit("run_command", "subscription.new");

      expect(await screen.findByRole("dialog", { name: "New Subscription" })).toBeTruthy();
    });

    describe.each([
      ["edit.undo", "undo", "Undid", "Nothing to undo"],
      ["edit.redo", "redo", "Redid", "Nothing to redo"],
    ] as const)("%s", (id, method, past, empty) => {
      it(`shows "${past} <label>" when the api returns the label of the change`, async () => {
        mocked[method].mockResolvedValue("Add subscription");
        renderApp();

        await emit("run_command", id);

        expect((await screen.findByRole("status")).textContent).toContain(`${past} Add subscription`);
      });

      it(`shows "${empty}" when there is nothing to ${method}`, async () => {
        mocked[method].mockResolvedValue(null);
        renderApp();

        await emit("run_command", id);

        expect((await screen.findByRole("status")).textContent).toContain(empty);
      });

      it("shows the error message when the call is rejected", async () => {
        mocked[method].mockRejectedValue({ kind: "io", message: "Disk full", code: null });
        renderApp();

        await emit("run_command", id);

        expect((await screen.findByRole("alert")).textContent).toContain("Disk full");
      });

      it(`leaves the change alone and uses the browser's ${method} while a text field has focus`, async () => {
        const execCommand = vi.fn().mockReturnValue(true);
        document.execCommand = execCommand;
        renderApp();
        await emit("run_command", "filter.focus");

        await emit("run_command", id);

        expect(execCommand).toHaveBeenCalledWith(method);
        expect(mocked[method]).not.toHaveBeenCalled();
      });
    });

    it("opens the Import sheet for the statement the owner chose for statement.import", async () => {
      mocked.statementImportChoose.mockResolvedValue(importPreview());
      renderApp();

      await emit("run_command", "statement.import");

      expect(await screen.findByRole("dialog", { name: "Import statement" })).toBeTruthy();
      expect(screen.getByText("statement-010-2026.pdf")).toBeTruthy();
    });

    it("opens no sheet for statement.import when the file dialog is cancelled", async () => {
      mocked.statementImportChoose.mockResolvedValue(null);
      renderApp();

      await emit("run_command", "statement.import");

      await waitFor(() => expect(mocked.statementImportChoose).toHaveBeenCalled());
      expect(screen.queryByRole("dialog")).toBeNull();
    });

    it("opens the Import sheet when a statement dropped on the window was read", async () => {
      renderApp();

      await emit("import_ready", { preview: importPreview(), error: null });

      expect(await screen.findByRole("dialog", { name: "Import statement" })).toBeTruthy();
    });

    it("shows why a dropped file was refused", async () => {
      renderApp();

      await emit("import_ready", { preview: null, error: "No account matches this IBAN" });

      expect((await screen.findByRole("alert")).textContent).toContain("No account matches this IBAN");
    });

    it("opens the Add document sheet for the invoice the owner chose for document.add", async () => {
      mocked.documentAddChoose.mockResolvedValue(addPreview());
      renderApp();

      await emit("run_command", "document.add");

      expect(await screen.findByRole("dialog", { name: "Add document" })).toBeTruthy();
      expect(screen.getByText("invoice-demo-1109.pdf")).toBeTruthy();
    });

    it("opens the Add document sheet when an invoice dropped on the window was read", async () => {
      renderApp();

      await emit("add_ready", { preview: addPreview(), error: null });

      expect(await screen.findByRole("dialog", { name: "Add document" })).toBeTruthy();
    });

    it("shows why a dropped invoice was refused", async () => {
      renderApp();

      await emit("add_ready", { preview: null, error: "This PDF has no text" });

      expect((await screen.findByRole("alert")).textContent).toContain("This PDF has no text");
    });

    it("ignores a command it does not handle", async () => {
      renderApp();

      await emit("run_command", "settings.open");

      expect(screen.getByRole("heading", { level: 1, name: "Today" })).toBeTruthy();
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });
});
