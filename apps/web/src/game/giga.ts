// A GIGA permanent drawn across its four zones (SPEC §3.2, R3546004). The engine marks only the
// anchor's view with `giga: true` and leaves the three covered zones empty, so this module reads
// the mark off the view like the yellow glow: `gigaCellOf` names which of the four cells a zone
// is — the anchor, a covered cell, or none — and `Zone.tsx` stamps it as `data-giga` for
// `board.css` to paint (CLAUDE.md rule 7).

import type { PlayerView, Row } from "@jackioh/shared";

import { hasKeyword } from "../wire/catalog.ts";
import { lookupFromDefs, type CardLookup } from "./catalog.ts";
import { laneIndex, sideView, testid, type Side } from "./contract.ts";

export type GigaCell = "anchor" | "covered";

/** Whether the unit or backrow entry in this zone carries the GIGA mark. */
function marked(view: PlayerView, side: Side, row: Row, lane: number): boolean {
  if (lane < 1) return false;
  const seat = sideView(view, side);
  const entry = row === "units" ? seat.units[laneIndex(lane)] : seat.backrow[laneIndex(lane)];
  return entry !== null && entry !== undefined && entry.giga === true;
}

/** #4995: one offered placement of a GIGA card: the anchor lane L of its row. */
export type GigaAnchor = { row: Row; lane: number };

/**
 * #4995: the four zones a GIGA played at `anchor` will take (SPEC §3.2): the anchor, the other
 * row's same lane, and both rows' lane L+1.
 */
export function gigaFootprint(anchor: GigaAnchor): GigaAnchor[] {
  const other: Row = anchor.row === "units" ? "backrow" : "units";
  return [anchor, { row: other, lane: anchor.lane }, { row: anchor.row, lane: anchor.lane + 1 }, { row: other, lane: anchor.lane + 1 }];
}

/** #4995: the viewer-relative testids of the four zones a GIGA played at `anchor` will take. */
export function gigaFootprintTestids(side: Side, anchor: GigaAnchor): string[] {
  return gigaFootprint(anchor).map((cell) => testid.zone(side, cell.row, cell.lane));
}

const ANCHOR_TESTID = /^zone-you-(units|backrow)-(\d+)$/;

/**
 * #4995: the offered GIGA anchor an offered zone testid names, or null. Plays land on the
 * viewer's own side, and `highlightFor` already scopes the offered anchors there, so only
 * `zone-you-…` testids answer.
 */
export function anchorOfTestid(id: string): GigaAnchor | null {
  const match = ANCHOR_TESTID.exec(id);
  if (match === null) return null;
  const row = match[1] as Row;
  const lane = Number(match[2]);
  if (!Number.isInteger(lane) || lane < 1) return null;
  return { row, lane };
}

/**
 * #4995: whether the hand card `instanceId` is a GIGA now. `CardView.keywords` is override-only
 * (B5 E38): when it is set it answers alone, covering hand-granted GIGA; otherwise the printed
 * face answers, covering printed GIGA with no live override.
 */
export function isGigaHandCard(view: PlayerView, lookup: CardLookup | null, instanceId: string): boolean {
  const hand = view.you.hand;
  if (!Array.isArray(hand)) return false;
  const found = hand.find((entry) => entry.instanceId === instanceId);
  if (found === undefined) return false;
  if (found.keywords !== undefined) return hasKeyword(found.keywords, "GIGA");
  const printed = lookup?.(found.defId, found.radiant)?.def ?? lookupFromDefs(view.defs ?? {})(found.defId, found.radiant)?.def;
  if (printed === undefined) return false;
  const face = found.radiant ? printed.radiant : printed.base;
  return hasKeyword(face.keywords, "GIGA");
}

export function gigaCellOf(view: PlayerView, side: Side, row: Row, lane: number): GigaCell | null {
  if (marked(view, side, row, lane)) return "anchor";
  const other: Row = row === "units" ? "backrow" : "units";
  if (marked(view, side, other, lane) || marked(view, side, "units", lane - 1) || marked(view, side, "backrow", lane - 1)) {
    return "covered";
  }
  return null;
}
