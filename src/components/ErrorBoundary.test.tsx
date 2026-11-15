import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { ErrorBoundary } from "./ErrorBoundary";

const mocked = vi.mocked(api, true);

let broken = true;

function Bomb() {
  if (broken) throw new Error("Demo failure");
  return <p>All fine</p>;
}

beforeEach(() => {
  vi.clearAllMocks();
  broken = true;
  vi.spyOn(console, "error").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ErrorBoundary", () => {
  it("renders its children while nothing fails", () => {
    broken = false;

    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>,
    );

    expect(screen.getByText("All fine")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows the error's message and a Try Again button instead of a crashing child", () => {
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>,
    );

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Something went wrong");
    expect(alert.textContent).toContain("Demo failure");
    expect(screen.getByRole("button", { name: "Try Again" })).toBeTruthy();
  });

  it("reports the crash to Rust with the message", async () => {
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>,
    );

    await waitFor(() => expect(mocked.logFrontendError).toHaveBeenCalledWith(expect.stringContaining("UI crash: Demo failure")));
  });

  it("still shows the fallback when reporting fails", async () => {
    mocked.logFrontendError.mockRejectedValue({ kind: "io", message: "Log unavailable", code: null });

    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>,
    );

    await waitFor(() => expect(mocked.logFrontendError).toHaveBeenCalled());
    expect(screen.getByRole("alert").textContent).toContain("Demo failure");
  });

  it("renders the children again on Try Again once the cause is gone", async () => {
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>,
    );
    broken = false;

    await userEvent.click(screen.getByRole("button", { name: "Try Again" }));

    expect(screen.getByText("All fine")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
