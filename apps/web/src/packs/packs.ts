// The pack-opening scene's model (#1308, SPEC §14.3, §14.4): constants, test ids and pure helpers for
// the scene, the pack object and the odds and pity the Purchases page shows (#1651).
//
// EVERY NUMBER SHOWN COMES FROM THE SERVER (CLAUDE.md rule 7): nothing here rolls, counts or decides,
// and the constants below are timings and looks, never a rate. A pack has no `PlayerView`, so the scene
// draws `CardFace` and mounts the fx director itself (`PackOpening.tsx`), over the cues built here.

import { GLOBAL_PACK_NAME } from "@jackioh/server-config";
import type { PrintedRarity } from "@jackioh/shared";

import { getAudioEngine } from "../audio/engine.ts";
import { raysCue, tid, tunedBurst, type BurstTuning } from "../fx/build.ts";
import type { FxCue } from "../fx/types.ts";
import { t } from "../i18n/i18n.ts";
import type { OpenedPackResponse, PackSlot, PacksResponse } from "../net/api.ts";
import { COSMETICS, type CosmeticRarity } from "../wire/generated/cosmetics.ts";

/** How long a torn pack takes to open before its cards show. */
export const PACK_TEAR_MS = 900;
/** How long a card of Epic or above is held before it turns over. */
export const PACK_HOLD_MS = 600;
export const PACK_FLIP_MS = 350;
/** How far a swipe across the sealed pack must travel to open it. */
export const PACK_SWIPE_PX = 40;
/** How long a Legendary's or Mythic's rays last. */
export const PACK_RAYS_MS = 1200;
/** How far, in degrees, a pack object leans toward the pointer at the edge of its face. */
export const PACK_TILT_DEG = 8;
/** The pack object's name for the cosmetic pack; a card pack is a shipped set's name or `GLOBAL_PACK_NAME`. */
export const COSMETIC_PACK = "cosmetic";
export const PACK_ODDS_DECIMALS = 1;

/** Rarities lowest first. */
export const RARITY_ORDER: readonly PrintedRarity[] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];

/** The five tiers of the server's odds, commonest first, in the order the panel lists them. */
export const ODDS_RARITIES = [
  { key: "common", rarity: "Common" },
  { key: "rare", rarity: "Rare" },
  { key: "epic", rarity: "Epic" },
  { key: "legendary", rarity: "Legendary" },
  { key: "mythic", rarity: "Mythic" },
] as const;

/** R1580000: the rarities pity counts (§14.4), lowest first, keyed as the server's counters are. */
export const PITY_RARITIES = [
  { key: "epic", rarity: "Epic" },
  { key: "legendary", rarity: "Legendary" },
  { key: "mythic", rarity: "Mythic" },
] as const;

/** A reveal's burst, by rarity: the particle count at intensity "normal" and the power it is thrown with. */
export const PACK_BURSTS: Readonly<Record<"Rare" | "Epic" | "Legendary" | "Mythic", BurstTuning>> = {
  Rare: { count: 18, power: 0.9 },
  Epic: { count: 30, power: 1.1 },
  Legendary: { count: 48, power: 1.3 },
  Mythic: { count: 60, power: 1.5 },
};

export const packTestid = {
  panel: "packs-panel",
  loading: "packs-loading",
  loadError: "packs-load-error",
  retry: "packs-retry",
  odds: "packs-odds",
  oddsRow: (key: string) => `packs-odds-${key}`,
  pity: (key: string) => `packs-pity-${key}`,
  pityLine: "packs-pity-line",
  pityPanel: "packs-pity-panel",
  duplicate: "packs-duplicate",
  held: "packs-held",
  none: "packs-none",
  heldRow: (pack: string) => `packs-held-${pack}`,
  open: (pack: string) => `packs-open-${pack}`,
  openAll: (pack: string) => `packs-open-all-${pack}`,
  scene: "pack-scene",
  /** R3349000: the Mythic pull's takeover, the pack torn open into the full-screen view with its own art. */
  takeover: "pack-takeover",
  error: "pack-error",
  slots: "pack-slots",
  skipAll: "pack-skip-all",
  next: "pack-next",
  close: "pack-close",
  fx: "pack-fx",
  pack: "pack",
  object: "pack-object",
  bars: "pack-bars",
  slot: (index: number) => `pack-slot-${String(index)}`,
  flip: (index: number) => `pack-slot-${String(index)}-flip`,
  outcome: (index: number) => `pack-slot-${String(index)}-outcome`,
  equip: (index: number) => `pack-slot-${String(index)}-equip`,
  /** R3015003: a slot's line saying an out-of-order pull granted the levels below its level too. */
  below: (index: number) => `pack-slot-${String(index)}-below`,
} as const;

/** A rate as a percentage to `PACK_ODDS_DECIMALS`, without trailing zeros. */
export function percent(rate: number): string {
  return `${String(Number((rate * 100).toFixed(PACK_ODDS_DECIMALS)))}%`;
}

/** The highest rarity among a pack's slots; Common for none. */
export function bestRarity(slots: readonly PackSlot[]): PrintedRarity {
  return slots.reduce<PrintedRarity>(
    (best, slot) => (RARITY_ORDER.indexOf(slot.rarity) > RARITY_ORDER.indexOf(best) ? slot.rarity : best),
    "Common",
  );
}

export function holdsToFlip(rarity: PrintedRarity): boolean {
  return RARITY_ORDER.indexOf(rarity) >= RARITY_ORDER.indexOf("Epic");
}

