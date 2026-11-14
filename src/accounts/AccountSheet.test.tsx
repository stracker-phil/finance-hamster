import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { account } from "../test/accounts";
import { renderWithQuery } from "../test/renderWithQuery";
import { AccountSheet } from "./AccountSheet";

const mocked = vi.mocked(api, true);

const messages = () => useStore.getState().toasts.map((t) => t.message);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
  mocked.addAccount.mockResolvedValue(account());
  mocked.editAccount.mockResolvedValue(undefined);
});

describe("AccountSheet adding", () => {
  const fill = async (name: string, iban: string) => {
    await userEvent.type(screen.getByLabelText("Name"), name);
    await userEvent.type(screen.getByLabelText("IBAN"), iban);
  };

  it("disables Add Account until both name and IBAN are filled", async () => {
    renderWithQuery(<AccountSheet onClose={() => {}} />);
    const submit = screen.getByRole("button", { name: "Add Account" });
    expect(submit).toBeDisabled();

    await userEvent.type(screen.getByLabelText("Name"), "Private");
    expect(submit).toBeDisabled();

    await userEvent.type(screen.getByLabelText("IBAN"), "AT61 1904 3002 3457 3201");
    expect(submit).toBeEnabled();
  });

  it("sends name, bank, IBAN and the own flag, then toasts and closes", async () => {
    const onClose = vi.fn();
    renderWithQuery(<AccountSheet onClose={onClose} />);
    await fill("Private", "AT61 1904 3002 3457 3201");
    await userEvent.type(screen.getByLabelText("Bank"), "Example Bank");
    await userEvent.click(screen.getByRole("switch", { name: /One of my accounts/ }));

    await userEvent.click(screen.getByRole("button", { name: "Add Account" }));

    await waitFor(() => expect(messages()).toEqual(["Added Private"]));
    expect(mocked.addAccount).toHaveBeenCalledWith({ name: "Private", bank: "Example Bank", iban: "AT61 1904 3002 3457 3201", own: false, statementName: null });
    expect(onClose).toHaveBeenCalled();
  });

  it("shows an invalid IBAN under its field and keeps the sheet open", async () => {
    mocked.addAccount.mockRejectedValue({ kind: "invalid_input", message: "account.iban: This isn't a valid IBAN", code: null, field: { path: "account.iban", message: "This isn't a valid IBAN" } });
    const onClose = vi.fn();
    renderWithQuery(<AccountSheet onClose={onClose} />);
    await fill("Private", "AT00");

    await userEvent.click(screen.getByRole("button", { name: "Add Account" }));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toBe("This isn't a valid IBAN");
    expect(screen.getByLabelText("IBAN").getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByLabelText("IBAN").getAttribute("aria-describedby")).toBe(alert.id);
    expect(screen.getByLabelText("Name").getAttribute("aria-invalid")).toBe("false");
    expect(onClose).not.toHaveBeenCalled();
  });

  it("shows an error without a field in the form's own line", async () => {
    mocked.addAccount.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    renderWithQuery(<AccountSheet onClose={() => {}} />);
    await fill("Private", "AT61 1904 3002 3457 3201");

    await userEvent.click(screen.getByRole("button", { name: "Add Account" }));

    expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
    expect(screen.getByLabelText("IBAN").getAttribute("aria-invalid")).toBe("false");
  });

  it("shows no alert before any error", () => {
    renderWithQuery(<AccountSheet onClose={() => {}} />);

    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("closes without saving on Cancel", async () => {
    const onClose = vi.fn();
    renderWithQuery(<AccountSheet onClose={onClose} />);

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onClose).toHaveBeenCalled();
    expect(mocked.addAccount).not.toHaveBeenCalled();
  });
});

describe("AccountSheet editing", () => {
  it("shows the account's values with the IBAN locked", () => {
    renderWithQuery(<AccountSheet account={account({ name: "Joint", bank: "Sample Bank" })} onClose={() => {}} />);

    expect(screen.getByRole("heading", { name: "Edit Account" })).toBeTruthy();
    expect(screen.getByLabelText<HTMLInputElement>("Name").value).toBe("Joint");
    expect(screen.getByLabelText<HTMLInputElement>("Bank").value).toBe("Sample Bank");
    expect(screen.getByLabelText("IBAN")).toBeDisabled();
  });

  it("sends the id with name, bank and own flag, then toasts Saved and closes", async () => {
    const onClose = vi.fn();
    renderWithQuery(<AccountSheet account={account({ id: "acc-7", name: "Joint", own: true })} onClose={onClose} />);
    await userEvent.clear(screen.getByLabelText("Name"));
    await userEvent.type(screen.getByLabelText("Name"), "Savings");
    await userEvent.click(screen.getByRole("switch", { name: /One of my accounts/ }));

    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(messages()).toEqual(["Saved Savings"]));
    expect(mocked.editAccount).toHaveBeenCalledWith("acc-7", { name: "Savings", bank: "Example Bank", own: false, statementName: null });
    expect(onClose).toHaveBeenCalled();
  });

  it("disables Save when the name is emptied", async () => {
    renderWithQuery(<AccountSheet account={account()} onClose={() => {}} />);

    await userEvent.clear(screen.getByLabelText("Name"));

    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
  });
});
