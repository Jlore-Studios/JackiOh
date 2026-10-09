// R502: the mana crystals the next refresh will not fill, marked on the board.
//
// #21 Hinder lowers its victim's next mana refresh (§6.3 Mana, R169). No event carries the amount
// (`modifierChanged` names only `nextTurnMana`); the view does, as `{ id: "nextTurnMana", label:
// "Next refresh −1 mana" }` in `SideView.modifiers` on both seats. `nextRefreshRider` is the one reader
// of that label, so the number is the view's and never a guess from the card.
//
// The mark is the board's own crystals, frosted and cracked: the last N the tray shows, N being how far
// the rider takes the refresh below zero. It is information, not an effect: drawn under reduced motion
// too (fx.css) and at any intensity. For a side whose rider the runner has just reached, it reads the
// newest view, so the crystals crack as the effect lands rather than when the whole burst has played.
//
// `applyManaMarks` writes `data-fx-hindered` on each marked `.mana-crystal` and the count on its tray.

import type { PlayerId, PlayerView, SideView } from "@jackioh/shared";

import { sideOf, type Side } from "../game/contract.ts";
import { FX_NEXT_REFRESH_MODIFIER_ID } from "./constants.ts";

/** The engine's caption: "Next refresh +2 mana" or "Next refresh −1 mana" (U+2212, or a hyphen). */
const RIDER_LABEL = /([+−-])\s*(\d+)/;

/** How far the next refresh moves (negative: lower), or null when the view lists no rider. */
export function nextRefreshRider(side: Pick<SideView, "modifiers">): number | null {
  const badge = side.modifiers.find((modifier) => modifier.id === FX_NEXT_REFRESH_MODIFIER_ID);
  if (badge === undefined) return null;
  const match = RIDER_LABEL.exec(badge.label);
  if (match === null) return null;
  const amount = Number(match[2]);
  if (!Number.isFinite(amount)) return null;
  return match[1] === "+" ? amount : -amount;
}

/** The crystals a tray draws for this mana (Board.tsx's ManaTray: max, or current when above it). */
export function crystalsShown(mana: SideView["mana"]): number {
  return Math.max(0, mana.max, mana.current);
}

/** The run of crystals a mark covers: the last `count` of the tray, starting at `from`. */
export type CrystalRun = { from: number; count: number };

const NONE: CrystalRun = { from: 0, count: 0 };

function sideFor(view: PlayerView, player: PlayerId): SideView {
  return sideOf(view, player) === "you" ? view.you : view.opponent;
}

/**
 * The crystals `player`'s next refresh will not fill: the rider read off `riderView`, laid on the tray
 * drawn from `trayView`. Nothing when there is no rider or it does not lower the refresh.
 */
export function lostCrystals(riderView: PlayerView | undefined, trayView: PlayerView, player: PlayerId): CrystalRun {
  if (riderView === undefined) return NONE;
  const rider = nextRefreshRider(sideFor(riderView, player));
  if (rider === null || rider >= 0) return NONE;
  const shown = crystalsShown(sideFor(trayView, player).mana);
  const count = Math.min(-rider, shown);
  return count > 0 ? { from: shown - count, count } : NONE;
}

/** One tray's mark. */
export type ManaMark = { side: Side; run: CrystalRun };

/** The mark on both trays of `view`; `early` names the sides whose rider is read off the newest view. */
export function manaMarks(view: PlayerView, early?: { view: PlayerView; sides: ReadonlySet<Side> }): ManaMark[] {
  return (["you", "opponent"] as const).map((side) => {
    const player = (side === "you" ? view.you : view.opponent).player;
    const riderView = early !== undefined && early.sides.has(side) ? early.view : view;
    return { side, run: lostCrystals(riderView, view, player) };
  });
}

export const HINDERED_ATTR = "data-fx-hindered";

/** Writes the marks onto the trays in `root`; a tray with no run loses any mark it had. */
export function applyManaMarks(root: ParentNode, marks: readonly ManaMark[]): void {
  for (const { side, run } of marks) {
    const tray = root.querySelector(`[data-testid="mana-${side}"]`);
    if (tray === null) continue;
    if (run.count > 0) tray.setAttribute(HINDERED_ATTR, String(run.count));
    else tray.removeAttribute(HINDERED_ATTR);
    tray.querySelectorAll(".mana-crystal").forEach((crystal, index) => {
      if (run.count > 0 && index >= run.from && index < run.from + run.count) crystal.setAttribute(HINDERED_ATTR, "true");
      else crystal.removeAttribute(HINDERED_ATTR);
    });
  }
}
