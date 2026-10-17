// R374 deals real, non-token Core cards into four rarities in random order, with the middle Radiant.
// Randomness is client-owned (CLAUDE.md rule 4) and injected so tests can deal deterministically.
// R1017002 deals only the cards that print whole on a fan card, a hero's full-art card favoured.

import { SHIPPED_SETS, type CardDef, type CardDefs, type Rarity } from "@jackioh/shared";
import catalogJson from "@jackioh/cards/catalog.json";

import { nameTier, textTier, type LengthTier } from "../cards/fit.ts";
import { isFullArt, showsFullArt } from "../cards/fullArt.ts";
import { faceModel } from "../cards/model.ts";
import { overLimit, shortFormOf, type TextLimit } from "../cards/shortText.ts";
import {
  FEATURE_PLAIN_MAX_TIER,
  FEATURE_WEIGHT_DENSE,
  FEATURE_WEIGHT_PLAIN,
  FULL_ART_WEIGHT_FACTOR,
} from "../stats/config.ts";

export type FanFace = { readonly def: CardDef; readonly radiant: boolean };

export type RandomSource = () => number;

export const FAN_FACES = 4;

export const FAN_RADIANT_AT = 2;

/** landing.css's widest fan card (`--fan-card-w` at 900 px and up), mirrored for the component test. */
export const FAN_CARD_MAX_PX = 158;

/**
 * R1017002: the most text a fan card prints whole, measured in Chromium on every catalog face. At
 * FAN_CARD_MAX_PX the rules box prints at the reading floor and holds 5 lines of about 25
 * characters before the art would give way (issue #1017's long layout); a narrower fan card only
 * shrinks its text. `faceLines` wraps by words to count, as for FACE_TEXT_LIMIT.
 */
export const FAN_TEXT_LIMIT: TextLimit = { charsPerLine: 25, lines: 5 };

/** JSON widens the unions to strings; the catalog is proved against SPEC §8 in crates/cards. */
const CATALOG = catalogJson as unknown as CardDefs;

/** Core cards a deck may hold, tokens aside (§2.6). */
export const FAN_POOL: readonly CardDef[] = Object.values(CATALOG).filter((def) => !def.token && def.set === "Core");

/** R639: non-token cards from shipped sets, in catalog order, for experienced-player rotation. */
export const ROTATION_POOL: readonly CardDef[] = Object.values(CATALOG).filter(
  (def) => !def.token && (SHIPPED_SETS as readonly string[]).includes(def.set),
);

/**
 * R1017002: whether both of `def`'s faces print whole on a fan card without the long layout: a
 * hero's full-art face prints no words, and any other face its whole catalog text, with no short
 * form (issue #1017), within FAN_TEXT_LIMIT. With the Full-art cards setting off, a hero's card is
 * held to its text like any card.
 */
export function fitsTheFan(def: CardDef, fullArtOn: boolean): boolean {
  return [false, true].every((radiant) => {
    const face = faceModel({ defId: def.id, def, radiant });
    if (showsFullArt(face, fullArtOn)) return true;
    return shortFormOf(def.id, radiant) === null && !overLimit(face.text.full, FAN_TEXT_LIMIT);
  });
}

const FITTING = new Map<string, readonly CardDef[]>();

function fitting(name: string, pool: readonly CardDef[], fullArtOn: boolean): readonly CardDef[] {
  const key = `${name}:${String(fullArtOn)}`;
  const known = FITTING.get(key);
  if (known !== undefined) return known;
  const cards = pool.filter((def) => fitsTheFan(def, fullArtOn));
  FITTING.set(key, cards);
  return cards;
}

/** R374's Core pool, R1017002: the cards in it that fit the fan. */
export function fanPool(fullArtOn: boolean): readonly CardDef[] {
  return fitting("core", FAN_POOL, fullArtOn);
}

/** R639's shipped pool, R1017002: the cards in it that fit the fan. */
export function rotationPool(fullArtOn: boolean): readonly CardDef[] {
  return fitting("shipped", ROTATION_POOL, fullArtOn);
}

/** Clamp a faulty `random` result of 1 so it cannot overrun. */
function below(random: RandomSource, n: number): number {
  return Math.min(n - 1, Math.floor(random() * n));
}

