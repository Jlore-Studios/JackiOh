// Full-art faces (R1017001, a follow-up to issue #1017): the most iconic cards print their art across
// the whole face and no words, as Hearthstone's Signature cards do: the cost, the rarity gem opposite
// it, the name banner at the foot and a Unit's attack and health (cards.css, "full art"). Their whole text is
// one hover or click away (inspect/Extended.tsx, the detail view's rules). R2655001: the cards that
// print one are the cosmetics registry's: a card prints full art when the registry has a `fullArt`
// entry for its card id (`FULL_ART_CARDS`), so the registry is the one list. The hero portraits' six
// cards keep theirs, and #2655 adds fifteen more, spells, field spells and a field trap among them.
// The Full-art cards setting (settings.ts `fullArt`) turns them off, and they print their text like any card.
// R1108000: a tutorial lesson's board prints no full art, whatever the setting: practice.tsx wraps it in
// `FullArtAllowed` of false, and the faces and inspect overlays below it read `useFullArt`.
// R1313000: each full art is a cosmetic its card's owner must own (`fullArt` in the cosmetics
// registry). While the server's `ECONOMY_LIVE` is on, in a production build (`FULL_ART_GATED`), a copy
// prints full art only where it is the viewer's own (`worn`, or below an `OwnCopy`: the deck builder,
// an own card's inspect overlay) and the viewer owns its full art (`FullArtOwned`, which
// ownedFullArt.ts mounts on every gated screen from `GET /api/cosmetics`). Every other copy, the
// opponent's among them, prints its normal face: the client never infers another's ownership. With
// no `FullArtOwned` above it (practice, hotseat, the Almanac, the landing page's fan), with the
// economy off, and in a development or e2e build, every full art counts as owned, as before.
// R1310004: a development build gates full art too when localStorage holds "1" under
// `E2E_FULL_ART_GATED_KEY` as it loads, which the one e2e spec that walks the economy (spec 45) sets.
//
// R1655000: a full art is a per-card pick, beside the foil, finish and border a card may carry
// (cosmetics.ts, `writeCardFullArt`): an own copy prints it by its card's pick first and by the
// Full-art cards setting where the card has none, always behind ownership. `FullArtOwned` now holds
// the viewer's ownership of every per-card cosmetic, `NO_FULL_ART` the empty one; the names stay.
//
// R2655000: every full-art card has three full arts, one of each rarity (`FullArtTier`): the Epic,
// the still face; the Legendary, animated (the art drifts in its crop and a light sweeps over it);
// and the Mythic, which moves as the Legendary does and signs the card's name across the art
// (FullArtSignature.tsx). A card's pick is `false`, `true` (its Epic, as every pick before #2655
// meant) or a full art's id (`fullArtPickOf`), and the face wears the tier it names
// (`useFullArtTier`), behind ownership of that tier; with no pick, the setting prints its Epic.
// R2655002: a token `fullArtTokens.json` lists under its parent card prints its parent's full art,
// at the parent's tier, wherever the parent's copy would: the parent's pick, the parent's ownership,
// the same setting and lesson gates (`fullArtSource`). The map is data, by card id, beside the
// registry; a token is never owned, so it has no entry of its own.
//
// R1834004: a copy that is not the viewer's own, below a `TheirCopy` (an opponent's, or any seat's on a
// watched or replayed online game), prints its full art where its owner's loadout says so for that
// card, at the tier it names, which the server checked against the owner's account (R1834001): the
// client still infers no one's ownership. The viewer's own Full-art cards setting and `FullArtAllowed`
// hold over it. A listed token reads its parent's entry, which the server sends once the parent is shown.
// R1834000: since every write of a card's pick is sent to the account, a preview writes none:
// `FullArtShown` shows a full art on an own copy in place of the card's pick, behind the same ownership.
//
// Presentation only (CLAUDE.md rule 7): the face is drawn differently and plays the same.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import { ECONOMY_LIVE } from "@jackioh/server-config";

