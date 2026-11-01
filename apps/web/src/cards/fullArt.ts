// Full-art faces (R1017001, #1017): a card with a `fullArt` entry in the cosmetics registry (R2655001, #2655) prints its
// art across the whole face, no words; the Full-art cards setting turns it off. R1313000: where ownership decides
// (`FULL_ART_GATED`) only the viewer's own copy of an owned full art prints it (`FullArtOwned`), else all count as owned.
// R2655000: three tiers (`FullArtTier`); R2655002: a token in `fullArtTokens.json` prints its parent's. CLAUDE.md rule 7.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import { ECONOMY_LIVE } from "@jackioh/server-config";

import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { OwnershipContext, TheirCopyContext, ownsCosmetic, useCardPicks, type CardOwnership, type OwnedByCard } from "./cosmetics.ts";
import fullArtTokens from "./fullArtTokens.json";
import type { FaceModel } from "./model.ts";
import { readCardSettings, useCardSettings } from "./settings.ts";
import { isGlitch } from "./glitch.ts";

/** R2655000: a full art's tier, the cosmetics' own rarity scale's: the still Epic, the animated Legendary, the signed Mythic. */
export type FullArtTier = "Epic" | "Legendary" | "Mythic";

/** R2655000: the tiers, commonest first, as the registry lists each card's full arts. */
export const FULL_ART_TIERS: readonly FullArtTier[] = ["Epic", "Legendary", "Mythic"];

function isFullArtTier(rarity: string): rarity is FullArtTier {
  return (FULL_ART_TIERS as readonly string[]).includes(rarity);
}

/** R2655000: each full-art card's full arts, by tier, from the registry's `fullArt` entries. */
export const FULL_ARTS_OF_CARD: ReadonlyMap<string, ReadonlyMap<FullArtTier, string>> = (() => {
  const out = new Map<string, Map<FullArtTier, string>>();
  for (const cosmetic of COSMETICS) {
    if (cosmetic.kind !== "fullArt" || !isFullArtTier(cosmetic.rarity)) continue;
    const tiers = out.get(cosmetic.card) ?? new Map<FullArtTier, string>();
    tiers.set(cosmetic.rarity, cosmetic.id);
    out.set(cosmetic.card, tiers);
  }
  return out;
})();

/** R2655001: the cards that print full art, by card id: every card the registry has a full art of. */
export const FULL_ART_CARDS: ReadonlySet<string> = new Set(FULL_ARTS_OF_CARD.keys());

/** Each full-art card, to the id of its Epic full art, what a pick of `true` names (R2655000). */
export const FULL_ART_OF_CARD: ReadonlyMap<string, string> = new Map(
  [...FULL_ARTS_OF_CARD].flatMap(([card, tiers]): [string, string][] => {
    const epic = tiers.get("Epic");
    return epic === undefined ? [] : [[card, epic]];
  }),
);

/** R2655002: each token that prints its parent card's full art, to that parent (`fullArtTokens.json`). */
export const FULL_ART_TOKEN_PARENT: ReadonlyMap<string, string> = new Map(
  Object.entries(fullArtTokens as Readonly<Record<string, readonly string[]>>).flatMap(([parent, tokens]) =>
    tokens.map((token): [string, string] => [token, parent]),
  ),
);

/** R2655002: the card whose full art `defId` prints: a listed token's parent, else the card itself. */
export function fullArtSource(defId: string): string {
  return FULL_ART_TOKEN_PARENT.get(defId) ?? defId;
}

/**
 * Whether `defId` has a full-art face (R2655001): a registry full art or a listed token's (R2655002), never
 * the Glitch. A Training Grounds Legacy card has its own id (R2511000), so the Core card's art is never its.
 */
export function isFullArtCard(defId: string): boolean {
  return !isGlitch(defId) && FULL_ART_CARDS.has(fullArtSource(defId));
}

/**
 * R2655000: the id of the full art of `defId` (a full-art card, not a token) that `pick` names: its Epic for
 * `true`, one of its own ids for that id; null otherwise.
 */
export function fullArtIdOf(defId: string, pick: boolean | string | null | undefined): string | null {
  const tiers = FULL_ARTS_OF_CARD.get(defId);
  if (tiers === undefined) return null;
  if (pick === true) return tiers.get("Epic") ?? null;
  if (typeof pick !== "string") return null;
  return [...tiers.values()].includes(pick) ? pick : null;
}

/** R2655000: the tier of the full art `pick` names on `defId` (`fullArtIdOf`), or null. */
export function fullArtTierOf(defId: string, pick: boolean | string | null | undefined): FullArtTier | null {
  const id = fullArtIdOf(defId, pick);
  if (id === null) return null;
  for (const [tier, tierId] of FULL_ARTS_OF_CARD.get(defId) ?? []) if (tierId === id) return tier;
  return null;
}

/** R2655000: the pick that wears the full art `id` on `defId`: `true` for its Epic, as the store has always held it, and the id for a Legendary or a Mythic. */
export function fullArtPickOf(defId: string, id: string): boolean | string {
  return FULL_ART_OF_CARD.get(defId) === id ? true : id;
}

/**
 * Whether `face`'s full layout prints full art: a full-art card's (`isFullArtCard`) while the Full-art
 * cards setting is on. A compact face and a board minion never do (CardFace, MinionFace).
 */
export function showsFullArt(face: FaceModel, fullArtOn: boolean = readCardSettings().fullArt): boolean {
  return fullArtOn && isFullArtCard(face.defId);
}

const FullArtAllowedContext = createContext(true);

