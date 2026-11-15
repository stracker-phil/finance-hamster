import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { checkQuestion } from "../test/year";
import { CheckRow } from "./CheckRow";

const mocked = vi.mocked(api, true);

type Props = ComponentProps<typeof CheckRow>;

function renderRow(patch: Partial<Props> = {}) {
  const p: Props = {
    year: 2026,
    question: checkQuestion(),
    focused: false,
    added: undefined,
    adding: false,
    onFocus: vi.fn(),
    onYes: vi.fn(),
    onAnswer: vi.fn(),
    onLater: vi.fn(),
    onAdded: vi.fn(),
    onCloseInput: vi.fn(),
    ...patch,
  };
  render(<CheckRow {...p} />);
  return p;
}

const numberQuestion = (patch = {}) => checkQuestion({ key: "home-days", kz: null, question: "How many days did you work from home?", add: null, unit: "days", value: "12", ...patch });

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
});

describe("CheckRow yes/no question", () => {
  it("shows the Kennzahl, the question and its hint", () => {
    renderRow({ question: checkQuestion({ kz: "720", question: "Did you buy work books?", hint: "Invoices count, also e-books." }) });

    const row = screen.getByRole("group", { name: "Did you buy work books?" });
    expect(within(row).getByText("KZ 720")).toBeTruthy();
    expect(within(row).getByText("Invoices count, also e-books.")).toBeTruthy();
  });

  it("labels a question without a Kennzahl as a plain Question", () => {
    renderRow({ question: checkQuestion({ kz: null }) });

    expect(screen.getByText("Question")).toBeTruthy();
  });

  it("offers Yes, No and Later", () => {
    renderRow();

    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Yes", "No", "Later"]);
  });

  it("calls back for Yes without answering the question", async () => {
    const p = renderRow();

    await userEvent.click(screen.getByRole("button", { name: "Yes" }));

    expect(p.onYes).toHaveBeenCalled();
    expect(p.onAnswer).not.toHaveBeenCalled();
  });

  it("answers the question on No", async () => {
    const p = renderRow();

    await userEvent.click(screen.getByRole("button", { name: "No" }));

    expect(p.onAnswer).toHaveBeenCalled();
    expect(mocked.yearAnswer).not.toHaveBeenCalled();
  });

  it("postpones the question on Later", async () => {
    const p = renderRow();

    await userEvent.click(screen.getByRole("button", { name: "Later" }));

    expect(p.onLater).toHaveBeenCalled();
    expect(p.onAnswer).not.toHaveBeenCalled();
  });
});

describe("CheckRow focus", () => {
  it("reports focus when something inside takes it", async () => {
    const p = renderRow();

    await userEvent.tab();

    expect(p.onFocus).toHaveBeenCalled();
  });

  it("reports focus on a mouse press anywhere in the row", () => {
    const p = renderRow();

    fireEvent.mouseDown(screen.getByText("Did you buy work books?"));

    expect(p.onFocus).toHaveBeenCalled();
  });
});

describe("CheckRow after a Yes", () => {
  const added = { count: 2, label: "Add Trip to KZ 721" };

  it("says what was added without the leading Add", () => {
    renderRow({ added });

    expect(screen.getByText(/Added 2 · Trip to KZ 721/)).toBeTruthy();
  });

  it("says what was added without the trailing German hinzufügen", () => {
    renderRow({ added: { count: 1, label: "Fahrt zu KZ 721 hinzufügen" } });

    expect(screen.getByText(/Added 1 · Fahrt zu KZ 721$/)).toBeTruthy();
  });

  it("offers Add Another and Done in place of Yes, No and Later", () => {
    renderRow({ added });

    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Add Another", "Done"]);
  });

  it("adds another on Add Another and leaves the run on Done", async () => {
    const p = renderRow({ added });

    await userEvent.click(screen.getByRole("button", { name: "Add Another" }));
    expect(p.onYes).toHaveBeenCalled();
    expect(p.onAnswer).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(p.onAnswer).toHaveBeenCalled();
  });
});

