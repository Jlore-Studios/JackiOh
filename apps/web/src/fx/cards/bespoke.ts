// R502, R670, R2784000: every recipe a `{ bespoke }` entry of a set's effects table may name: the card
// recipes that claim events of their play (cardFx.ts) and the marquee entrances (entrances.ts). R2797000:
// Stinkyscape's three (fx/cards/stinkyscape/).

import { ENTRANCE_TRAUMA, type CardRecipe, type EntranceKey } from "./recipe.ts";
import { bloodDrain } from "./core/bloodRidden.ts";
import { fusionForge } from "./core/craftACard.ts";
import { manaCrack } from "./core/hinder.ts";
import { legendaryTide } from "./core/transmogulate.ts";
import { perfectWish } from "./core/zephrys.ts";
import { crushingWalls } from "./classicplus/crushingWalls.ts";
import { jadeCrack } from "./meditative/auspiciousRock.ts";
import { cnSweep } from "./meditative/spirituallyZhongguo.ts";
import { harmonyWave } from "./meditativeplus/harmonySpirit.ts";
import { battleshipWarp } from "./stinkyscape/battleshipWarp.ts";
import { magiBoot } from "./stinkyscape/magiBoot.ts";
import { terminusRift } from "./stinkyscape/terminusRift.ts";

export const CARD_RECIPES = {
  manaCrack,
  bloodDrain,
  crushingWalls,
  legendaryTide,
  perfectWish,
  fusionForge,
  jadeCrack,
  cnSweep,
  harmonyWave,
  terminusRift,
  magiBoot,
  battleshipWarp,
} as const satisfies Readonly<Record<string, CardRecipe>>;

export type CardRecipeKey = keyof typeof CARD_RECIPES;

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
