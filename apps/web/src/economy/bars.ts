// The pure parts of the bar that fills and the reward reveal (#1658, part of #1651, SPEC §14.5):
// the segments one `FillBar` plays, the points each slot sends flying into an opening's bars
// (R1953000, R1999000), the reward a bar's fill reveals in an opening, and the sting a reward plays. Nothing
// here reads the DOM or a clock; the components keep the timers.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). A bar's points, its size, each slot's points and what a fill
// pays all come from the server's answer; these functions only lay that answer out in time.

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

/** How long one reward takes to fly out. */
export const REVEAL_FLY_MS = 450;

/** How long a slot's points take to fly from the slot into the bars. */
export const BAR_FLIGHT_MS = 650;

/** The pause between one slot's points taking off and the next's. */
export const BAR_FLIGHT_STAGGER_MS = 180;

/** One stretch of a bar's animation: a sweep from one number of points to another. */
export type Segment = {
  from: number;
  to: number;
  /** The bar is full at the end of this sweep: it bursts and a fill is counted. */
  burst: boolean;
};

/** A bar's state in an opening's answer: its points before and after, its size and how often it paid. */
export type OpeningBar = OpeningBarsAnswer["card"];

/**
 * One slot's points on their way to the bars (R1953000, R1999000): the server's points, the slot's
 * rarity (a card pack's card rarity, a cosmetic pack's cosmetic rarity), which tints the flight and
 * sizes its burst, and the testid of the slot it takes off from.
 */
export type BarFlight = {
  key: string;
  points: number;
  rarity: PrintedRarity | CosmeticRarity;
  source: string;
};

/** What a reward is, which the reveal draws and the sting follows. */
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
 * The size of the bar a sweep crosses (R2728000): the `step`th of `sizes`, the server's size of each
 * step a bar's fills cross, else `size`, the one size of a bar whose every step is the same.
 */
export function sizeAt(size: number, sizes: readonly number[] | undefined, step: number): number {
  return sizes?.[step] ?? size;
}

/**
 * The sweeps a bar plays, `from` its points before to `value` after, over a bar of `size`. Each of
 * `fills` is a sweep to full that bursts and restarts at 0 (the first starts at `from`), and the
 * last sweep ends at `value`. A bar that paid nothing is one sweep. Points past either end of the
 * bar are held to it. With `sizes` (R2728000), sweep `i` crosses a bar of `sizes[i]`, the last one
 * the bar it rests on; without, every sweep crosses `size`.
 */
export function segments(size: number, value: number, from: number, fills: number, sizes?: readonly number[]): Segment[] {
  const top = (step: number): number => Math.max(0, sizeAt(size, sizes, step));
  const hold = (points: number, step: number): number => Math.min(top(step), Math.max(0, points));
  const bursts = Math.max(0, Math.floor(fills));
  const out: Segment[] = [];
  for (let burst = 0; burst < bursts; burst += 1) {
    out.push({ from: burst === 0 ? hold(from, 0) : 0, to: top(burst), burst: true });
  }
  out.push({ from: bursts === 0 ? hold(from, 0) : 0, to: hold(value, bursts), burst: false });
  return out;
}

/** How long a segment's sweep takes: its share of the bar, of `FILL_MS`. */
export function segmentMs(segment: Segment, size: number): number {
  if (!(size > 0)) return 0;
  return Math.round((FILL_MS * Math.abs(segment.to - segment.from)) / size);
}

/**
 * The flights of an opening (R1953000): one for each slot the answer's `slotPoints` gives points,
 * which since R1999000 is every slot, in slot order, with slot `index`'s rarity in `rarities` and
 * its testid from `sourceOf`.
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

/** How long `count` flights take, the last one's take-off its stagger after the one before; none for none. */
export function flightsMs(count: number): number {
  return count > 0 ? (count - 1) * BAR_FLIGHT_STAGGER_MS + BAR_FLIGHT_MS : 0;
}

/** The printed rarity of each cycle step that pays a token (R1305001, R2728000). */
const STEP_RARITY = { rare: "Rare", epic: "Epic", legendary: "Legendary" } as const;

/**
 * The reward a bar's `fill`th fill (counting from 1) reveals in an opening: a card token for the
 * card bar, the cycle's step the opening paid for the cosmetic bar (R1305001). None when the
 * answer names no such fill.
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
  return { key, kind: "cosmeticToken", label, rarity: STEP_RARITY[step] };
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
