// R374 deals real, non-token Core cards into four rarities in random order, with the middle Radiant.
// Randomness is client-owned (CLAUDE.md rule 4) and injected so tests can deal deterministically.

import { SHIPPED_SETS, type CardDef, type CardDefs, type Rarity } from "@jackioh/shared";
import catalogJson from "@jackioh/cards/catalog.json";

import { nameTier, textTier, type LengthTier } from "../cards/fit.ts";
import { faceModel } from "../cards/model.ts";
import { FEATURE_PLAIN_MAX_TIER, FEATURE_WEIGHT_DENSE, FEATURE_WEIGHT_PLAIN } from "../stats/config.ts";

export type FanFace = { readonly def: CardDef; readonly radiant: boolean };

export type RandomSource = () => number;

export const FAN_FACES = 4;

export const FAN_RADIANT_AT = 2;

/** JSON widens the unions to strings; the catalog is proved against SPEC §8 in crates/cards. */
const CATALOG = catalogJson as unknown as CardDefs;

/** Core cards a deck may hold, tokens aside (§2.6). */
export const FAN_POOL: readonly CardDef[] = Object.values(CATALOG).filter((def) => !def.token && def.set === "Core");

/** R639: non-token cards from shipped sets, in catalog order, for experienced-player rotation. */
export const ROTATION_POOL: readonly CardDef[] = Object.values(CATALOG).filter(
  (def) => !def.token && (SHIPPED_SETS as readonly string[]).includes(def.set),
);

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

/** R639 favours faces whose name and rules text print at the largest size, but never excludes dense cards. */
export function featureWeight(def: CardDef): number {
  const known = WEIGHTS.get(def.id);
  if (known !== undefined) return known;
  const face = faceModel({ defId: def.id, def, radiant: false });
  const fits =
    TIER_RANK[nameTier(face.name)] <= TIER_RANK[FEATURE_PLAIN_MAX_TIER] &&
    TIER_RANK[textTier(face.text.full)] <= TIER_RANK[FEATURE_PLAIN_MAX_TIER];
  const weight = fits ? FEATURE_WEIGHT_PLAIN : FEATURE_WEIGHT_DENSE;
  WEIGHTS.set(def.id, weight);
  return weight;
}

const WEIGHTS = new Map<string, number>();

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
