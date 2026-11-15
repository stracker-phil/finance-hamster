import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { HOME } from "../../state/route";
import { useStore } from "../../state/store";
import { otherGroup, otherListRow } from "../../test/documents";
import { renderWithQuery } from "../../test/renderWithQuery";
import { OtherList } from "./OtherList";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [] });
  mocked.othersList.mockResolvedValue([]);
});

describe("OtherList", () => {
  it("lists the documents under their year, the undated under Without date, each with its date", async () => {
    mocked.othersList.mockResolvedValue([
      otherGroup({ documents: [otherListRow(), otherListRow({ id: "other-2", title: "Demo Policy", date: "2 Mar 2026" })] }),
      otherGroup({ title: "Without date", year: null, documents: [otherListRow({ id: "other-3", title: "Demo Letter", date: "" })] }),
    ]);

    renderWithQuery(<OtherList query="" empty={null} />);

    const year = await screen.findByRole("region", { name: "2026" });
    expect(within(year).getByRole("button", { name: /Demo Lease/ }).textContent).toContain("10 Nov 2026");
    expect(within(year).getByRole("button", { name: /Demo Policy/ }).textContent).toContain("2 Mar 2026");
    const undated = screen.getByRole("region", { name: "Without date" });
    expect(within(undated).getByRole("button", { name: "Demo Letter" })).toBeTruthy();
  });

  it("asks for the documents matching the query", async () => {
    mocked.othersList.mockResolvedValue([otherGroup()]);

    renderWithQuery(<OtherList query="lease" empty={null} />);

    await screen.findByRole("region", { name: "2026" });
    expect(mocked.othersList).toHaveBeenCalledWith("lease");
  });

  it("opens the document when its row is clicked", async () => {
    mocked.othersList.mockResolvedValue([otherGroup()]);
    renderWithQuery(<OtherList query="" empty={null} />);

    await userEvent.click(await screen.findByRole("button", { name: /Demo Lease/ }));

    expect(useStore.getState().route).toEqual({ page: "other", otherId: "other-1" });
  });

  it("shows what it is given instead when there are no documents", async () => {
    renderWithQuery(<OtherList query="" empty={<p>Nothing yet</p>} />);

    expect(await screen.findByText("Nothing yet")).toBeTruthy();
  });
});
