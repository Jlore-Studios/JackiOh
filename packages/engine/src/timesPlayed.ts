// R429: how many times a card has been played (SPEC §10.1, §10.5 step 4), for the one Core card that
// counts its own plays, #31 KY's Math Equation — "Deal Fib(times played + 1) damage".
//
// The count lives on the instance, like `costMod`, so it rides the card through every zone and
// through leaving the field (R78 leaves it alone), and a copy or a Transform — a new instance — starts
// its own (R57). It is written in one place, §10.5 step 4, where every play is counted (the turn log,
// the game's `played` counter): a cast is a play there too (R70), and a countered play never reaches
// that step, so it is never counted. Only a card whose script sets `StaticFlags.countsPlays` carries
// the field at all, so a game without one hashes as it did before the field existed.

import type { CardInstance } from "./state";
import { flagsOf } from "./scripts";

/** R429: the plays this card has had so far, the one under way included once step 4 has run. */
export function timesPlayedOf(card: CardInstance): number {
  const count = card.timesPlayed;
  return typeof count === "number" && Number.isFinite(count) && count > 0 ? Math.trunc(count) : 0;
}

/** R429, §10.5 step 4: one more play of this card, when its script counts them. */
export function countPlay(card: CardInstance): void {
  if (flagsOf(card).countsPlays !== true) return;
  card.timesPlayed = timesPlayedOf(card) + 1;
}
