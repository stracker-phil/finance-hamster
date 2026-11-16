import { beforeEach, describe, expect, it } from "vitest";
import { HOME, sidebarKey, type Route } from "./route";
import { useStore } from "./store";

const subscriptions: Route = { page: "subscriptions" };
const statements: Route = { page: "statements" };

beforeEach(() => {
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", scroll: 0, toasts: [], confirmation: null });
  useStore.getState().noteScroll(0);
});

describe("navigation", () => {
  it("pushes the current visit to back, clears forward and starts the new page without filter or scroll", () => {
    useStore.setState({ filter: "rent", scroll: 40, forward: [{ route: statements, filter: "", scroll: 0 }] });
    useStore.getState().noteScroll(120);

    useStore.getState().navigate(subscriptions);

    const s = useStore.getState();
    expect(s.route).toEqual(subscriptions);
    expect(s.back).toEqual([{ route: HOME, filter: "rent", scroll: 120 }]);
    expect(s.forward).toEqual([]);
    expect(s.filter).toBe("");
    expect(s.scroll).toBe(0);
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
    expect(useStore.getState().forward.map((v) => v.route)).toEqual([statements]);

    useStore.getState().goBack();
    expect(useStore.getState().route).toEqual(HOME);

    useStore.getState().goForward();
    expect(useStore.getState().route).toEqual(subscriptions);
    expect(useStore.getState().back.map((v) => v.route)).toEqual([HOME]);
  });

  it("keeps the route when there is no history to go back or forward to", () => {
    useStore.getState().goBack();
    useStore.getState().goForward();

    expect(useStore.getState().route).toEqual(HOME);
  });

  it("restores the filter and scroll a page was left with when going back", () => {
    useStore.getState().setFilter("rent");
    useStore.getState().noteScroll(300);
    useStore.getState().navigate(subscriptions);

    useStore.getState().goBack();

    const s = useStore.getState();
    expect(s.route).toEqual(HOME);
    expect(s.filter).toBe("rent");
    expect(s.scroll).toBe(300);
  });

  it("restores the filter and scroll of the page left by going back when going forward", () => {
    useStore.getState().navigate(subscriptions);
    useStore.getState().setFilter("netflix");
    useStore.getState().noteScroll(75);
    useStore.getState().goBack();

    useStore.getState().goForward();

    const s = useStore.getState();
    expect(s.route).toEqual(subscriptions);
    expect(s.filter).toBe("netflix");
    expect(s.scroll).toBe(75);
  });

  it("keeps each page's own position across several steps back", () => {
    useStore.getState().setFilter("a");
    useStore.getState().noteScroll(10);
    useStore.getState().navigate(subscriptions);
    useStore.getState().setFilter("b");
    useStore.getState().noteScroll(20);
    useStore.getState().navigate(statements);

    useStore.getState().goBack();
    expect(useStore.getState()).toMatchObject({ route: subscriptions, filter: "b", scroll: 20 });

    useStore.getState().goBack();
    expect(useStore.getState()).toMatchObject({ route: HOME, filter: "a", scroll: 10 });
  });

  it("remembers the scroll of the page it goes back from, without needing a new note", () => {
    useStore.getState().navigate(subscriptions);
    useStore.getState().noteScroll(55);
    useStore.getState().goBack();
    useStore.getState().goForward();
    useStore.getState().goBack();

    expect(useStore.getState().route).toEqual(HOME);
    useStore.getState().goForward();

    expect(useStore.getState().scroll).toBe(55);
  });

  it("starts a newly visited page with an empty filter and scroll 0", () => {
    useStore.getState().setFilter("rent");
    useStore.getState().noteScroll(300);

    useStore.getState().navigate(subscriptions);

    expect(useStore.getState().filter).toBe("");
    expect(useStore.getState().scroll).toBe(0);
    useStore.getState().navigate(statements);
    useStore.getState().goBack();
    expect(useStore.getState().scroll).toBe(0);
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

describe("leaveProfile", () => {
  const preview = { title: "preview" } as never;

  it("goes back to Today with no history, no filter and nothing open", () => {
    useStore.getState().navigate(subscriptions);
    useStore.getState().navigate(statements);
    useStore.getState().goBack();
    useStore.getState().noteScroll(200);
    useStore.setState({
      filter: "rent",
      scroll: 200,
      paletteOpen: true,
      paletteQuery: "pixelforge",
      importPreview: preview,
      addPreview: preview,
      addOnAdded: () => {},
      addKind: "payslip",
      documentsTab: "payslips",
      statementsYear: 2024,
      subscriptionSheet: { kind: "edit", id: "s1", form: {} as never },
      menu: { id: 1, entries: [], at: { x: 0, y: 0 }, label: "Menu", opener: null },
    });

    useStore.getState().leaveProfile();

    const s = useStore.getState();
    expect(s.route).toEqual(HOME);
    expect(s.back).toEqual([]);
    expect(s.forward).toEqual([]);
    expect(s.filter).toBe("");
    expect(s.scroll).toBe(0);
    expect(s.paletteOpen).toBe(false);
    expect(s.paletteQuery).toBe("");
    expect(s.importPreview).toBeNull();
    expect(s.addPreview).toBeNull();
    expect(s.addOnAdded).toBeNull();
    expect(s.addKind).toBe("invoice");
    expect(s.documentsTab).toBe("invoices");
    expect(s.statementsYear).toBeNull();
    expect(s.subscriptionSheet).toBeNull();
    expect(s.menu).toBeNull();
  });

  it("answers an open question false and closes it", async () => {
    const answer = useStore.getState().confirm({ title: "Delete?", text: "Gone.", action: "Delete" });

    useStore.getState().leaveProfile();

    expect(await answer).toBe(false);
    expect(useStore.getState().confirmation).toBeNull();
  });

  it("works when nothing is open", () => {
    useStore.getState().leaveProfile();

    expect(useStore.getState().route).toEqual(HOME);
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
