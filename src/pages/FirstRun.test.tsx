import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { account } from "../test/accounts";
import { importPreview } from "../test/statements";
import { renderWithQuery } from "../test/renderWithQuery";
import { FirstRun } from "./FirstRun";

const mocked = vi.mocked(api, true);

function renderFirstRun() {
  const onAccountAdded = vi.fn();
  const onDone = vi.fn();
  renderWithQuery(<FirstRun onAccountAdded={onAccountAdded} onDone={onDone} />);
  return { onAccountAdded, onDone };
}

async function addAccount() {
  await userEvent.type(screen.getByLabelText("Name"), "Private");
  await userEvent.type(screen.getByLabelText("IBAN"), "AT00 0000 0000 0000 0001");
  await userEvent.click(screen.getByRole("button", { name: "Continue" }));
}

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ importPreview: null, toasts: [] });
  mocked.addAccount.mockResolvedValue(account());
});

describe("FirstRun account step", () => {
  it("starts with step 1 of 2 asking for the first account", () => {
    renderFirstRun();

    expect(screen.getByText("Step 1 of 2")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Add your first account" })).toBeTruthy();
  });

  it("keeps Continue disabled until the account has a name and an IBAN", async () => {
    renderFirstRun();
    const next = screen.getByRole("button", { name: "Continue" });
    expect(next.hasAttribute("disabled")).toBe(true);

    await userEvent.type(screen.getByLabelText("Name"), "Private");
    expect(next.hasAttribute("disabled")).toBe(true);

    await userEvent.type(screen.getByLabelText("IBAN"), "AT00 0000 0000 0000 0001");
    expect(next.hasAttribute("disabled")).toBe(false);
  });

  it("saves the account, tells the app and moves on to the statement step", async () => {
    const { onAccountAdded } = renderFirstRun();

    await addAccount();

    expect(await screen.findByRole("heading", { name: "Import your first statement" })).toBeTruthy();
    expect(mocked.addAccount).toHaveBeenCalledWith(expect.objectContaining({ name: "Private", iban: "AT00 0000 0000 0000 0001", own: true }));
    expect(onAccountAdded).toHaveBeenCalled();
    expect(screen.getByText("Step 2 of 2")).toBeTruthy();
  });

  it("stays on the account step and shows the problem when the account is rejected", async () => {
    mocked.addAccount.mockRejectedValue({ kind: "invalid_input", message: "Invalid input", code: null, field: { path: "account.iban", message: "This IBAN is not valid" } });
    const { onAccountAdded } = renderFirstRun();

    await addAccount();

    expect((await screen.findByRole("alert")).textContent).toBe("This IBAN is not valid");
    expect(onAccountAdded).not.toHaveBeenCalled();
    expect(screen.getByText("Step 1 of 2")).toBeTruthy();
  });
});

describe("FirstRun statement step", () => {
  it("names the open tax year and its rules", async () => {
    mocked.listTaxYears.mockResolvedValue([
      { year: 2025, state: "closed", ruleset: "2025 v2" },
      { year: 2026, state: "open", ruleset: "2026 v1" },
    ]);
    renderFirstRun();

    await addAccount();

    expect(await screen.findByText("Tax year 2026 is ready with the rules 2026 v1.")).toBeTruthy();
  });

  it("mentions no tax year when none is open", async () => {
    mocked.listTaxYears.mockResolvedValue([{ year: 2025, state: "closed", ruleset: "2025 v2" }]);
    renderFirstRun();

    await addAccount();

    await screen.findByRole("heading", { name: "Import your first statement" });
    await waitFor(() => expect(mocked.listTaxYears).toHaveBeenCalled());
    expect(screen.queryByText(/is ready with the rules/)).toBeNull();
  });

  it("finishes on Skip for Now without importing", async () => {
    const { onDone } = renderFirstRun();
    await addAccount();

    await userEvent.click(await screen.findByRole("button", { name: "Skip for Now" }));

    expect(onDone).toHaveBeenCalled();
    expect(mocked.statementImportChoose).not.toHaveBeenCalled();
  });

  it("asks for a statement PDF on Import Statement and opens the Import sheet with what was read", async () => {
    const preview = importPreview();
    mocked.statementImportChoose.mockResolvedValue(preview);
    renderFirstRun();
    await addAccount();

    await userEvent.click(await screen.findByRole("button", { name: "Import Statement…" }));

    await waitFor(() => expect(useStore.getState().importPreview).toEqual(preview));
  });
});
