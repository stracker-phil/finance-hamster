import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { statementFact, statementView } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { StatementPage } from "./StatementPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: { page: "statement", statementId: "st-10" }, back: [], forward: [], toasts: [], pdfCovers: 0 });
});

describe("StatementPage", () => {
  it("shows the statement's title and each fact under its label in a sidebar named after the statement", async () => {
    mocked.statementView.mockResolvedValue(statementView({ facts: [statementFact({ label: "Period", value: "1 Nov 2026 – 10 Nov 2026" }), statementFact({ label: "Closing balance", value: "€1,675.86" })] }));
    renderWithQuery(<StatementPage statementId="st-10" />);

    const side = await screen.findByRole("complementary", { name: "Private · Nr. 010/2026" });
    expect(within(side).getByRole("heading", { name: "Private · Nr. 010/2026" })).toBeTruthy();
    expect(within(side).getByText("Period").nextElementSibling?.textContent).toBe("1 Nov 2026 – 10 Nov 2026");
    expect(within(side).getByText("Closing balance").nextElementSibling?.textContent).toBe("€1,675.86");
    expect(mocked.statementView).toHaveBeenCalledWith("st-10");
  });

  it("shows the statement's PDF beside the facts", async () => {
    mocked.statementView.mockResolvedValue(statementView());
    renderWithQuery(<StatementPage statementId="st-10" />);

    await vi.waitFor(() => expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "statement", id: "st-10" }, expect.anything()));
    expect(await screen.findByText("Select text in the PDF to copy it.")).toBeTruthy();
  });

  it("names the sidebar Statement while the statement loads", () => {
    mocked.statementView.mockReturnValue(new Promise(() => {}));
    renderWithQuery(<StatementPage statementId="st-10" />);

    expect(screen.getByRole("complementary", { name: "Statement" })).toBeTruthy();
  });

  it("offers no way to change the statement", async () => {
    mocked.statementView.mockResolvedValue(statementView());
    renderWithQuery(<StatementPage statementId="st-10" />);

    await screen.findByRole("heading", { name: "Private · Nr. 010/2026" });
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(screen.queryByRole("button", { name: /Delete/ })).toBeNull();
  });

  it("opens the account scrolled to this statement's lines from Show the Lines", async () => {
    mocked.statementView.mockResolvedValue(statementView({ accountId: "acc-2" }));
    renderWithQuery(<StatementPage statementId="st-10" />);

    await userEvent.click(await screen.findByRole("button", { name: "Show the Lines" }));

    expect(useStore.getState().route).toEqual({ page: "account", accountId: "acc-2", statementId: "st-10" });
  });

  it("says the statement was not found, with no facts and no error toast, when it cannot be read", async () => {
    mocked.statementView.mockRejectedValue({ kind: "not_found", message: "No such statement", code: null });
    renderWithQuery(<StatementPage statementId="st-gone" />);

    expect(await screen.findByText("Statement not found")).toBeTruthy();
    expect(screen.getByText("It may have been deleted.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Show the Lines" })).toBeNull();
    expect(useStore.getState().toasts).toEqual([]);
  });
});