export function packName(pack: string): string {
  if (pack === GLOBAL_PACK_NAME) return t("Global pack");
  if (pack === COSMETIC_PACK) return t("Cosmetic pack");
  return t("{set} pack", { set: t(pack) });
}

export function cosmeticName(id: string): string {
  return COSMETICS.find((cosmetic) => cosmetic.id === id)?.name ?? id;
}

/**
 * What a slot became, as the server decided it (R1301002). Every slot adds its `points`, the opening's
 * `slotPoints`, to each bar (R1953000, R1999000), and its flight shows them; only a spent duplicate,
 * which grants nothing else, says them too.
 */
export function slotOutcome(slot: PackSlot, points: number): string {
  switch (slot.grant.kind) {
    case "card":
      return t("New card");
    case "cosmetic":
      return t("New cosmetic: {name}", { name: cosmeticName(slot.grant.cosmeticId) });
    case "bars":
      return t("Duplicate: +{card} card token bar, +{cosmetic} cosmetic token bar", { card: points, cosmetic: points });
  }
}

/** The panel after an opening: the server's pity counters and its count of the kind opened, a kind with none left dropped. */
export function withOpening(info: PacksResponse, opened: OpenedPackResponse): PacksResponse {
  return {
    ...info,
    pity: opened.pity,
    unopened: info.unopened
      .map((entry) => (entry.pack === opened.pack ? { ...entry, count: opened.unopened } : entry))
      .filter((entry) => entry.count > 0),
  };
}

/** The sting a reveal plays on the existing audio buses. */
export function playStinger(rarity: PrintedRarity): void {
  const audio = getAudioEngine();
  switch (rarity) {
    case "Common":
      audio.playSfx("sting");
      return;
    case "Rare":
      audio.playSfx("sting", { tier: "rare" });
      return;
    case "Epic":
      audio.playSfx("sting", { tier: "epic" });
      return;
    case "Legendary":
      audio.playSfx("entrance");
      return;
    case "Mythic":
      audio.playSfx("entrance", { mythic: true });
      return;
  }
}

/** How long a variant-level grant reveal holds the card's previous look before it wipes into the new level. */
export const GRANT_BEFORE_MS = 700;
/** How long a variant-level grant reveal's wipe into the new level takes; `grantReveal.css` reads it as `--grant-wipe`. */
export const GRANT_WIPE_MS = 600;
/** How long a full-screen grant moment holds under `bigMoments` "short": half the takeover's hold (R2785000). */
export const GRANT_TAKEOVER_SHORT_MS = 500;

/**
 * How a variant-level grant is staged, by the level's own rarity (R3349000, R1333000): Rare and Epic
 * play on the panel, Legendary dims it and grows the card in, Mythic takes the screen.
 */
export type GrantTreatment = "panel" | "grow" | "takeover";
export const GRANT_TREATMENT: Readonly<Record<CosmeticRarity, GrantTreatment>> = {
  Common: "panel",
  Rare: "panel",
  Epic: "panel",
  Legendary: "grow",
  Mythic: "takeover",
};

/**
 * A chase buy takes the screen from Legendary up (R3349000): a Legendary or Mythic chase plays the
 * takeover, any other grant its own rarity's treatment.
 */
export function treatmentOf(rarity: CosmeticRarity, chase: boolean): GrantTreatment {
  if (rarity === "Mythic" || (chase && rarity === "Legendary")) return "takeover";
  return GRANT_TREATMENT[rarity];
}

/**
 * Grants rarest last, ties keeping their order, so several grants at once reveal in that order while a
 * pack's own pulls keep the pack's (R3349000): only grant queues are sorted, never pack slots.
 */
export function orderGrantsRarestLast<T extends { rarity: CosmeticRarity }>(items: readonly T[]): T[] {
  return items
    .map((item, index) => ({ item, index }))
    .sort((a, b) => RARITY_ORDER.indexOf(a.item.rarity) - RARITY_ORDER.indexOf(b.item.rarity) || a.index - b.index)
    .map(({ item }) => item);
}

/**
 * A Legendary pull's flare on its sealed tile: gold rays with a rising burst, mirroring `revealCues`'
 * Legendary entry, played once the pack tears and before any flip (R3349000).
 */
export function legendaryFlareCues(testid: string, intensity: number): FxCue[] {
  const at = tid(testid);
  return [raysCue(PACK_RAYS_MS, "legendary", at, 0), tunedBurst(intensity, "gold", at, "area", 0, PACK_BURSTS.Legendary)];
}

export function revealCues(rarity: PrintedRarity, testid: string, intensity: number): FxCue[] {
  const at = tid(testid);
  switch (rarity) {
    case "Common":
      return [];
    case "Rare":
      return [tunedBurst(intensity, "sparkle", at, "area", 0, PACK_BURSTS.Rare)];
    case "Epic":
      return [tunedBurst(intensity, "arcane", at, "area", 0, PACK_BURSTS.Epic)];
    case "Legendary":
      return [raysCue(PACK_RAYS_MS, "legendary", at, 0), tunedBurst(intensity, "gold", at, "area", 0, PACK_BURSTS.Legendary)];
    case "Mythic":
      return [raysCue(PACK_RAYS_MS, "mythic", at, 0), tunedBurst(intensity, "prismatic", at, "area", 0, PACK_BURSTS.Mythic)];
  }
}
