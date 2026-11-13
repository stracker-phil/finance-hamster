import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({ ...(await orig<object>()), api: (await import("./test/apiMock")).createApiMock() }));

import { api } from "./api";
import App from "./App";
import { useStore } from "./state/store";

const mocked = vi.mocked(api, true);

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ appInfo: null, launchError: null });
    mocked.takeLaunchEvents.mockResolvedValue([]);
    mocked.appInfo.mockResolvedValue({ name: "Hamster (Debug)", version: "0.1.0" });
    mocked.listen.mockResolvedValue(() => {});
  });

  it("shows the app name and version from the app info", async () => {
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Hamster (Debug)" })).toBeTruthy();
    expect(await screen.findByText("Version 0.1.0")).toBeTruthy();
  });

  it("shows no alert when no launch event is queued", async () => {
    render(<App />);

    await screen.findByText("Version 0.1.0");
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows the message of an error launch event taken on mount", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Could not open the database" }]);

    render(<App />);

    expect((await screen.findByRole("alert")).textContent).toBe("Could not open the database");
  });

  it("shows the error message when the launch events cannot be taken", async () => {
    mocked.takeLaunchEvents.mockRejectedValue({ kind: "io", message: "Socket closed", code: null });

    render(<App />);

    expect((await screen.findByRole("alert")).textContent).toBe("Socket closed");
  });

  it("shows the plain name without a version when the app info cannot be loaded", async () => {
    mocked.appInfo.mockRejectedValue({ kind: "internal", message: "No info", code: null });
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Sync point" }]);

    render(<App />);

    await screen.findByRole("alert");
    expect(screen.getByRole("heading", { name: "Hamster" })).toBeTruthy();
    expect(screen.queryByText(/^Version/)).toBeNull();
  });

  it("takes the launch events again when the launch event fires", async () => {
    render(<App />);
    await screen.findByText("Version 0.1.0");
    expect(screen.queryByRole("alert")).toBeNull();
    const [event, handler] = mocked.listen.mock.calls[0];
    expect(event).toBe("launch");

    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "Second launch failed" }]);
    handler(undefined as never);

    expect((await screen.findByRole("alert")).textContent).toBe("Second launch failed");
  });

  it("unsubscribes from the launch event on unmount", async () => {
    const unlisten = vi.fn();
    mocked.listen.mockResolvedValue(unlisten);
    const { unmount } = render(<App />);
    await screen.findByText("Version 0.1.0");

    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalled());
  });
});
