import { act, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { account } from "../test/accounts";
import { renderWithQuery } from "../test/renderWithQuery";
import { useAccounts } from "./accounts";
import { useTaxYears } from "./taxYears";
import { useEvents } from "./useEvents";

const mocked = vi.mocked(api, true);

function Probe() {
  useEvents();
  const { data: accounts } = useAccounts();
  const { data: years } = useTaxYears();
  return (
    <>
      <p>Accounts: {accounts?.map((a) => a.name).join(", ")}</p>
      <p>Years: {years?.map((y) => y.year).join(", ")}</p>
    </>
  );
}

async function emitDataChanged(tables: string[]) {
  await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "data_changed")).toBe(true));
  const handler = mocked.listen.mock.calls.find(([name]) => name === "data_changed")![1] as (payload: { tables: string[] }) => void;
  await act(async () => handler({ tables }));
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.listen.mockResolvedValue(() => {});
  mocked.listAccounts.mockResolvedValue([account({ name: "Private" })]);
  mocked.listTaxYears.mockResolvedValue([{ year: 2026, state: "open", ruleset: "2026 v1" }]);
});

describe("useEvents", () => {
  it("refetches the accounts when data_changed names the accounts table", async () => {
    renderWithQuery(<Probe />);
    await screen.findByText("Accounts: Private");
    mocked.listAccounts.mockResolvedValue([account({ name: "Private" }), account({ id: "acc-2", name: "Savings" })]);

    await emitDataChanged(["accounts"]);

    expect(await screen.findByText("Accounts: Private, Savings")).toBeTruthy();
    expect(screen.getByText("Years: 2026")).toBeTruthy();
  });

  it("refetches the tax years when data_changed names the tax_years table", async () => {
    renderWithQuery(<Probe />);
    await screen.findByText("Years: 2026");
    mocked.listTaxYears.mockResolvedValue([
      { year: 2027, state: "open", ruleset: "2027 v1" },
      { year: 2026, state: "closed", ruleset: "2026 v1" },
    ]);

    await emitDataChanged(["tax_years"]);

    expect(await screen.findByText("Years: 2027, 2026")).toBeTruthy();
    expect(screen.getByText("Accounts: Private")).toBeTruthy();
  });

  it("ignores tables it shows nothing of", async () => {
    renderWithQuery(<Probe />);
    await screen.findByText("Accounts: Private");
    mocked.listAccounts.mockResolvedValue([]);
    mocked.listTaxYears.mockResolvedValue([]);

    await emitDataChanged(["documents"]);

    expect(screen.getByText("Accounts: Private")).toBeTruthy();
    expect(screen.getByText("Years: 2026")).toBeTruthy();
  });
});
