import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../state/store";
import { MenuHost, PopUpButton } from "./menu";
import { Modal } from "./Modal";

function TwoSheets() {
  const [outer, setOuter] = useState(true);
  const [inner, setInner] = useState(true);
  return (
    <>
      {outer && (
        <Modal onClose={() => setOuter(false)} labelledBy="outer-title">
          <h2 id="outer-title">Outer</h2>
        </Modal>
      )}
      {inner && (
        <Modal onClose={() => setInner(false)} labelledBy="inner-title">
          <h2 id="inner-title">Inner</h2>
        </Modal>
      )}
    </>
  );
}

function SheetWithPopUp() {
  const [open, setOpen] = useState(true);
  const [value, setValue] = useState("a");
  return (
    <>
      {open && (
        <Modal onClose={() => setOpen(false)} labelledBy="title">
          <h2 id="title">Sheet</h2>
          <label htmlFor="letter">Letter</label>
          <PopUpButton
            id="letter"
            value={value}
            onChange={setValue}
            options={[
              { value: "a", label: "Alpha" },
              { value: "b", label: "Beta" },
            ]}
          />
        </Modal>
      )}
      <MenuHost />
    </>
  );
}

beforeEach(() => {
  useStore.setState({ menu: null });
});

function renderModal(props: { dismissOnScrim?: boolean } = {}) {
  const onClose = vi.fn();
  render(
    <Modal onClose={onClose} labelledBy="title" {...props}>
      <h2 id="title">Sheet</h2>
      <input aria-label="Name" />
    </Modal>,
  );
  return { onClose, scrim: screen.getByRole("dialog").parentElement! };
}

describe("Modal", () => {
  it("stays open when the scrim is pressed by default", async () => {
    const { onClose, scrim } = renderModal();

    await userEvent.pointer({ keys: "[MouseLeft>]", target: scrim });

    expect(onClose).not.toHaveBeenCalled();
  });

  it("closes when the scrim is pressed with dismissOnScrim", async () => {
    const { onClose, scrim } = renderModal({ dismissOnScrim: true });

    await userEvent.pointer({ keys: "[MouseLeft>]", target: scrim });

    expect(onClose).toHaveBeenCalled();
  });

  it("stays open when the sheet itself is pressed with dismissOnScrim", async () => {
    const { onClose } = renderModal({ dismissOnScrim: true });

    await userEvent.click(screen.getByLabelText("Name"));

    expect(onClose).not.toHaveBeenCalled();
  });

  it.each([false, true])("closes on Escape with dismissOnScrim %s", async (dismissOnScrim) => {
    const { onClose } = renderModal({ dismissOnScrim });

    await userEvent.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalled();
  });

  it("closes only the topmost of two sheets on each Escape", async () => {
    render(<TwoSheets />);
    expect(screen.getAllByRole("dialog")).toHaveLength(2);

    await userEvent.keyboard("{Escape}");
    expect(screen.getAllByRole("dialog")).toHaveLength(1);
    expect(screen.getByRole("dialog", { name: "Outer" })).toBeTruthy();

    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("lets Escape close an open pop-up menu first and the sheet on the next press", async () => {
    render(<SheetWithPopUp />);
    await userEvent.click(screen.getByRole("button", { name: "Letter" }));
    expect(screen.getByRole("menu")).toBeTruthy();

    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByRole("dialog", { name: "Sheet" })).toBeTruthy();

    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
