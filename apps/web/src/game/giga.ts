// A GIGA permanent drawn across its four zones (SPEC §3.2, R3546004). The engine marks only the
// anchor's view with `giga: true` and leaves the three covered zones empty, so this module reads
// the mark off the view like the yellow glow: `gigaCellOf` names which of the four cells a zone
// is — the anchor, a covered cell, or none — and `Zone.tsx` stamps it as `data-giga` for
// `board.css` to paint (CLAUDE.md rule 7).

import type { PlayerView, Row } from "@jackioh/shared";

import { laneIndex, sideView, type Side } from "./contract.ts";

export type GigaCell = "anchor" | "covered";

/** Whether the unit or backrow entry in this zone carries the GIGA mark. */
function marked(view: PlayerView, side: Side, row: Row, lane: number): boolean {
  if (lane < 1) return false;
  const seat = sideView(view, side);
  const entry = row === "units" ? seat.units[laneIndex(lane)] : seat.backrow[laneIndex(lane)];
  return entry !== null && entry !== undefined && entry.giga === true;
}

export function gigaCellOf(view: PlayerView, side: Side, row: Row, lane: number): GigaCell | null {
  if (marked(view, side, row, lane)) return "anchor";
  const other: Row = row === "units" ? "backrow" : "units";
  if (marked(view, side, other, lane) || marked(view, side, "units", lane - 1) || marked(view, side, "backrow", lane - 1)) {
    return "covered";
  }
  return null;
}
