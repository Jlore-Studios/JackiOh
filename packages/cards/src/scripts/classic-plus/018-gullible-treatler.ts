// C+ #18 Gullible Treatler (SPEC §8.7 row 18): at its controller's start of turn it Tributes itself
// when its controller (Radiant: any player) controls no Field Spell, Trap or Field Trap in the backrow.

import { cardAt, cardTypeOf, slotsOf, type ConditionContext, type GameState, type Script } from "@jackioh/engine";
import { sacrifice } from "@jackioh/engine/effects";
import { PLAYER_IDS, type CardType, type PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-018");

/** "Field Spell or Trap": a Field Trap is a Trap too (§8 Conventions). */
const COUNTED: readonly CardType[] = ["Field Spell", "Trap", "Field Trap"];

function controlsFieldSpellOrTrap(state: GameState, player: PlayerId): boolean {
  return slotsOf(player, "backrow").some((ref) => {
    const card = cardAt(state, ref);
    return card !== null && COUNTED.includes(cardTypeOf(state, card));
  });
}

/** Base: its controller controls none. Radiant: no player controls one. */
function tributesItself(state: GameState, controller: PlayerId, radiant: boolean): boolean {
  const players = radiant ? PLAYER_IDS : [controller];
  return !players.some((player) => controlsFieldSpellOrTrap(state, player));
}

function treatler(radiant: boolean): Script {
  return {
    startOfTurn: (ctx) =>
      ctx.self !== null && tributesItself(ctx.state, ctx.controller, radiant) ? [sacrifice({ target: { of: "self" } })] : [],
    conditionMet: (ctx: ConditionContext) => ctx.zone === "field" && tributesItself(ctx.state, ctx.controller, radiant),
  };
}

export const base: Script = treatler(false);

export const radiant: Script = treatler(true);
