import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { useStore } from "../../state/store";
import { vendorView, vendorsView } from "../../test/documents";
import { renderWithQuery } from "../../test/renderWithQuery";
import { VendorsPage } from "./VendorsPage";

const mocked = vi.mocked(api, true);
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

const renderPage = () =>
  renderWithQuery(
    <>
      <VendorsPage />
      <ConfirmHost />
    </>,
  );

const openSheet = async (name = "Demo Cloud Ltd") => {
  await screen.findByText(name);
  const index = screen.getAllByRole("button", { name: "Edit…" });
  const names = ["Demo Cloud Ltd", "Example Market"];
  await userEvent.click(index[names.indexOf(name)] ?? index[0]);
  return screen.getByRole("dialog", { name: "Edit Vendor" });
};

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], confirmation: null });
  mocked.vendorsView.mockResolvedValue(vendorsView());
  mocked.vendorChange.mockResolvedValue("Changed vendor");
});

describe("VendorsPage", () => {
  describe("vendor list", () => {
    it("lists each vendor with its detail and an Edit… button", async () => {
      renderPage();

      expect(await screen.findByText("Demo Cloud Ltd")).toBeTruthy();
      expect(screen.getByText("2 documents · KZ 169")).toBeTruthy();
      expect(screen.getByText("Example Market")).toBeTruthy();
      expect(screen.getByText("1 document · KZ 720")).toBeTruthy();
      expect(screen.getAllByRole("button", { name: "Edit…" })).toHaveLength(2);
    });

    it("explains where vendors come from when there are none", async () => {
      mocked.vendorsView.mockResolvedValue(vendorsView({ vendors: [] }));
      renderPage();

      expect(await screen.findByText("No vendors yet")).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Edit…" })).toBeNull();
    });
  });

  describe("edit sheet", () => {
    it("is prefilled with the vendor's name and default item", async () => {
      mocked.vendorsView.mockResolvedValue(vendorsView({ vendors: [vendorView({ defaultItemKey: "kz-720" })] }));
      renderPage();

      await openSheet();

      expect(screen.getByLabelText<HTMLInputElement>("Name").value).toBe("Demo Cloud Ltd");
      expect(screen.getByLabelText<HTMLSelectElement>("Default item").value).toBe("kz-720");
    });

    it("saves a rename and a new default item as two changes in order, and toasts both labels", async () => {
      mocked.vendorChange.mockResolvedValueOnce("Renamed vendor").mockResolvedValueOnce("Set default item");
      renderPage();
      await openSheet();

      await userEvent.clear(screen.getByLabelText("Name"));
      await userEvent.type(screen.getByLabelText("Name"), "Demo Cloud");
      await userEvent.selectOptions(screen.getByLabelText("Default item"), "kz-720");
      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.vendorChange.mock.calls).toEqual([
        ["ven-1", { kind: "rename", name: "Demo Cloud" }],
        ["ven-1", { kind: "defaultItem", key: "kz-720" }],
      ]);
      expect(toasts()).toEqual([["success", "Renamed vendor · Set default item"]]);
      expect(screen.queryByRole("dialog", { name: "Edit Vendor" })).toBeNull();
    });

    it("saves the empty default item as null", async () => {
      mocked.vendorsView.mockResolvedValue(vendorsView({ vendors: [vendorView({ defaultItemKey: "kz-720" })] }));
      renderPage();
      await openSheet();

      await userEvent.selectOptions(screen.getByLabelText("Default item"), "The item of its last document");
      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.vendorChange).toHaveBeenCalledWith("ven-1", { kind: "defaultItem", key: null });
    });

    it("just closes when nothing changed", async () => {
      renderPage();
      await openSheet();

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.vendorChange).not.toHaveBeenCalled();
      expect(toasts()).toEqual([]);
      expect(screen.queryByRole("dialog", { name: "Edit Vendor" })).toBeNull();
    });

    it("closes on Cancel without saving", async () => {
      renderPage();
      await openSheet();
      await userEvent.type(screen.getByLabelText("Name"), " Group");

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(mocked.vendorChange).not.toHaveBeenCalled();
      expect(screen.queryByRole("dialog", { name: "Edit Vendor" })).toBeNull();
    });

    it("shows the field message of a rejected change and stays open", async () => {
      mocked.vendorChange.mockRejectedValue({ kind: "invalid_input", message: "vendor.name: Name is already taken", code: null, field: { path: "vendor.name", message: "Name is already taken" } });
      renderPage();
      await openSheet();
      await userEvent.type(screen.getByLabelText("Name"), " Group");

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect((await screen.findByRole("alert")).textContent).toBe("Name is already taken");
      expect(screen.getByRole("dialog", { name: "Edit Vendor" })).toBeTruthy();
      expect(toasts()).toEqual([]);
    });

    it("shows the message of any other error", async () => {
      mocked.vendorChange.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderPage();
      await openSheet();
      await userEvent.type(screen.getByLabelText("Name"), " Group");

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
    });
  });

  describe("merge", () => {
    it("offers the other vendors, and enables Merge… once one is chosen", async () => {
      renderPage();
      await openSheet();
      const target = screen.getByLabelText("Merge into");

      expect(within(target).getAllByRole("option").map((o) => o.textContent)).toEqual(["Choose a vendor", "Example Market"]);
      expect(screen.getByRole("button", { name: "Merge…" })).toBeDisabled();

      await userEvent.selectOptions(target, "ven-2");
      expect(screen.getByRole("button", { name: "Merge…" })).toBeEnabled();
    });

    it("merges into the chosen vendor after the confirmation", async () => {
      mocked.vendorChange.mockResolvedValue("Merged Demo Cloud Ltd into Example Market");
      renderPage();
      await openSheet();
      await userEvent.selectOptions(screen.getByLabelText("Merge into"), "ven-2");

      await userEvent.click(screen.getByRole("button", { name: "Merge…" }));
      const confirm = await screen.findByRole("alertdialog", { name: "Merge Demo Cloud Ltd into Example Market?" });
      expect(mocked.vendorChange).not.toHaveBeenCalled();
      await userEvent.click(within(confirm).getByRole("button", { name: "Merge" }));

      await vi.waitFor(() => expect(mocked.vendorChange).toHaveBeenCalledWith("ven-1", { kind: "mergeInto", into: "ven-2" }));
      await vi.waitFor(() => expect(toasts()).toEqual([["success", "Merged Demo Cloud Ltd into Example Market"]]));
      expect(screen.queryByRole("dialog", { name: "Edit Vendor" })).toBeNull();
    });

    it("merges nothing when the confirmation is cancelled", async () => {
      renderPage();
      await openSheet();
      await userEvent.selectOptions(screen.getByLabelText("Merge into"), "ven-2");
      await userEvent.click(screen.getByRole("button", { name: "Merge…" }));
      const confirm = await screen.findByRole("alertdialog");

      await userEvent.click(within(confirm).getByRole("button", { name: "Cancel" }));

      expect(mocked.vendorChange).not.toHaveBeenCalled();
    });

    it("offers no merge when it is the only vendor", async () => {
      mocked.vendorsView.mockResolvedValue(vendorsView({ vendors: [vendorView()] }));
      renderPage();
      await openSheet();

      expect(screen.queryByLabelText("Merge into")).toBeNull();
    });
  });

  describe("delete", () => {
    it("deletes a vendor no document uses", async () => {
      mocked.vendorsView.mockResolvedValue(vendorsView({ vendors: [vendorView({ unused: true, detail: "0 documents" })] }));
      mocked.vendorChange.mockResolvedValue("Deleted vendor Demo Cloud Ltd");
      renderPage();
      await openSheet();

      await userEvent.click(screen.getByRole("button", { name: "Delete" }));

      await vi.waitFor(() => expect(mocked.vendorChange).toHaveBeenCalledWith("ven-1", { kind: "delete" }));
      await vi.waitFor(() => expect(toasts()).toEqual([["success", "Deleted vendor Demo Cloud Ltd"]]));
    });

    it("offers no Delete while a document uses the vendor", async () => {
      renderPage();
      await openSheet();

      expect(screen.queryByRole("button", { name: "Delete" })).toBeNull();
    });
  });
});
