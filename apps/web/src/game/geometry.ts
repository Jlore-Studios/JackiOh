// Board geometry marks (SPEC §3, R3554000–R3554007). The engine tells which lane the Black Hole
// holds and which the slash voided, and which damage chain a Unit is in, so this module reads the
// marks off the view like the yellow glow: `laneMarkOf` names what a lane column wears — the hole
// or the slash — `blackHoleCount` what its badge counts, and `chainMarkOf` which chain a zone's
// Unit is in. `Board.tsx` stamps the lane, `Zone.tsx` the chain, for `board.css` to paint
// (CLAUDE.md rule 7).

import type { PlayerView, Row } from "@jackioh/shared";

import { laneIndex, sideView, type Side } from "./contract.ts";

export type LaneMark = "blackHole" | "slashed";

/** What the lane column wears: the hole, the slash, or nothing. */
export function laneMarkOf(view: PlayerView, lane: number): LaneMark | null {
  if (view.blackHole?.lane === lane) return "blackHole";
  if (view.slashedLane === lane) return "slashed";
  return null;
}

/** What the hole lane's badge counts (`"n/capacity"`), or nothing while no hole is open. */
export function blackHoleCount(view: PlayerView): string | null {
  const hole = view.blackHole;
  if (hole === undefined || hole === null) return null;
  return `${hole.swallowed}/${hole.capacity}`;
}

export type ChainMark = "shared" | "split" | "both";

/** Which damage chain the zone's Unit is in: shared, split, both, or none. */
export function chainMarkOf(view: PlayerView, side: Side, row: Row, lane: number): ChainMark | null {
  const seat = sideView(view, side);
  const entry = row === "units" ? seat.units[laneIndex(lane)] : seat.backrow[laneIndex(lane)];
  const shared = (entry as { sharedChain?: number } | null | undefined)?.sharedChain;
  const split = (entry as { splitChain?: number } | null | undefined)?.splitChain;
  if (shared !== undefined && split !== undefined) return "both";
  if (shared !== undefined) return "shared";
  if (split !== undefined) return "split";
  return null;
}
