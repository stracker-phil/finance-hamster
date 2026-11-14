import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type CheckQuestion } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { addPreview } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { checkQuestion, yearCheckView } from "../test/year";
import { YearCheckPage } from "./YearCheckPage";

const mocked = vi.mocked(api, true);

const items = (count: number) => Array.from({ length: count }, (_, i) => checkQuestion({ key: `item-${i + 1}`, question: `Question ${i + 1}` }));
const commute = (patch: Partial<CheckQuestion> = {}) => checkQuestion({ key: "commute", kz: null, question: "Days worked from home", add: null, unit: "days", ...patch });
const trip = () => checkQuestion({ key: "trips", kz: "721", question: "Did you travel for work?", add: "trip" });
const amount = () => checkQuestion({ key: "other", kz: "722", question: "Did you pay for other work costs?", add: "amount" });

const show = async (questions: CheckQuestion[], patch: Parameters<typeof yearCheckView>[0] = {}) => {
  mocked.yearCheckView.mockResolvedValue(yearCheckView({ questions, ...patch }));
  const view = renderWithQuery(<YearCheckPage year={2026} />);
  await screen.findByRole("heading", { name: /^Year check 2026/ });
  return view;
};

const asked = () => screen.getAllByRole("group").map((g) => g.getAttribute("aria-label"));
const row = (name: string) => within(screen.getByRole("group", { name }));
const press = (name: string, question: string) => userEvent.click(row(question).getByRole("button", { name }));
const heading = (open: number) => screen.findByRole("heading", { name: `Year check 2026 · ${open} open` });

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [], addPreview: null, addOnAdded: null });
  mocked.documentAddChoose.mockResolvedValue(null);
  mocked.yearEntryAdd.mockResolvedValue("Add entry");
  mocked.yearAnswer.mockResolvedValue(null);
  mocked.yearCheckComplete.mockResolvedValue(yearCheckView({ lastRun: "14 Nov 2026" }));
});