/** R1108000: the faces below it print full art only when `allowed` (practice.tsx: not in a lesson). */
export function FullArtAllowed({ allowed, children }: { allowed: boolean; children?: ReactNode }): ReactElement {
  return createElement(FullArtAllowedContext.Provider, { value: allowed }, children);
}

/**
 * R1313000: what the faces below a `FullArtOwned` own: `"every"`, or the cosmetics the viewer owns
 * by card. R1655000: not the full arts alone, every per-card cosmetic.
 */
export type FullArtOwnership = CardOwnership;

/** R1313000: no cosmetic owned past the defaults and starters: an account's while its ownership is unread, or a pending one's. */
export const NO_FULL_ART: OwnedByCard = new Map();

/** R1310004: the localStorage key whose "1", set before the page loads, gates full art in a development build (e2e spec 45). */
export const E2E_FULL_ART_GATED_KEY = "jackioh.e2e.fullArtGated";

/**
 * R1313000: whether ownership decides full art: once the economy is `live`, in a production build; dev, e2e
 * and tests count every full art as owned. R1310004: unless a development build's e2e run asks (`e2eGated`).
 */
export function fullArtGated(live: boolean, mode: string, e2eGated: boolean = false): boolean {
  return live && (mode === "production" || (mode === "development" && e2eGated));
}

function e2eAsksFullArtGate(): boolean {
  if (import.meta.env.MODE !== "development") return false;
  try {
    return window.localStorage.getItem(E2E_FULL_ART_GATED_KEY) === "1";
  } catch {
    // A private window or blocked site data: no e2e run asked.
    return false;
  }
}

/** R1313000: this build's `fullArtGated`. */
export const FULL_ART_GATED: boolean = fullArtGated(ECONOMY_LIVE, import.meta.env.MODE, e2eAsksFullArtGate());

/** R1313000, R1655000: the faces below it own only what `owned` holds; `"every"` with none above it. */
export function FullArtOwned({ owned, children }: { owned: FullArtOwnership; children?: ReactNode }): ReactElement {
  return createElement(OwnershipContext.Provider, { value: owned }, children);
}

const OwnCopyContext = createContext(false);

/** R1313000: the faces below it are the viewer's own copies when `own`, or when an `OwnCopy` above it says so. */
export function OwnCopy({ own, children }: { own: boolean; children?: ReactNode }): ReactElement {
  const outer = useContext(OwnCopyContext);
  return createElement(OwnCopyContext.Provider, { value: own || outer }, children);
}

/** R1655000: whether the face drawn here is the viewer's own copy: what the inspect overlays and the detail view pass a `CardFace` as `worn`. */
export function useOwnCopy(): boolean {
  return useContext(OwnCopyContext);
}

/**
 * R1313000: whether a copy of `defId` may print its full art of `tier` (R2655000): any copy where every
 * cosmetic is owned, else only an own copy whose full art of that tier is owned (R1655000).
 */
export function ownsFullArt(owned: FullArtOwnership, defId: string, own: boolean, tier: FullArtTier = "Epic"): boolean {
  if (owned === "every") return true;
  const id = FULL_ARTS_OF_CARD.get(defId)?.get(tier);
  return own && id !== undefined && ownsCosmetic(owned, defId, id);
}

/** R1834000: a full-art pick shown on a card's own copies below it in place of the one the store holds. */
const FullArtShownContext = createContext<{ defId: string; pick: boolean | string } | null>(null);

/**
 * R1834000: the viewer's own copies of `defId` below it print as if its full-art pick were `pick`, without
 * the store holding it: the Collection's preview, which must write no pick (cards/cardPicksSync.ts sends each).
 */
export function FullArtShown({ defId, pick, children }: { defId: string; pick: boolean | string; children?: ReactNode }): ReactElement {
  return createElement(FullArtShownContext.Provider, { value: { defId, pick } }, children);
}

/**
 * R2655000: the tier of full art a copy of `defId` prints, or null. R1108000: none below a `FullArtAllowed` of false.
 * R1313000: where ownership decides, only an own copy (`worn`, or below an `OwnCopy`) with that tier owned.
 * R1655000: its pick (`writeCardFullArt`, a `FullArtShown`'s) decides first, the setting (Epic) otherwise. R1834004:
 * below a `TheirCopy`, its owner's loadout tier (R1834001). R2655002: a listed token reads its parent (`fullArtSource`).
 */
export function useFullArtTier(defId: string, worn: boolean = false): FullArtTier | null {
  const { fullArt } = useCardSettings();
  const allowed = useContext(FullArtAllowedContext);
  const owned = useContext(OwnershipContext);
  const ownCopy = useContext(OwnCopyContext);
  const theirs = useContext(TheirCopyContext);
  const shown = useContext(FullArtShownContext);
  const source = fullArtSource(defId);
  const stored = useCardPicks(source).fullArt;
  if (!allowed || !isFullArtCard(defId)) return null;
  const own = worn || ownCopy;
  if (!own && theirs !== null) return fullArt ? fullArtTierOf(source, theirs.cards[source]?.fullArt) : null;
  const pick = shown?.defId === source ? shown.pick : stored;
  const picked = fullArtTierOf(source, pick);
  const tier = own && (pick === false || picked !== null) ? picked : fullArt ? "Epic" : null;
  return tier !== null && ownsFullArt(owned, source, own, tier) ? tier : null;
}

/** Whether a copy of `defId` prints full art at all, at any tier (`useFullArtTier`). */
export function useFullArt(defId: string, worn: boolean = false): boolean {
  return useFullArtTier(defId, worn) !== null;
}
