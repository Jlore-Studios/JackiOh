// #59 Unbiased Immigration (SPEC §8.3): Field Spell, cost 2 embiggen 4, Rare. "Start of turn: add a
// random card to your hand (paid 4: it costs 0)" / radiant "A random Radiant card (paid 4: it costs
// 0)". Engine cell: "Non-token pool excluding #59" — every set's since patch v0.2.0 (R380).
//
// §8 Conventions: the radiant cell restates the whole clause, so it replaces it — one random card
// per start of turn either way, Radiant on the radiant face. The parenthesis is restated too and
// means the same thing on both faces.
//
// THE EMBIGGEN PRICE IS NOT A PROMPT (R81, §10.6): "Zone, X, embiggen, Tribute … travel in the
// `play` action", and "No Core card opens an `x`, `embiggen`, `zone`, `tribute` or `direction`
// prompt, since all five are play choices". So this card declares NOTHING — no `targets`, no
// `modes`. `reduce`'s `playCard` writes the answer onto the instance (`card.embiggened`) and
// `makeContext` hands it to every hook of that instance as `ctx.embiggened`, which is why a trigger
// that fires turns later still knows what was paid (R65: the chosen embiggen price is the cost).
//
// R65 is also why the discount is a `costOverride` and not a `costMod`: "start from `costOverride`,
// else the printed cost … floor at 0" — an override of 0 makes the added card free whatever it was
// printed at, and it persists in every zone (R78).
//
// R60: the pool may repeat across turns; nothing here says "different". §5.1: a random pool never
// offers a Token-tagged card nor the generating card, which `excludeDefId` spells out (R387). R380:
// "a random card" names no set, so it draws from every set. R4: an eleventh card is burned to the
// graveyard by the add-to-hand pipeline.
//
// A hook may not roll dice — `ctx.rng.*` advances `rngCursor`, which is state — so the pick happens
// inside `addRandomFromCatalog` (engine/src/effects/addToHand.ts), the verb #54 Straaza and #57
// Conjure KY use too.

import type { CatalogQueryArgs, Effect, EffectContext, Hook, Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-059");

/** §5.1's "a random card": the non-token catalog of every set (R380), minus this card (R387). */
const CARD_POOL: CatalogQueryArgs = { excludeDefId: def.id };

/**
 * One add. `ctx.embiggened` is the price this Field Spell was played for (R65), read at the moment
 * the trigger resolves rather than remembered by the script, so the two faces differ only in the
 * radiant flag they put on the created card (§5.2, R74).
 */
function addRandomCard(ctx: EffectContext, asRadiant: boolean): Effect {
  return addRandomFromCatalog({
    query: CARD_POOL,
    count: 1,
    ...(asRadiant ? { radiant: true } : {}),
    // "(paid 4: it costs 0)" — the embiggen price, not the mana actually spent after discounts.
    ...(ctx.embiggened ? { costOverride: 0 } : {}),
  });
}

const startOfTurnBase: Hook = (ctx) => [addRandomCard(ctx, false)];

const startOfTurnRadiant: Hook = (ctx) => [addRandomCard(ctx, true)];

export const base: Script = { startOfTurn: startOfTurnBase };

export const radiant: Script = { startOfTurn: startOfTurnRadiant };
