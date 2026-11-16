import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { PdfCoversProbe } from "../../test/PdfCoversProbe";
import { MenuHost, openMenu, type MenuEntry } from ".";

const mocked = vi.mocked(api, true);

const names = ["Archive", "Log", "Export", "Delete"] as const;

/** A button that opens a menu whose rows report the chosen text on the page. */
function Harness({ entries }: { entries?: (choose: (text: string) => void) => MenuEntry[] }) {
  const [chosen, setChosen] = useState("nothing");
  const rows =
    entries?.(setChosen) ??
    names.map((text) => ({ text, run: () => setChosen(text), enabled: text !== "Log" }));
  return (
    <>
      <button onClick={(e) => openMenu(rows, e.currentTarget, "Actions")}>Open</button>
      <button onClick={() => openMenu(rows, { x: 40, y: 60 }, "Actions")}>Open at point</button>
      <button>Elsewhere</button>
      <p>Chose {chosen}</p>
      <MenuHost />
    </>
  );
}

const opener = () => screen.getByRole("button", { name: "Open" });
const row = (name: string | RegExp) => screen.getByRole("menuitem", { name });
const menu = () => screen.queryByRole("menu");

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ menu: null, pdfCovers: [] });
});

describe("Menu", () => {
  describe("opening", () => {
    it("opens as a menu with its accessible label, listing its rows", async () => {
      render(<Harness />);

      await userEvent.click(opener());

      expect(screen.getByRole("menu", { name: "Actions" })).toBeTruthy();
      expect(screen.getAllByRole("menuitem").map((r) => r.textContent)).toEqual([...names]);
    });

    it("opens under the button it was opened from", async () => {
      render(<Harness />);
      vi.spyOn(opener(), "getBoundingClientRect").mockReturnValue({ left: 100, top: 20, right: 180, bottom: 50, width: 80, height: 30, x: 100, y: 20, toJSON: () => ({}) });

      await userEvent.click(opener());

      const panel = screen.getByRole("menu");
      expect(panel.style.left).toBe("100px");
      expect(panel.style.top).toBe("54px");
    });

    it("opens at the point it was given, with no row highlighted", async () => {
      render(<Harness />);

      await userEvent.click(screen.getByRole("button", { name: "Open at point" }));

      const panel = screen.getByRole("menu");
      expect(panel.style.left).toBe("40px");
      expect(panel.style.top).toBe("60px");
      expect(panel).toHaveFocus();
    });

    it("starts on the first enabled row of a button's menu", async () => {
      render(<Harness />);

      await userEvent.click(opener());

      expect(row("Archive")).toHaveFocus();
    });

    it("starts on the checked row of a button's menu", async () => {
      render(
        <Harness
          entries={() => [
            { text: "Light", checked: false },
            { text: "Dark", checked: true },
            { text: "Automatic", checked: false },
          ]}
        />,
      );

      await userEvent.click(opener());

      expect(screen.getByRole("menuitemradio", { name: "Dark" })).toHaveFocus();
    });

    it("marks choice rows as radio items with their checked state", async () => {
      render(
        <Harness
          entries={() => [
            { text: "Light", checked: false },
            { text: "Dark", checked: true },
          ]}
        />,
      );

      await userEvent.click(opener());

      expect(screen.getByRole("menuitemradio", { name: "Light" }).getAttribute("aria-checked")).toBe("false");
      expect(screen.getByRole("menuitemradio", { name: "Dark" }).getAttribute("aria-checked")).toBe("true");
      expect(screen.queryByRole("menuitem")).toBeNull();
    });

    it("shows a separator, a caption and a row's hint", async () => {
      render(<Harness entries={() => [{ caption: "Recent" }, { text: "Statement", hint: "⌘I" }, "separator", { text: "Clear" }]} />);

      await userEvent.click(opener());

      expect(screen.getByText("Recent")).toBeTruthy();
      expect(screen.getByRole("separator")).toBeTruthy();
      expect(row(/Statement/).textContent).toBe("Statement⌘I");
      expect(screen.getAllByRole("menuitem")).toHaveLength(2);
    });
  });

  describe("choosing", () => {
    it("runs the row and closes when it is clicked", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.click(row("Export"));

      expect(screen.getByText("Chose Export")).toBeTruthy();
      expect(menu()).toBeNull();
    });

    it("does not choose a disabled row and stays open", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.click(row("Log"));

      expect(screen.getByText("Chose nothing")).toBeTruthy();
      expect(menu()).toBeTruthy();
    });

    it("chooses the highlighted row on Enter", async () => {
      render(<Harness />);
      await userEvent.click(opener());
      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(screen.getByText("Chose Export")).toBeTruthy();
      expect(menu()).toBeNull();
    });

    it("chooses the highlighted row on Space and stays closed when the key is released", async () => {
      render(<Harness />);
      await userEvent.click(opener());
      await userEvent.keyboard("{ArrowDown}[Space]");

      expect(screen.getByText("Chose Export")).toBeTruthy();
      expect(menu()).toBeNull();
    });
  });

  describe("keys", () => {
    it("moves down and up over the enabled rows, skipping the disabled one", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.keyboard("{ArrowDown}");
      expect(row("Export")).toHaveFocus();

      await userEvent.keyboard("{ArrowUp}");
      expect(row("Archive")).toHaveFocus();
    });

    it("wraps from the last row to the first and from the first to the last", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.keyboard("{ArrowUp}");
      expect(row("Delete")).toHaveFocus();

      await userEvent.keyboard("{ArrowDown}");
      expect(row("Archive")).toHaveFocus();
    });

    it("jumps to the last and the first enabled row on End and Home", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.keyboard("{End}");
      expect(row("Delete")).toHaveFocus();

      await userEvent.keyboard("{Home}");
      expect(row("Archive")).toHaveFocus();
    });

    it("jumps to the next row starting with the typed letter, ignoring case", async () => {
      render(<Harness entries={() => [{ text: "Archive" }, { text: "Export" }, { text: "Edit" }]} />);
      await userEvent.click(opener());

      await userEvent.keyboard("e");
      expect(row("Export")).toHaveFocus();

      await userEvent.keyboard("E");
      expect(row("Edit")).toHaveFocus();
    });

    it("keeps the highlight when no row starts with the typed letter", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.keyboard("z");

      expect(row("Archive")).toHaveFocus();
    });
  });

  describe("closing", () => {
    it("closes on Escape without running a row", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.keyboard("{Escape}");

      expect(menu()).toBeNull();
      expect(screen.getByText("Chose nothing")).toBeTruthy();
    });

    it("closes on a press outside the menu", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.click(screen.getByRole("button", { name: "Elsewhere" }));

      expect(menu()).toBeNull();
    });

    it("closes when the window loses focus", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      fireEvent.blur(window);

      expect(menu()).toBeNull();
    });

    it("closes when the window is resized", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      fireEvent(window, new Event("resize"));

      expect(menu()).toBeNull();
    });

    it("closes when its button is clicked a second time", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.click(opener());

      expect(menu()).toBeNull();
    });

    it("gives the focus back to the button after Escape", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.keyboard("{Escape}");

      expect(opener()).toHaveFocus();
    });

    it("gives the focus back to the button after a row is chosen", async () => {
      render(<Harness />);
      await userEvent.click(opener());

      await userEvent.click(row("Export"));

      expect(opener()).toHaveFocus();
    });
  });

  describe("PDF preview", () => {
    it("lets clicks through while the menu is open and is uncovered once it closes", async () => {
      render(
        <>
          <PdfCoversProbe />
          <Harness />
        </>,
      );
      expect(mocked.pdfViewCover).not.toHaveBeenCalled();

      await userEvent.click(opener());
      await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith(expect.objectContaining({ hidden: false, clickThrough: true })));

      await userEvent.keyboard("{Escape}");
      await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: false }));
    });
  });
});
