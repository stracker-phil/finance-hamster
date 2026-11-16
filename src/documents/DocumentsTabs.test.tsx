import { render, renderHook, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { DocumentsTabs, useDocumentsTab } from "./DocumentsTabs";

const tabButton = (name: string) => screen.getByRole("button", { name });

beforeEach(() => {
  useStore.setState({ route: HOME, back: [], forward: [], documentsTab: "invoices" });
});

describe("DocumentsTabs", () => {
  it("offers Invoices, Statements, Payslips and Other as one group with the remembered tab pressed", () => {
    render(<DocumentsTabs />);

    const group = screen.getByRole("group", { name: "Kind of document" });
    expect(within(group).getAllByRole("button").map((b) => b.textContent)).toEqual(["Invoices", "Statements", "Payslips", "Other"]);
    expect(tabButton("Invoices").getAttribute("aria-pressed")).toBe("true");
    expect(tabButton("Other").getAttribute("aria-pressed")).toBe("false");
  });

  it("names the key of each tab in its tooltip", () => {
    render(<DocumentsTabs />);

    expect(["Invoices", "Statements", "Payslips", "Other"].map((name) => tabButton(name).title)).toEqual(["Invoices (⌥1)", "Statements (⌥2)", "Payslips (⌥3)", "Other (⌥4)"]);
  });

  it("presses the tab the route asks for over the remembered one", () => {
    render(<DocumentsTabs tab="other" />);

    expect(tabButton("Other").getAttribute("aria-pressed")).toBe("true");
    expect(tabButton("Invoices").getAttribute("aria-pressed")).toBe("false");
  });

  it.each([
    ["Statements", "statements"],
    ["Payslips", "payslips"],
    ["Other", "other"],
    ["Invoices", "invoices"],
  ] as const)("remembers %s and navigates to it when its segment is clicked", async (name, tab) => {
    render(<DocumentsTabs tab={tab === "invoices" ? "other" : "invoices"} />);

    await userEvent.click(tabButton(name));

    expect(useStore.getState().documentsTab).toBe(tab);
    expect(useStore.getState().route).toEqual({ page: "documents", tab });
  });
});

describe("useDocumentsTab", () => {
  it("is the route's tab when there is one", () => {
    useStore.setState({ documentsTab: "statements" });

    expect(renderHook(() => useDocumentsTab("payslips")).result.current).toBe("payslips");
  });

  it("is the tab opened last when the route names none", () => {
    useStore.setState({ documentsTab: "statements" });

    expect(renderHook(() => useDocumentsTab()).result.current).toBe("statements");
  });
});
