import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { useStore } from "../state/store";
import { TooltipHost } from "./TooltipHost";

beforeEach(() => {
  useStore.setState({ pdfCovers: 0 });
});

describe("TooltipHost", () => {
  it("shows the title of a hovered control as a tooltip", async () => {
    render(
      <>
        <TooltipHost />
        <button title="Back (⌘[)">←</button>
      </>,
    );

    await userEvent.hover(screen.getByRole("button"));

    expect((await screen.findByRole("tooltip")).textContent).toBe("Back (⌘[)");
  });

  it("covers the PDF preview while a tooltip shows and uncovers it when it hides", async () => {
    render(
      <>
        <TooltipHost />
        <button title="Back (⌘[)">←</button>
      </>,
    );
    expect(useStore.getState().pdfCovers).toBe(0);

    await userEvent.hover(screen.getByRole("button"));
    await screen.findByRole("tooltip");
    expect(useStore.getState().pdfCovers).toBe(1);

    await userEvent.keyboard("a");

    expect(screen.queryByRole("tooltip")).toBeNull();
    expect(useStore.getState().pdfCovers).toBe(0);
  });
});
