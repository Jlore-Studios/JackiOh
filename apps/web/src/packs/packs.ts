// The pack-opening scene's model (#1308, SPEC §14.3, §14.4): its constants, its test ids and the pure
// helpers behind the scene and the odds panel, all of them plain data in and out.
//
// EVERY NUMBER SHOWN COMES FROM THE SERVER (CLAUDE.md rule 7). `GET /api/packs` says the odds, the
// pity counters and their thresholds; `POST /api/packs/open` says every slot before anything
// animates. Nothing here rolls, counts or decides: `withOpening` copies the server's counters into
// the panel, `tierPercents` only formats the server's odds, and the constants below are timings and
// looks, never a rate.
//
// `CardShowcase` and `FxLayer` read a game's `PlayerView`, and a pack has none, so the scene draws
// `CardFace` itself and mounts the fx director itself (`PackOpening.tsx`), over the cues built here.

import {
  CARD_TOKEN_BAR_POINTS_PER_DUPLICATE,
  COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE,
  GLOBAL_PACK_NAME,
} from "@jackioh/server-config";
import type { PrintedRarity } from "@jackioh/shared";

import { getAudioEngine } from "../audio/engine.ts";
import { raysCue, tid, tunedBurst, type BurstTuning } from "../fx/build.ts";
import type { FxCue } from "../fx/types.ts";
import { t } from "../i18n/i18n.ts";
import type { OpenedPackResponse, PackSlot, PacksResponse, TierOdds } from "../net/api.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";

/** How long a torn pack takes to open before its cards show. */
export const PACK_TEAR_MS = 900;
/** How long a card of Epic or above is held before it turns over. */
export const PACK_HOLD_MS = 600;
/** How long a card takes to turn over. */
export const PACK_FLIP_MS = 350;
/** How far a swipe across the sealed pack must travel to open it. */
export const PACK_SWIPE_PX = 40;
/** How long a Legendary's or Mythic's rays last. */
export const PACK_RAYS_MS = 1200;
/** The decimals an odds figure is shown to. */
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
  duplicate: "packs-duplicate",
  held: "packs-held",
  none: "packs-none",
  heldRow: (pack: string) => `packs-held-${pack}`,
  open: (pack: string) => `packs-open-${pack}`,
  openAll: (pack: string) => `packs-open-all-${pack}`,
  scene: "pack-scene",
  error: "pack-error",
  slots: "pack-slots",
  skipAll: "pack-skip-all",
  next: "pack-next",
  close: "pack-close",
  fx: "pack-fx",
  pack: "pack",
  slot: (index: number) => `pack-slot-${String(index)}`,
  flip: (index: number) => `pack-slot-${String(index)}-flip`,
  outcome: (index: number) => `pack-slot-${String(index)}-outcome`,
} as const;

/** A rate as a percentage to `PACK_ODDS_DECIMALS`, without trailing zeros: 0.29 is "29%", 0.082 "8.2%". */
export function percent(rate: number): string {
  return `${String(Number((rate * 100).toFixed(PACK_ODDS_DECIMALS)))}%`;
}

/** The server's odds of each tier as percentages, by tier key, for a sentence to name. */
export function tierPercents(odds: TierOdds): Record<keyof TierOdds, string> {
  return {
    common: percent(odds.common),
    rare: percent(odds.rare),
    epic: percent(odds.epic),
    legendary: percent(odds.legendary),
    mythic: percent(odds.mythic),
  };
}

/** The highest rarity among a pack's slots; Common for none. */
export function bestRarity(slots: readonly PackSlot[]): PrintedRarity {
  return slots.reduce<PrintedRarity>(
    (best, slot) => (RARITY_ORDER.indexOf(slot.rarity) > RARITY_ORDER.indexOf(best) ? slot.rarity : best),
    "Common",
  );
}

/** Whether a card of this rarity turns over only after a hold: Epic and above. */
export function holdsToFlip(rarity: PrintedRarity): boolean {
  return RARITY_ORDER.indexOf(rarity) >= RARITY_ORDER.indexOf("Epic");
}

/** A pack in a player's words: the global pack, or a set's. */
export function packName(pack: string): string {
  return pack === GLOBAL_PACK_NAME ? t("Global pack") : t("{set} pack", { set: t(pack) });
}

/** A cosmetic's name from the registry, else its id. */
export function cosmeticName(id: string): string {
  return COSMETICS.find((cosmetic) => cosmetic.id === id)?.name ?? id;
}

/** What a slot became, as the server decided it (R1301002). */
export function slotOutcome(slot: PackSlot): string {
  switch (slot.grant.kind) {
    case "card":
      return t("New card");
    case "cosmetic":
      return t("New cosmetic: {name}", { name: cosmeticName(slot.grant.cosmeticId) });
    case "bars":
      return t("Duplicate: +{card} card token bar, +{cosmetic} cosmetic token bar", {
        card: CARD_TOKEN_BAR_POINTS_PER_DUPLICATE,
        cosmetic: COSMETIC_TOKEN_BAR_POINTS_PER_DUPLICATE,
      });
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

/** The sting a reveal plays on the existing audio buses: the played-card sting below Legendary, the entrance from it up. */
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

/** A reveal's effects on the card at `testid`: none for Common, a burst for Rare and Epic, rays and a burst above. */
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
