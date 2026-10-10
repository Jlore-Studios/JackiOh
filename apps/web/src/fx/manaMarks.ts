// R502: #21 Hinder reads `nextTurnMana` from `SideView.modifiers`, not an event, to mark the crystals its next refresh will not fill (§6.3 Mana, R169).
// This is persistent board information, including under reduced motion; newly reached riders read the newest view.

import type { PlayerId, PlayerView, SideView } from "@jackioh/shared";

import { sideOf, type Side } from "../game/contract.ts";
import { FX_NEXT_REFRESH_MODIFIER_ID } from "./constants.ts";

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

/** Board.tsx ManaTray count: maximum mana or current mana if higher. */
export function crystalsShown(mana: SideView["mana"]): number {
  return Math.max(0, mana.max, mana.current);
}

export type CrystalRun = { from: number; count: number };

const NONE: CrystalRun = { from: 0, count: 0 };

function sideFor(view: PlayerView, player: PlayerId): SideView {
  return sideOf(view, player) === "you" ? view.you : view.opponent;
}

/** Reads the rider from `riderView` but lays the mark on `trayView`. */
export function lostCrystals(riderView: PlayerView | undefined, trayView: PlayerView, player: PlayerId): CrystalRun {
  if (riderView === undefined) return NONE;
  const rider = nextRefreshRider(sideFor(riderView, player));
  if (rider === null || rider >= 0) return NONE;
  const shown = crystalsShown(sideFor(trayView, player).mana);
  const count = Math.min(-rider, shown);
  return count > 0 ? { from: shown - count, count } : NONE;
}

export type ManaMark = { side: Side; run: CrystalRun };

export function manaMarks(view: PlayerView, early?: { view: PlayerView; sides: ReadonlySet<Side> }): ManaMark[] {
  return (["you", "opponent"] as const).map((side) => {
    const player = (side === "you" ? view.you : view.opponent).player;
    const riderView = early !== undefined && early.sides.has(side) ? early.view : view;
    return { side, run: lostCrystals(riderView, view, player) };
  });
}

export const HINDERED_ATTR = "data-fx-hindered";

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
