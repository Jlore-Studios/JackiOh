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

import type { CardDef, CardDefs, Rarity } from "@jackioh/shared";
import catalogJson from "@jackioh/cards/catalog.json";

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

/**
 * R374: a fresh hand for the fan. Four of the pool's rarities, chosen at random, one random card of
 * each, in a random order, and the middle card Radiant. A pool with fewer than four rarities repeats
 * none and deals fewer cards; an empty one deals none.
 */
export function dealLandingFan(random: RandomSource, pool: readonly CardDef[] = FAN_POOL): FanFace[] {
  const byRarity = new Map<Rarity, CardDef[]>();
  for (const def of pool) byRarity.set(def.rarity, [...(byRarity.get(def.rarity) ?? []), def]);
  const rarities = shuffled([...byRarity.keys()], random).slice(0, FAN_FACES);
  const picks = rarities.map((rarity) => {
    const cards = byRarity.get(rarity) ?? [];
    return cards[below(random, cards.length)];
  });
  return picks
    .filter((def): def is CardDef => def !== undefined)
    .map((def, index) => ({ def, radiant: index === FAN_RADIANT_AT }));
}
