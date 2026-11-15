import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { otherView } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { OtherPage } from "./OtherPage";

const mocked = vi.mocked(api, true);

const show = async (patch: Parameters<typeof otherView>[0] = {}) => {
  mocked.otherView.mockResolvedValue(otherView(patch));
  renderWithQuery(<OtherPage otherId="other-1" />);
  await screen.findByRole("complementary", { name: patch.title ?? "Demo Lease" });
};

const title = () => screen.getByLabelText<HTMLInputElement>("Title");
const date = () => screen.getByLabelText<HTMLInputElement>("Date");
const note = () => screen.getByRole<HTMLTextAreaElement>("textbox", { name: "Note" });
const toastMessages = () => useStore.getState().toasts.map((t) => t.message);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: { page: "other", otherId: "other-1" }, back: [], forward: [], toasts: [], pdfCovers: 0 });
  mocked.otherEdit.mockResolvedValue(null);
  mocked.otherDelete.mockResolvedValue("Delete Demo Lease");
});

describe("OtherPage", () => {
  it("shows the PDF beside a sidebar named after the document, with its title, date and note", async () => {
    await show();

    expect(mocked.otherView).toHaveBeenCalledWith("other-1");
    expect(title().value).toBe("Demo Lease");
    expect(date().value).toBe("2026-11-10");
    expect(note().value).toBe("Signed copy");
    await vi.waitFor(() => expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "other", id: "other-1" }, expect.anything()));
  });

  it("shows an empty date for a document without one", async () => {
    await show({ date: "" });

    expect(date().value).toBe("");
  });

  describe("saving", () => {
    it("saves a retyped title on blur", async () => {
      await show();

      await userEvent.clear(title());
      await userEvent.type(title(), "Demo Policy");
      await userEvent.tab();

      expect(mocked.otherEdit).toHaveBeenCalledWith("other-1", { field: "title", text: "Demo Policy" });
    });

    it("saves a retyped title on Enter", async () => {
      await show();

      await userEvent.type(title(), " 2{Enter}");

      expect(mocked.otherEdit).toHaveBeenCalledWith("other-1", { field: "title", text: "Demo Lease 2" });
    });

    it("saves a chosen date as it is typed in the field", async () => {
      await show();

      await userEvent.click(date());
      fireEvent.change(date(), { target: { value: "2026-12-01" } });
      await userEvent.tab();

      expect(mocked.otherEdit).toHaveBeenCalledWith("other-1", { field: "date", value: "2026-12-01" });
    });

    it("saves an emptied date as a blank value", async () => {
      await show();

      await userEvent.click(date());
      fireEvent.change(date(), { target: { value: "" } });
      await userEvent.tab();

      expect(mocked.otherEdit).toHaveBeenCalledWith("other-1", { field: "date", value: "" });
    });

    it("saves the note on blur", async () => {
      await show();

      await userEvent.type(note(), " Original in the folder.");
      await userEvent.tab();

      expect(mocked.otherEdit).toHaveBeenCalledWith("other-1", { field: "note", text: "Signed copy Original in the folder." });
    });

    it("saves nothing when a field is left unchanged", async () => {
      await show();

      await userEvent.click(title());
      await userEvent.tab();

      expect(mocked.otherEdit).not.toHaveBeenCalled();
    });

    it("says what changed with Undo when the save returns a label", async () => {
      mocked.otherEdit.mockResolvedValue("Rename Demo Lease to Demo Policy");
      await show();

      await userEvent.type(title(), "x{Enter}");

      await vi.waitFor(() => expect(toastMessages()).toEqual(["Rename Demo Lease to Demo Policy"]));
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });
  });

  describe("a refused change", () => {
    it.each([
      ["title", "other.title", "A title is needed", title, ""],
      ["date", "other.date", "This isn't a date", date, "2026-12-31"],
    ])("shows the %s's message under that field only, with no toast", async (_name, path, message, field, typed) => {
      mocked.otherEdit.mockRejectedValue({ kind: "invalid_input", message: `${path}: ${message}`, code: null, field: { path, message } });
      await show();

      await userEvent.click(field());
      fireEvent.change(field(), { target: { value: typed } });
      await userEvent.tab();

      const alert = await screen.findByRole("alert");
      expect(alert.textContent).toBe(message);
      expect(field().nextElementSibling).toBe(alert);
      expect(screen.getAllByRole("alert")).toHaveLength(1);
      expect(toastMessages()).toEqual([]);
    });

    it("clears the field message once a later save succeeds", async () => {
      mocked.otherEdit.mockRejectedValueOnce({ kind: "invalid_input", message: "A title is needed", code: null, field: { path: "other.title", message: "A title is needed" } });
      await show();
      await userEvent.clear(title());
      await userEvent.tab();
      await screen.findByRole("alert");

      await userEvent.type(title(), "Demo Policy{Enter}");

      await vi.waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
    });

    it("reports a failure that names no field as an error toast", async () => {
      mocked.otherEdit.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await show();

      await userEvent.type(title(), "x{Enter}");

      await vi.waitFor(() => expect(toastMessages()).toEqual(["Disk full"]));
      expect(screen.queryByRole("alert")).toBeNull();
    });
  });

  describe("Delete Document", () => {
    it("deletes the document, says so with Undo and goes back to the Other tab", async () => {
      await show();

      await userEvent.click(screen.getByRole("button", { name: "Delete Document" }));

      expect(mocked.otherDelete).toHaveBeenCalledWith("other-1");
      await vi.waitFor(() => expect(useStore.getState().route).toEqual({ page: "documents", tab: "other" }));
      expect(toastMessages()).toEqual(["Delete Demo Lease"]);
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });

    it("stays on the document and reports an error toast when the delete is refused", async () => {
      mocked.otherDelete.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await show();

      await userEvent.click(screen.getByRole("button", { name: "Delete Document" }));

      await vi.waitFor(() => expect(toastMessages()).toEqual(["Disk full"]));
      expect(useStore.getState().route).toEqual({ page: "other", otherId: "other-1" });
    });
  });

  it("says the document was not found, with no form and no error toast, when it cannot be read", async () => {
    mocked.otherView.mockRejectedValue({ kind: "not_found", message: "No such document", code: null });
    renderWithQuery(<OtherPage otherId="other-gone" />);

    expect(await screen.findByText("Document not found")).toBeTruthy();
    expect(screen.getByText("It may have been deleted.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Delete Document" })).toBeNull();
    expect(toastMessages()).toEqual([]);
  });

  it("keeps the PDF while the document loads and shows no form yet", () => {
    mocked.otherView.mockReturnValue(new Promise(() => {}));
    renderWithQuery(<OtherPage otherId="other-1" />);

    expect(within(screen.getByRole("complementary", { name: "Document" })).queryByLabelText("Title")).toBeNull();
    expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "other", id: "other-1" }, expect.anything());
  });
});
