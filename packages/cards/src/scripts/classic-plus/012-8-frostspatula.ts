// C+ #12.8 Frostspatula (SPEC §8.7 row 12.8, R409): (2) Field Spell, Pancake, Token (printed Legendary),
// 10/3 → 20/6.
//   Base:    "Animated on your turn, Rush. Death: Summon a copy of every Unit this destroyed."
//   Radiant: "… and make them Radiant."
// Animated on your turn and Rush are catalog keywords the engine runs (B3.1, R383). It remembers each
// Unit it killed (R42's killer) by definition and face, in memory, which its animations keep (R383);
// its Death — as a Unit or destroyed in the backrow — summons a fresh copy of each for its controller,
// tokens included, until the board is full, with no Cry (R1). The originals stay put (R409).

import type { GameEvent } from "@jackioh/shared";
import type { EffectContext, Script } from "@jackioh/engine";
import { recalled } from "@jackioh/engine";
import { remember, summon } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012-8");

const KILLS = "kills";

type Kill = { id: string; defId: string; radiant: boolean };

function killsOf(ctx: Pick<EffectContext, "self" | "data">): Kill[] {
  const kills = recalled(ctx, KILLS);
  return Array.isArray(kills) ? (kills as Kill[]) : [];
}

/** A `destroyed` event this card is the killer of, as a remembered kill. */
function killIn(ctx: EffectContext, event: GameEvent): Kill | null {
  if (event.type !== "destroyed" || ctx.self === null || event.killerId !== ctx.self.id) return null;
  return { id: event.instanceId, defId: event.defId, radiant: event.radiant === true };
}

function frostspatula(radiant: boolean): Script {
  return {
    triggers: [
      {
        id: "frostspatula-kill",
        on: ["destroyed"],
        // Not a trap, so the condition is read in `run` (only traps consult `when`, R99).
        run: (ctx) => {
          const kill = killIn(ctx, ctx.event);
          return kill === null ? [] : [remember({ key: KILLS, value: [...killsOf(ctx), kill] })];
        },
      },
    ],
    death: (ctx) => {
      // A kill in the same pass as its own death never reached the trigger: read it off this action's events.
      const known = killsOf(ctx);
      const late = ctx.events.flatMap((event) => {
        const kill = killIn(ctx, event);
        return kill === null || known.some((seen) => seen.id === kill.id) ? [] : [kill];
      });
      return [...known, ...late].map((kill) => summon({ defId: kill.defId, radiant: radiant || kill.radiant }));
    },
  };
}

export const base: Script = frostspatula(false);

export const radiant: Script = frostspatula(true);
