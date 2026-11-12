// The menus' shell's navigation (R3728000, issue #3728): inside `menu-shell-nav`, Home, Play, Collection
// and Shop and then Friends (the Friends button signed in, once the friends feature has answered; a link to
// Play together signed out) are drawn, each box inside the window, at every one of `MENU_SIZES`: the labelled
// rail over 1024 px, the icon rail down to 601 px, the tab bar at 600 px and under. Presence and place only:
// spec 47 measures their tap areas and spec 51 follows them. Spec 46 calls it on every menu inside the shell,
// spec 38 before its axe runs on the shell.

import { menuShellTestid } from "../../apps/web/src/shell/testids.ts";
import { timeouts } from "./config.ts";
import { FRIENDS_BUTTON, ts } from "./testids.ts";

/** A box's edge may sit this far past the window's (a sub-pixel border) and still be inside it. */
const EDGE_SLACK_PX = 1;

/** The shell's five items in its navigation, each shown and inside the window. */
export function expectShellNav(signedIn: boolean): void {
  const items = [menuShellTestid.home, menuShellTestid.play, menuShellTestid.collection, menuShellTestid.shop, signedIn ? FRIENDS_BUTTON : menuShellTestid.friends];
  cy.get(ts(menuShellTestid.nav), { timeout: timeouts.view }).within(() => {
    for (const item of items) {
      cy.get(ts(item), { timeout: timeouts.view })
        .should("be.visible")
        .and(($item) => {
          const el = $item[0];
          const win = el?.ownerDocument.defaultView;
          if (el === undefined || win === null || win === undefined) throw new Error(`${item} is in no window`);
          const box = el.getBoundingClientRect();
          expect(box.left, `${item}'s left edge`).to.be.at.least(-EDGE_SLACK_PX);
          expect(box.top, `${item}'s top edge`).to.be.at.least(-EDGE_SLACK_PX);
          expect(box.right, `${item}'s right edge`).to.be.at.most(win.innerWidth + EDGE_SLACK_PX);
          expect(box.bottom, `${item}'s bottom edge`).to.be.at.most(win.innerHeight + EDGE_SLACK_PX);
        });
    }
  });
}
