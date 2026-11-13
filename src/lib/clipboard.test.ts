import { afterEach, describe, expect, it, vi } from "vitest";
import { copyText } from "./clipboard";

const clipboard = navigator.clipboard;

afterEach(() => {
  Object.defineProperty(navigator, "clipboard", { value: clipboard, configurable: true });
  vi.restoreAllMocks();
});

describe("copyText", () => {
  it("writes the text through the async clipboard API", async () => {
    await copyText("1228,40");

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith("1228,40");
  });

  it("copies through a hidden field when the async API is missing, and leaves no field behind", async () => {
    Object.defineProperty(navigator, "clipboard", { value: undefined, configurable: true });
    let copied: string | undefined;
    Object.defineProperty(document, "execCommand", {
      value: vi.fn(() => {
        copied = document.querySelector("textarea")?.value;
        return true;
      }),
      configurable: true,
    });

    await copyText("1228,40");

    expect(copied).toBe("1228,40");
    expect(document.querySelector("textarea")).toBeNull();
  });

  it("rejects when the async API fails", async () => {
    vi.mocked(navigator.clipboard.writeText).mockRejectedValueOnce(new Error("denied"));

    await expect(copyText("1")).rejects.toThrow("denied");
  });
});
