import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { fileCheck, fileProblem } from "../test/integrity";
import { renderWithQuery } from "../test/renderWithQuery";
import { FilesBanner } from "./FilesBanner";

const mocked = vi.mocked(api, true);

describe("FilesBanner", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ toasts: [] });
    mocked.filesLastCheck.mockResolvedValue(null);
    mocked.settingsShow.mockResolvedValue(undefined);
  });

  it("shows nothing while no check has run", async () => {
    renderWithQuery(<FilesBanner />);

    await waitFor(() => expect(mocked.filesLastCheck).toHaveBeenCalled());
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows nothing when the last check found no problem", async () => {
    mocked.filesLastCheck.mockResolvedValue(fileCheck());
    renderWithQuery(<FilesBanner />);

    await waitFor(() => expect(mocked.filesLastCheck).toHaveBeenCalled());
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("names the single problem file", async () => {
    mocked.filesLastCheck.mockResolvedValue(fileCheck({ problems: [fileProblem()] }));
    renderWithQuery(<FilesBanner />);

    expect((await screen.findByRole("alert")).textContent).toContain("1 stored file is missing or changed.");
  });

  it("counts several problem files", async () => {
    mocked.filesLastCheck.mockResolvedValue(fileCheck({ problems: [fileProblem(), fileProblem({ file: "docs/inbox/other.pdf", trouble: "changed" }), fileProblem({ file: "docs/inbox/third.pdf" })] }));
    renderWithQuery(<FilesBanner />);

    expect((await screen.findByRole("alert")).textContent).toContain("3 stored files are missing or changed.");
  });

  it("opens the Verify files setting from Show Files", async () => {
    mocked.filesLastCheck.mockResolvedValue(fileCheck({ problems: [fileProblem()] }));
    renderWithQuery(<FilesBanner />);

    await userEvent.click(await screen.findByRole("button", { name: "Show Files" }));

    expect(mocked.settingsShow).toHaveBeenCalledWith("verify-files");
  });

  it("shows the error as a toast when Settings cannot be shown", async () => {
    mocked.filesLastCheck.mockResolvedValue(fileCheck({ problems: [fileProblem()] }));
    mocked.settingsShow.mockRejectedValue({ kind: "internal", message: "No window available", code: null });
    renderWithQuery(<FilesBanner />);

    await userEvent.click(await screen.findByRole("button", { name: "Show Files" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["No window available"]));
  });
});
