import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { beforeEach, describe, expect, it } from "vitest";
import { useStore } from "../state/store";
import { ConfirmHost } from "./ConfirmSheet";

beforeEach(() => {
  useStore.setState({ confirmation: null });
});

function ask() {
  let answer!: Promise<boolean>;
  act(() => {
    answer = useStore.getState().confirm({ title: "Delete subscription?", text: "It can be undone.", action: "Delete" });
  });
  return answer;
}

describe("ConfirmHost", () => {
  it("shows nothing while no question is asked", () => {
    render(<ConfirmHost />);

    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("shows the question as an alert dialog with its title, text and action", () => {
    render(<ConfirmHost />);

    void ask();

    expect(screen.getByRole("alertdialog", { name: "Delete subscription?" })).toBeTruthy();
    expect(screen.getByText("It can be undone.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Delete" })).toBeTruthy();
  });

  it("resolves false on Cancel and closes", async () => {
    render(<ConfirmHost />);
    const answer = ask();

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(await answer).toBe(false);
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("resolves true on the action button", async () => {
    render(<ConfirmHost />);
    const answer = ask();

    await userEvent.click(screen.getByRole("button", { name: "Delete" }));

    expect(await answer).toBe(true);
  });

  it("resolves false on Escape", async () => {
    render(<ConfirmHost />);
    const answer = ask();

    await userEvent.keyboard("{Escape}");

    expect(await answer).toBe(false);
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });
});
