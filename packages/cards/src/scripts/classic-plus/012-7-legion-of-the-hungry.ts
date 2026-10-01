// C+ #12.7 Legion of the Hungry (SPEC §8.7 row 12.7, R408): (2) Field Spell, Pancake, Token (printed Legendary).
//   Base:    "Cry: Exile {cards} random cards from your deck. Summon the Units among them."
//   Radiant: "… Summon the Units among them and make them Radiant."
// R408's Cry reading. Five different cards (R60), all of them if fewer; the Units among them are
// summoned out of exile in the order they were exiled (no Cry, R1), per R64, until the board is full,
// the rest staying exiled; a unit-token card exiled from a deck has ceased to exist (R11).

import type { CardInstance, EffectContext, Effect, Script } from "@jackioh/engine";
import { cardTypeOf, findInstance, param, zoneCards } from "@jackioh/engine";
import { exile, forEachCard, setRadiant, summon } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012-7");

function legion(radiant: boolean): Script {
  return {
    cry: (ctx: EffectContext): Effect[] => {
      // ponytail: the picks are drawn as the Cry resolves; nothing in this list can pause between them.
      const picks: CardInstance[] = ctx.rng.shuffle(zoneCards(ctx.state, ctx.controller, "library")).slice(0, param(ctx, "cards"));
      const units = picks.filter((card) => cardTypeOf(ctx.state, card) === "Unit").map((card) => card.id);
      const onField = (at: EffectContext): string[] => units.filter((id) => findInstance(at.state, id)?.zone.z === "field");
      return [
        forEachCard({ cards: () => picks, each: (instanceId) => exile({ target: { of: "instance", instanceId } }) }),
        forEachCard({ cards: () => units, each: (instanceId) => summon({ instance: { of: "instance", instanceId } }) }),
        ...(radiant ? [forEachCard({ cards: onField, each: (instanceId) => setRadiant({ instanceId }) })] : []),
      ];
    },
  };
}

export const base: Script = legion(false);

export const radiant: Script = legion(true);
