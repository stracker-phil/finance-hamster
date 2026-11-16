import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DocumentsTabs } from "../documents/DocumentsTabs";
import { HOME, type Route } from "../state/route";
import { useStore } from "../state/store";
import { useWindowKeys } from "./useWindowKeys";

function Harness({ children }: { children?: ReactNode }) {
  useWindowKeys();
  return (
    <div className="panel">
      {children}
      <section className="page" aria-label="Page" />
    </div>
  );
}

function Rows({ rows }: { rows: { className: string; name: string; disabled?: boolean }[] }) {
  useWindowKeys();
  return (
    <div className="panel">
      <section className="page" aria-label="Page">
        {rows.map((r) => (
          <button key={r.name} className={r.className} disabled={r.disabled}>
            {r.name}
          </button>
        ))}
      </section>
    </div>
  );
}

const payslip: Route = { page: "payslip", payslipId: "ps-1", year: 2026 };
const route = () => useStore.getState().route;

beforeEach(() => {
  Element.prototype.scrollIntoView = vi.fn();
  useStore.setState({ route: HOME, back: [], forward: [], documentsTab: "invoices", paletteOpen: false, menu: null });
});

describe("Esc", () => {
  it("goes back to where the page was opened from when there is history", async () => {
    useStore.setState({ route: { page: "year", year: 2026 } });
    useStore.getState().navigate(payslip);
    render(<Harness />);

    await userEvent.keyboard("{Escape}");

    expect(route()).toEqual({ page: "year", year: 2026 });
  });

  it("goes to the parent page when there is no history", async () => {
    useStore.setState({ route: payslip });
    render(<Harness />);

    await userEvent.keyboard("{Escape}");

    expect(route()).toEqual({ page: "documents", tab: "payslips" });
  });

  it("does nothing on a sidebar page", async () => {
    useStore.setState({ route: { page: "subscriptions" } });
    render(<Harness />);

    await userEvent.keyboard("{Escape}");

    expect(route()).toEqual({ page: "subscriptions" });
  });

  it("only leaves a focused field first, and goes back on the second press", async () => {
    useStore.setState({ route: payslip });
    render(
      <Harness>
        <input aria-label="Note" />
      </Harness>,
    );
    const field = screen.getByRole("textbox", { name: "Note" });
    field.focus();

    await userEvent.keyboard("{Escape}");

    expect(document.activeElement).not.toBe(field);
    expect(route()).toEqual(payslip);

    await userEvent.keyboard("{Escape}");

    expect(route()).toEqual({ page: "documents", tab: "payslips" });
  });

  it.each([
    ["a sheet", () => <div className="overlay" />, () => {}],
    ["the palette", () => null, () => useStore.setState({ paletteOpen: true })],
    ["a menu", () => null, () => useStore.setState({ menu: { id: 1, entries: [], at: { x: 0, y: 0 }, label: "Menu", opener: null } })],
  ])("does nothing while %s is open", async (_name, cover, open) => {
    useStore.setState({ route: payslip });
    open();
    render(<Harness>{cover()}</Harness>);

    await userEvent.keyboard("{Escape}");

    expect(route()).toEqual(payslip);
  });
});

describe("⌥1 to ⌥9", () => {
  it("opens the segment of the first segmented control that the digit names", async () => {
    render(
      <Harness>
        <DocumentsTabs />
      </Harness>,
    );

    await userEvent.keyboard("{Alt>}3{/Alt}");

    expect(route()).toEqual({ page: "documents", tab: "payslips" });
  });

  it("does nothing for a digit beyond the last segment", async () => {
    render(
      <Harness>
        <DocumentsTabs />
      </Harness>,
    );

    await userEvent.keyboard("{Alt>}9{/Alt}");

    expect(route()).toEqual(HOME);
  });

  it("picks from a year switch as well", async () => {
    const picked: string[] = [];
    render(
      <Harness>
        <div className="year-switch">
          <button className="seg-btn" onClick={() => picked.push("first")} />
          <button className="seg-btn" onClick={() => picked.push("second")} />
        </div>
      </Harness>,
    );

    await userEvent.keyboard("{Alt>}2{/Alt}");

    expect(picked).toEqual(["second"]);
  });

  it("does nothing while typing in a field", async () => {
    render(
      <Harness>
        <DocumentsTabs />
        <input aria-label="Note" />
      </Harness>,
    );
    screen.getByRole("textbox", { name: "Note" }).focus();

    await userEvent.keyboard("{Alt>}3{/Alt}");

    expect(route()).toEqual(HOME);
  });
});

describe("row keys", () => {
  const rows = [
    { className: "docs-row", name: "First" },
    { className: "sub-row", name: "Second" },
    { className: "year-row", name: "Third", disabled: true },
    { className: "today-row", name: "Fourth" },
  ];
  const focused = () => document.activeElement?.textContent;

  it.each([
    ["↓", "{ArrowDown}"],
    ["J", "j"],
  ])("takes the first row on %s, then the next, skipping disabled rows", async (_name, key) => {
    render(<Rows rows={rows} />);

    await userEvent.keyboard(key);
    expect(focused()).toBe("First");
    await userEvent.keyboard(key);
    expect(focused()).toBe("Second");
    await userEvent.keyboard(key);
    expect(focused()).toBe("Fourth");
  });

  it.each([
    ["↑", "{ArrowUp}"],
    ["K", "k"],
  ])("moves to the previous row on %s and stays on the first", async (_name, key) => {
    render(<Rows rows={rows} />);
    screen.getByRole("button", { name: "Fourth" }).focus();

    await userEvent.keyboard(key);
    expect(focused()).toBe("Second");
    await userEvent.keyboard(key);
    expect(focused()).toBe("First");
    await userEvent.keyboard(key);
    expect(focused()).toBe("First");
  });

  it("stays on the last row on ↓", async () => {
    render(<Rows rows={rows} />);
    screen.getByRole("button", { name: "Fourth" }).focus();

    await userEvent.keyboard("{ArrowDown}");

    expect(focused()).toBe("Fourth");
  });

  it("walks the rows of the coverage and year lists too", async () => {
    render(
      <Rows
        rows={[
          { className: "cov-row__open", name: "Month" },
          { className: "year-row", name: "Entry" },
        ]}
      />,
    );

    await userEvent.keyboard("jj");

    expect(focused()).toBe("Entry");
  });

  it("does nothing when the page has no rows", async () => {
    render(<Harness />);

    await userEvent.keyboard("{ArrowDown}j");

    expect(document.activeElement).toBe(document.body);
  });

  it("does nothing while typing in a field", async () => {
    render(
      <>
        <Rows rows={rows} />
        <input aria-label="Note" />
      </>,
    );
    const field = screen.getByRole("textbox", { name: "Note" });
    field.focus();

    await userEvent.keyboard("j{ArrowDown}");

    expect(document.activeElement).toBe(field);
  });
});
