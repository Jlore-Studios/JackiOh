// C #62 Living Bomb (SPEC §8.6 row 62). (1) Field Spell, Rare.
//   Base:    "At the start of each player's turn: Destroy every permanent that player controls with a
//            Plague Token on it."
//   Radiant: "At the start of your opponent's turn: Destroy every permanent they control with a Plague
//            Token on it."
//   Engine:  "A start-of-turn trigger on both players' turns (Radiant: the opponent's only), in R68's
//            order; the designer's "Plague Counter" is the Plague Token. "They destroy all cards" is the
//            turn player's own permanents, face-down ones included (R400): the only reading under which
//            the Radiant face is the stronger one. Indestructible permanents stay (§6.1). Tunes: none."
//
// R400: at the start of a player's turn, every permanent THAT player controls with at least one Plague
// Token on it is destroyed — the top of each unit pile and every backrow card, face-down ones included,
// Living Bomb itself when it is the turn player's and carries a token — and the other player's are left
// alone. Each is an ordinary §6.3 destroy, all in one effect, so they die together at the one state
// check after it (§4.5, R59): an Indestructible one is knocked into Attack Position and stays (R46), a
// Reborn unit comes back, Death hooks run. A card's tokens are counters, which R78 clears when it leaves
// the field, so a card bounced and played again carries none. The set is read as the trigger resolves.
//
// Where each turn's half sits:
//   - its controller's own turn (the base face only): the card's `startOfTurn` hook, which the engine
//     queues with the active player's start-of-turn triggers at R62's point, in R68's order;
//   - the opponent's turn (both faces): `startOfTurn` hooks are queued for the active player's cards
//     alone, so this half answers the opponent's `turnStarted` instead — an ordinary trigger, queued in
//     R68's order with whatever else answers that event. The engine dispatches `turnStarted` at the
//     first settle of the turn (after the Brittle tick), ahead of R62's start-of-turn trigger point.
//
// No declared numbers: the text has none.

import {
  permanentsOnField,
  plagueOn,
  type EffectContext,
  type Effect,
  type Hook,
  type Script,
  type TriggerDef,
} from "@jackioh/engine";
import { destroy } from "@jackioh/engine/effects";
import { opponentOf, type GameEvent, type PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-062");

/** R400: one destroy for each permanent `player` controls with a Plague Token on it, in R68's order. */
function destroyPlagued(ctx: EffectContext, player: PlayerId): Effect[] {
  return permanentsOnField(ctx.state, player)
    .filter((card) => card.controller === player && plagueOn(card) > 0)
    .map((card) => destroy({ target: { of: "instance", instanceId: card.id } }));
}

/** The opponent's turn beginning: `turnStarted` for the player who is not this card's controller. */
function opponentsTurnStarted(ctx: EffectContext & { event: GameEvent }): PlayerId | null {
  const event = ctx.event;
  if (event.type !== "turnStarted" || event.player !== opponentOf(ctx.controller)) return null;
  return event.player;
}

const atOpponentsTurn: TriggerDef = {
  id: "living-bomb-opponent",
  on: ["turnStarted"],
  when: (ctx) => opponentsTurnStarted(ctx) !== null,
  run: (ctx) => {
    const player = opponentsTurnStarted(ctx);
    return player === null ? [] : destroyPlagued(ctx, player);
  },
};

/** Its controller's own turn: the start-of-turn hook runs on that turn only (§2.2, R62). */
const atYourTurn: Hook = (ctx) => destroyPlagued(ctx, ctx.controller);

export const base: Script = { startOfTurn: atYourTurn, triggers: [atOpponentsTurn] };

export const radiant: Script = { triggers: [atOpponentsTurn] };
