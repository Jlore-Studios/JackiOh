// The top bar's geometry (R3218000, issue #3218): on a menu, Back is the leftmost control of BackLink's bar and
// the settings gear the rightmost, whatever else the bar holds (the pill, Friends, language, the screen's
// extras) and at both of BUILD M5-T1's sizes. A measure of the drawn boxes, so it fails a control that is
// out of order however the stylesheets place it. Spec 46 calls it on every menu; spec 13 on the replay's bar.

import { NAV_BACK, ts } from "./testids.ts";
import { SETTINGS_OPEN_NAV } from "./ux.ts";

/** An edge may sit this far past another's (a sub-pixel border, or a phone's second row ending at the gear's own edge) and tie. */
const EDGE_SLACK_PX = 1;

/** The bar of BackLink: `nav.screen-nav` (routes/nav.tsx). */
const BACK_LINK_BAR = "nav.screen-nav";
/** The landing's corner: its buttons beside the gear (routes/landing.tsx), with no Back. */
const LANDING_CORNER = ".landing-corner";

type Drawn = { id: string; left: number; right: number };

/** The bar's buttons and links that take room: a control the phone hides (the language button) has no box. */
function drawn(bar: HTMLElement): Drawn[] {
  return Array.from(bar.querySelectorAll<HTMLElement>("button, a[href]")).flatMap((control) => {
    const box = control.getBoundingClientRect();
    if (box.width === 0 || box.height === 0) return [];
    return [{ id: control.dataset["testid"] ?? control.tagName.toLowerCase(), left: box.left, right: box.right }];
  });
}

function gearIsRightmost(controls: Drawn[], where: string): Drawn {
  const gear = controls.find((control) => control.id === SETTINGS_OPEN_NAV);
  if (gear === undefined) throw new Error(`${where}: the settings gear is not drawn (${controls.map((control) => control.id).join(", ")})`);
  for (const control of controls) {
    expect(gear.right, `${where}: the gear's right edge is at or past ${control.id}'s`).to.be.at.least(control.right - EDGE_SLACK_PX);
  }
  return gear;
}

/** On the first BackLink bar the page draws: Back's left edge is the leftmost of its controls, the gear's right edge the rightmost. */
export function expectBackLinkBarOrder(): void {
  cy.get(BACK_LINK_BAR)
    .first()
    .should(($bar) => {
      const bar = $bar[0];
      if (bar === undefined) throw new Error("the page draws no BackLink bar");
      const controls = drawn(bar);
      const back = controls.find((control) => control.id === NAV_BACK);
      if (back === undefined) throw new Error(`the bar draws no Back (${controls.map((control) => control.id).join(", ")})`);
      for (const control of controls) {
        expect(back.left, `Back's left edge is at or before ${control.id}'s`).to.be.at.most(control.left + EDGE_SLACK_PX);
      }
      gearIsRightmost(controls, "the bar");
    });
}

/** On the landing's corner, which has no Back: the gear's right edge is the rightmost of its controls. */
export function expectLandingCornerOrder(): void {
  cy.get(LANDING_CORNER)
    .first()
    .should(($corner) => {
      const corner = $corner[0];
      if (corner === undefined) throw new Error("the landing draws no corner");
      gearIsRightmost(drawn(corner), "the landing's corner");
    });
}

/**
 * A Back-only bar (the replays', whose gear is in the board's control bar): its Back is the pill the bar's
 * skin draws, and the leftmost of the bar's controls (all it holds).
 */
export function expectReplayBackIsBarPill(testid: string): void {
  cy.get(ts(testid))
    .should("have.class", "top-bar-button--text")
    .closest(BACK_LINK_BAR)
    .should(($bar) => {
      const bar = $bar[0];
      if (bar === undefined) throw new Error("the replay's Back is in no BackLink bar");
      const controls = drawn(bar);
      expect(controls.map((control) => control.id), "a Back-only bar's controls").to.deep.equal([testid]);
    });
}
