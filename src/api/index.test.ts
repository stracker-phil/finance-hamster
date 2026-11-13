import { describe, expect, it } from "vitest";
import { toAppError } from "./index";

describe("toAppError", () => {
  it.each(["io", "invalid_input", "not_found", "internal"])("keeps an error of the known kind %s", (kind) => {
    expect(toAppError({ kind, message: "Nope" })).toEqual({ kind, message: "Nope" });
  });

  it("keeps extra fields of a known error such as the field name", () => {
    expect(toAppError({ kind: "invalid_input", message: "Too long", field: "name" })).toEqual({
      kind: "invalid_input",
      message: "Too long",
      field: "name",
    });
  });

  it("turns the message of a known error into a string", () => {
    expect(toAppError({ kind: "not_found", message: 42 })).toEqual({ kind: "not_found", message: "42" });
  });

  it.each([
    ["a string", "boom", "boom"],
    ["an Error", new Error("disk full"), "disk full"],
    ["an object with an unknown kind", { kind: "mystery", message: "odd" }, "odd"],
    ["an object without a message", { kind: "io" }, "[object Object]"],
    ["null", null, "null"],
  ])("wraps %s as an io error", (_name, input, message) => {
    expect(toAppError(input)).toEqual({ kind: "io", message });
  });
});
