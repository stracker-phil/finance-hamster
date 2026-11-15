import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { setLanguage } from "../i18n";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { step, todayView, upcomingCharge, yearSummary } from "../test/today";
import { TodayPage } from "./TodayPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: { page: "today" }, toasts: [] });
  mocked.todayView.mockResolvedValue(todayView());
});

describe("TodayPage", () => {
  describe("the headline", () => {
    it("shows the date, the title as sent, its line and the button", async () => {
      mocked.todayView.mockResolvedValue(todayView({ next: step() }));

      renderWithQuery(<TodayPage />);

      expect(await screen.findByRole("heading", { name: "12 lines to review" })).toBeTruthy();
      expect(screen.getByText("Friday, 13 November")).toBeTruthy();
      expect(screen.getByText("Decide what each one is: ANV, subscription or ignore.")).toBeTruthy();
      expect(screen.getByRole("button", { name: "Start review" })).toBeTruthy();
    });

    it.each([
      ["review", step({ button: "Start review", target: { kind: "review" } }), { page: "review" }],
      ["the tax year", step({ button: "Open tax year", target: { kind: "year", year: 2026, attention: null } }), { page: "year", year: 2026 }],
      ["the missing invoices of the tax year", step({ button: "Attach invoices", target: { kind: "year", year: 2026, attention: "missing_invoice" } }), { page: "year", year: 2026, attention: "missing_invoice" }],
      ["a document", step({ button: "Attach PDF", target: { kind: "document", id: "doc-7", year: 2026 } }), { page: "document", documentId: "doc-7", year: 2026 }],
      ["the year check", step({ button: "Start year check", target: { kind: "yearCheck", year: 2026 } }), { page: "yearCheck", year: 2026 }],
    ])("sends the button to %s", async (_name, next, route) => {
      mocked.todayView.mockResolvedValue(todayView({ next }));
      renderWithQuery(<TodayPage />);

      await userEvent.click(await screen.findByRole("button", { name: next.button }));

      expect(useStore.getState().route).toEqual(route);
    });

    it("asks for a statement PDF when the button imports", async () => {
      mocked.todayView.mockResolvedValue(todayView({ next: step({ key: "import", button: "Import statement", target: { kind: "import" } }) }));
      renderWithQuery(<TodayPage />);

      await userEvent.click(await screen.findByRole("button", { name: "Import statement" }));

      expect(mocked.statementImportChoose).toHaveBeenCalled();
    });
  });

  describe("in German", () => {
    beforeEach(() => setLanguage("de"));

    it("says everything is up to date in German", async () => {
      renderWithQuery(<TodayPage />);

      expect(await screen.findByRole("heading", { name: "Du bist auf dem neuesten Stand." })).toBeTruthy();
      expect(screen.getByText("Nichts zu prüfen oder anzuhängen.")).toBeTruthy();
      expect(screen.getByRole("region", { name: "Demnächst" })).toBeTruthy();
    });
  });

  describe("nothing open", () => {
    it("says everything is up to date and shows no step rows", async () => {
      renderWithQuery(<TodayPage />);

      expect(await screen.findByRole("heading", { name: "You’re up to date." })).toBeTruthy();
      expect(screen.getByText("Nothing to review or attach.")).toBeTruthy();
      expect(screen.queryByRole("group", { name: "Also open" })).toBeNull();
    });
  });

  describe("other open steps", () => {
    const also = [
      step({ key: "attach", title: "2 invoices need a PDF", detail: "about 3 minutes", target: { kind: "document", id: "doc-3", year: 2026 } }),
      step({ key: "import", title: "Statement for October is missing", detail: "about 1 minute", target: { kind: "import" } }),
    ];

    it("lists each as a button with its title and detail", async () => {
      mocked.todayView.mockResolvedValue(todayView({ next: step(), also }));

      renderWithQuery(<TodayPage />);

      const group = await screen.findByRole("group", { name: "Also open" });
      expect(group.textContent).toContain("2 invoices need a PDF");
      expect(group.textContent).toContain("about 3 minutes");
      expect(screen.getByRole("button", { name: /Statement for October is missing/ }).textContent).toContain("about 1 minute");
    });

    it("opens the target of the row that is clicked", async () => {
      mocked.todayView.mockResolvedValue(todayView({ next: step(), also }));
      renderWithQuery(<TodayPage />);

      await userEvent.click(await screen.findByRole("button", { name: /2 invoices need a PDF/ }));

      expect(useStore.getState().route).toEqual({ page: "document", documentId: "doc-3", year: 2026 });
    });

    it("starts the import for an import row", async () => {
      mocked.todayView.mockResolvedValue(todayView({ next: step(), also }));
      renderWithQuery(<TodayPage />);

      await userEvent.click(await screen.findByRole("button", { name: /Statement for October is missing/ }));

      expect(mocked.statementImportChoose).toHaveBeenCalled();
    });
  });

  describe("coming up", () => {
    it("shows each charge with its letter, name, ANV pill, when and amount", async () => {
      mocked.todayView.mockResolvedValue(
        todayView({
          upcoming: [
            upcomingCharge(),
            upcomingCharge({ id: "sub-2", name: "Pixelforge Cloud", letter: "P", anv: true, when: "tomorrow", soon: true, amount: "€9.99" }),
          ],
        }),
      );

      renderWithQuery(<TodayPage />);

      const section = await screen.findByRole("region", { name: "Coming up" });
      const text = section.textContent;
      for (const part of ["L", "Lumen AI", "in 12 days", "€20.00", "P", "Pixelforge Cloud", "tomorrow", "€9.99"]) expect(text).toContain(part);
      expect(screen.getAllByText("ANV")).toHaveLength(1);
      expect(screen.getByText("Pixelforge Cloud").textContent).toContain("ANV");
    });

    it("opens the subscription of a charge that is clicked", async () => {
      mocked.todayView.mockResolvedValue(todayView({ upcoming: [upcomingCharge({ id: "sub-2", name: "Pixelforge Cloud" })] }));
      renderWithQuery(<TodayPage />);

      await userEvent.click(await screen.findByRole("button", { name: /Pixelforge Cloud/ }));

      expect(useStore.getState().route).toEqual({ page: "subscriptions", subscriptionId: "sub-2" });
    });

    it("says no charges are ahead when there are none", async () => {
      renderWithQuery(<TodayPage />);

      expect(await screen.findByText("No subscription charges ahead.")).toBeTruthy();
    });

    it("opens the subscriptions from All subscriptions", async () => {
      renderWithQuery(<TodayPage />);

      await userEvent.click(await screen.findByRole("button", { name: "All subscriptions" }));

      expect(useStore.getState().route).toEqual({ page: "subscriptions" });
    });
  });

  describe("tax year", () => {
    it("shows the year, total, caption and progress", async () => {
      mocked.todayView.mockResolvedValue(todayView({ year: yearSummary() }));

      renderWithQuery(<TodayPage />);

      const section = await screen.findByRole("region", { name: "Tax year 2026" });
      expect(section.textContent).toContain("Tax year 2026");
      expect(screen.getByText("€1,228.40")).toBeTruthy();
      expect(screen.getByText("Werbungskosten so far")).toBeTruthy();
      expect(screen.getByText("4 of 6 items answered")).toBeTruthy();
    });

    it("opens the year page from Show Tax Year", async () => {
      mocked.todayView.mockResolvedValue(todayView({ year: yearSummary({ year: 2026 }) }));
      renderWithQuery(<TodayPage />);

      await userEvent.click(await screen.findByRole("button", { name: "Show Tax Year" }));

      expect(useStore.getState().route).toEqual({ page: "year", year: 2026 });
    });

    it("joins the statement reach parts with a middle dot", async () => {
      mocked.todayView.mockResolvedValue(
        todayView({
          year: yearSummary(),
          statements: [
            { text: "Private until 10 Nov", behind: false },
            { text: "Business until 30 Sep", behind: true },
          ],
        }),
      );

      renderWithQuery(<TodayPage />);

      expect(await screen.findByText(/Statements:/)).toBeTruthy();
      expect(screen.getByText(/Statements:/).textContent).toBe("Statements: Private until 10 Nov · Business until 30 Sep");
    });

    it("shows no statements line when there is no reach to report", async () => {
      mocked.todayView.mockResolvedValue(todayView({ year: yearSummary(), statements: [] }));

      renderWithQuery(<TodayPage />);

      await screen.findByText("Werbungskosten so far");
      expect(screen.queryByText(/Statements:/)).toBeNull();
    });

    it("shows no tax year heading or total when there is no year", async () => {
      renderWithQuery(<TodayPage />);

      await screen.findByText("No subscription charges ahead.");
      expect(screen.queryByText(/Tax year/)).toBeNull();
      expect(screen.queryByText("Werbungskosten so far")).toBeNull();
      expect(screen.queryByRole("button", { name: "Show Tax Year" })).toBeNull();
    });

    it("still shows the statements line in a Statements section when there is no year", async () => {
      mocked.todayView.mockResolvedValue(todayView({ statements: [{ text: "Private until 10 Nov", behind: false }] }));

      renderWithQuery(<TodayPage />);

      const section = await screen.findByRole("region", { name: "Statements" });
      expect(section.textContent).toBe("Statements: Private until 10 Nov");
      expect(screen.queryByText(/Tax year/)).toBeNull();
    });
  });
});
