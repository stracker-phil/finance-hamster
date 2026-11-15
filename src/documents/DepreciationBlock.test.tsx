import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { depreciationView } from "../test/documents";
import { choose, popUpValue, withMenus } from "../test/popup";
import { DepreciationBlock } from "./DepreciationBlock";

const renderBlock = (patch: Parameters<typeof depreciationView>[0] = {}, disabled = false) => {
  const onYears = vi.fn();
  render(withMenus(<DepreciationBlock depreciation={depreciationView(patch)} disabled={disabled} onYears={onYears} />));
  return onYears;
};

describe("DepreciationBlock", () => {
  it("shows the heading, the rule beside the useful life and why the purchase is spread", () => {
    renderBlock();

    expect(screen.getByRole("heading", { name: "Depreciation" })).toBeTruthy();
    expect(screen.getByText("3 years for computer in ruleset 2026 v1")).toBeTruthy();
    expect(screen.getByText("Why")).toBeTruthy();
    expect(screen.getByText("€1,500.00 is above the GWG limit, so it is spread over its useful life")).toBeTruthy();
  });

  describe("useful life", () => {
    it("shows As the rules say while the years are the rule's", () => {
      renderBlock({ own: false });

      expect(popUpValue("Useful life")).toBe("As the rules say");
    });

    it("shows the document's own years once they differ from the rule", () => {
      renderBlock({ own: true, years: 5 });

      expect(popUpValue("Useful life")).toBe("5 years");
    });

    it("uses the singular for one year", () => {
      renderBlock({ own: true, years: 1 });

      expect(popUpValue("Useful life")).toBe("1 year");
    });

    it("reports the chosen number of years", async () => {
      const onYears = renderBlock();

      await choose(userEvent, "Useful life", "5 years");

      expect(onYears).toHaveBeenCalledWith(5);
    });

    it("reports null when As the rules say is chosen after own years", async () => {
      const onYears = renderBlock({ own: true, years: 5 });

      await choose(userEvent, "Useful life", "As the rules say");

      expect(onYears).toHaveBeenCalledWith(null);
    });

    it("reports nothing when the shown choice is chosen again", async () => {
      const onYears = renderBlock({ own: true, years: 5 });

      await choose(userEvent, "Useful life", "5 years");

      expect(onYears).not.toHaveBeenCalled();
    });

    it("reports nothing when As the rules say is chosen again while it is shown", async () => {
      const onYears = renderBlock({ own: false });

      await choose(userEvent, "Useful life", "As the rules say");

      expect(onYears).not.toHaveBeenCalled();
    });

    it("is disabled when the document is locked", () => {
      renderBlock({}, true);

      expect(screen.getByRole("button", { name: "Useful life" })).toBeDisabled();
    });
  });

  describe("slices", () => {
    it("lists one slice per year with its amount and note", () => {
      renderBlock();

      const items = within(screen.getByRole("list", { name: "Slices" })).getAllByRole("listitem");
      expect(items.map((li) => li.textContent)).toEqual(["2026€250.00half year", "2027€500.00", "2028€500.00", "2029€250.00"]);
    });

    it("shows the frozen note on a slice of a closed year", () => {
      renderBlock({
        slices: [
          { year: 2026, amount: "€250.00", note: "frozen", frozen: true },
          { year: 2027, amount: "€500.00", note: null, frozen: false },
        ],
      });

      const items = within(screen.getByRole("list", { name: "Slices" })).getAllByRole("listitem");
      expect(items[0].textContent).toBe("2026€250.00frozen");
      expect(items[1].textContent).toBe("2027€500.00");
    });

    it("still renders a frozen slice whose note is German", () => {
      renderBlock({ slices: [{ year: 2026, amount: "€250.00", note: "eingefroren", frozen: true }] });

      const [item] = within(screen.getByRole("list", { name: "Slices" })).getAllByRole("listitem");
      expect(item.textContent).toBe("2026€250.00eingefroren");
    });
  });
});
