import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { account } from "../test/accounts";
import { AccountForm } from "./AccountForm";

const mocked = vi.mocked(api, true);

const STATEMENT_NAME = "Name on other statements";

beforeEach(() => {
  vi.clearAllMocks();
  mocked.addAccount.mockResolvedValue(account());
  mocked.editAccount.mockResolvedValue(undefined);
});

describe("AccountForm statement name", () => {
  it("shows the account's statement name when editing", () => {
    render(<AccountForm account={account({ statementName: "Haushaltskonto" })} submitLabel="Save" onSaved={() => {}} />);

    expect(screen.getByLabelText<HTMLInputElement>(STATEMENT_NAME).value).toBe("Haushaltskonto");
  });

  it("starts empty for an account without one", () => {
    render(<AccountForm account={account({ statementName: null })} submitLabel="Save" onSaved={() => {}} />);

    expect(screen.getByLabelText<HTMLInputElement>(STATEMENT_NAME).value).toBe("");
  });

  it("explains the field with a hint", () => {
    render(<AccountForm submitLabel="Add Account" onSaved={() => {}} />);

    expect(screen.getByText(/How your other accounts' statements name this one/)).toBeTruthy();
  });

  it("disables the field while One of my accounts is off and keeps it in place", async () => {
    render(<AccountForm submitLabel="Add Account" onSaved={() => {}} />);
    const field = screen.getByLabelText(STATEMENT_NAME);
    expect(field).toBeEnabled();

    await userEvent.click(screen.getByRole("switch", { name: /One of my accounts/ }));

    expect(screen.getByLabelText(STATEMENT_NAME)).toBe(field);
    expect(field).toBeDisabled();
  });

  describe("when adding", () => {
    const fillRequired = async () => {
      await userEvent.type(screen.getByLabelText("Name"), "Private");
      await userEvent.type(screen.getByLabelText("IBAN"), "AT61 1904 3002 3457 3201");
    };

    it("sends the trimmed name", async () => {
      render(<AccountForm submitLabel="Add Account" onSaved={() => {}} />);
      await fillRequired();
      await userEvent.type(screen.getByLabelText(STATEMENT_NAME), "  Haushaltskonto ");

      await userEvent.click(screen.getByRole("button", { name: "Add Account" }));

      await waitFor(() =>
        expect(mocked.addAccount).toHaveBeenCalledWith({ name: "Private", bank: "", iban: "AT61 1904 3002 3457 3201", own: true, statementName: "Haushaltskonto" }),
      );
    });

    it.each(["", "   "])("sends null when the field holds %j", async (typed) => {
      render(<AccountForm submitLabel="Add Account" onSaved={() => {}} />);
      await fillRequired();
      if (typed) await userEvent.type(screen.getByLabelText(STATEMENT_NAME), typed);

      await userEvent.click(screen.getByRole("button", { name: "Add Account" }));

      await waitFor(() => expect(mocked.addAccount).toHaveBeenCalledWith(expect.objectContaining({ statementName: null })));
    });
  });

  describe("when editing", () => {
    it("sends the trimmed name", async () => {
      render(<AccountForm account={account({ id: "acc-7" })} submitLabel="Save" onSaved={() => {}} />);
      await userEvent.type(screen.getByLabelText(STATEMENT_NAME), " Sparbuch ");

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      await waitFor(() =>
        expect(mocked.editAccount).toHaveBeenCalledWith("acc-7", { name: "Private", bank: "Example Bank", own: true, statementName: "Sparbuch" }),
      );
    });

    it("sends null after the name is cleared", async () => {
      render(<AccountForm account={account({ id: "acc-7", statementName: "Haushaltskonto" })} submitLabel="Save" onSaved={() => {}} />);
      await userEvent.clear(screen.getByLabelText(STATEMENT_NAME));

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      await waitFor(() => expect(mocked.editAccount).toHaveBeenCalledWith("acc-7", expect.objectContaining({ statementName: null })));
    });
  });

  it("shows a field error under the field and hides the hint", async () => {
    mocked.editAccount.mockRejectedValue({
      kind: "invalid_input",
      message: "account.statementName: Already used by another account",
      code: null,
      field: { path: "account.statementName", message: "Already used by another account" },
    });
    render(<AccountForm account={account()} submitLabel="Save" onSaved={() => {}} />);
    await userEvent.type(screen.getByLabelText(STATEMENT_NAME), "Haushaltskonto");

    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toBe("Already used by another account");
    const field = screen.getByLabelText(STATEMENT_NAME);
    expect(field.getAttribute("aria-invalid")).toBe("true");
    expect(field.getAttribute("aria-describedby")).toBe(alert.id);
    expect(screen.queryByText(/How your other accounts' statements name this one/)).toBeNull();
  });
});
