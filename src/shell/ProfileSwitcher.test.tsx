import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { MenuHost } from "../components/menu";
import { Toasts } from "../components/Toasts";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { ProfileSwitcher } from "./ProfileSwitcher";

const mocked = vi.mocked(api, true);

const TWO = { active: "p1", profiles: [{ id: "p1", name: "Demo" }, { id: "p2", name: "Alex" }] };

function renderSwitcher() {
  renderWithQuery(
    <>
      <ProfileSwitcher />
      <MenuHost />
      <Toasts />
    </>,
  );
}

const openMenu = async () => userEvent.click(await screen.findByRole("button", { name: "Profile: Demo" }));
const chooseItem = async (name: string | RegExp) => {
  await openMenu();
  await userEvent.click(screen.getByRole("menuitem", { name }));
};

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], menu: null });
  mocked.profilesList.mockResolvedValue(TWO);
  mocked.profileCreate.mockResolvedValue(undefined);
  mocked.profileRename.mockResolvedValue(undefined);
  mocked.profileSwitch.mockResolvedValue(undefined);
});

describe("ProfileSwitcher", () => {
  it("shows the active profile's name on a button named Profile: <name>", async () => {
    renderSwitcher();

    expect((await screen.findByRole("button", { name: "Profile: Demo" })).textContent).toBe("Demo");
  });

  it("renders nothing when the active profile is not in the list", async () => {
    mocked.profilesList.mockResolvedValue({ active: "gone", profiles: [{ id: "p1", name: "Demo" }] });
    renderSwitcher();

    await waitFor(() => expect(mocked.profilesList).toHaveBeenCalled());
    expect(screen.queryByRole("button", { name: /^Profile/ })).toBeNull();
  });

  describe("menu", () => {
    it("lists every profile with the active one checked, then New Profile… and Rename Profile…", async () => {
      renderSwitcher();

      await openMenu();

      expect(screen.getByText("Profiles")).toBeTruthy();
      expect(screen.getByRole("menuitemradio", { name: "Demo" }).getAttribute("aria-checked")).toBe("true");
      expect(screen.getByRole("menuitemradio", { name: "Alex" }).getAttribute("aria-checked")).toBe("false");
      expect(screen.getByRole("menuitem", { name: "New Profile…" })).toBeTruthy();
      expect(screen.getByRole("menuitem", { name: "Rename Profile…" })).toBeTruthy();
    });

    it("switches to the chosen profile", async () => {
      renderSwitcher();
      await openMenu();

      await userEvent.click(screen.getByRole("menuitemradio", { name: "Alex" }));

      expect(mocked.profileSwitch).toHaveBeenCalledWith("p2");
    });

    it("does not switch when the active profile is chosen", async () => {
      renderSwitcher();
      await openMenu();

      await userEvent.click(screen.getByRole("menuitemradio", { name: "Demo" }));

      expect(mocked.profileSwitch).not.toHaveBeenCalled();
      expect(screen.queryByRole("menu")).toBeNull();
    });

    it("shows an error toast when the switch is rejected", async () => {
      mocked.profileSwitch.mockRejectedValue({ kind: "io", message: "Folder is locked", code: null });
      renderSwitcher();
      await openMenu();

      await userEvent.click(screen.getByRole("menuitemradio", { name: "Alex" }));

      expect(await screen.findByText("Folder is locked")).toBeTruthy();
      expect(screen.getByRole("alert")).toBeTruthy();
    });
  });

  describe("New Profile sheet", () => {
    it("opens with an empty Name field and a disabled Create and Open button", async () => {
      renderSwitcher();

      await chooseItem("New Profile…");

      expect(screen.getByRole("dialog", { name: "New Profile" })).toBeTruthy();
      expect(screen.getByRole<HTMLInputElement>("textbox", { name: "Name" }).value).toBe("");
      expect(screen.getByRole("button", { name: "Create and Open" })).toBeDisabled();
    });

    it("keeps Create and Open disabled while the name is only spaces", async () => {
      renderSwitcher();
      await chooseItem("New Profile…");

      await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "   ");

      expect(screen.getByRole("button", { name: "Create and Open" })).toBeDisabled();
    });

    it("says the new, empty profile opens right away", async () => {
      renderSwitcher();

      await chooseItem("New Profile…");

      expect(screen.getByText(/Hamster opens the new, empty profile right away\./)).toBeTruthy();
    });

    it("creates the profile with the typed name and closes the sheet", async () => {
      renderSwitcher();
      await chooseItem("New Profile…");

      await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Casey");
      await userEvent.click(screen.getByRole("button", { name: "Create and Open" }));

      expect(mocked.profileCreate).toHaveBeenCalledWith("Casey");
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    });

    it("shows a rejection inline as an alert and stays open", async () => {
      mocked.profileCreate.mockRejectedValue({ kind: "invalid_input", message: "There is already a profile named Alex", code: null });
      renderSwitcher();
      await chooseItem("New Profile…");

      await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Alex");
      await userEvent.click(screen.getByRole("button", { name: "Create and Open" }));

      const alert = await screen.findByRole("alert");
      expect(alert.textContent).toBe("There is already a profile named Alex");
      expect(screen.getByRole("dialog", { name: "New Profile" })).toBeTruthy();
    });

    it("clears the inline error when the name is edited", async () => {
      mocked.profileCreate.mockRejectedValue({ kind: "invalid_input", message: "There is already a profile named Alex", code: null });
      renderSwitcher();
      await chooseItem("New Profile…");
      await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Alex");
      await userEvent.click(screen.getByRole("button", { name: "Create and Open" }));
      await screen.findByRole("alert");

      await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "2");

      expect(screen.queryByRole("alert")).toBeNull();
    });

    it("closes on Cancel without creating", async () => {
      renderSwitcher();
      await chooseItem("New Profile…");

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("dialog")).toBeNull();
      expect(mocked.profileCreate).not.toHaveBeenCalled();
    });
  });

  describe("Rename Profile sheet", () => {
    it("opens prefilled with the active profile's name", async () => {
      renderSwitcher();

      await chooseItem("Rename Profile…");

      expect(screen.getByRole("dialog", { name: "Rename Profile" })).toBeTruthy();
      expect(screen.getByRole<HTMLInputElement>("textbox", { name: "Name" }).value).toBe("Demo");
    });

    it("renames the active profile and closes the sheet", async () => {
      renderSwitcher();
      await chooseItem("Rename Profile…");

      const name = screen.getByRole("textbox", { name: "Name" });
      await userEvent.clear(name);
      await userEvent.type(name, "Casey");
      await userEvent.click(screen.getByRole("button", { name: "Rename" }));

      expect(mocked.profileRename).toHaveBeenCalledWith("p1", "Casey");
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    });

    it("shows a rejection inline and keeps the sheet open", async () => {
      mocked.profileRename.mockRejectedValue({ kind: "invalid_input", message: "There is already a profile named Alex", code: null });
      renderSwitcher();
      await chooseItem("Rename Profile…");

      const name = screen.getByRole("textbox", { name: "Name" });
      await userEvent.clear(name);
      await userEvent.type(name, "Alex");
      await userEvent.click(screen.getByRole("button", { name: "Rename" }));

      expect((await screen.findByRole("alert")).textContent).toBe("There is already a profile named Alex");
      expect(screen.getByRole("dialog", { name: "Rename Profile" })).toBeTruthy();
    });
  });
});
