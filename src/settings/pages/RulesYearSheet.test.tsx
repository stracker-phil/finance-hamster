import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { renderWithQuery } from "../../test/renderWithQuery";
import { rulesField, rulesOverview, rulesYear, rulesYearView } from "../../test/rules";
import { categoryChoice } from "../../test/year";
import { RulesYearSheet } from "./RulesYearSheet";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => ({ kind: t.kind, message: t.message }));
const button = (name: string) => screen.getByRole("button", { name });

async function renderSheet(onClose = vi.fn()) {
  renderWithQuery(<RulesYearSheet year={2026} onClose={onClose} />);
  await screen.findByRole("dialog", { name: "Tax Rules 2026" });
  return onClose;
}

const limit = () => screen.findByLabelText<HTMLInputElement>("Home office limit");

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
  mocked.rulesYear.mockResolvedValue(rulesYearView());
  mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear()]));
  mocked.rulesSetValue.mockResolvedValue("Change rules value");
  mocked.rulesUseBuiltin.mockResolvedValue("Use built-in rules");
  mocked.rulesJson.mockResolvedValue("demo-json");
  mocked.rulesPreviewJson.mockResolvedValue({ problems: [], changes: [] });
  mocked.rulesSetJson.mockResolvedValue("Edit rules JSON");
  mocked.yearCategories.mockResolvedValue([]);
  mocked.yearSetCategoryShown.mockResolvedValue(null);
});

