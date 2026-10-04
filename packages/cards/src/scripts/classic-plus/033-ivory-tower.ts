// C+ #33 Ivory Tower (SPEC §8.7 row 33, R418, R651): (2) Field Spell, Rare.
//   Base:    "The first Unit you stack onto this is fused into it."
//   Radiant: "The first Unit you stack onto this becomes Radiant and is fused into it."
// The engine owns the stacking (`fusesCarried`, R446, R651): a Unit you play may name this zone while
// no Unit has stood on it this stay, and stands on it while its play resolves, its Cry included. Once
// that play has resolved, the Unit on it is fused into this per R77, this the kept card: a Field Spell
// still, with the Unit's text and keywords, and the Unit ceases to exist. The Radiant face makes the
// Unit Radiant as it lands, so its Cry runs on that face, and fuses it in on its Radiant face (R469).

import type { GameEvent } from "@jackioh/shared";
import type { CardInstance, EffectContext, GameState, Script, TriggerDef } from "@jackioh/engine";
import { carriedAt, slotOf, stackedOnto } from "@jackioh/engine";
import { fuseCards, setRadiant } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-033");

/** The Unit standing on this Tower now, if any. */
function riderOf(state: GameState, tower: CardInstance): CardInstance | null {
  const at = slotOf(state, tower);
  return at === null ? null : carriedAt(state, at);
}

type TriggerContext = EffectContext & { event: GameEvent };

/** R651: the event is the play of the Unit stacked onto this Tower, landing or resolved. */
function isStackedPlay(ctx: TriggerContext): ctx is TriggerContext & { self: CardInstance } {
  const event = ctx.event;
  if (ctx.self === null) return false;
  return (event.type === "cardPlayed" || event.type === "cardResolved") && stackedOnto(ctx.self) === event.instanceId;
}

/**
 * R651: once the stacked Unit's play has resolved, the Unit standing on this — that card, or what an
 * answer to the play left in its place — is fused into it.
 */
function fuseIn(radiant: boolean): TriggerDef {
  return {
    id: "ivory-tower-fuse",
    on: ["cardResolved"],
    // Not a trap, so the condition is read in `run` (only traps consult `when`, R99).
    run: (ctx) => {
      if (!isStackedPlay(ctx)) return [];
      const rider = riderOf(ctx.state, ctx.self);
      if (rider === null) return [];
      return [fuseCards({ instanceIds: [rider.id], targetInstanceId: ctx.self.id, ...(radiant ? { radiantIngredients: true } : {}) })];
    },
  };
}

export const base: Script = {
  staticFlags: { fusesCarried: true },
  triggers: [fuseIn(false)],
};

export const radiant: Script = {
  ...base,
  triggers: [
    {
      id: "ivory-tower-radiant",
      on: ["cardPlayed"],
      run: (ctx) => (isStackedPlay(ctx) && ctx.event.type === "cardPlayed" ? [setRadiant({ instanceId: ctx.event.instanceId })] : []),
    },
    fuseIn(true),
  ],
};
