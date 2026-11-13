import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { runPageCommand } from "./pageCommands";

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [] });
});

describe("runPageCommand", () => {
  it("opens the Review page for go.review", () => {
    runPageCommand("go.review");

    expect(useStore.getState().route).toEqual({ page: "review" });
  });
});