describe("CheckRow inline entry", () => {
  it.each([
    ["amount", "Add an amount to KZ 720"],
    ["trip", "Add a trip to KZ 720"],
  ] as const)("opens the %s row while adding", (add, name) => {
    renderRow({ question: checkQuestion({ add }), adding: true });

    expect(screen.getByRole("group", { name })).toBeTruthy();
  });

  it("shows no inline row for a document question, which opens its own sheet", () => {
    renderRow({ question: checkQuestion({ add: "document" }), adding: true });

    expect(screen.queryByRole("group", { name: /^Add (an amount|a trip)/ })).toBeNull();
  });

  it("shows no inline row while not adding", () => {
    renderRow({ question: checkQuestion({ add: "amount" }), adding: false });

    expect(screen.queryByRole("group", { name: /^Add (an amount|a trip)/ })).toBeNull();
  });

  it("passes the label of a saved amount on and closes the row", async () => {
    mocked.yearEntryAdd.mockResolvedValue("Add entry");
    const p = renderRow({ question: checkQuestion({ add: "amount" }), adding: true });

    await userEvent.type(screen.getByLabelText("Amount"), "19.90");
    await userEvent.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() => expect(p.onAdded).toHaveBeenCalledWith("Add entry"));
    expect(p.onCloseInput).toHaveBeenCalled();
  });
});

describe("CheckRow number question", () => {
  it("shows the saved value with its unit and Save and Later buttons", () => {
    renderRow({ question: numberQuestion() });

    expect(screen.getByRole<HTMLInputElement>("textbox", { name: "How many days did you work from home?" }).value).toBe("12");
    expect(screen.getByText("days")).toBeTruthy();
    expect(screen.getByText("Question")).toBeTruthy();
    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Save", "Later"]);
  });

  it("sends the typed value for its key and year, then answers", async () => {
    const p = renderRow({ question: numberQuestion() });
    const field = screen.getByRole("textbox");

    await userEvent.clear(field);
    await userEvent.type(field, "40");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(mocked.yearAnswer).toHaveBeenCalledWith(2026, "home-days", "40");
    await waitFor(() => expect(p.onAnswer).toHaveBeenCalled());
  });

  it("saves on Return in the field", async () => {
    const p = renderRow({ question: numberQuestion({ value: "" }) });

    await userEvent.type(screen.getByRole("textbox"), "7{Enter}");

    expect(mocked.yearAnswer).toHaveBeenCalledWith(2026, "home-days", "7");
    await waitFor(() => expect(p.onAnswer).toHaveBeenCalled());
  });

  it("shows the field message of a rejected value and keeps the question open", async () => {
    mocked.yearAnswer.mockRejectedValue({ kind: "invalid_input", message: "Invalid input", code: null, field: { path: "answer", message: "Enter whole days between 0 and 366" } });
    const p = renderRow({ question: numberQuestion() });

    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect((await screen.findByRole("alert")).textContent).toBe("Enter whole days between 0 and 366");
    expect(p.onAnswer).not.toHaveBeenCalled();
  });

  it("shows the message of another failure, such as a closed year", async () => {
    mocked.yearAnswer.mockRejectedValue({ kind: "invalid_input", message: "Tax year 2026 is closed", code: null });
    renderRow({ question: numberQuestion() });

    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect((await screen.findByRole("alert")).textContent).toBe("Tax year 2026 is closed");
  });

  it("clears an earlier error when saving again", async () => {
    mocked.yearAnswer.mockRejectedValueOnce({ kind: "invalid_input", message: "Enter a number", code: null });
    mocked.yearAnswer.mockResolvedValueOnce(null);
    const p = renderRow({ question: numberQuestion() });

    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await screen.findByRole("alert");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(p.onAnswer).toHaveBeenCalled());
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("postpones on Later without saving", async () => {
    const p = renderRow({ question: numberQuestion() });

    await userEvent.click(screen.getByRole("button", { name: "Later" }));

    expect(p.onLater).toHaveBeenCalled();
    expect(mocked.yearAnswer).not.toHaveBeenCalled();
  });
});
