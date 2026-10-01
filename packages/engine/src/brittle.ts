// Brittle X (docs/classic-sets.md B3.3, R385): the count on a card instance, its start-of-turn tick
// and its crumbling. `turn.ts` runs `brittleTick` as a stage of the start of a turn, right after the
// mana refresh (§2.2, R62), and settles after it like its other stages: the tick itself only moves
// counts, marks and cards, and opens no prompt, so everything it causes — a Death hook of a unit it
// destroyed, a trap answering a crumble — resolves in that settle and parks on `state.work` there.
//
// The count's readers and its writes that need no sink are `brittleCount.ts`'s, so `zones.ts` and
// `layers.ts` can read them without importing the destroy this module needs.
//
// Hidden information (R440): a count that ticks on a card the other player may not read — in a hand,
// in a deck, a face-down trap — ticks silently, since a `counterChanged` there, redacted or not,
// would tell the other player that a hidden card is Brittle. Its owner reads the count on the card
// (`CardView.brittle`). A crumble is never silent: the card goes to a graveyard, which is public.

import type { PlayerId } from "@jackioh/shared";
import { activeBrittleCount } from "./brittleCount";
import { BRITTLE_FIRST_TICK_TURNS, BRITTLE_TICK } from "./config";
import { destroy } from "./effects/destroy";
import { isFaceDown } from "./preview";
import { makeContext, type EngineSink } from "./resolve";
import type { CardInstance, GameState } from "./state";
import { activeUnitsOf, cardAt, moveToZone, slotsOf } from "./zones";

export {
  activeBrittleCount,
  gainBrittleCount,
  giveBrittleCount,
  printedBrittleOf,
  startPrintedBrittle,
} from "./brittleCount";

/** B3.3 rule 3: where a crumbling card was, as `crumbled` names it. */
type CrumbleZone = "field" | "hand" | "library";

/**
 * B3.3 rule 2: every card whose count this player's start of turn ticks, in R68's order — the cards
 * they control on the field (units by lane, the top of each pile only, since a card dormant under a
 * Stack is not on the field, R13; then the backrow by lane), then their hand, then their deck top
 * down, whose cards are their own (a count in a hand or a deck ticks on its owner's turn).
 */
function tickedCards(state: GameState, player: PlayerId): { card: CardInstance; zone: CrumbleZone }[] {
  const side = state.players[player];
  const backrow = slotsOf(player, "backrow").flatMap((ref) => {
    const card = cardAt(state, ref);
    return card === null ? [] : [card];
  });
  return [
    ...activeUnitsOf(state, player).map((card) => ({ card, zone: "field" as const })),
    ...backrow.map((card) => ({ card, zone: "field" as const })),
    ...side.hand.map((card) => ({ card, zone: "hand" as const })),
    ...side.library.map((card) => ({ card, zone: "library" as const })),
  ];
}

/**
 * B3.3 rule 2: a count started on player-turn `since` has had its full turn cycle once the turn is
 * `since + BRITTLE_FIRST_TICK_TURNS` or later — the rest of the turn it started on and a whole turn of
 * the other player's (B9 #41's t + 2). After that it ticks at each start of its controller's turn.
 */
export function brittleDue(state: GameState, card: Pick<CardInstance, "brittle">): boolean {
  const brittle = card.brittle;
  return brittle !== undefined && state.turn >= brittle.since + BRITTLE_FIRST_TICK_TURNS;
}

/** R440: whether both players read this card where it is — a unit, or a face-up backrow card. */
function isPublicOnField(state: GameState, card: CardInstance, zone: CrumbleZone): boolean {
  return zone === "field" && !isFaceDown(state, card);
}

/**
 * B3.3 rule 3: a count at 0 crumbles its card. On the field that is an ordinary destroy (§6.3), so
 * the next state check collects it, Indestructible ignores it (R46) and the count stays at 0, checked
 * again at every tick. In a hand or a deck the card goes to its owner's graveyard — no discard, so
 * "whenever you discard" does not see it — and a unit-token card ceases to exist instead (R11). R215
 * resets it there, which spends the count (R441).
 */
function crumble(sink: EngineSink, card: CardInstance, zone: CrumbleZone): void {
  const owner = card.owner;
  sink.events.push({ type: "crumbled", instanceId: card.id, defId: card.defId, owner, zone });
  if (zone === "field") {
    const ctx = makeContext(sink, null, { controller: card.controller });
    destroy({ target: { of: "instance", instanceId: card.id } }).apply(ctx);
    return;
  }
  if (moveToZone(sink.state, card, "graveyard") === "moved") {
    sink.events.push({ type: "enteredGraveyard", instanceId: card.id, defId: card.defId, owner });
  }
}

/**
 * B3.3 rule 2: at the start of `player`'s turn, every Brittle count of theirs that has had a full
 * turn cycle drops by 1, and a count that reaches 0 crumbles its card (rule 3). The cards are read
 * once, before any count moves, so a card the tick moves (a crumble into a graveyard) is not met
 * twice. Opens no prompt; the stage that calls it settles what it caused (`turn.ts`).
 */
export function brittleTick(sink: EngineSink, player: PlayerId): void {
  const state = sink.state;
  for (const { card, zone } of tickedCards(state, player)) {
    const count = activeBrittleCount(card);
    if (count === null || card.brittle === undefined || !brittleDue(state, card)) continue;
    if (count > 0) {
      const next = Math.max(0, count - BRITTLE_TICK);
      card.brittle = { ...card.brittle, count: next };
      if (isPublicOnField(state, card, zone)) {
        sink.events.push({ type: "counterChanged", instanceId: card.id, counter: "brittle", value: next });
      }
      if (next > 0) continue;
    }
    crumble(sink, card, zone);
  }
}
