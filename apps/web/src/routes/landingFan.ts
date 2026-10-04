// The hand the landing hero fans out (routes/landing.tsx): real Core cards drawn by the cards
// module's CardFace, as they look everywhere else in the game, rather than a second card style of the
// landing's own (integration QA: glyph gems, an empty name ribbon and grey bars for rules text).
//
// v0.1.1 (R374): the hand is dealt at random on every visit, from every non-token Core card, instead
// of the same four each time. It keeps the fixed hand's shape: four cards of four different rarities
// in a random order, the middle one on its Radiant face. The cards come straight from
// packages/cards/catalog.json (about 8.5 KB gzipped), so the landing always shows them as they are
// printed now; the fixed hand was four copies kept equal to it by a test.
//
// Randomness here is the client's own, never the game's (CLAUDE.md rule 4 binds the engine, the
// cards and the AI, not the page), and the source is passed in, so a test deals the same hand every
// time (landingFan.test.ts) while the page uses `Math.random`.

import { SHIPPED_SETS, type CardDef, type CardDefs, type Rarity } from "@jackioh/shared";
import catalogJson from "@jackioh/cards/catalog.json";

import { nameTier, textTier, type LengthTier } from "../cards/fit.ts";
import { faceModel } from "../cards/model.ts";
import { FEATURE_PLAIN_MAX_TIER, FEATURE_WEIGHT_DENSE, FEATURE_WEIGHT_PLAIN } from "../stats/config.ts";

/** One face-up card of the fan: its catalog definition and the face it shows. */
export type FanFace = { readonly def: CardDef; readonly radiant: boolean };

/** A source of numbers in [0, 1), `Math.random`'s shape. */
export type RandomSource = () => number;

/** The face-up cards; a card back follows them as the fan's fifth card. */
export const FAN_FACES = 4;

/** Which face-up card, from the left, shows its Radiant face: the middle of the five. */
export const FAN_RADIANT_AT = 2;

/** JSON widens the unions to strings; the catalog is proved against SPEC §8 in packages/cards. */
const CATALOG = catalogJson as unknown as CardDefs;

/** Every card a deck may hold: the Core cards, tokens aside (§2.6), in catalog order. */
export const FAN_POOL: readonly CardDef[] = Object.values(CATALOG).filter((def) => !def.token && def.set === "Core");

/**
 * R639: every card a deck may hold from any shipped set (Core, Classic and Classic+), tokens aside,
 * in catalog order — the pool the fan rotates through once the device has logged enough games.
 */
export const ROTATION_POOL: readonly CardDef[] = Object.values(CATALOG).filter(
  (def) => !def.token && (SHIPPED_SETS as readonly string[]).includes(def.set),
);

/** A whole number in [0, n) from `random`, clamped so a source that returns 1 cannot overrun. */
function below(random: RandomSource, n: number): number {
  return Math.min(n - 1, Math.floor(random() * n));
}

/** Fisher–Yates over a copy, driven by `random`. */
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

/** How readily a card is drawn: a positive number, in proportion to the others'. */
export type CardWeight = (def: CardDef) => number;

/** Every card equally likely: R374's deal, which draws uniformly from a rarity, and R655's swaps below R639's threshold. */
export const EVEN: CardWeight = () => 1;

const TIER_RANK: Readonly<Record<LengthTier, number>> = { s: 0, m: 1, l: 2, xl: 3, xxl: 4 };

/**
 * R639: a card that looks good on the homescreen is one whose name and rules text print at the
 * largest size — its two shortest length tiers (`fit.ts`) — rather than shrunk to fit. Those weigh
 * `FEATURE_WEIGHT_PLAIN`; the rest weigh `FEATURE_WEIGHT_DENSE`, which is above 0: a dense card is
 * less likely, never left out.
 */
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

/** The weights already worked out: a card's face is fixed for the life of the page. */
const WEIGHTS = new Map<string, number>();

/**
 * One card of `cards`, chosen in proportion to its weight with a single draw from `random`; a weight
 * that is not above 0 counts as 1, so no card is ever unreachable. Undefined for an empty list.
 */
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
 * R639: the fan after one rotation step. The card in `slot` (taken modulo the fan's cards) is swapped
 * for one of the pool's cards of the same rarity that the fan is not already showing, drawn by
 * weight, so the hand keeps its four rarities and never shows a card twice. A slot with nothing to
 * swap in keeps its card. The Radiant face stays on its slot (`FAN_RADIANT_AT`).
 */
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

/**
 * R374: a fresh hand for the fan. Four of the pool's rarities, chosen at random, one random card of
 * each, in a random order, and the middle card Radiant. A pool with fewer than four rarities repeats
 * none and deals fewer cards; an empty one deals none.
 */
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
