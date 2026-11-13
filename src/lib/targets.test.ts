import { describe, expect, it } from "vitest";
import type { Target } from "../api";
import type { Route } from "../state/route";
import { routeOf } from "./targets";

describe("routeOf", () => {
  it.each<[Target, Route]>([
    [{ page: "documents" }, { page: "documents" }],
    [{ page: "document", documentId: "doc-9", year: 2026 }, { page: "document", documentId: "doc-9", year: 2026 }],
    [{ page: "document", documentId: "doc-9", year: null }, { page: "document", documentId: "doc-9", year: null }],
    [{ page: "account", accountId: "acc-2" }, { page: "account", accountId: "acc-2" }],
  ])("leads %j to %j", (target, route) => {
    expect(routeOf(target)).toEqual(route);
  });
});
