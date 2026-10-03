// C+ #12.6 Frozen Wastes (SPEC §8.7 row 12.6, R408): (2) Spell, Pancake, Token (printed Legendary;
// balance patch 1: a Spell, not a Field Spell).
//   Base:    "Cry: Destroy all Units. Exile the top card of your deck for each one destroyed."
//   Radiant: "… Exile the top card of your opponent's deck for each one destroyed."
// A Spell's unlabelled one-time text is its Cry; the card then goes to the graveyard (R408).
// "Each one" is the Units the destroy dooms, read as it resolves (the exile follows in the same list,
// before the state check, R59): an Indestructible one isn't, a Reborn one is. A short deck exiles
// what it has, with no fatigue. The preview (R280) is how many cards it would exile now.

import { hasKeyword, opponentOf, type PlayerId } from "@jackioh/shared";
import type { GameState, Script } from "@jackioh/engine";
import { activeUnitsOf, unitView, zoneCards, zoneCount } from "@jackioh/engine";
import { destroyAll, exile, forEachCard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012-6");

/** The Units a destroy of all Units dooms now: every acting Unit but the Indestructible ones (R46). */
function doomed(state: GameState): number {
  return (["p1", "p2"] as const)
    .flatMap((player) => activeUnitsOf(state, player))
    .filter((unit) => !hasKeyword(unitView(state, unit).keywords, "Indestructible")).length;
}

function frozenWastes(deckOf: (controller: PlayerId) => PlayerId): Script {
  return {
    cry: (ctx) => {
      const count = doomed(ctx.state);
      const deck = deckOf(ctx.controller);
      return [
        destroyAll({ side: "any" }),
        forEachCard({
          cards: (at) => zoneCards(at.state, deck, "library").slice(0, count),
          each: (instanceId) => exile({ target: { of: "instance", instanceId } }),
        }),
      ];
    },
    preview: ({ state, controller }) => [
      { label: "for each one destroyed", value: Math.min(doomed(state), zoneCount(state, deckOf(controller), "library")) },
    ],
  };
}

export const base: Script = frozenWastes((controller) => controller);

export const radiant: Script = frozenWastes(opponentOf);
