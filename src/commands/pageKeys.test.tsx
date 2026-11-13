import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { isTypingTarget, usePageKeys } from "./pageKeys";

function Page({ keys }: { keys: Record<string, () => void> }) {
  usePageKeys(keys);
  return <input aria-label="Note" />;
}

describe("usePageKeys", () => {
  it("runs the action of a pressed single key", async () => {
    const approve = vi.fn();
    render(<Page keys={{ a: approve }} />);

    await userEvent.keyboard("a");

    expect(approve).toHaveBeenCalled();
  });

  it("matches a mapped key whatever its case", async () => {
    const approve = vi.fn();
    render(<Page keys={{ a: approve }} />);

    await userEvent.keyboard("A");

    expect(approve).toHaveBeenCalled();
  });

  it("ignores keys nobody mapped", async () => {
    const approve = vi.fn();
    render(<Page keys={{ a: approve }} />);

    await userEvent.keyboard("b");

    expect(approve).not.toHaveBeenCalled();
  });

  it.each(["{Meta>}a{/Meta}", "{Control>}a{/Control}", "{Alt>}a{/Alt}"])("leaves %s to the menu bar", async (keys) => {
    const approve = vi.fn();
    render(<Page keys={{ a: approve }} />);

    await userEvent.keyboard(keys);

    expect(approve).not.toHaveBeenCalled();
  });

  it("ignores keys typed into a field", async () => {
    const approve = vi.fn();
    render(<Page keys={{ a: approve }} />);

    await userEvent.type(screen.getByRole("textbox", { name: "Note" }), "a");

    expect(approve).not.toHaveBeenCalled();
  });

  it("ignores keys while a sheet is open over the page", async () => {
    const approve = vi.fn();
    render(
      <>
        <Page keys={{ a: approve }} />
        <div className="overlay" />
      </>,
    );

    await userEvent.keyboard("a");

    expect(approve).not.toHaveBeenCalled();
  });

  it("uses the keys of the latest render", async () => {
    const first = vi.fn();
    const second = vi.fn();
    const { rerender } = render(<Page keys={{ a: first }} />);
    rerender(<Page keys={{ a: second }} />);

    await userEvent.keyboard("a");

    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalled();
  });

  it("stops listening on unmount", async () => {
    const approve = vi.fn();
    const { unmount } = render(<Page keys={{ a: approve }} />);
    unmount();

    await userEvent.keyboard("a");

    expect(approve).not.toHaveBeenCalled();
  });
});

describe("isTypingTarget", () => {
  it.each(["input", "textarea", "select"])("is true for a %s", (tag) => {
    expect(isTypingTarget(document.createElement(tag))).toBe(true);
  });

  it("is true for editable content", () => {
    const div = document.createElement("div");
    Object.defineProperty(div, "isContentEditable", { value: true });

    expect(isTypingTarget(div)).toBe(true);
  });

  it("is false for a button and for no target", () => {
    expect(isTypingTarget(document.createElement("button"))).toBe(false);
    expect(isTypingTarget(null)).toBe(false);
  });
});
