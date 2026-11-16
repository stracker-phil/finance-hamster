import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { YearStepper } from "./YearStepper";

function stepper(year: number, years: number[], onChange = vi.fn()) {
  render(<YearStepper year={year} years={years} label="Year" previousLabel="Previous Year" nextLabel="Next Year" onChange={onChange} />);
  return {
    onChange,
    previous: screen.getByRole<HTMLButtonElement>("button", { name: "Previous Year" }),
    next: screen.getByRole<HTMLButtonElement>("button", { name: "Next Year" }),
  };
}

describe("YearStepper", () => {
  it("shows the year between its two arrows in a labelled group", () => {
    stepper(2026, [2025, 2026, 2027]);

    const group = screen.getByRole("group", { name: "Year" });
    expect(within(group).getByText("2026")).toBeTruthy();
    expect(within(group).getAllByRole("button")).toHaveLength(2);
  });

  it("steps to the year before and the year after", async () => {
    const { previous, next, onChange } = stepper(2026, [2025, 2026, 2027]);

    await userEvent.click(previous);
    await userEvent.click(next);

    expect(onChange.mock.calls).toEqual([[2025], [2027]]);
  });

  it("skips the gaps in its years", async () => {
    const { previous, next, onChange } = stepper(2026, [2020, 2023, 2026, 2029, 2031]);

    await userEvent.click(previous);
    await userEvent.click(next);

    expect(onChange.mock.calls).toEqual([[2023], [2029]]);
  });

  it("steps from a year that is not in its list to its neighbours", async () => {
    const { previous, next, onChange } = stepper(2026, [2024, 2028]);

    await userEvent.click(previous);
    await userEvent.click(next);

    expect(onChange.mock.calls).toEqual([[2024], [2028]]);
  });

  it("disables the previous arrow at the oldest year but keeps it", async () => {
    const { previous, next, onChange } = stepper(2025, [2025, 2026]);

    expect(previous.disabled).toBe(true);
    expect(next.disabled).toBe(false);
    await userEvent.click(previous);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("disables the next arrow at the newest year but keeps it", async () => {
    const { previous, next, onChange } = stepper(2026, [2025, 2026]);

    expect(next.disabled).toBe(true);
    expect(previous.disabled).toBe(false);
    await userEvent.click(next);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("disables both arrows when its year is the only one", () => {
    const { previous, next } = stepper(2026, [2026]);

    expect(previous.disabled).toBe(true);
    expect(next.disabled).toBe(true);
  });
});
