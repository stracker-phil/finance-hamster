import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { useStore } from "../../state/store";
import { ruleForm, ruleView, rulesView } from "../../test/lineRules";
import { renderWithQuery } from "../../test/renderWithQuery";
import { RulesPage } from "./RulesPage";

const mocked = vi.mocked(api, true);
const messages = () => useStore.getState().toasts.map((t) => t.message);

const renderPage = () =>
  renderWithQuery(
    <>
      <RulesPage />
      <ConfirmHost />
    </>,
  );

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], confirmation: null });
  mocked.rulesView.mockResolvedValue(rulesView());
});

describe("RulesPage", () => {
  describe("rule list", () => {
    it("lists each group with its rules, their detail and an Edit… button", async () => {
      mocked.rulesView.mockResolvedValue(
        rulesView({
          groups: [
            { title: "Ignore", rules: [ruleView()] },
            { title: "Assign item", rules: [ruleView({ id: "rule-2", label: "Bank text contains CLOUDBOX", detail: "→ assign item KZ 169 · 10 matches" })] },
          ],
        }),
      );
      renderPage();

      expect(await screen.findByRole("heading", { name: "Ignore" })).toBeTruthy();
      expect(screen.getByRole("heading", { name: "Assign item" })).toBeTruthy();
      expect(screen.getByText("Bank text contains ALPENMARKT")).toBeTruthy();
      expect(screen.getByText("→ ignore · 3 matches · last 7 Nov")).toBeTruthy();
      expect(screen.getByText("→ assign item KZ 169 · 10 matches")).toBeTruthy();
      expect(screen.getAllByRole("button", { name: "Edit…" })).toHaveLength(2);
    });

    it("shows only the Add group when there are no rules", async () => {
      mocked.rulesView.mockResolvedValue(rulesView({ groups: [] }));
      renderPage();

      expect(await screen.findByRole("button", { name: "New Rule…" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Re-run…" })).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Edit…" })).toBeNull();
    });
  });

  describe("sheets", () => {
    it("opens an empty New Rule sheet from New Rule…", async () => {
      renderPage();

      await userEvent.click(await screen.findByRole("button", { name: "New Rule…" }));

      expect(screen.getByRole("dialog", { name: "New Rule" })).toBeTruthy();
      expect(screen.getByLabelText<HTMLInputElement>("Bank text contains").value).toBe("");
    });

    it("opens the Edit Rule sheet prefilled from the rule whose Edit… is clicked", async () => {
      mocked.rulesView.mockResolvedValue(rulesView({ groups: [{ title: "Ignore", rules: [ruleView({ form: ruleForm({ contains: ["BILLA", "SPAR"] }) })] }] }));
      renderPage();

      await userEvent.click(await screen.findByRole("button", { name: "Edit…" }));

      expect(screen.getByRole("dialog", { name: "Edit Rule" })).toBeTruthy();
      expect(screen.getByLabelText<HTMLInputElement>("Bank text contains").value).toBe("BILLA, SPAR");
    });

    it("closes the sheet on Cancel", async () => {
      renderPage();
      await userEvent.click(await screen.findByRole("button", { name: "New Rule…" }));

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  describe("Re-run…", () => {
    it("asks first, then applies the rules and toasts the result", async () => {
      mocked.rulesRerun.mockResolvedValue("Rules applied · 4 lines changed");
      renderPage();
      await userEvent.click(await screen.findByRole("button", { name: "Re-run…" }));
      expect(mocked.rulesRerun).not.toHaveBeenCalled();
      expect(screen.getByRole("alertdialog", { name: "Apply the rules again?" })).toBeTruthy();

      await userEvent.click(screen.getByRole("button", { name: "Re-run" }));

      await vi.waitFor(() => expect(messages()).toEqual(["Rules applied · 4 lines changed"]));
    });

    it("does nothing when the question is cancelled", async () => {
      renderPage();
      await userEvent.click(await screen.findByRole("button", { name: "Re-run…" }));

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(mocked.rulesRerun).not.toHaveBeenCalled();
      expect(messages()).toEqual([]);
    });

    it("toasts the error when the re-run fails", async () => {
      mocked.rulesRerun.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderPage();
      await userEvent.click(await screen.findByRole("button", { name: "Re-run…" }));

      await userEvent.click(screen.getByRole("button", { name: "Re-run" }));

      await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", "Disk full"]]));
    });
  });
});
