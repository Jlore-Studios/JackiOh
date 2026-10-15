// Where on a board a ping points (#936 B64): the element Game.tsx draws for a hero, a zone or a hand
// card, read back from its testid (`game/contract.ts`), and the S6 `PingTarget` it means in the duel
// that board shows. Game.tsx stays the one board, so the Duos layer finds its anchors by the testids
// every other layer uses, and adds nothing to the board itself.
//
// A board always shows a view of the viewer's own team: its "you" side is the team's player in that
// duel (team A's `p1`, team B's `p2`), its "opponent" side the enemy's. A hand target only ever names
// "you" (S6: the sender's own team's side), the one hand the view holds as cards.

import type { PlayerView, Row } from "@jackioh/shared";

import { testid, type Side } from "../game/contract.ts";
import type { PingTarget } from "../net/duos.ts";

/** A pingable place on a board, as the board names it: lanes 1-based, as its zone testids count. */
export type BoardSpot =
  | { at: "hand"; instanceId: string }
  | { at: "hero"; side: Side }
  | { at: "zone"; side: Side; row: Row; lane: number };

const HAND_CARD_PREFIX = testid.handCard("");
const ZONE_PREFIX = "zone-";

/** Every element a spot is read from: a hand card, a hero, or a zone (whose card a hover lands in). */
export const SPOT_SELECTOR = [
  `[data-testid^="${HAND_CARD_PREFIX}"]`,
  `[data-testid="${testid.hero("you")}"]`,
  `[data-testid="${testid.hero("opponent")}"]`,
  `[data-testid^="${ZONE_PREFIX}"][data-side][data-row][data-lane]`,
].join(", ");

function sideOfAttr(value: string | null): Side | null {
  return value === "you" || value === "opponent" ? value : null;
}

/** The spot the element is, or is inside; null for anything else on the page. */
export function spotOfElement(element: Element | null): BoardSpot | null {
  const hit = element?.closest(SPOT_SELECTOR) ?? null;
  if (hit === null) return null;
  const id = hit.getAttribute("data-testid") ?? "";
  if (id.startsWith(HAND_CARD_PREFIX)) {
    const instanceId = id.slice(HAND_CARD_PREFIX.length);
    return instanceId.length === 0 ? null : { at: "hand", instanceId };
  }
  if (id === testid.hero("you")) return { at: "hero", side: "you" };
  if (id === testid.hero("opponent")) return { at: "hero", side: "opponent" };
  const side = sideOfAttr(hit.getAttribute("data-side"));
  const row = hit.getAttribute("data-row");
  const lane = Number(hit.getAttribute("data-lane"));
  if (side === null || (row !== "units" && row !== "backrow") || !Number.isInteger(lane) || lane < 1) return null;
  return { at: "zone", side, row, lane };
}

/** The testid of the element a spot is drawn as. */
export function spotTestid(spot: BoardSpot): string {
  switch (spot.at) {
    case "hand":
      return testid.handCard(spot.instanceId);
    case "hero":
      return testid.hero(spot.side);
    case "zone":
      return testid.zone(spot.side, spot.row, spot.lane);
  }
}

export function sameSpot(a: BoardSpot | null, b: BoardSpot | null): boolean {
  if (a === null || b === null) return a === b;
  return spotTestid(a) === spotTestid(b);
}

/** The viewer's own hand as cards, or none (a view only ever holds its own hand as cards). */
function handOf(view: PlayerView): readonly { instanceId: string }[] {
  return Array.isArray(view.you.hand) ? view.you.hand : [];
}

/** What a spot on `view`'s board points at in `duel`; null for a hand card the view no longer holds. */
export function targetOfSpot(spot: BoardSpot, view: PlayerView, duel: 0 | 1): PingTarget | null {
  switch (spot.at) {
    case "hand": {
      const index = handOf(view).findIndex((card) => card.instanceId === spot.instanceId);
      return index < 0 ? null : { duel, at: "hand", side: view.you.player, index };
    }
    case "hero":
      return { duel, at: "hero", side: spot.side === "you" ? view.you.player : view.opponent.player };
    case "zone":
      return {
        duel,
        at: spot.row === "units" ? "unit" : "backrow",
        side: spot.side === "you" ? view.you.player : view.opponent.player,
        lane: spot.lane - 1,
      };
  }
}

/**
 * The spot on `view`'s board a target names, when that board is of the target's duel; null when it
 * is the other duel's (the portal flashes instead) or names a hand card the view does not hold.
 */
export function spotOfTarget(target: PingTarget, view: PlayerView, duel: 0 | 1): BoardSpot | null {
  if (target.duel !== duel) return null;
  const side: Side = target.side === view.you.player ? "you" : "opponent";
  switch (target.at) {
    case "hero":
      return { at: "hero", side };
    case "unit":
    case "backrow":
      return { at: "zone", side, row: target.at === "unit" ? "units" : "backrow", lane: target.lane + 1 };
    case "hand": {
      if (side !== "you") return null;
      const card = handOf(view)[target.index];
      return card === undefined ? null : { at: "hand", instanceId: card.instanceId };
    }
  }
}
