// R502: which `cardPlayed` in a redacted stream is a cast on draw.
//
// SPEC §2.4 and R58: a card with "Cast on draw" (#21 Hinder, #27 Blood Ridden Glowy Jelly Bean, and
// anything a Classic+ card gives the keyword) is cast the moment it is drawn, and never enters the
// hand. The engine says so only by the order of its events: `drawn` for the card, then (after any
// `cardAnnounced` of the same card, B5 E1) its `cardPlayed`. Every other draw is followed by the
// card arriving somewhere, `addedToHand` or `burned`, so a `cardPlayed` right after its own `drawn` is
// the one reliable signal, and it needs no card's definition to read.
//
// R97 keeps both events' seats and replaces a card the viewer may not read with the sentinel in
// both, so the sentinel matches any id here: a hidden cast on draw is still one (the viewer is told a
// card was cast as it was drawn, never which). R227: a card set face-down as it is cast takes a fresh
// id, and its `cardPlayed` carries the id it was drawn with as `formerId`.
//
// Pure: it reads its arguments and nothing else.

import type { GameEvent } from "@jackioh/shared";

import { HIDDEN_ID } from "../game/animations.ts";


function sameCard(a: string, b: string): boolean {
  return a === b || a === HIDDEN_ID || b === HIDDEN_ID;
}

/** Whether `events[at]` is a `cardPlayed` whose card was cast as it was drawn. */
export function castOnDrawAt(events: readonly GameEvent[], at: number): boolean {
  const played = events[at];
  if (played === undefined || played.type !== "cardPlayed") return false;
  const ids = [played.instanceId, ...(played.formerId === undefined ? [] : [played.formerId])];
  const same = (id: string): boolean => ids.some((own) => sameCard(own, id));
  for (let i = at - 1; i >= 0; i -= 1) {
    const before = events[i];
    if (before === undefined) return false;
    if (before.type === "cardAnnounced" && before.player === played.player && same(before.instanceId)) continue;
    return before.type === "drawn" && before.player === played.player && same(before.instanceId);
  }
  return false;
}
