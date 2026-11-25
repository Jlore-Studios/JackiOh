// R374 deals real, non-token Core cards into four rarities in random order, with the middle Radiant.
// Randomness is client-owned (CLAUDE.md rule 4) and injected so tests can deal deterministically.
// R1017002 deals only the cards that print whole on a fan card, a full-art card (R2655001) favored.

import { SHIPPED_SETS, type CardDef, type Rarity } from "@jackioh/shared";

import { nameTier, overLimit, textTier, type LengthTier, type TextLimit } from "../cards/fit.ts";
import { currentCatalog } from "../game/currentCatalog.ts";
import { isFullArtCard } from "../cards/fullArt.ts";
import { faceModel } from "../cards/model.ts";
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
 * R1017002: the most text a fan card prints whole at FAN_CARD_MAX_PX and the reading floor before the art
 * gives way (issue #1017's long layout), measured in Chromium on every catalog face.
 */
export const FAN_TEXT_LIMIT: TextLimit = { charsPerLine: 25, lines: 5 };

/**
 * #4180: the pools over the engine module's catalog (`game/currentCatalog.ts`), memoised like the
 * `FITTING` map below: the module is in before the fan is dealt (`useEngineReady`), so reading them
 * at import would throw.
 */
let core: readonly CardDef[] | null = null;
let shipped: readonly CardDef[] | null = null;

/** Core cards a deck may hold, tokens aside (§2.6), in catalog order. */
export function corePool(): readonly CardDef[] {
  core ??= Object.values(currentCatalog().cards).filter((def) => !def.token && def.set === "Core");
  return core;
}

/** R639: non-token cards from shipped sets, in catalog order, for experienced-player rotation. */
export function shippedPool(): readonly CardDef[] {
  shipped ??= Object.values(currentCatalog().cards).filter(
    (def) => !def.token && (SHIPPED_SETS as readonly string[]).includes(def.set),
  );
  return shipped;
}

/**
 * R1017002: whether both of `def`'s faces print whole on a fan card: a full-art face prints no words,
 * any other its whole catalog text, with no short form, within FAN_TEXT_LIMIT.
 */
export function fitsTheFan(def: CardDef, fullArtOn: boolean): boolean {
  // R2725000: the setting prints full art on the 21 full-art cards alone; a version needs its own pick.
  // Checked before either face is worked out, so a full-art card costs no crossing while full art is on.
  if (fullArtOn && isFullArtCard(def.id)) return true;
  return [false, true].every((radiant) => {
    const face = faceModel({ defId: def.id, def, radiant });
    return !face.hasShortForm && !overLimit(face.text.full, FAN_TEXT_LIMIT);
  });
}

const FITTING = new Map<string, readonly CardDef[]>();

/**
 * #4511: whether each card fits the fan, by card and setting: the sampler asks again for every
 * draw, so without this each draw would cross for the card's two faces. The pools above stay as
 * they are; this caches the one card the fast path keeps asking about.
 */
const FIT = new Map<string, boolean>();

/** #4511: `fitsTheFan`, memoized per card and setting for the sampler's repeated draws. */
function fitsCached(def: CardDef, fullArtOn: boolean): boolean {
  const key = `${def.id}:${fullArtOn ? "1" : "0"}`;
  const known = FIT.get(key);
  if (known !== undefined) return known;
  const fits = fitsTheFan(def, fullArtOn);
  FIT.set(key, fits);
  return fits;
}

/** #4511: clears the fan's caches (`FITTING`, `FIT`, `WEIGHTS`) so a test can deal from cold. */
export function resetLandingFanCachesForTests(): void {
  FITTING.clear();
  FIT.clear();
  WEIGHTS.clear();
}

function fitting(name: string, pool: readonly CardDef[], fullArtOn: boolean): readonly CardDef[] {
  const key = `${name}:${String(fullArtOn)}`;
  const known = FITTING.get(key);
  if (known !== undefined) return known;
  const cards = pool.filter((def) => fitsCached(def, fullArtOn));
  FITTING.set(key, cards);
  return cards;
}

/** R374's Core pool, R1017002: the cards in it that fit the fan. */
export function fanPool(fullArtOn: boolean): readonly CardDef[] {
  return fitting("core", corePool(), fullArtOn);
}

/** R639's shipped pool, R1017002: the cards in it that fit the fan. */
export function rotationPool(fullArtOn: boolean): readonly CardDef[] {
  return fitting("shipped", shippedPool(), fullArtOn);
}

/** Clamp a faulty `random` result of 1 so it cannot overrun. */
export function below(random: RandomSource, n: number): number {
  return Math.min(n - 1, Math.floor(random() * n));
}

export function shuffled<T>(items: readonly T[], random: RandomSource): T[] {
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
 * R639 favors faces whose name and rules text print at the largest size, never excluding dense cards.
 * A full-art card counts as one (R1017002, R2655001): it prints art and no words, and with full art off it
 * is dealt only when its text fits. A Training Grounds Legacy card of one is named apart (R2511000) and
 * prints its words, so it does not.
 */
export function featureWeight(def: CardDef): number {
  const known = WEIGHTS.get(def.id);
  if (known !== undefined) return known;
  const face = faceModel({ defId: def.id, def, radiant: false });
  const fits =
    isFullArtCard(def.id) ||
    (TIER_RANK[nameTier(face.name)] <= TIER_RANK[FEATURE_PLAIN_MAX_TIER] &&
      TIER_RANK[textTier(face.text.full)] <= TIER_RANK[FEATURE_PLAIN_MAX_TIER]);
  const weight = fits ? FEATURE_WEIGHT_PLAIN : FEATURE_WEIGHT_DENSE;
  WEIGHTS.set(def.id, weight);
  return weight;
}

const WEIGHTS = new Map<string, number>();

/** R1017002: `weigh`, with a full-art card FULL_ART_WEIGHT_FACTOR times as likely while full art is on. */
export function favoringFullArt(weigh: CardWeight, fullArtOn: boolean): CardWeight {
  return (def) => weigh(def) * (fullArtOn && isFullArtCard(def.id) ? FULL_ART_WEIGHT_FACTOR : 1);
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

/**
 * #4511: how many uniform draws `sampleFittingCard` tries for one rarity before falling back to
 * today's full filter, so a rarity whose fitting cards are nearly unreachable stays correct.
 */
export const FAN_SAMPLE_MAX_DRAWS = 32;

/**
 * #4511: the face-call bound the fan keeps for a returning player's deal from the shipped pool
 * (landingFan.test.ts pins it): a few crossings per rarity, not every shipped card's two faces.
 */
export const LANDING_DEAL_FACE_BUDGET = 64;

/**
 * #4511: one rarity's card by rejection sampling: draw uniformly from `cards`, skip the cards that
 * do not fit the fan, and accept a fitting card with probability `weigh(def) / maxWeight`. Each
 * fitting card's chance stays exactly its weight over the fitting cards' summed weights (as R374,
 * R639, R704 and R1017002's weighted draws have it), while the deal costs a few face calls instead
 * of every card's. After `FAN_SAMPLE_MAX_DRAWS` draws without an acceptance, fall back to today's
 * full filter, so a rarity with few or no fitting cards deals exactly what today's would.
 */
export function sampleFittingCard(
  cards: readonly CardDef[],
  random: RandomSource,
  weigh: CardWeight,
  fullArtOn: boolean,
  maxWeight: number = FEATURE_WEIGHT_PLAIN * FULL_ART_WEIGHT_FACTOR,
): CardDef | undefined {
  for (let draw = 0; draw < FAN_SAMPLE_MAX_DRAWS; draw += 1) {
    if (cards.length === 0) return undefined;
    const def = cards[below(random, cards.length)];
    if (def === undefined || !fitsCached(def, fullArtOn)) continue;
    const weighed = weigh(def);
    const weight = Number.isFinite(weighed) && weighed > 0 ? weighed : 1;
    // A weight above `maxWeight` always accepts, since `random()` stays below 1.
    if (random() * maxWeight < weight) return def;
  }
  return pickWeighted(
    cards.filter((def) => fitsCached(def, fullArtOn)),
    random,
    weigh,
  );
}

/** R639 swaps one slot for a weighted, unseen card of the same rarity; its Radiant face stays put. */
export function rotateFan(
  hand: readonly FanFace[],
  slot: number,
  random: RandomSource,
  pool: readonly CardDef[] = shippedPool(),
  weigh: CardWeight = featureWeight,
  fullArtOn = false,
): FanFace[] {
  if (hand.length === 0) return [...hand];
  const at = ((slot % hand.length) + hand.length) % hand.length;
  const current = hand[at];
  if (current === undefined) return [...hand];
  const shown = new Set(hand.map(({ def }) => def.id));
  const candidates = pool.filter((def) => def.rarity === current.def.rarity && !shown.has(def.id));
  const next = sampleFittingCard(candidates, random, weigh, fullArtOn);
  if (next === undefined) return [...hand];
  return hand.map((face, index) => (index === at ? { def: next, radiant: face.radiant } : face));
}

/** R374 draws one card per rarity in random order; sparse pools deal fewer cards rather than repeat. */
export function dealLandingFan(
  random: RandomSource,
  pool: readonly CardDef[] = corePool(),
  weigh: CardWeight = EVEN,
  fullArtOn = false,
): FanFace[] {
  const byRarity = new Map<Rarity, CardDef[]>();
  for (const def of pool) byRarity.set(def.rarity, [...(byRarity.get(def.rarity) ?? []), def]);
  const rarities = shuffled([...byRarity.keys()], random).slice(0, FAN_FACES);
  const picks = rarities.map((rarity) => sampleFittingCard(byRarity.get(rarity) ?? [], random, weigh, fullArtOn));
  return picks
    .filter((def): def is CardDef => def !== undefined)
    .map((def, index) => ({ def, radiant: index === FAN_RADIANT_AT }));
}
