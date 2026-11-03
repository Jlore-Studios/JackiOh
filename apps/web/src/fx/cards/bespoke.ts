// R502, R670, R2784000: every recipe a `{ bespoke }` entry of a set's effects table may name: the card
// recipes that claim events of their play (cardFx.ts) and the marquee entrances (entrances.ts). R2797000:
// Stinkyscape's three (fx/cards/stinkyscape/).

import type { DrawnMotif } from "../motifs/names.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type EntranceKey } from "./recipe.ts";
import { bloodDrain } from "./core/bloodRidden.ts";
import { manaCrack } from "./core/hinder.ts";
import { crushingWalls } from "./classicplus/crushingWalls.ts";
import { battleshipWarp } from "./stinkyscape/battleshipWarp.ts";
import { magiBoot } from "./stinkyscape/magiBoot.ts";
import { terminusRift } from "./stinkyscape/terminusRift.ts";

export const CARD_RECIPES = { manaCrack, bloodDrain, crushingWalls, terminusRift, magiBoot, battleshipWarp } as const satisfies Readonly<Record<string, CardRecipe>>;

export type CardRecipeKey = keyof typeof CARD_RECIPES;

/**
 * R2797000: the motif fixture (motifs/gallery.ts) a card recipe plays over in the /dev/fx gallery: the
 * events its card's play makes. A recipe with none is left out of the gallery's cards.
 */
export const CARD_RECIPE_FIXTURES: Readonly<Partial<Record<CardRecipeKey, DrawnMotif>>> = {
  terminusRift: "forge",
  magiBoot: "conjure",
  battleshipWarp: "conjure",
};

export type BespokeKey = CardRecipeKey | EntranceKey;

export function isCardRecipeKey(key: string): key is CardRecipeKey {
  return Object.prototype.hasOwnProperty.call(CARD_RECIPES, key);
}

export function isEntranceKey(key: string): key is EntranceKey {
  return Object.prototype.hasOwnProperty.call(ENTRANCE_TRAUMA, key);
}

export function isBespokeKey(key: string): key is BespokeKey {
  return isCardRecipeKey(key) || isEntranceKey(key);
}
