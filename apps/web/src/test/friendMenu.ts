// A friend row's ⋯ menu in a test (R3732002): the row's one primary button is on the row, everything else is
// an item of its menu, so a test opens the menu before it reaches for Chat, Daily challenge, Invite to lobby,
// Watch, Remove or Block.

import { fireEvent, within } from "@testing-library/react";

import { togetherTestid } from "../together/testids.ts";

/** Opens `row`'s ⋯ menu and answers its list; the menu already open is left open. */
export function openFriendMenu(row: HTMLElement): HTMLElement {
  const button = within(row).getByTestId(togetherTestid.menu);
  if (button.getAttribute("aria-expanded") !== "true") fireEvent.click(button);
  return within(row).getByRole("menu");
}

/** Opens `row`'s menu and presses the item with `testid`; answers the item pressed. */
export function pressFriendMenuItem(row: HTMLElement, testid: string): HTMLElement {
  const item = within(openFriendMenu(row)).getByTestId(testid);
  fireEvent.click(item);
  return item;
}