function shuffled<T>(items: readonly T[], random: RandomSource): T[] {
  const out = [...items];
  for (let i = out.length - 1; i > 0; i -= 1) {
    const j = below(random, i + 1);
    const a = out[i] as T;
    out[i] = out[j] as T;
    out[j] = a;
  }
  return out;
}

export type CardWeight = (def: CardDef) => number;

/** R374 draws uniformly by rarity; R704 uses it below R639's threshold. */
export const EVEN: CardWeight = () => 1;

const TIER_RANK: Readonly<Record<LengthTier, number>> = { s: 0, m: 1, l: 2, xl: 3, xxl: 4 };

/**
 * R639 favours faces whose name and rules text print at the largest size, but never excludes dense
 * cards. A hero's card counts as one (R1017002): in the fan it prints its art and no words, and with
 * full art off it is dealt only when its text fits the fan.
 */
export function featureWeight(def: CardDef): number {
  const known = WEIGHTS.get(def.id);
  if (known !== undefined) return known;
  const face = faceModel({ defId: def.id, def, radiant: false });
  const fits =
    isFullArt(def.name) ||
    (TIER_RANK[nameTier(face.name)] <= TIER_RANK[FEATURE_PLAIN_MAX_TIER] &&
      TIER_RANK[textTier(face.text.full)] <= TIER_RANK[FEATURE_PLAIN_MAX_TIER]);
  const weight = fits ? FEATURE_WEIGHT_PLAIN : FEATURE_WEIGHT_DENSE;
  WEIGHTS.set(def.id, weight);
  return weight;
}

const WEIGHTS = new Map<string, number>();

/** R1017002: `weigh`, with a hero's full-art card FULL_ART_WEIGHT_FACTOR times as likely while full art is on. */
export function favouringFullArt(weigh: CardWeight, fullArtOn: boolean): CardWeight {
  return (def) => weigh(def) * (fullArtOn && isFullArt(def.name) ? FULL_ART_WEIGHT_FACTOR : 1);
}

/** A nonpositive weight counts as one so no card becomes unreachable. */
export function pickWeighted(cards: readonly CardDef[], random: RandomSource, weigh: CardWeight = EVEN): CardDef | undefined {
  if (cards.length === 0) return undefined;
  const weights = cards.map((def) => {
    const weight = weigh(def);
    return Number.isFinite(weight) && weight > 0 ? weight : 1;
  });
  const total = weights.reduce((sum, weight) => sum + weight, 0);
  let point = Math.min(random(), 1 - Number.EPSILON) * total;
  for (let i = 0; i < cards.length; i += 1) {
    point -= weights[i] ?? 0;
    if (point < 0) return cards[i];
  }
  return cards[cards.length - 1];
}

/** R639 swaps one slot for a weighted, unseen card of the same rarity; its Radiant face stays put. */
export function rotateFan(
  hand: readonly FanFace[],
  slot: number,
  random: RandomSource,
  pool: readonly CardDef[] = ROTATION_POOL,
  weigh: CardWeight = featureWeight,
): FanFace[] {
  if (hand.length === 0) return [...hand];
  const at = ((slot % hand.length) + hand.length) % hand.length;
  const current = hand[at];
  if (current === undefined) return [...hand];
  const shown = new Set(hand.map(({ def }) => def.id));
  const candidates = pool.filter((def) => def.rarity === current.def.rarity && !shown.has(def.id));
  const next = pickWeighted(candidates, random, weigh);
  if (next === undefined) return [...hand];
  return hand.map((face, index) => (index === at ? { def: next, radiant: face.radiant } : face));
}

/** R374 draws one card per rarity in random order; sparse pools deal fewer cards rather than repeat. */
export function dealLandingFan(
  random: RandomSource,
  pool: readonly CardDef[] = FAN_POOL,
  weigh: CardWeight = EVEN,
): FanFace[] {
  const byRarity = new Map<Rarity, CardDef[]>();
  for (const def of pool) byRarity.set(def.rarity, [...(byRarity.get(def.rarity) ?? []), def]);
  const rarities = shuffled([...byRarity.keys()], random).slice(0, FAN_FACES);
  const picks = rarities.map((rarity) => pickWeighted(byRarity.get(rarity) ?? [], random, weigh));
  return picks
    .filter((def): def is CardDef => def !== undefined)
    .map((def, index) => ({ def, radiant: index === FAN_RADIANT_AT }));
}
