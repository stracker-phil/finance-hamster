import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { setLanguage } from "../i18n";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { addPreview } from "../test/documents";
import { accountCoverage, coverage } from "../test/statements";
import { Sidebar } from "./Sidebar";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], addPreview: null });
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

  it("counts the lines to review regardless of the header filter", async () => {
    useStore.setState({ filter: "market" });

    render(<Sidebar years={[]} />);

    await screen.findByRole("button", { name: "Statements" });
    expect(mocked.statementsCoverage).toHaveBeenCalledWith("");
    expect(mocked.statementsCoverage).not.toHaveBeenCalledWith("market");
  });

  it("shows no badge when no line waits for review", async () => {
    render(<Sidebar years={[]} />);

    await screen.findByRole("button", { name: "Statements" });
    expect(screen.queryByLabelText(/to review/)).toBeNull();
  });

  it("starts adding an invoice when Choose File… in the drop zone is clicked", async () => {
    const preview = addPreview();
    mocked.documentAddChoose.mockResolvedValue(preview);
    render(<Sidebar years={[]} />);

    const choose = await screen.findByRole("button", { name: "Choose File…" });
    expect(choose).toBeEnabled();
    await userEvent.click(choose);

    await waitFor(() => expect(useStore.getState().addPreview).toEqual(preview));
  });

  describe("in German", () => {
    beforeEach(() => setLanguage("de"));

    it.each(["Heute", "Abos", "Kontoauszüge", "Belege"])("names the %s page in German", async (name) => {
      render(<Sidebar years={[]} />);

      expect(await screen.findByRole("button", { name })).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Statements" })).toBeNull();
    });

    it("writes the badge, the tax years and the drop zone in German", async () => {
      mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ toReview: 3 })] }));

      render(<Sidebar years={[{ year: 2026, closed: false }]} />);

      expect(await screen.findByLabelText("3 zu prüfen")).toBeTruthy();
      expect(screen.getByRole("button", { name: /Steuerjahr 2026/ })).toBeTruthy();
      expect(screen.getByText("Rechnungs-PDF hier ablegen")).toBeTruthy();
      expect(screen.getByRole("button", { name: "Datei auswählen …" })).toBeTruthy();
    });
  });
});
