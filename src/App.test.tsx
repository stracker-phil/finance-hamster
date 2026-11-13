import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({ ...(await orig<object>()), api: (await import("./test/apiMock")).createApiMock() }));

import { api } from "./api";
import App from "./App";
import { HOME } from "./state/route";
import { useStore } from "./state/store";

const mocked = vi.mocked(api, true);

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], confirmation: null });
    mocked.takeLaunchEvents.mockResolvedValue([]);
    mocked.listen.mockResolvedValue(() => {});
  });

  it("shows the main window on the Today page", async () => {
    render(<App />);

    expect(screen.getByRole("heading", { level: 1, name: "Today" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Today" }).getAttribute("aria-current")).toBe("page");
    await waitFor(() => expect(mocked.takeLaunchEvents).toHaveBeenCalled());
  });

  it("shows no alert when no launch event is queued", async () => {
    render(<App />);

    await waitFor(() => expect(mocked.takeLaunchEvents).toHaveBeenCalled());
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows the message of an error launch event taken on mount as an error toast", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Could not open the database" }]);

    render(<App />);

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Something went wrong");
    expect(alert.textContent).toContain("Could not open the database");
  });

  it("shows the error message when the launch events cannot be taken", async () => {
    mocked.takeLaunchEvents.mockRejectedValue({ kind: "io", message: "Socket closed", code: null });

    render(<App />);

    expect((await screen.findByRole("alert")).textContent).toContain("Socket closed");
  });

  it("takes the launch events again when the launch event fires", async () => {
    render(<App />);
    await waitFor(() => expect(mocked.listen).toHaveBeenCalled());
    expect(screen.queryByRole("alert")).toBeNull();
    const [event, handler] = mocked.listen.mock.calls[0];
    expect(event).toBe("launch");

    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Second launch failed" }]);
    handler(undefined as never);

    expect((await screen.findByRole("alert")).textContent).toContain("Second launch failed");
  });

  it("unsubscribes from the launch event on unmount", async () => {
    const unlisten = vi.fn();
    mocked.listen.mockResolvedValue(unlisten);
    const { unmount } = render(<App />);
    await waitFor(() => expect(mocked.listen).toHaveBeenCalled());

    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalled());
  });
});
