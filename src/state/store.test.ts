import { beforeEach, describe, expect, it } from "vitest";
import { HOME, sidebarKey, type Route } from "./route";
import { useStore } from "./store";

const subscriptions: Route = { page: "subscriptions" };
const statements: Route = { page: "statements" };

beforeEach(() => {
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], confirmation: null });
});

describe("navigation", () => {
  it("pushes the current route to back, clears forward and clears the filter", () => {
    useStore.setState({ filter: "rent", forward: [statements] });

    useStore.getState().navigate(subscriptions);

    const s = useStore.getState();
    expect(s.route).toEqual(subscriptions);
    expect(s.back).toEqual([HOME]);
    expect(s.forward).toEqual([]);
    expect(s.filter).toBe("");
  });

  it("does nothing when navigating to the current route", () => {
    useStore.setState({ filter: "rent" });

    useStore.getState().navigate({ page: "today" });

    const s = useStore.getState();
    expect(s.back).toEqual([]);
    expect(s.filter).toBe("rent");
  });

  it("goes back and forward between visited routes", () => {
    useStore.getState().navigate(subscriptions);
    useStore.getState().navigate(statements);

    useStore.getState().goBack();
    expect(useStore.getState().route).toEqual(subscriptions);
    expect(useStore.getState().forward).toEqual([statements]);

    useStore.getState().goBack();
    expect(useStore.getState().route).toEqual(HOME);

    useStore.getState().goForward();
    expect(useStore.getState().route).toEqual(subscriptions);
    expect(useStore.getState().back).toEqual([HOME]);
  });

  it("keeps the route when there is no history to go back or forward to", () => {
    useStore.getState().goBack();
    useStore.getState().goForward();

    expect(useStore.getState().route).toEqual(HOME);
  });

  it("clears the filter when going back", () => {
    useStore.getState().navigate(subscriptions);
    useStore.getState().setFilter("rent");

    useStore.getState().goBack();

    expect(useStore.getState().filter).toBe("");
  });
});

describe("sidebarKey", () => {
  it.each<[Route, string]>([
    [{ page: "today" }, "today"],
    [{ page: "subscriptions" }, "subscriptions"],
    [{ page: "statements" }, "statements"],
    [{ page: "documents" }, "documents"],
    [{ page: "review" }, "statements"],
    [{ page: "account", accountId: "a1" }, "statements"],
    [{ page: "document", documentId: "d1", year: null }, "documents"],
    [{ page: "document", documentId: "d1", year: 2025 }, "year:2025"],
    [{ page: "log", year: 2025 }, "year:2025"],
    [{ page: "archive", year: 2024 }, "year:2024"],
    [{ page: "year", year: 2023 }, "year:2023"],
  ])("maps %j to %s", (route, key) => {
    expect(sidebarKey(route)).toBe(key);
  });
});

describe("palette", () => {
  beforeEach(() => {
    useStore.setState({ paletteOpen: false, paletteQuery: "" });
  });

  it("opens with the text it searches everywhere for", () => {
    useStore.getState().searchEverywhere("pixelforge");

    expect(useStore.getState().paletteOpen).toBe(true);
    expect(useStore.getState().paletteQuery).toBe("pixelforge");
  });

  it("forgets the text when it closes", () => {
    useStore.getState().searchEverywhere("pixelforge");

    useStore.getState().setPaletteOpen(false);

    expect(useStore.getState().paletteOpen).toBe(false);
    expect(useStore.getState().paletteQuery).toBe("");
  });

  it("opens empty when opened directly", () => {
    useStore.getState().setPaletteOpen(true);

    expect(useStore.getState().paletteOpen).toBe(true);
    expect(useStore.getState().paletteQuery).toBe("");
  });
});

describe("toasts", () => {
  it("collapses an identical consecutive message into a count of 2", () => {
    useStore.getState().showToast("error", "Disk full");
    useStore.getState().showToast("error", "Disk full");

    const { toasts } = useStore.getState();
    expect(toasts).toHaveLength(1);
    expect(toasts[0].count).toBe(2);
  });

  it("keeps a different message and a toast with an action as separate toasts", () => {
    useStore.getState().showToast("error", "Disk full");
    useStore.getState().showToast("error", "Disk full", { label: "Retry", run: () => {} });
    useStore.getState().showToast("error", "Other");

    expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Disk full", "Disk full", "Other"]);
  });

  it("removes a dismissed toast", () => {
    useStore.getState().showToast("info", "One");
    useStore.getState().showToast("info", "Two");
    const [one] = useStore.getState().toasts;

    useStore.getState().dismissToast(one.id);

    expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Two"]);
  });
});

describe("confirm", () => {
  const question = { title: "Delete?", text: "Gone.", action: "Delete" };

  it("resolves true when confirmed", async () => {
    const answer = useStore.getState().confirm(question);

    useStore.getState().confirmation?.answer(true);

    expect(await answer).toBe(true);
    expect(useStore.getState().confirmation).toBeNull();
  });

  it("resolves false when declined", async () => {
    const answer = useStore.getState().confirm(question);

    useStore.getState().confirmation?.answer(false);

    expect(await answer).toBe(false);
  });

  it("answers the first question false when a second one is asked", async () => {
    const first = useStore.getState().confirm(question);
    const second = useStore.getState().confirm({ ...question, title: "Other?" });

    expect(await first).toBe(false);
    expect(useStore.getState().confirmation?.title).toBe("Other?");
    useStore.getState().confirmation?.answer(true);
    expect(await second).toBe(true);
  });
});
