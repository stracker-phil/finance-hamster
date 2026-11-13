import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { renderWithQuery } from "../../test/renderWithQuery";
import { reloadPreview } from "../../test/rules";
import { ReloadSheet } from "./ReloadSheet";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => ({ kind: t.kind, message: t.message }));
const apply = () => screen.getByRole("button", { name: "Apply" });

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
  mocked.rulesReloadApply.mockResolvedValue("Reload tax rules");
});

describe("ReloadSheet", () => {
  it("lists the changes of each year under its ruleset change", () => {
    renderWithQuery(
      <ReloadSheet
        previews={[reloadPreview(), reloadPreview({ year: 2027, from: "2027 v1", to: "2027 v1", changes: ["Nothing new"] })]}
        onClose={() => {}}
      />,
    );

    expect(screen.getByRole("heading", { name: "Tax year 2026: 2026 v1 → 2026 v2" })).toBeTruthy();
    expect(screen.getByText("Home office limit changed")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Tax year 2027: 2027 v1" })).toBeTruthy();
    expect(apply()).toBeEnabled();
  });

  it("shows the problems instead of changes and disables Apply", () => {
    renderWithQuery(<ReloadSheet previews={[reloadPreview({ to: null, changes: [], problems: ["items[0].kz is missing"] })]} onClose={() => {}} />);

    expect(screen.getByRole("heading", { name: "Tax year 2026: 2026 v1" })).toBeTruthy();
    expect(screen.getByText("items[0].kz is missing")).toBeTruthy();
    expect(screen.getByText("The file can't be loaded. Fix it in your editor, then reload again.")).toBeTruthy();
    expect(apply()).toBeDisabled();
  });

  it("disables Apply when only one of several years has a problem", () => {
    renderWithQuery(<ReloadSheet previews={[reloadPreview(), reloadPreview({ year: 2027, problems: ["bad json"] })]} onClose={() => {}} />);

    expect(apply()).toBeDisabled();
  });

  it("applies the reload, toasts and closes", async () => {
    const onClose = vi.fn();
    renderWithQuery(<ReloadSheet previews={[reloadPreview()]} onClose={onClose} />);

    await userEvent.click(apply());

    await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Reloaded the tax rules" }]));
    expect(mocked.rulesReloadApply).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("toasts that nothing changed when Rust applied no change", async () => {
    mocked.rulesReloadApply.mockResolvedValue(null);
    renderWithQuery(<ReloadSheet previews={[reloadPreview()]} onClose={() => {}} />);

    await userEvent.click(apply());

    await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Nothing changed" }]));
  });

  it("keeps the sheet open and shows the error when applying fails", async () => {
    mocked.rulesReloadApply.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    const onClose = vi.fn();
    renderWithQuery(<ReloadSheet previews={[reloadPreview()]} onClose={onClose} />);

    await userEvent.click(apply());

    await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "Disk full" }]));
    expect(onClose).not.toHaveBeenCalled();
    expect(apply()).toBeEnabled();
  });

  it("closes without applying on Cancel", async () => {
    const onClose = vi.fn();
    renderWithQuery(<ReloadSheet previews={[reloadPreview()]} onClose={onClose} />);

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(onClose).toHaveBeenCalled();
    expect(mocked.rulesReloadApply).not.toHaveBeenCalled();
  });
});
