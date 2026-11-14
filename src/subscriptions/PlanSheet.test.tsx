import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type PlanForm } from "../api";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { reviewLine } from "../test/review";
import { PlanSheet, PlanSheetHost } from "./PlanSheet";

const mocked = vi.mocked(api, true);

const FORM: PlanForm = { name: "", amount: "25.00", currency: "EUR", interval: "monthly", start: "2026-11-10" };

function renderSheet(onClose = vi.fn()) {
  return renderWithQuery(<PlanSheet id="sub-1" name="Lumen AI" currentPlan="Plus · €20.00 monthly" initial={FORM} onClose={onClose} />);
}

const submit = () => userEvent.click(screen.getByRole("button", { name: "Change Plan" }));

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], subscriptionSheet: null });
  mocked.subscriptionChangePlan.mockResolvedValue("Change plan Lumen AI");
});

describe("PlanSheet", () => {
  it("is titled with the subscription and names the current plan", () => {
    renderSheet();

    expect(screen.getByRole("dialog", { name: "Change plan · Lumen AI" })).toBeTruthy();
    expect(screen.getByText("Current: Plus · €20.00 monthly")).toBeTruthy();
  });

  it("starts with the given amount, currency, interval and start", () => {
    renderSheet();

    expect(screen.getByLabelText<HTMLInputElement>("New plan").value).toBe("");
    expect(screen.getByLabelText<HTMLInputElement>("Amount").value).toBe("25.00");
    expect(screen.getByLabelText<HTMLInputElement>("Currency").value).toBe("EUR");
    expect(screen.getByRole("button", { name: "Monthly" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByLabelText<HTMLInputElement>("Starts").value).toBe("2026-11-10");
  });

  it("sends the edited plan and closes with an Undo toast", async () => {
    const onClose = vi.fn();
    renderSheet(onClose);
    await userEvent.type(screen.getByLabelText("New plan"), "Pro");
    await userEvent.clear(screen.getByLabelText("Amount"));
    await userEvent.type(screen.getByLabelText("Amount"), "29.00");
    await userEvent.click(screen.getByRole("button", { name: "Yearly" }));
    fireEvent.change(screen.getByLabelText("Starts"), { target: { value: "2026-12-01" } });

    await submit();

    expect(mocked.subscriptionChangePlan).toHaveBeenCalledWith("sub-1", { name: "Pro", amount: "29.00", currency: "EUR", interval: "yearly", start: "2026-12-01" });
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(useStore.getState().toasts.at(-1)).toMatchObject({ kind: "success", message: "Change plan Lumen AI", action: { label: "Undo" } });
  });

  it("shows the bank line it was opened from", () => {
    renderWithQuery(<PlanSheet id="sub-1" name="Lumen AI" currentPlan="Plus · €20.00 monthly" initial={FORM} source={reviewLine()} onClose={vi.fn()} />);

    const line = within(screen.getByLabelText("Bank line"));
    expect(line.getByText("Example Bookshop")).toBeTruthy();
    expect(line.getByText("−€34.90")).toBeTruthy();
    expect(line.getByText(/Booking text 4711/)).toBeTruthy();
  });

  it("shows no bank line without a source", () => {
    renderSheet();

    expect(screen.queryByLabelText("Bank line")).toBeNull();
  });

  it("explains when the current plan ends once a start date is set", () => {
    renderSheet();

    expect(screen.getByText(/The current plan ends on 9 Nov 2026\./)).toBeTruthy();
  });

  it("asks for a start date when it is cleared", () => {
    renderSheet();

    fireEvent.change(screen.getByLabelText("Starts"), { target: { value: "" } });

    expect(screen.getByText("Choose when the new plan starts.")).toBeTruthy();
    expect(screen.queryByText(/The current plan ends on/)).toBeNull();
  });

  it("calls onClose and changes nothing with Cancel", async () => {
    const onClose = vi.fn();
    renderSheet(onClose);

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onClose).toHaveBeenCalled();
    expect(mocked.subscriptionChangePlan).not.toHaveBeenCalled();
  });

  describe("when changing fails", () => {
    it.each([
      ["subscription.planName", "Name the new plan.", "New plan"],
      ["subscription.amount", "Enter an amount.", "Amount"],
      ["subscription.currency", "Unknown currency.", "Amount"],
      ["subscription.start", "The plan cannot start before the last one.", "Starts"],
    ])("shows the %s error under its field", async (path, message, label) => {
      mocked.subscriptionChangePlan.mockRejectedValue({ kind: "invalid_input", message: "Invalid input", code: null, field: { path, message } });
      const onClose = vi.fn();
      renderSheet(onClose);

      await submit();

      expect((await screen.findByRole("alert")).textContent).toBe(message);
      expect(screen.getByLabelText(label).getAttribute("aria-invalid")).toBe(path === "subscription.currency" ? "false" : "true");
      expect(screen.getByRole<HTMLButtonElement>("button", { name: "Change Plan" }).disabled).toBe(false);
      expect(onClose).not.toHaveBeenCalled();
    });

    it("shows an error without a field at the bottom of the sheet", async () => {
      mocked.subscriptionChangePlan.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderSheet();

      await submit();

      expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
    });
  });
});

describe("PlanSheetHost", () => {
  it("shows nothing while no sheet is open", () => {
    renderWithQuery(<PlanSheetHost />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("shows nothing for the New and Edit sheets", () => {
    useStore.setState({ subscriptionSheet: { kind: "new", form: null } });

    renderWithQuery(<PlanSheetHost />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("shows the Change Plan sheet from the store and clears it on Cancel", async () => {
    useStore.setState({ subscriptionSheet: { kind: "plan", id: "sub-1", name: "Lumen AI", currentPlan: "Plus · €20.00 monthly", form: FORM } });
    renderWithQuery(<PlanSheetHost />);

    expect(screen.getByRole("dialog", { name: "Change plan · Lumen AI" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(useStore.getState().subscriptionSheet).toBeNull();
  });
});
