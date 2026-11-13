import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { accountCoverage, coverage } from "../test/statements";
import { Sidebar } from "./Sidebar";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [] });
  mocked.statementsCoverage.mockResolvedValue(coverage());
});

describe("Sidebar", () => {
  it("badges Statements with the lines to review summed over all accounts", async () => {
    mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ toReview: 3 }), accountCoverage({ id: "acc-2", toReview: 4 })] }));

    render(<Sidebar years={[]} />);

    const badge = await screen.findByLabelText("7 to review");
    expect(badge.textContent).toBe("7");
    expect(screen.getByRole("button", { name: /Statements/ }).contains(badge)).toBe(true);
  });

  it("shows no badge when no line waits for review", async () => {
    render(<Sidebar years={[]} />);

    await screen.findByRole("button", { name: "Statements" });
    expect(screen.queryByLabelText(/to review/)).toBeNull();
  });
});
