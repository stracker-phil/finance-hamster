import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { lineChoice } from "../test/documents";
import { PdfCoversProbe } from "../test/PdfCoversProbe";
import { renderWithQuery } from "../test/renderWithQuery";
import { LinePicker } from "./LinePicker";

const mocked = vi.mocked(api, true);

const renderPicker = () => {
  const onPick = vi.fn();
  const onClose = vi.fn();
  renderWithQuery(
    <>
      <p>Outside</p>
      <LinePicker documentId="doc-1" onPick={onPick} onClose={onClose} />
    </>,
  );
  return { onPick, onClose };
};

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ pdfCovers: [] });
  mocked.documentLineOptions.mockResolvedValue([lineChoice(), lineChoice({ id: "line-2", title: "Example Market", detail: "Business · Mon, 9 Nov · EUR 12.00", amount: "−€12.00" })]);
});

describe("LinePicker", () => {
  it("lists the likely bank lines with detail and amount", async () => {
    renderPicker();

    expect(await screen.findByRole("option", { name: /Demo Cloud Ltd/ })).toBeTruthy();
    expect(screen.getByRole("dialog", { name: "Link a bank line" })).toBeTruthy();
    expect(screen.getByText("Business · Mon, 9 Nov · EUR 12.00")).toBeTruthy();
    expect(screen.getByText("−€12.00")).toBeTruthy();
  });

  it("searches the bank lines with the typed text", async () => {
    renderPicker();
    await screen.findByRole("option", { name: /Demo Cloud Ltd/ });
    mocked.documentLineOptions.mockResolvedValue([lineChoice({ id: "line-2", title: "Example Market" })]);

    await userEvent.type(screen.getByRole("textbox", { name: "Search bank lines" }), "market");

    expect(mocked.documentLineOptions).toHaveBeenLastCalledWith("doc-1", "market");
    expect(await screen.findByRole("option", { name: /Example Market/ })).toBeTruthy();
    expect(screen.queryByRole("option", { name: /Demo Cloud Ltd/ })).toBeNull();
  });

  it("lets clicks on the PDF preview through while open and sends the uncovered state once closed", async () => {
    function Host() {
      const [shown, setShown] = useState(true);
      return (
        <>
          <PdfCoversProbe />
          {shown && <LinePicker documentId="doc-1" onPick={() => {}} onClose={() => setShown(false)} />}
        </>
      );
    }
    renderWithQuery(<Host />);
    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith(expect.objectContaining({ hidden: false, clickThrough: true })));

    await userEvent.keyboard("{Escape}");

    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: false, holes: [], clickThrough: false }));
  });

  it("hands the chosen line's id on", async () => {
    const { onPick } = renderPicker();

    await userEvent.click(await screen.findByRole("option", { name: /Example Market/ }));

    expect(onPick).toHaveBeenCalledWith("line-2");
  });

  it("closes on Escape", async () => {
    const { onClose } = renderPicker();
    await screen.findByRole("option", { name: /Demo Cloud Ltd/ });

    await userEvent.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalled();
  });

  it("closes on a press outside, not on one inside", async () => {
    const { onClose } = renderPicker();
    await screen.findByRole("option", { name: /Demo Cloud Ltd/ });

    await userEvent.click(screen.getByRole("textbox", { name: "Search bank lines" }));
    expect(onClose).not.toHaveBeenCalled();

    await userEvent.click(screen.getByText("Outside"));
    expect(onClose).toHaveBeenCalled();
  });

  it("asks to search when no bank line is likely", async () => {
    mocked.documentLineOptions.mockResolvedValue([]);
    renderPicker();

    expect(await screen.findByText("No likely bank line; search to find one")).toBeTruthy();
  });

  it("says no bank line matches when a search finds none", async () => {
    mocked.documentLineOptions.mockResolvedValue([]);
    renderPicker();
    await screen.findByText("No likely bank line; search to find one");

    await userEvent.type(screen.getByRole("textbox", { name: "Search bank lines" }), "zzz");

    expect(await screen.findByText("No bank line matches")).toBeTruthy();
  });
});
