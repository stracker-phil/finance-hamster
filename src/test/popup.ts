import { screen } from "@testing-library/react";
import type userEvent from "@testing-library/user-event";
import { createElement, Fragment, type ReactElement } from "react";
import { MenuHost } from "../components/menu";

/** `ui` next to the `<MenuHost />` that shows the menus of its pop-up buttons. */
export function withMenus(ui: ReactElement): ReactElement {
  return createElement(Fragment, null, ui, createElement(MenuHost));
}

const popUpButton = (label: string | RegExp) => screen.getByRole("button", { name: label });

/** Opens the pop-up button named `label` and clicks the row named `option`. */
export async function choose(user: Pick<typeof userEvent, "click">, label: string | RegExp, option: string | RegExp) {
  await user.click(popUpButton(label));
  await user.click(await screen.findByRole("menuitemradio", { name: option }));
}

/** The choice the pop-up button named `label` shows. */
export function popUpValue(label: string | RegExp): string {
  return popUpButton(label).querySelector(".popup-button__value > span")?.textContent ?? "";
}

/** The rows the pop-up button named `label` offers, in order; closes the menu again. */
export async function popUpOptions(user: Pick<typeof userEvent, "click" | "keyboard">, label: string | RegExp): Promise<string[]> {
  await user.click(popUpButton(label));
  const rows = (await screen.findAllByRole("menuitemradio")).map((row) => row.textContent ?? "");
  await user.keyboard("{Escape}");
  return rows;
}
