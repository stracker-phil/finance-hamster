import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { PdfCoversProbe } from "../test/PdfCoversProbe";
import { ConfirmHost } from "./ConfirmSheet";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ confirmation: null, pdfCovers: [] });
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

  it("hides the PDF preview while the question is open and sends the uncovered state once answered", async () => {
    render(
      <>
        <PdfCoversProbe />
        <ConfirmHost />
      </>,
    );
    const answer = ask();
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: true, holes: [], clickThrough: false }));

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await answer;

    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: false }));
  });

  it("resolves false on Escape", async () => {
    render(<ConfirmHost />);
    const answer = ask();

    await userEvent.keyboard("{Escape}");

    expect(await answer).toBe(false);
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });
});
