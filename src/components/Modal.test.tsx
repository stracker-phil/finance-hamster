import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Modal } from "./Modal";

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
});
