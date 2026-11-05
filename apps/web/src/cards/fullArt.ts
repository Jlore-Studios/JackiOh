// Full-art faces (R1017001, #1017): a card with a `fullArt` entry in the cosmetics registry (R2655001, #2655) prints its
// art across the whole face, no words; the Full-art cards setting turns it off. R1313000: where ownership decides
// (`FULL_ART_GATED`) only the viewer's own copy of an owned full art prints it (`FullArtOwned`), else all count as owned.
// R2655000: three tiers (`FullArtTier`); R2655002: a token in `fullArtTokens.json` prints its parent's. CLAUDE.md rule 7.
// R2723000: in the viewer's own hand it keeps only the cost gem and a Unit's attack and health (CardFace's `ownHand`).
// R2715000: a fourth, the Animated art (`animated-<slug>`, tier "Animated"), prints the normal face with its art moving
// (R2715001) and no full art; a pick of `true` still names the still Epic.
// R2725000: a face wears a look (versions.ts, `useCardLook`). The 21 still print full art by the setting alone; any
// other card wears a version only by its own pick. A retired full art's pick looks as it did (`tierOfLook` gives its
// tier back). R2940001: Alt full art prints the full-art layout as Full art does (`isFullBleed`), on any card it is
// offered. R3015005: the look is one level, the one a pick names (`levelOfPick`, an older shape mapped as the server
// maps it), worn where owned and offered, else the highest below it that is (`wornLevel`); R3015002: every full-art
// level prints the full-art layout (`wearsFullArt`, `isFullBleed`).

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import { ECONOMY_LIVE } from "@jackioh/server-config";

import { COSMETICS, COSMETIC_LEVELS } from "../wire/generated/cosmetics.ts";
import { FULL_ART_MOTION, artMotionFor, type ArtMotion } from "./art/motion.ts";
import {
  ANIMATED_ART_ID_PREFIX,
  OwnershipContext,
  TheirCopyContext,
  ownsCosmetic,
  useCardPicks,
  type CardOwnership,
  type Cosmetic,
  type OwnedByCard,
} from "./cosmetics.ts";
import fullArtTokens from "./fullArtTokens.json";
import type { FaceModel } from "./model.ts";
import { readCardSettings, useCardSettings } from "./settings.ts";
import { isGlitch } from "./glitch.ts";
import { getStored } from "../net/storage.ts";
import {
  NORMAL_LOOK,
  VERSION_ANIMATED_ID,
  VERSION_FULL_ART_ID,
  isFullBleed,
  levelOfPick,
  lookOfLevel,
  offeredOn,
  wornLevel,
  type CardLook,
} from "./versions.ts";

/**
 * R2655000: a full art's tier, the cosmetics' own rarity scale's: the still Epic, the animated
 * Legendary, the signed Mythic; R2715000: and the Animated art, an Epic that prints the normal face.
 */
export type FullArtTier = "Epic" | "Legendary" | "Mythic" | "Animated";

/** R2655000, R2715000: the tiers, as the registry lists each card's full arts (the Animated art appended last). */
export const FULL_ART_TIERS: readonly FullArtTier[] = ["Epic", "Legendary", "Mythic", "Animated"];

/** R2655000: the tiers that print full art, the face's whole layout. */
export const FULL_ART_LAYOUT_TIERS: readonly FullArtTier[] = ["Epic", "Legendary", "Mythic"];

/**
 * R2655000, R2715000: the tier a registry entry is: "Animated" for an Epic `animated-` full art, its
 * rarity for any other full art of a layout tier, and null for anything else.
 */
export function fullArtTierOfEntry(cosmetic: Cosmetic): FullArtTier | null {
  if (cosmetic.kind !== "fullArt") return null;
  if (cosmetic.id.startsWith(ANIMATED_ART_ID_PREFIX)) return cosmetic.rarity === "Epic" ? "Animated" : null;
  return (FULL_ART_LAYOUT_TIERS as readonly string[]).includes(cosmetic.rarity) ? (cosmetic.rarity as FullArtTier) : null;
}

