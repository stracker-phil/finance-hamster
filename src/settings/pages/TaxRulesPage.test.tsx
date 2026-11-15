import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type RulesYear } from "../../api";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { useStore } from "../../state/store";
import { renderWithQuery } from "../../test/renderWithQuery";
import { rulesOverview, rulesYear, rulesYearView } from "../../test/rules";
import { TaxRulesPage, describeYear } from "./TaxRulesPage";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => ({ kind: t.kind, message: t.message }));
const button = (name: string) => screen.getByRole("button", { name });

const CLOSED_2025 = rulesYear({ year: 2025, state: "closed", ruleset: "2025 v2" });

function renderPage() {
  return renderWithQuery(
    <>
      <TaxRulesPage />
      <ConfirmHost />
    </>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], confirmation: null });
  mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear(), CLOSED_2025]));
  mocked.rulesYear.mockResolvedValue(rulesYearView());
  mocked.rulesStartNext.mockResolvedValue("Start 2027");
});

describe("describeYear", () => {
  it.each<[string, Partial<RulesYear>, string]>([
    ["an open year on the built-in rules", {}, "Built-in rules 2026 v1 · open"],
    ["an open year on the owner's version", { source: "own", basedOn: "2026 v1" }, "Your version, based on 2026 v1 · open"],
    ["an owner's version without a base", { source: "own", basedOn: null }, "Your version · open"],
    ["a closed year", { state: "closed", ruleset: "2025 v2" }, "Built-in rules 2025 v2 · closed, frozen"],
    ["a year with a newer built-in version", { source: "own", basedOn: "2026 v1", builtinNewer: "2026 v2" }, "Your version, based on 2026 v1 · open · built-in 2026 v2 is newer"],
    ["a year whose newer built-in version can't be taken on", { source: "own", basedOn: "2026 v1", builtinNewer: "2026 v2", problems: ["items[0].kz is missing", "other"] }, "Your version, based on 2026 v1 · open · built-in 2026 v2 can't be taken on: items[0].kz is missing"],
    ["a year with unverified items", { unverified: 3 }, "Built-in rules 2026 v1 · open · 3 unverified"],
  ])("describes %s", (_name, patch, expected) => {
    expect(describeYear(rulesYear(patch))).toBe(expected);
  });
});

describe("TaxRulesPage", () => {
  describe("years", () => {
    it("lists each year with where its rules come from and its state", async () => {
      renderPage();

      expect(await screen.findByText("Built-in rules 2026 v1 · open")).toBeTruthy();
      expect(screen.getByText("Built-in rules 2025 v2 · closed, frozen")).toBeTruthy();
    });

    it("offers Edit for an open year and Show for a closed one", async () => {
      renderPage();
      await screen.findByText("Built-in rules 2026 v1 · open");

      expect(screen.getAllByRole("button", { name: "Edit" })).toHaveLength(1);
      expect(screen.getAllByRole("button", { name: "Show" })).toHaveLength(1);
    });

    it("opens the year's rules sheet when Edit is clicked", async () => {
      renderPage();
      await screen.findByText("Built-in rules 2026 v1 · open");

      await userEvent.click(button("Edit"));

      const sheet = await screen.findByRole("dialog", { name: "Tax Rules 2026" });
      expect(await within(sheet).findByLabelText("Home office limit")).toBeTruthy();
      expect(mocked.rulesYear).toHaveBeenCalledWith(2026);
    });

    it("opens a closed year's rules sheet when Show is clicked", async () => {
      renderPage();
      await screen.findByText("Built-in rules 2026 v1 · open");

      await userEvent.click(button("Show"));

      expect(await screen.findByRole("dialog", { name: "Tax Rules 2025" })).toBeTruthy();
      expect(mocked.rulesYear).toHaveBeenCalledWith(2025);
    });

    it("closes the sheet when Done is clicked", async () => {
      renderPage();
      await screen.findByText("Built-in rules 2026 v1 · open");
      await userEvent.click(button("Edit"));
      await screen.findByRole("dialog", { name: "Tax Rules 2026" });

      await userEvent.click(button("Done"));

      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  describe("start", () => {
    it("offers no Start row when there is no next year to begin", async () => {
      renderPage();
      await screen.findByText("Built-in rules 2026 v1 · open");

      expect(screen.queryByRole("button", { name: "Start…" })).toBeNull();
    });

    it("says it copies the newest year's rules when the next year has no built-in rules", async () => {
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear()], { nextYear: 2027, nextIsBuiltin: false }));
      renderPage();

      expect(await screen.findByText("Start 2027")).toBeTruthy();
      expect(screen.getByText("Copies the rules of 2026. Check every value against the new year's forms.")).toBeTruthy();
    });

    it("says it uses the built-in rules when the next year has some", async () => {
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear()], { nextYear: 2027, nextIsBuiltin: true }));
      renderPage();

      expect(await screen.findByText("Uses the rules built into Hamster.")).toBeTruthy();
    });

    describe("with a next year", () => {
      beforeEach(() => {
        mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear()], { nextYear: 2027, nextIsBuiltin: true }));
      });

      it("asks before starting and starts the next year once confirmed", async () => {
        renderPage();
        await userEvent.click(await screen.findByRole("button", { name: "Start…" }));

        const confirm = await screen.findByRole("alertdialog", { name: "Start 2027" });
        expect(mocked.rulesStartNext).not.toHaveBeenCalled();
        await userEvent.click(within(confirm).getByRole("button", { name: "Start" }));

        await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Start 2027" }]));
        expect(mocked.rulesStartNext).toHaveBeenCalled();
      });

      it("starts nothing when the question is cancelled", async () => {
        renderPage();
        await userEvent.click(await screen.findByRole("button", { name: "Start…" }));

        const confirm = await screen.findByRole("alertdialog");
        await userEvent.click(within(confirm).getByRole("button", { name: "Cancel" }));

        expect(screen.queryByRole("alertdialog")).toBeNull();
        expect(mocked.rulesStartNext).not.toHaveBeenCalled();
      });

      it("shows the error as a toast when starting fails", async () => {
        mocked.rulesStartNext.mockRejectedValue({ kind: "invalid_input", message: "No rules for 2027", code: null });
        renderPage();
        await userEvent.click(await screen.findByRole("button", { name: "Start…" }));

        await userEvent.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Start" }));

        await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "No rules for 2027" }]));
      });
    });
  });
});
