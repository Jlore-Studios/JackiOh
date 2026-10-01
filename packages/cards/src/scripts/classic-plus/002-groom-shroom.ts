// C+ #2 Groom Shroom (SPEC §8.7 row 2, R405). (3) Trap, Felinor, Epic.
// Fires in C+ #1's window (an enemy Unit's declared attack on your hero) and fills every open unit zone
// of yours with a random non-token Felinor Unit of any set (R64, R380, R60), each summoned with no Cry
// (R1) and granted Taunt; Radiant, on their Radiant faces. The attack still hits your hero (R405).

import type { Script, TrapTrigger } from "@jackioh/engine";
import { attackTargetOf, cardAt, fillBoardZones, findInstance } from "@jackioh/engine";
import { forEachCard, grantKeyword, summonRandom } from "@jackioh/engine/effects";
import { opponentOf, type Tag } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-002");

/** R405: "Felinors" are Felinor-tagged Units; a pool names no set, so every set (R380). */
const FELINOR_UNITS = { type: "Unit" as const, tags: ["Felinor" as Tag] };

/** A declared attack (a forced one opens no window, R121) by an enemy Unit on this trap's controller's hero. */
const attacksYourHero: NonNullable<TrapTrigger["when"]> = ({ event, state, controller }) => {
  if (event.type !== "attackDeclared" || event.forced) return false;
  const target = attackTargetOf(state, event.targetId);
  return findInstance(state, event.attackerId)?.controller === opponentOf(controller) && target?.kind === "hero" && target.player === controller;
};

function groomShroom(radiant: boolean): Script {
  return {
    triggers: [
      {
        id: "groom-shroom",
        on: ["attackDeclared"],
        when: attacksYourHero,
        run: (ctx) => {
          const lanes = fillBoardZones(ctx.state, ctx.controller).map((zone) => zone.lane);
          return [
            ...lanes.map((lane) => summonRandom({ query: FELINOR_UNITS, lane, radiant })),
            // "Give them Taunt": the Units now standing in the zones the fill took.
            forEachCard({
              cards: (now) => lanes.flatMap((lane) => cardAt(now.state, { player: now.controller, row: "units", lane }) ?? []),
              each: (instanceId) => grantKeyword({ target: { of: "instance", instanceId }, keyword: { kind: "Taunt" } }),
            }),
          ];
        },
      },
    ],
  };
}

export const base: Script = groomShroom(false);

export const radiant: Script = groomShroom(true);