describe("RulesYearSheet", () => {
  describe("contents", () => {
    it("says where the year's rules come from and lists the groups, facts and sources", async () => {
      mocked.rulesYear.mockResolvedValue(
        rulesYearView({ groups: [{ title: "Home office", fields: [rulesField()], facts: ["Applies per calendar year"] }], sources: ["Example Tax Act §16"] }),
      );
      await renderSheet();

      expect(await screen.findByText("Built-in rules 2026 v1")).toBeTruthy();
      const group = screen.getByRole("region", { name: "Home office" });
      expect((await within(group).findByLabelText<HTMLInputElement>("Home office limit")).value).toBe("300");
      expect(within(group).getByText("Applies per calendar year")).toBeTruthy();
      expect(within(screen.getByRole("region", { name: "Sources" })).getByText("Example Tax Act §16")).toBeTruthy();
    });

    it("names the version the owner's rules are based on", async () => {
      mocked.rulesYear.mockResolvedValue(rulesYearView({ source: "own", basedOn: "2026 v1" }));
      await renderSheet();

      expect(await screen.findByText("Your version, based on 2026 v1")).toBeTruthy();
    });

    it("lists a switch per category of the year and saves a toggle for that year", async () => {
      mocked.yearCategories.mockResolvedValue([categoryChoice(), categoryChoice({ key: "union", label: "Union dues", examples: null, shown: false })]);
      await renderSheet();

      const group = within(await screen.findByRole("region", { name: "Categories" }));
      expect((await group.findByRole<HTMLInputElement>("switch", { name: "Work books" })).checked).toBe(true);
      expect(group.getByRole<HTMLInputElement>("switch", { name: "Union dues" }).checked).toBe(false);
      expect(mocked.yearCategories).toHaveBeenCalledWith(2026);

      await userEvent.click(group.getByRole("switch", { name: "Union dues" }));

      expect(mocked.yearSetCategoryShown).toHaveBeenCalledWith(2026, "union", true);
    });

    it("shows each field's unit", async () => {
      await renderSheet();
      await limit();

      expect(screen.getByText("EUR")).toBeTruthy();
    });
  });

  describe("changing a value", () => {
    it("saves the new value with its path when the field loses focus, and says what changed", async () => {
      await renderSheet();
      const field = await limit();

      await userEvent.clear(field);
      await userEvent.type(field, "350");
      await userEvent.tab();

      await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Change rules value" }]));
      expect(mocked.rulesSetValue).toHaveBeenCalledWith(2026, "homeOffice/limit", "350");
    });

    it("saves on Enter", async () => {
      await renderSheet();
      const field = await limit();

      await userEvent.clear(field);
      await userEvent.type(field, "350{Enter}");

      await waitFor(() => expect(mocked.rulesSetValue).toHaveBeenCalledWith(2026, "homeOffice/limit", "350"));
    });

    it("saves nothing when the value is unchanged", async () => {
      await renderSheet();
      const field = await limit();

      await userEvent.click(field);
      await userEvent.tab();

      expect(mocked.rulesSetValue).not.toHaveBeenCalled();
      expect(toasts()).toEqual([]);
    });

    it("puts the old value back and shows the error when saving fails", async () => {
      mocked.rulesSetValue.mockRejectedValue({ kind: "invalid_input", message: "Not an amount", code: null });
      await renderSheet();
      const field = await limit();

      await userEvent.clear(field);
      await userEvent.type(field, "abc");
      await userEvent.tab();

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "Not an amount" }]));
      expect(field.value).toBe("300");
    });
  });

  describe("of a closed year", () => {
    beforeEach(() => {
      mocked.rulesYear.mockResolvedValue(rulesYearView({ editable: false }));
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear({ state: "closed", source: "own", basedOn: "2026 v1" })]));
    });

    it("shows the values read-only", async () => {
      await renderSheet();

      expect((await limit()).readOnly).toBe(true);
    });

    it("offers neither Advanced nor a way back to the built-in rules", async () => {
      await renderSheet();
      await limit();

      expect(screen.queryByText("Advanced")).toBeNull();
      expect(screen.queryByRole("button", { name: "Back to the Built-in Rules" })).toBeNull();
    });
  });

  describe("built-in rules", () => {
    it("takes the year back to the built-in rules and says so", async () => {
      mocked.rulesYear.mockResolvedValue(rulesYearView({ source: "own", basedOn: "2026 v1" }));
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear({ source: "own", basedOn: "2026 v1" })]));
      await renderSheet();

      await userEvent.click(await screen.findByRole("button", { name: "Back to the Built-in Rules" }));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Use built-in rules" }]));
      expect(mocked.rulesUseBuiltin).toHaveBeenCalledWith(2026);
    });

    it("names the newer built-in version it takes on", async () => {
      mocked.rulesYear.mockResolvedValue(rulesYearView({ source: "own", basedOn: "2026 v1" }));
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear({ source: "own", basedOn: "2026 v1", builtinNewer: "2026 v2" })]));
      await renderSheet();

      await userEvent.click(await screen.findByRole("button", { name: "Use Built-in 2026 v2" }));

      expect(mocked.rulesUseBuiltin).toHaveBeenCalledWith(2026);
    });

    it("shows the error when going back fails", async () => {
      mocked.rulesYear.mockResolvedValue(rulesYearView({ source: "own" }));
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear({ source: "own" })]));
      mocked.rulesUseBuiltin.mockRejectedValue({ kind: "invalid_input", message: "Built-in rules can't be used", code: null });
      await renderSheet();

      await userEvent.click(await screen.findByRole("button", { name: "Back to the Built-in Rules" }));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "Built-in rules can't be used" }]));
    });

    it.each([
      ["a year already on the built-in rules", rulesYearView(), rulesYear()],
      ["an owner's year without built-in rules", rulesYearView({ source: "own" }), rulesYear({ source: "own", hasBuiltin: false })],
    ])("offers no way back for %s", async (_name, view, year) => {
      mocked.rulesYear.mockResolvedValue(view);
      mocked.rulesOverview.mockResolvedValue(rulesOverview([year]));
      await renderSheet();
      await limit();
      await waitFor(() => expect(mocked.rulesOverview).toHaveBeenCalled());

      expect(screen.queryByRole("button", { name: /Back to the Built-in Rules|Use Built-in/ })).toBeNull();
    });
  });

  describe("Edit as JSON", () => {
    const openJson = async () => {
      await renderSheet();
      await limit();
      await userEvent.click(screen.getByText("Advanced"));
      await userEvent.click(button("Edit as JSON…"));
      const textarea = await screen.findByRole<HTMLTextAreaElement>("textbox", { name: "Tax Rules 2026 as JSON" });
      await waitFor(() => expect(textarea.value).toBe("demo-json"));
      return textarea;
    };

    beforeEach(() => {
      mocked.rulesJson.mockResolvedValue("demo-json");
    });

    it("shows the year's JSON with Save disabled until it was checked", async () => {
      await openJson();

      expect(mocked.rulesJson).toHaveBeenCalledWith(2026);
      expect(button("Save")).toBeDisabled();
    });

    it("lists the changes of a clean check and enables Save", async () => {
      mocked.rulesPreviewJson.mockResolvedValue({ problems: [], changes: ["Home office limit 300 → 350"] });
      await openJson();

      await userEvent.click(button("Check"));

      const changes = await screen.findByRole("region", { name: "Changes" });
      expect(within(changes).getByText("Home office limit 300 → 350")).toBeTruthy();
      expect(button("Save")).toBeEnabled();
      expect(mocked.rulesPreviewJson).toHaveBeenCalledWith(2026, "demo-json");
    });

    it("says nothing changed for a clean check without changes", async () => {
      await openJson();

      await userEvent.click(button("Check"));

      expect(await screen.findByText("Nothing changed.")).toBeTruthy();
      expect(button("Save")).toBeEnabled();
    });

    it("lists the problems of a check and keeps Save disabled", async () => {
      mocked.rulesPreviewJson.mockResolvedValue({ problems: ["items[0].kz is missing"], changes: [] });
      await openJson();

      await userEvent.click(button("Check"));

      const problems = await screen.findByRole("region", { name: "Problems" });
      expect(within(problems).getByText("items[0].kz is missing")).toBeTruthy();
      expect(button("Save")).toBeDisabled();
    });

    it("disables Save again when the text changes after a check", async () => {
      const textarea = await openJson();
      await userEvent.click(button("Check"));
      await waitFor(() => expect(button("Save")).toBeEnabled());

      await userEvent.type(textarea, "x");

      expect(button("Save")).toBeDisabled();
      expect(screen.queryByText("Nothing changed.")).toBeNull();
    });

    it("saves the checked text, says what changed and returns to the year's rules", async () => {
      const textarea = await openJson();
      await userEvent.type(textarea, "x");
      await userEvent.click(button("Check"));
      await waitFor(() => expect(button("Save")).toBeEnabled());

      await userEvent.click(button("Save"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Edit rules JSON" }]));
      expect(mocked.rulesSetJson).toHaveBeenCalledWith(2026, "demo-jsonx");
      expect(screen.queryByRole("dialog", { name: "Tax Rules 2026 as JSON" })).toBeNull();
      expect(screen.getByRole("dialog", { name: "Tax Rules 2026" })).toBeTruthy();
    });

    it("keeps the sheet open and shows the error when saving fails", async () => {
      mocked.rulesSetJson.mockRejectedValue({ kind: "invalid_input", message: "Rules are not valid", code: null });
      await openJson();
      await userEvent.click(button("Check"));
      await waitFor(() => expect(button("Save")).toBeEnabled());

      await userEvent.click(button("Save"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "Rules are not valid" }]));
      expect(screen.getByRole("dialog", { name: "Tax Rules 2026 as JSON" })).toBeTruthy();
    });

    it("returns to the year's rules on Cancel without saving", async () => {
      await openJson();

      await userEvent.click(button("Cancel"));

      expect(screen.getByRole("dialog", { name: "Tax Rules 2026" })).toBeTruthy();
      expect(mocked.rulesSetJson).not.toHaveBeenCalled();
    });
  });

  it("calls onClose when Done is clicked", async () => {
    const onClose = await renderSheet();

    await userEvent.click(button("Done"));

    expect(onClose).toHaveBeenCalled();
  });
});
