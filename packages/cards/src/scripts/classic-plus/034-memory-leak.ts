// C+ #34 Memory Leak (SPEC §8.7 row 34): (3) Field Spell, Epic.
//   Base:    Choose one: "At the end of your turn, Lock a random zone on your opponent's side"; or
//            "After your opponent plays a Unit or Field Spell, Lock its zone."
//   Radiant: both, no choice.
// The mode is declared with the play (R81) and remembered on the instance (`memory.mode`). "A random
// zone" is one of the opponent's ten not Locked yet (`lockRandomZone`, nothing when all are); "its
// zone" is where the played card landed, Locked once it resolves (a cast counts, R70).

import type { GameEvent } from "@jackioh/shared";
import type { EffectContext, Script } from "@jackioh/engine";
import { defOf, recalled } from "@jackioh/engine";
import { chosenOptions, lockPlayedZone, lockRandomZone, remember } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-034");

const MODE = "mode";
const END_OF_TURN = "endOfTurn";
const AFTER_PLAY = "afterPlay";

function opponentPlayedUnitOrFieldSpell(ctx: EffectContext & { event: GameEvent }): boolean {
  const event = ctx.event;
  if (event.type !== "cardResolved" || event.player === ctx.controller) return false;
  const type = defOf(ctx.state, event.defId).type;
  return type === "Unit" || type === "Field Spell";
}

function memoryLeak(has: (ctx: EffectContext, mode: string) => boolean): Script {
  return {
    endOfTurn: (ctx) => (has(ctx, END_OF_TURN) ? [lockRandomZone({ side: "enemy" })] : []),
    triggers: [
      {
        id: "memory-leak",
        on: ["cardResolved"],
        // A trigger that is no trap reads its condition in `run` (only traps consult `when`, R99).
        run: (ctx) =>
          has(ctx, AFTER_PLAY) && opponentPlayedUnitOrFieldSpell(ctx) ? [lockPlayedZone({ event: ctx.event })] : [],
      },
    ],
  };
}

export const base: Script = {
  ...memoryLeak((ctx, mode) => recalled(ctx, MODE) === mode),
  modes: [{ kind: "mode", options: [END_OF_TURN, AFTER_PLAY] }],
  cry: (ctx) => {
    const mode = chosenOptions(ctx)[0];
    return mode === END_OF_TURN || mode === AFTER_PLAY ? [remember({ key: MODE, value: mode })] : [];
  },
};

export const radiant: Script = memoryLeak(() => true);