/** R2655000: each full-art card's full arts, by tier, from the registry's `fullArt` entries. */
export const FULL_ARTS_OF_CARD: ReadonlyMap<string, ReadonlyMap<FullArtTier, string>> = (() => {
  const out = new Map<string, Map<FullArtTier, string>>();
  for (const cosmetic of COSMETICS) {
    const tier = fullArtTierOfEntry(cosmetic);
    if (tier === null || cosmetic.kind !== "fullArt") continue;
    const tiers = out.get(cosmetic.card) ?? new Map<FullArtTier, string>();
    tiers.set(tier, cosmetic.id);
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
 * R2715001: the moving art `defId` draws (motion.ts): its full-art card's, a listed token's parent's, or (R2725000) a
 * borrowed one on a card offered an animated level (R3015000: the Default's, `artMotionFor`); none otherwise.
 */
export function artMotionOf(defId: string): ArtMotion | null {
  const source = fullArtSource(defId);
  return FULL_ART_MOTION[source] ?? (offeredOn(VERSION_ANIMATED_ID, source) ? artMotionFor(source) : null);
}

/**
 * Whether `defId` has a full-art face (R2655001): a registry full art or a listed token's (R2655002), never
 * the Glitch. A Training Grounds Legacy card has its own id (R2511000), so the Core card's art is never its.
 * R2725000: these are the cards the Full-art cards setting prints full art on with no pick of their own.
 */
export function isFullArtCard(defId: string): boolean {
  return !isGlitch(defId) && FULL_ART_CARDS.has(fullArtSource(defId));
}

/** R3015002: the levels that add the full-art frame: Full art's first, Alt art's and the Sketch's second. */
const FULL_ART_LEVELS: readonly string[] = Object.entries(COSMETIC_LEVELS)
  .filter(([, level]) => level.adds === "fullArt")
  .map(([id]) => id);

/**
 * R2725000: whether a copy of `defId` may wear full art at all: a full-art card, or one offered a level that adds the
 * full-art frame (R2940001: Alt full art; R3015002: the Sketch's full art), every level above it wearing it too;
 * never the Glitch.
 */
export function wearsFullArt(defId: string): boolean {
  const source = fullArtSource(defId);
  return isFullArtCard(defId) || (!isGlitch(defId) && FULL_ART_LEVELS.some((id) => offeredOn(id, source)));
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

/** R2655000: the pick that wears the full art `id` on `defId`: `true` for its Epic, as the store has always held it, and the id for a Legendary, a Mythic or (R2715000) an Animated art. */
export function fullArtPickOf(defId: string, id: string): boolean | string {
  return FULL_ART_OF_CARD.get(defId) === id ? true : id;
}

/**
 * Whether `face`'s full layout prints full art: with no `fullArtOn`, a full-art card's (`isFullArtCard`) while the
 * Full-art cards setting is on; R2725000: given the caller's `useFullArt(defId)`, any card that may wear it
 * (`wearsFullArt`). A compact face and a board minion never do (CardFace, MinionFace).
 */
export function showsFullArt(face: FaceModel, fullArtOn?: boolean): boolean {
  return fullArtOn === undefined ? readCardSettings().fullArt && isFullArtCard(face.defId) : fullArtOn && wearsFullArt(face.defId);
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
  // A private window or blocked site data: no e2e run asked.
  return getStored(E2E_FULL_ART_GATED_KEY) === "1";
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

/** R1834000: a full-art pick (R2725000: and augments) shown on a card's own copies below it in place of the store's. */
const FullArtShownContext = createContext<{ defId: string; pick: boolean | string | undefined; augments: readonly string[] | undefined } | null>(null);

/**
 * R1834000: the viewer's own copies of `defId` below it print as if its full-art pick were `pick`, without
 * the store holding it: the Collection's preview, which must write no pick (cards/cardPicksSync.ts sends each).
 * R2725000: and as if its older augments were `augments`, none when not given: the store's never layer on a shown pick
 * (the Stats drill-down gives the setting's pick alone, R3011000); with no `pick`, the setting decides the level,
 * as with no pick of the card's own. R3015005: the Collection shows a level by its id, with no augments.
 */
export function FullArtShown({
  defId,
  pick,
  augments,
  children,
}: {
  defId: string;
  pick?: boolean | string;
  augments?: readonly string[];
  children?: ReactNode;
}): ReactElement {
  return createElement(FullArtShownContext.Provider, { value: { defId, pick, augments } }, children);
}

/**
 * R2725000, R3015005: what a copy of `defId` wears: one level's look (`lookOfLevel`). NORMAL_LOOK below a
 * `FullArtAllowed` of false (R1108000) and on the Glitch. Below a `TheirCopy` and not the viewer's own, its owner's
 * loadout entry's level (R1834004; a loadout frozen before #3015 read by `levelOfPick`) where the card is offered it,
 * which the server holds to ownership (R1834001), while the Full-art cards setting is on. The viewer's own copy (`worn`,
 * or below an `OwnCopy`): the level its pick names (a `FullArtShown`'s, else the store's; with none, `true` while the
 * setting is on, a full-art card's Full art), worn where offered and owned (R1313000, R2725001), else the highest below
 * it that is (`wornLevel`), as the server freezes it (R1834002). Any other copy wears the setting's, where every
 * cosmetic counts as owned. R1310004: with no ownership above it every level counts as owned on the cards it is
 * offered. R2655002: a listed token reads its parent's (`fullArtSource`).
 */
export function useCardLook(defId: string, worn: boolean = false): CardLook {
  const { fullArt } = useCardSettings();
  const allowed = useContext(FullArtAllowedContext);
  const owned = useContext(OwnershipContext);
  const ownCopy = useContext(OwnCopyContext);
  const theirs = useContext(TheirCopyContext);
  const shown = useContext(FullArtShownContext);
  const source = fullArtSource(defId);
  const stored = useCardPicks(source);
  if (!allowed || isGlitch(defId)) return NORMAL_LOOK;
  const own = worn || ownCopy;
  if (!own && theirs !== null) {
    if (!fullArt) return NORMAL_LOOK;
    const entry = theirs.cards[source];
    return lookOfLevel(wornLevel(levelOfPick(source, entry?.fullArt, entry?.augments), (id) => offeredOn(id, source)));
  }
  const showing = shown?.defId === source;
  const pick = own ? (showing ? shown.pick : stored.fullArt) : undefined;
  const augments = own ? ((showing ? shown.augments : stored.augments) ?? []) : [];
  const wears = (id: string): boolean => offeredOn(id, source) && (owned === "every" || (own && ownsCosmetic(owned, source, id)));
  return lookOfLevel(wornLevel(levelOfPick(source, pick ?? fullArt, augments), wears));
}

/**
 * R2725002: the tier a look printed as before #2725, the face's `data-full-art-tier`: the Full art version the Epic,
 * with Animated the Legendary and with both augments the Mythic; the normal face with Animated the Animated art; null
 * for the normal face otherwise.
 */
export function tierOfLook(look: CardLook): FullArtTier | null {
  if (look.version === VERSION_FULL_ART_ID) return look.animated ? (look.signed ? "Mythic" : "Legendary") : "Epic";
  return look.version === null && look.animated ? "Animated" : null;
}

/**
 * Whether a copy of `defId` prints full art at all (R2725000: it wears the Full art version, R2940001: or Alt full art;
 * R3015002: any full-art level or one above it), not the Animated art's normal face (R2715000).
 */
export function useFullArt(defId: string, worn: boolean = false): boolean {
  return isFullBleed(useCardLook(defId, worn).version);
}