import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { OwnershipContext, TheirCopyContext, ownsCosmetic, useCardPicks, type CardOwnership, type OwnedByCard } from "./cosmetics.ts";
import fullArtTokens from "./fullArtTokens.json";
import type { FaceModel } from "./model.ts";
import { readCardSettings, useCardSettings } from "./settings.ts";
import { isGlitch } from "./glitch.ts";
import { getStored } from "../net/storage.ts";

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
 * Whether the card `defId` has a full-art face at all (R2655001): one the registry has a full art of,
 * or a token listed under one (R2655002), never the Glitch. A Training Grounds Legacy card has an id
 * of its own (R2511000), so the Core card's full art is never its.
 */
export function isFullArtCard(defId: string): boolean {
  return !isGlitch(defId) && FULL_ART_CARDS.has(fullArtSource(defId));
}

/**
 * R2655000: the id of the full art of the card `defId` (a full-art card, not a token) that `pick`
 * names: its Epic for `true`, one of its own ids for that id; null for `false`, no pick, an id of
 * another card's or one this client does not know.
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

/** R1108000: whether the faces below it may print full art; false on a tutorial lesson's board. */
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
 * R1313000: whether ownership decides full art: once the economy is `live`, in a production build.
 * A dev or e2e build (`--mode development`) and the tests treat every full art as owned. R1310004:
 * except a development build whose e2e run asks for the gate (`e2eGated`, from
 * `E2E_FULL_ART_GATED_KEY`), as the spec that walks the economy (spec 45) does.
 */
export function fullArtGated(live: boolean, mode: string, e2eGated: boolean = false): boolean {
  return live && (mode === "production" || (mode === "development" && e2eGated));
}

/** R1310004: whether a development build's e2e run asks for the gate; never read in another build, and false when storage throws. */
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

/** R1313000: whether the faces below it are the viewer's own copies. */
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
 * R1313000: whether a copy of `defId` may print its full art of `tier` (its Epic unless named,
 * R2655000): every one may where every cosmetic is owned, else only an own copy whose full art of
 * that tier is owned (R1655000: on that card, as any cosmetic is).
 */
export function ownsFullArt(owned: FullArtOwnership, defId: string, own: boolean, tier: FullArtTier = "Epic"): boolean {
  if (owned === "every") return true;
  const id = FULL_ARTS_OF_CARD.get(defId)?.get(tier);
  return own && id !== undefined && ownsCosmetic(owned, defId, id);
}

/** R1834000: a full-art pick shown on a card's own copies below it in place of the one the store holds. */
const FullArtShownContext = createContext<{ defId: string; pick: boolean | string } | null>(null);

/**
 * R1834000: the viewer's own copies of `defId` below it print as if the card's full-art pick were
 * `pick` (`false`, `true` or a full art's id, R2655000), without the store holding it: the
 * Collection's preview of a full art, which must write no pick, since every write of one is sent to
 * the account (cards/cardPicksSync.ts).
 */
export function FullArtShown({ defId, pick, children }: { defId: string; pick: boolean | string; children?: ReactNode }): ReactElement {
  return createElement(FullArtShownContext.Provider, { value: { defId, pick } }, children);
}

/**
 * R2655000: the tier of full art a copy of `defId` prints, or null for its normal face. R1108000: none
 * below a `FullArtAllowed` of false. R1313000: none where ownership decides and it is not the viewer's
 * own (`worn`, or below an `OwnCopy`) with that tier owned. R1655000: an own copy's own pick
 * (`writeCardFullArt`) decides first, `false` or a tier it can read; the Full-art cards setting prints
 * the Epic where the card has none, and for every copy that is not the viewer's own. R1834004: a copy
 * that is not, below a `TheirCopy`, prints the tier its owner's loadout names for the card, while the
 * setting is on. Below a `FullArtShown` of the card, an own copy reads its pick from there. R2655002:
 * a listed token reads all of this off its parent card (`fullArtSource`).
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
