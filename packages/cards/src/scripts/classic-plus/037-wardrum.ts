// C+ #37 Wardrum (SPEC §8.7 row 37): (5) Unit, Quickdraw, Legendary, 5/5 → 10/10.
//   Base:    "While this is in your hand or deck: After the Spells, Field Spells and Traps you play in a
//            turn reach {threshold}, summon this. End of turn: Cast a copy of a random Spell, Field Spell
//            or Trap you played this turn."
//   Radiant: "… End of turn: Cast a copy of each Spell, Field Spell and Trap you played this turn."
// A hand and deck trigger (B5 E26) on the resolution of the play that is the threshold-th non-Unit play
// of the turn (casts count, R70); `summonThis` puts it in the leftmost open unit zone with no Cry, or
// leaves it where it is. The end of turn casts fresh copies by definition and face (E12, R87), skipping
// played cards that no longer exist (R86).

import type { CardType, GameEvent, PlayerId } from "@jackioh/shared";
import type { CardInstance, ConditionContext, EffectContext, GameState, Script, TriggerDef } from "@jackioh/engine";
import { cardTypeOf, findInstance, firstFreeZone, param, playedIdsThisTurn, playedThisTurnOfType } from "@jackioh/engine";
import { castNew, forEachCard, summonThis } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-037");

const NON_UNIT: readonly CardType[] = ["Spell", "Field Spell", "Trap", "Field Trap"];

/** The type a card was played as: its running face's, wherever it stands now (B2.7). */
function nonUnit(state: GameState, card: CardInstance): boolean {
  return NON_UNIT.includes(cardTypeOf(state, { defId: card.defId, radiant: card.radiant }));
}

/** This turn's non-Unit plays of `player` that still exist, in play order (R86). */
function nonUnitPlays(state: GameState, player: PlayerId): CardInstance[] {
  return playedIdsThisTurn(state, player).flatMap((id) => {
    const card = findInstance(state, id);
    return card !== undefined && nonUnit(state, card) ? [card] : [];
  });
}

/**
 * R578: whether the play this event resolves is the controller's threshold-th non-Unit play of the turn.
 * Its place is the turn's count of such plays (which keeps a play whose card has since ceased to exist)
 * less those logged after its latest play — the casts its own resolution made — so a 4th cast inside the
 * 3rd's resolution answers neither as the 3rd, and a card played again this turn is placed by this play.
 */
function reachesThreshold(ctx: EffectContext & { event: GameEvent }): boolean {
  const event = ctx.event;
  if (event.type !== "cardResolved" || event.player !== ctx.controller) return false;
  if (!NON_UNIT.includes(cardTypeOf(ctx.state, { defId: event.defId, radiant: event.radiant }))) return false;
  const log = playedIdsThisTurn(ctx.state, ctx.controller);
  const at = log.lastIndexOf(event.instanceId);
  if (at < 0) return false;
  // ponytail: a cast inside this resolution whose card has ceased to exist since is not taken off.
  const later = log.slice(at + 1).filter((id) => {
    const card = findInstance(ctx.state, id);
    return card !== undefined && nonUnit(ctx.state, card);
  }).length;
  return playedThisTurnOfType(ctx.state, ctx.controller, NON_UNIT) - later === param(ctx, "threshold");
}

const summonAtThreshold: TriggerDef = {
  id: "wardrum-summon",
  on: ["cardResolved"],
  // Not a trap, so the condition is read in `run` (only traps consult `when`, R99).
  run: (ctx) => (reachesThreshold(ctx) ? [summonThis()] : []),
};

/** R195: in hand, on your turn, your next Spell, Field Spell or Trap would summon it into an open zone. */
function nextPlaySummons(ctx: ConditionContext): boolean {
  if (ctx.zone !== "hand" || !ctx.yourTurn) return false;
  const threshold = param({ state: ctx.state, self: ctx.self, radiant: ctx.radiant }, "threshold");
  return (
    playedThisTurnOfType(ctx.state, ctx.controller, NON_UNIT) === threshold - 1 &&
    firstFreeZone(ctx.state, ctx.controller, "units") !== null
  );
}

const copyOf = (id: string) => (ctx: EffectContext) => {
  const card = findInstance(ctx.state, id);
  return card === undefined ? null : { defId: card.defId, radiant: card.radiant };
};

function wardrum(endOfTurn: Script["endOfTurn"]): Script {
  return {
    staticFlags: { quickdraw: true },
    handTriggers: [summonAtThreshold],
    deckTriggers: [summonAtThreshold],
    conditionMet: nextPlaySummons,
    endOfTurn,
  };
}

export const base: Script = wardrum(() => [
  castNew({
    def: (ctx) => {
      const card = ctx.rng.pick(nonUnitPlays(ctx.state, ctx.controller));
      return card === undefined ? null : { defId: card.defId, radiant: card.radiant };
    },
  }),
]);

export const radiant: Script = wardrum(() => [
  forEachCard({ cards: (ctx) => nonUnitPlays(ctx.state, ctx.controller), each: (id) => castNew({ def: copyOf(id) }) }),
]);