describe("YearCheckPage", () => {
  describe("the stack", () => {
    it("counts the open questions in the header", async () => {
      await show(items(3));

      expect(await heading(3)).toBeTruthy();
      expect(asked()).toEqual(["Question 1", "Question 2", "Question 3"]);
    });

    it("shows the first five questions and says how many wait below", async () => {
      await show(items(7));

      expect(await heading(7)).toBeTruthy();
      expect(asked()).toEqual(["Question 1", "Question 2", "Question 3", "Question 4", "Question 5"]);
      expect(screen.getByText("2 more below, they move up as you answer")).toBeTruthy();
    });

    it("says nothing about more questions when five or fewer are open", async () => {
      await show(items(5));

      expect(screen.queryByText(/more below/)).toBeNull();
    });

    it("moves the next question up when one is answered with No", async () => {
      await show(items(6));

      await press("No", "Question 1");

      expect(await heading(5)).toBeTruthy();
      expect(asked()).toEqual(["Question 2", "Question 3", "Question 4", "Question 5", "Question 6"]);
      expect(screen.queryByText(/more below/)).toBeNull();
    });

    it("moves a question to the end on Later and keeps it open", async () => {
      await show(items(3));

      await press("Later", "Question 1");

      expect(asked()).toEqual(["Question 2", "Question 3", "Question 1"]);
      expect(await heading(3)).toBeTruthy();
    });

    it("keeps the last question open on Later and does not complete the check", async () => {
      await show(items(1));

      await press("Later", "Question 1");

      expect(await heading(1)).toBeTruthy();
      expect(asked()).toEqual(["Question 1"]);
      expect(mocked.yearCheckComplete).not.toHaveBeenCalled();
    });

    it("starts with every question open after leaving and coming back", async () => {
      const first = await show(items(3));
      await press("No", "Question 1");
      await heading(2);
      first.unmount();

      await show(items(3));

      expect(asked()).toEqual(["Question 1", "Question 2", "Question 3"]);
    });
  });

  describe("a question that asks for a document", () => {
    const start = async () => {
      mocked.documentAddChoose.mockResolvedValue(addPreview());
      await show(items(2));
      await press("Yes", "Question 1");
      await waitFor(() => expect(useStore.getState().addOnAdded).not.toBeNull());
    };
    const added = (label: string) => act(() => useStore.getState().addOnAdded?.(label));

    it("opens Add Document for the chosen PDF with the question's item", async () => {
      await start();

      expect(useStore.getState().addPreview?.itemKey).toBe("item-1");
    });

    it("opens nothing and keeps the question when no PDF is chosen", async () => {
      await show(items(2));

      await press("Yes", "Question 1");

      await waitFor(() => expect(mocked.documentAddChoose).toHaveBeenCalled());
      expect(useStore.getState().addPreview).toBeNull();
      expect(row("Question 1").getByRole("button", { name: "Yes" })).toBeTruthy();
    });

    it("shows what was added without the leading Add, with Add Another and Done", async () => {
      await start();

      added("Add Example Books Order");

      expect(screen.getByText("Added 1 · Example Books Order")).toBeTruthy();
      expect(row("Question 1").getByRole("button", { name: "Add Another" })).toBeTruthy();
      expect(row("Question 1").getByRole("button", { name: "Done" })).toBeTruthy();
      expect(row("Question 1").queryByRole("button", { name: "Yes" })).toBeNull();
    });

    it("answers the question on Done", async () => {
      await start();
      added("Add Example Books Order");

      await press("Done", "Question 1");

      expect(await heading(1)).toBeTruthy();
      expect(asked()).toEqual(["Question 2"]);
    });

    it("opens Add Document again on Add Another", async () => {
      await start();
      added("Add Example Books Order");
      act(() => useStore.getState().setAddPreview(null));

      await press("Add Another", "Question 1");

      await waitFor(() => expect(useStore.getState().addPreview?.itemKey).toBe("item-1"));
    });

    it("shows nothing added while Add Document is cancelled", async () => {
      await start();

      act(() => useStore.getState().setAddPreview(null));

      expect(screen.queryByText(/^Added /)).toBeNull();
      expect(row("Question 1").getByRole("button", { name: "Yes" })).toBeTruthy();
    });
  });

  describe.each([
    ["trip", trip, "Add a trip to KZ 721", "Destination and purpose", "Linz · client workshop"],
    ["amount", amount, "Add an amount to KZ 722", "Note", "Desk lamp"],
  ] as const)("a question that asks for a %s", (_kind, question, groupName, field, text) => {
    const open = async () => {
      mocked.yearEntryAdd.mockResolvedValue(`Add ${text}`);
      await show([question(), ...items(1)]);
      await press("Yes", question().question);
      return within(await screen.findByRole("group", { name: groupName }));
    };
    const addEntry = async (fields: Awaited<ReturnType<typeof open>>, typed: string) => {
      await userEvent.type(fields.getByRole("textbox", { name: field }), typed);
      await userEvent.click(fields.getByRole("button", { name: "Add" }));
    };

    it("opens the inline row inside the question", async () => {
      await open();

      expect(row(question().question).getByRole("group", { name: groupName })).toBeTruthy();
    });

    it("shows the typed text as added after adding", async () => {
      const fields = await open();

      await addEntry(fields, text);

      expect(await screen.findByText(`Added 1 · ${text}`)).toBeTruthy();
      expect(screen.queryByRole("group", { name: groupName })).toBeNull();
    });

    it("leaves the question unchanged on Escape", async () => {
      const fields = await open();

      await userEvent.type(fields.getByRole("textbox", { name: field }), "{Escape}");

      expect(screen.queryByRole("group", { name: groupName })).toBeNull();
      expect(screen.queryByText(/^Added /)).toBeNull();
      expect(row(question().question).getByRole("button", { name: "Yes" })).toBeTruthy();
    });

    it("counts a second add after Add Another", async () => {
      const fields = await open();
      await addEntry(fields, text);
      await screen.findByText(`Added 1 · ${text}`);

      await press("Add Another", question().question);
      await addEntry(within(await screen.findByRole("group", { name: groupName })), " again");

      expect(await screen.findByText(`Added 2 · again`)).toBeTruthy();
    });
  });

  describe("a number question", () => {
    it("shows its unit and the current value", async () => {
      await show([commute({ value: "42" })]);

      expect(screen.getByRole<HTMLInputElement>("textbox", { name: "Days worked from home" }).value).toBe("42");
      expect(screen.getByText("days")).toBeTruthy();
    });

    it("saves the typed value and answers the question", async () => {
      await show([commute(), ...items(1)]);

      const input = screen.getByRole("textbox", { name: "Days worked from home" });
      await userEvent.type(input, "42");
      await press("Save", "Days worked from home");

      await waitFor(() => expect(mocked.yearAnswer).toHaveBeenCalledWith(2026, "commute", "42"));
      expect(await heading(1)).toBeTruthy();
      expect(asked()).toEqual(["Question 1"]);
    });

    it("shows a refused value and keeps the question open", async () => {
      mocked.yearAnswer.mockRejectedValue({ kind: "invalid_input", message: "Enter a number", code: null });
      await show([commute(), ...items(1)]);

      await userEvent.type(screen.getByRole("textbox", { name: "Days worked from home" }), "x");
      await press("Save", "Days worked from home");

      expect((await screen.findByRole("alert")).textContent).toBe("Enter a number");
      expect(await heading(2)).toBeTruthy();
    });

    it("prefers the message of the refused field", async () => {
      mocked.yearAnswer.mockRejectedValue({ kind: "invalid_input", message: "answer.value: Whole days", code: null, field: { path: "answer.value", message: "Whole days" } });
      await show([commute()]);

      await press("Save", "Days worked from home");

      expect((await screen.findByRole("alert")).textContent).toBe("Whole days");
    });
  });

  describe("keys", () => {
    it("answers the first question with N by default", async () => {
      await show(items(3));

      await userEvent.keyboard("n");

      expect(await heading(2)).toBeTruthy();
      expect(asked()).toEqual(["Question 2", "Question 3"]);
    });

    it("moves the first question to the end with L", async () => {
      await show(items(3));

      await userEvent.keyboard("l");

      expect(asked()).toEqual(["Question 2", "Question 3", "Question 1"]);
    });

    it("starts adding for the first question with Y", async () => {
      mocked.documentAddChoose.mockResolvedValue(addPreview());
      await show(items(3));

      await userEvent.keyboard("y");

      await waitFor(() => expect(useStore.getState().addPreview?.itemKey).toBe("item-1"));
    });

    it("acts on the question that ArrowDown moved to", async () => {
      await show(items(3));

      await userEvent.keyboard("{ArrowDown}n");

      expect(asked()).toEqual(["Question 1", "Question 3"]);
    });

    it("acts on the question that ArrowUp moved back to", async () => {
      await show(items(3));

      await userEvent.keyboard("{ArrowDown}{ArrowDown}{ArrowUp}n");

      expect(asked()).toEqual(["Question 1", "Question 3"]);
    });

    it("keeps ArrowDown on the last shown question", async () => {
      await show(items(2));

      await userEvent.keyboard("{ArrowDown}{ArrowDown}{ArrowDown}n");

      expect(asked()).toEqual(["Question 1"]);
    });

    it("leaves a number question alone on Y and N", async () => {
      await show([commute(), ...items(1)]);

      await userEvent.keyboard("yn");

      expect(await heading(2)).toBeTruthy();
      expect(asked()).toEqual(["Days worked from home", "Question 1"]);
      expect(mocked.documentAddChoose).not.toHaveBeenCalled();
    });

    it("moves a number question to the end with L", async () => {
      await show([commute(), ...items(1)]);

      await userEvent.keyboard("l");

      expect(asked()).toEqual(["Question 1", "Days worked from home"]);
    });

    it("does not answer while typing in a number question", async () => {
      await show([commute(), ...items(1)]);

      await userEvent.type(screen.getByRole("textbox", { name: "Days worked from home" }), "nyl");

      expect(asked()).toEqual(["Days worked from home", "Question 1"]);
    });
  });

  describe("when the last question is answered", () => {
    const finish = async (count = 1, patch: Parameters<typeof yearCheckView>[0] = {}) => {
      mocked.yearCheckComplete.mockResolvedValue(yearCheckView({ lastRun: "14 Nov 2026", ...patch }));
      await show(items(count));
      for (let i = 0; i < count; i++) await userEvent.keyboard("n");
      await screen.findByText("Year check done");
    };

    it("completes the check for the year and says when it ran and how many were answered", async () => {
      await finish(2);

      expect(mocked.yearCheckComplete).toHaveBeenCalledWith(2026);
      expect(screen.getByText("Checked on 14 Nov 2026 · 2 questions answered")).toBeTruthy();
    });

    it("counts a single answer in the singular", async () => {
      await finish(1);

      expect(screen.getByText("Checked on 14 Nov 2026 · 1 question answered")).toBeTruthy();
    });

    it("reminds that the year isn't over while it isn't", async () => {
      await finish(1, { yearOver: false });

      expect(screen.getByText("2026 isn’t over yet. Run the check again after 31 Dec; Close Year will remind you.")).toBeTruthy();
    });

    it("leaves the reminder out once the year is over", async () => {
      await finish(1, { yearOver: true });

      expect(screen.queryByText(/isn’t over yet/)).toBeNull();
    });

    it("counts a number question answered with Save", async () => {
      mocked.yearCheckComplete.mockResolvedValue(yearCheckView({ lastRun: "14 Nov 2026" }));
      await show([commute()]);

      await userEvent.type(screen.getByRole("textbox", { name: "Days worked from home" }), "42");
      await press("Save", "Days worked from home");

      expect(await screen.findByText("Checked on 14 Nov 2026 · 1 question answered")).toBeTruthy();
    });

    it("starts again with all questions open on Run Again", async () => {
      await finish(2);

      await userEvent.click(screen.getByRole("button", { name: "Run Again" }));

      expect(await heading(2)).toBeTruthy();
      expect(asked()).toEqual(["Question 1", "Question 2"]);
    });

    it("goes to the tax year on Back to Tax Year", async () => {
      await finish(1);

      await userEvent.click(screen.getByRole("button", { name: "Back to Tax Year 2026" }));

      expect(useStore.getState().route).toEqual({ page: "year", year: 2026 });
    });
  });

  describe("without a check to run", () => {
    it("says the tax year is closed", async () => {
      mocked.yearCheckView.mockResolvedValue(yearCheckView({ open: false, questions: items(2) }));
      renderWithQuery(<YearCheckPage year={2026} />);

      expect(await screen.findByText("Tax year 2026 is closed")).toBeTruthy();
      expect(screen.queryByRole("group")).toBeNull();
    });

    it("says there is nothing to ask when the rules have no questions", async () => {
      mocked.yearCheckView.mockResolvedValue(yearCheckView({ questions: [] }));
      renderWithQuery(<YearCheckPage year={2026} />);

      expect(await screen.findByText("Nothing to ask")).toBeTruthy();
      expect(mocked.yearCheckComplete).not.toHaveBeenCalled();
    });
  });
});
