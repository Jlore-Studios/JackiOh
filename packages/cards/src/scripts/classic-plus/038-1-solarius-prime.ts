// C+ #38.1 Solarius-Prime (SPEC §8.7 row 38.1): (4) Unit, Token (printed Epic), 9/5 → 18/10.
//   Base:    "Spell Damage +3. Cry: Cast {casts} random Spells. Each targets enemies when it can."
//   Radiant: "Spell Damage +7. Cry: Cast {casts} random Radiant Spells. Each targets enemies when it can."
// E12's random casts (R452): non-token Spells of every set (R380), repeats allowed (R60), every choice
// random, each target pick narrowed to enemies when one is legal. Its own Spell Damage (the catalog
// keyword) raises their hits, since it is on the field during its Cry.

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { castRandom } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-038-1");

function solariusPrime(radiant: boolean): Script {
  return {
    cry: (ctx) => [castRandom({ query: { type: "Spell" }, count: param(ctx, "casts"), radiant, targetEnemies: true })],
  };
}

export const base: Script = solariusPrime(false);

export const radiant: Script = solariusPrime(true);
