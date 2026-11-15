import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { reviewLine } from "../test/review";
import { SourceLine } from "./SourceLine";

afterEach(() => {
  window.getSelection()!.removeAllRanges();
});

describe("SourceLine", () => {
  it("shows the account, date, title, amount and raw text of the bank line", () => {
    render(<SourceLine line={reviewLine({ accountName: "Private", date: "Tue, 10 Nov", title: "Example Bookshop", amount: "−€34.90", raw: "Example Bookshop Vienna\nBooking text 4711" })} />);

    const line = screen.getByLabelText("Bank line");
    expect(line.textContent).toContain("Private · Tue, 10 Nov");
    expect(line.textContent).toContain("Example Bookshop");
    expect(line.textContent).toContain("−€34.90");
    expect(line.textContent).toContain("Booking text 4711");
  });

  it("hands selected text over upper-cased and on one line", () => {
    const onSelect = vi.fn();
    render(<SourceLine line={reviewLine({ raw: "Lumen Ai\nContract 4711" })} onSelect={onSelect} />);
    const raw = screen.getByText(/Contract 4711/);
    window.getSelection()!.selectAllChildren(raw);

    fireEvent.mouseUp(raw);

    expect(onSelect).toHaveBeenCalledWith("LUMEN AI CONTRACT 4711");
  });

  it("hands nothing over for a mouse release without a selection", () => {
    const onSelect = vi.fn();
    render(<SourceLine line={reviewLine()} onSelect={onSelect} />);

    fireEvent.mouseUp(screen.getByText(/Booking text 4711/));

    expect(onSelect).not.toHaveBeenCalled();
  });

  it("ignores a selection when no handler is given", () => {
    render(<SourceLine line={reviewLine()} />);
    const raw = screen.getByText(/Booking text 4711/);
    window.getSelection()!.selectAllChildren(raw);

    expect(() => fireEvent.mouseUp(raw)).not.toThrow();
  });
});
