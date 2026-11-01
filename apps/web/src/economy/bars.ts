// The pure parts of the bar that fills and the reward reveal (#1658, part of #1651, SPEC §14.5). No
// DOM or clock; the components keep the timers. THE SERVER DECIDES (CLAUDE.md rule 7): these only lay
// the server's answer out in time.

import type { PrintedRarity } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import type { OpeningBarsAnswer } from "../net/api.ts";
import { RARITY_ORDER } from "../packs/packs.ts";
import { nextStepWords } from "../shop/shop.ts";
import type { CosmeticRarity } from "../wire/generated/cosmetics.ts";

/** How long a sweep across a whole bar takes; a shorter stretch takes its share of it. */
export const FILL_MS = 900;

/** How long a full bar bursts before it restarts at 0. */
export const BURST_MS = 500;

/** The pause before each reward flies out, after the one before it (or after it was handed in). */
export const REVEAL_STAGGER_MS = 350;

export const REVEAL_FLY_MS = 450;

/** How long a slot's points take to fly from the slot into the bars. */
export const BAR_FLIGHT_MS = 650;

/** The pause between one slot's points taking off and the next's. */
export const BAR_FLIGHT_STAGGER_MS = 180;

export type Segment = {
  from: number;
  to: number;
  /** The bar is full at the end of this sweep: it bursts and a fill is counted. */
  burst: boolean;
};

export type OpeningBar = OpeningBarsAnswer["card"];

/**
 * One slot's points on their way to the bars (R1953000, R1999000): the server's points, the slot's
 * rarity, which tints the flight and sizes its burst, and the testid of the slot it takes off from.
 */
export type BarFlight = {
  key: string;
  points: number;
  rarity: PrintedRarity | CosmeticRarity;
  source: string;
};

export type RevealKind = "cardToken" | "cosmeticToken" | "cosmeticPack" | "jlore" | "setPack" | "level";

/** One reward in the reveal; `label` is already through `t()`. */
export type RevealItem = {
  key: string;
  kind: RevealKind;
  label: string;
  /** A printed rarity's name sets the sting and the look; anything else falls back to the kind's. */
  rarity?: string;
};

/** The sting a reward without a rarity of its own plays, by kind (`packs.ts::playStinger`'s tiers). */
const KIND_STING: Readonly<Record<RevealKind, PrintedRarity>> = {
  cardToken: "Rare",
  cosmeticToken: "Rare",
  cosmeticPack: "Epic",
  jlore: "Common",
  setPack: "Rare",
  level: "Rare",
};

/** `points` of `size` as the fraction a `scaleX` draws, held within the bar. */
export function fraction(points: number, size: number): number {
  if (!(size > 0)) return 0;
  return Math.min(1, Math.max(0, points / size));
}

/**
 * The sweeps a bar plays from `from` to `value` over a bar of `size`: each of `fills` sweeps to full,
 * bursts and restarts at 0, and the last ends at `value`. Points past either end are held to the bar.
 */
export function segments(size: number, value: number, from: number, fills: number): Segment[] {
  const top = Math.max(0, size);
  const hold = (points: number): number => Math.min(top, Math.max(0, points));
  const bursts = Math.max(0, Math.floor(fills));
  const out: Segment[] = [];
  for (let burst = 0; burst < bursts; burst += 1) {
    out.push({ from: burst === 0 ? hold(from) : 0, to: top, burst: true });
  }
  out.push({ from: bursts === 0 ? hold(from) : 0, to: hold(value), burst: false });
  return out;
}

export function segmentMs(segment: Segment, size: number): number {
  if (!(size > 0)) return 0;
  return Math.round((FILL_MS * Math.abs(segment.to - segment.from)) / size);
}

/**
 * The flights of an opening (R1953000): one for each slot `slotPoints` gives points (every slot,
 * R1999000), in slot order, with its rarity from `rarities` and its testid from `sourceOf`.
 */
export function flightsOf(
  bars: OpeningBarsAnswer,
  rarities: readonly (PrintedRarity | CosmeticRarity)[],
  sourceOf: (index: number) => string,
): BarFlight[] {
  return bars.slotPoints.flatMap((points, index) =>
    points > 0
      ? [{ key: `flight-${String(index)}`, points, rarity: rarities[index] ?? "Common", source: sourceOf(index) }]
      : [],
  );
}

export function flightsMs(count: number): number {
  return count > 0 ? (count - 1) * BAR_FLIGHT_STAGGER_MS + BAR_FLIGHT_MS : 0;
}

/**
 * The reward a bar's `fill`th fill (from 1) reveals: a card token for the card bar, the cycle's step
 * the opening paid for the cosmetic bar (R1305001); none when the answer names no such fill.
 */
export function fillReward(bars: OpeningBarsAnswer, bar: "card" | "cosmetic", fill: number): RevealItem | null {
  if (bar === "card") {
    return fill >= 1 && fill <= bars.card.pays ? { key: `card-${String(fill)}`, kind: "cardToken", label: t("Card token") } : null;
  }
  const step = bars.cosmeticPaid[fill - 1];
  if (step === undefined) return null;
  const key = `cosmetic-${String(fill)}`;
  const label = nextStepWords(step);
  if (step === "cosmeticPack") return { key, kind: "cosmeticPack", label };
  return { key, kind: "cosmeticToken", label, rarity: step === "epic" ? "Epic" : "Rare" };
}

/** The rarity whose sting a reward plays: its own, when it names a printed one, else its kind's. */
export function stingOf(item: RevealItem): PrintedRarity {
  return RARITY_ORDER.find((rarity) => rarity === item.rarity) ?? KIND_STING[item.kind];
}

/** The strongest sting among rewards shown at once; Common for none. */
export function strongestSting(items: readonly RevealItem[]): PrintedRarity {
  return items.reduce<PrintedRarity>((best, item) => {
    const sting = stingOf(item);
    return RARITY_ORDER.indexOf(sting) > RARITY_ORDER.indexOf(best) ? sting : best;
  }, "Common");
}
