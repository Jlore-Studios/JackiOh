// C #62 Living Bomb (SPEC §8.6 row 62). (1) Field Spell, Rare.
//   Base:    "At the start of each player's turn: Destroy every permanent that player controls with a
//            Plague Counter on it."
//   Radiant: "At the start of your opponent's turn: Destroy every permanent they control with a Plague
//            Counter on it."
//   Engine:  "A start-of-turn trigger on both players' turns (Radiant: the opponent's only), in R68's
//            order; the designer's "Plague Counter" is the Plague Counter. "They destroy all cards" is the
//            turn player's own permanents, face-down ones included (R400): the only reading under which
//            the Radiant face is the stronger one. Indestructible permanents stay (§6.1). Tunes: none."
//
// R400: at the start of a player's turn, every permanent THAT player controls with at least one Plague
// Counter on it is destroyed — the top of each unit pile and every backrow card, face-down ones included,
// Living Bomb itself when it is the turn player's and carries a token — and the other player's are left
// alone. Each is an ordinary §6.3 destroy, all in one effect, so they die together at the one state
// check after it (§4.5, R59): an Indestructible one is knocked into Attack Position and stays (R46), a
// Reborn unit comes back, Death hooks run. A card's tokens are counters, which R78 clears when it leaves
// the field, so a card bounced and played again carries none. The set is read as the trigger resolves.
//
// Each turn's half is a start-of-turn hook queued at R62's start-of-turn trigger point in R68's order:
// its controller's own turn is its `startOfTurn` (the base face only), the opponent's its
// `startOfOpponentTurn`, which the engine queues right after the active player's own hooks — so a
// plagued Fed Fauci of the turn player's still gains its mana before it is destroyed.
//
// No declared numbers: the text has none.

import { permanentsOnField, plagueOn, type EffectContext, type Effect, type Hook, type Script } from "@jackioh/engine";
import { destroy } from "@jackioh/engine/effects";
import { opponentOf, type PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-062");

/** R400: one destroy for each permanent `player` controls with a Plague Counter on it, in R68's order. */
function destroyPlagued(ctx: EffectContext, player: PlayerId): Effect[] {
  return permanentsOnField(ctx.state, player)
    .filter((card) => card.controller === player && plagueOn(card) > 0)
    .map((card) => destroy({ target: { of: "instance", instanceId: card.id } }));
}

const atYourTurn: Hook = (ctx) => destroyPlagued(ctx, ctx.controller);
const atOpponentsTurn: Hook = (ctx) => destroyPlagued(ctx, opponentOf(ctx.controller));

export const base: Script = { startOfTurn: atYourTurn, startOfOpponentTurn: atOpponentsTurn };

export const radiant: Script = { startOfOpponentTurn: atOpponentsTurn };
