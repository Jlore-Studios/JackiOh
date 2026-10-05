// Brittle X's count on a card instance (docs/classic-sets.md B3.3, R385, R441, R638): the readers and
// the writes that need no sink. The count lives on the instance (`CardInstance.brittle`) and is kept in
// every zone the card passes through, hand to field included, like `radiant` and `costMod` (R78), but
// it only ticks on the field (R638); a copy never inherits it (R57). The start-of-turn tick and the
// crumbling are `brittle.ts`'s.
//
// This module imports nothing heavier than the catalog, so the modules every field arrival and every
// stat read pass through — `zones.ts` (`startBrittleOnField`) and `layers.ts` (`activeBrittleCount`) —
// can read it without pulling the destroy and state-check machinery the tick needs.

import { numberedSum, tunedCount } from "./tuning";
import { cardTypeOf, runningFace } from "./faces";
import type { CardInstance, GameState } from "./state";

/**
 * B3.3 rule 1: the Brittle a card prints on its running face, as its tuning leaves it (a Degrade or
 * Upgrade of the printed number before the count has started, B3.4), or null when it prints none.
 * A Vanilla card prints nothing (§6.3).
 */
export function printedBrittleOf(state: GameState, card: CardInstance): number | null {
  if (card.vanilla) return null;
  const printed = numberedSum(runningFace(state, card).keywords, "Brittle");
  if (printed === null) return null;
  // The same step `tuning.tunedKeywords` prints the keyword with, so the layers and the count agree.
  return tunedCount(card, "Brittle", printed);
}

/**
 * B3.3 rule 5: the count that is in force on the card now, or null for none. A count its printed
 * Brittle started is the card's text, which a Vanilla switches off while it lasts; a count an effect
 * gave stays, as §10.4 keeps every granted keyword.
 */
export function activeBrittleCount(card: Pick<CardInstance, "brittle" | "vanilla">): number | null {
  const brittle = card.brittle;
  if (brittle === undefined) return null;
  if (brittle.printed === true && card.vanilla) return null;
  return brittle.count;
}

/**
 * B3.3 rule 1, R638: "a printed Brittle starts when the card enters the field" — called where every
 * field arrival passes (`zones.placeOnField`, `zones.replaceInZone`). A card that already has a count
 * keeps it (a count is kept in every zone, so a card that left the field and came back ticks on), and
 * one that prints no Brittle starts nothing. `fromOffField` is a card that arrives from a hand, a
 * deck, a graveyard or the resolving zone rather than from another field zone: a count it held
 * there never ticked, so its turn cycle starts now, and its first tick waits for a whole round on the
 * field (`BRITTLE_FIRST_TICK_TURNS`) however long it was held.
 *
 * R666: a backrow Trap or Field Trap that enters face-down starts no count — there is no Brittle
 * while it is unrevealed (Classic+ #74). The count starts when the card reveals: its own subsystem
 * starts it with its first activation, and the `reveal` effect starts one for any card it shows.
 */
export function startBrittleOnField(state: GameState, card: CardInstance, fromOffField: boolean): void {
  if (card.brittle !== undefined) {
    if (fromOffField) card.brittle = { ...card.brittle, since: state.turn };
    return;
  }
  if (card.zone.z === "field" && card.zone.row === "backrow") {
    const type = cardTypeOf(state, card);
    if ((type === "Trap" || type === "Field Trap") && card.faceUp !== true && card.revealed !== true) return;
  }
  const printed = printedBrittleOf(state, card);
  if (printed === null || printed <= 0) return;
  card.brittle = { count: printed, since: state.turn, printed: true };
}

/**
 * B3.3 rule 4: "Give Brittle N" sets the count to N and starts it now, whatever the card had; the
 * count is a given one from then on, so a Vanilla keeps it.
 */
export function giveBrittleCount(state: GameState, card: CardInstance, count: number): void {
  card.brittle = { count: Math.max(0, Math.trunc(count)), since: state.turn };
}

/**
 * B3.3 rule 4: "gain +N Brittle" adds N to the count in force. A card with no count yet — a given one
 * or one its printed Brittle started — starts one now at N more than it prints (R441), which a Vanilla
 * then leaves alone as a given one.
 */
export function gainBrittleCount(state: GameState, card: CardInstance, amount: number): void {
  const add = Math.trunc(amount);
  const active = activeBrittleCount(card);
  if (active !== null && card.brittle !== undefined) {
    card.brittle = { ...card.brittle, count: Math.max(0, active + add) };
    return;
  }
  const printed = printedBrittleOf(state, card) ?? 0;
  card.brittle = { count: Math.max(0, printed + add), since: state.turn };
}

/**
 * R441: a count that has crumbled its card is spent. R78's reset (`zones.resetInstance`) keeps a
 * Brittle count in every zone but this one, so a crumbled card that comes back — from its graveyard,
 * or a Reborn body — is not Brittle 0 for ever, crumbling again at every tick.
 */
export function dropSpentBrittle(card: CardInstance): void {
  if (card.brittle !== undefined && card.brittle.count <= 0) delete card.brittle;
}
