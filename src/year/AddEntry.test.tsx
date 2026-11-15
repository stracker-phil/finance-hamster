import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { AddEntry } from "./AddEntry";

const mocked = vi.mocked(api, true);

const item = { key: "other", kz: "722", label: "Other costs", add: "amount" } as const;

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
  mocked.yearEntryAdd.mockResolvedValue("Add Example Desk Lamp");
});

describe("AddEntry", () => {
  it.each([
    ["amount", "Add an amount to Other costs"],
    ["trip", "Add a trip to Other costs"],
  ] as const)("names its %s group after the item's label", (add, name) => {
    render(<AddEntry year={2026} item={{ ...item, add, label: "Other costs" }} onDone={() => {}} />);

    expect(screen.getByRole("group", { name })).toBeTruthy();
  });

  describe("onAdded", () => {
    it("gets the typed text after a successful add", async () => {
      const onAdded = vi.fn();
      const onDone = vi.fn();
      render(<AddEntry year={2026} item={item} onDone={onDone} onAdded={onAdded} />);

      await userEvent.type(screen.getByRole("textbox", { name: "Note" }), "  Desk lamp ");
      await userEvent.type(screen.getByRole("textbox", { name: "Amount" }), "29,90");
      await userEvent.click(screen.getByRole("button", { name: "Add" }));

      await vi.waitFor(() => expect(onAdded).toHaveBeenCalledWith("Desk lamp"));
      expect(onDone).toHaveBeenCalled();
    });

    it("gets the label when the text is empty", async () => {
      const onAdded = vi.fn();
      render(<AddEntry year={2026} item={item} onDone={() => {}} onAdded={onAdded} />);

      await userEvent.type(screen.getByRole("textbox", { name: "Amount" }), "29,90");
      await userEvent.click(screen.getByRole("button", { name: "Add" }));

      await vi.waitFor(() => expect(onAdded).toHaveBeenCalledWith("Add Example Desk Lamp"));
    });

    it("is not called when the add is refused", async () => {
      mocked.yearEntryAdd.mockRejectedValue({ kind: "invalid_input", message: "Enter an amount", code: null });
      const onAdded = vi.fn();
      render(<AddEntry year={2026} item={item} onDone={() => {}} onAdded={onAdded} />);

      await userEvent.click(screen.getByRole("button", { name: "Add" }));

      await screen.findByRole("alert");
      expect(onAdded).not.toHaveBeenCalled();
    });

    it("is not called on Escape", async () => {
      const onAdded = vi.fn();
      const onDone = vi.fn();
      render(<AddEntry year={2026} item={item} onDone={onDone} onAdded={onAdded} />);

      await userEvent.type(screen.getByRole("textbox", { name: "Note" }), "Desk lamp{Escape}");

      expect(onDone).toHaveBeenCalled();
      expect(onAdded).not.toHaveBeenCalled();
      expect(mocked.yearEntryAdd).not.toHaveBeenCalled();
    });
  });
});
