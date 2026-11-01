// Full-art faces (R1017001, a follow-up to issue #1017): the hero portraits' cards print their art
// across the whole face and no words, as Hearthstone's Signature cards do (cards.css, "full art");
// their whole text is one hover or click away (inspect/Extended.tsx). A card is one by name, as
// R641's roster (`PORTRAITS` in src/wire/emotes.ts) names it, so a portrait the roster adds needs no
// line here. The Full-art cards setting (settings.ts `fullArt`) turns them off.
//
// R1313000: each full art is a cosmetic its card's owner must own. Where ownership decides
// (`FULL_ART_GATED`), a copy prints full art only where it is the viewer's own and the viewer owns it
// (`FullArtOwned`, which ownedFullArt.ts mounts on every gated screen from `GET /api/cosmetics`):
// the client never infers another's ownership. With no `FullArtOwned` above it (practice, hotseat,
// the Almanac, the landing page's fan), every full art counts as owned.
//
// Presentation only (CLAUDE.md rule 7): the face is drawn differently and plays the same.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import { ECONOMY_LIVE } from "@jackioh/server-config";
import { PORTRAITS } from "@jackioh/shared";

import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { OwnershipContext, TheirCopyContext, ownsCosmetic, useCardPicks, type CardOwnership, type OwnedByCard } from "./cosmetics.ts";
import type { FaceModel } from "./model.ts";
import { readCardSettings, useCardSettings } from "./settings.ts";
import { isGlitch } from "./glitch.ts";

/** The English names of the cards whose faces print full art: every hero portrait's card. */
export const FULL_ART_NAMES: ReadonlySet<string> = new Set(Object.values(PORTRAITS).map((portrait) => portrait.cardName));

/** Whether the card named `englishName` prints full art (a Chinese face passes its English name). */
export function isFullArt(englishName: string): boolean {
  return FULL_ART_NAMES.has(englishName);
}

/**
 * Whether the card `defId`, named `englishName` in English, has a full-art face at all: a hero
 * portrait's card, never the Glitch. A Training Grounds Legacy card is named apart from its Core card
 * (`Legacy: …`, R2511000), so the Core card's full art (R1313000) is never its.
 */
export function isFullArtCard(defId: string, englishName: string): boolean {
  return !isGlitch(defId) && isFullArt(englishName);
}

/**
 * Whether `face`'s full layout prints full art: a full-art card's (`isFullArtCard`) while the Full-art
 * cards setting is on. A compact face and a board minion never do (CardFace, MinionFace).
 */
export function showsFullArt(face: FaceModel, fullArtOn: boolean = readCardSettings().fullArt): boolean {
  return fullArtOn && isFullArtCard(face.defId, face.englishName ?? face.name);
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

/** Each hero portrait's card, to the id of its full art (the registry's `fullArt` entries). */
export const FULL_ART_OF_CARD: ReadonlyMap<string, string> = new Map(
  COSMETICS.flatMap((cosmetic): [string, string][] => (cosmetic.kind === "fullArt" ? [[cosmetic.card, cosmetic.id]] : [])),
);

/** R1310004: the localStorage key whose "1", set before the page loads, gates full art in a development build (e2e spec 45). */
export const E2E_FULL_ART_GATED_KEY = "jackioh.e2e.fullArtGated";

/**
 * R1313000: whether ownership decides full art: once the economy is `live`, in a production build.
 * A dev or e2e build (`--mode development`) and the tests treat every full art as owned. R1310004:
 * except a development build whose e2e run asks for the gate (`e2eGated`).
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
 * R1313000: whether a copy of `defId` may print its full art: every one may where every cosmetic is
 * owned, else only an own copy whose full art is owned (R1655000: on that card, as any cosmetic is).
 */
export function ownsFullArt(owned: FullArtOwnership, defId: string, own: boolean): boolean {
  if (owned === "every") return true;
  const id = FULL_ART_OF_CARD.get(defId);
  return own && id !== undefined && ownsCosmetic(owned, defId, id);
}

/** R1834000: a full-art pick shown on a card's own copies below it in place of the one the store holds. */
const FullArtShownContext = createContext<{ defId: string; on: boolean } | null>(null);

/**
 * R1834000: the viewer's own copies of `defId` below it print as if the card's full-art pick were
 * `on`, without the store holding it: the Collection's preview of a full art, which must write no pick,
 * since every write of one is sent to the account (cards/cardPicksSync.ts).
 */
export function FullArtShown({ defId, on, children }: { defId: string; on: boolean; children?: ReactNode }): ReactElement {
  return createElement(FullArtShownContext.Provider, { value: { defId, on } }, children);
}

/**
 * R1108000: whether a copy of `defId` prints full art: never below a `FullArtAllowed` of false.
 * R1313000: where ownership decides, only an own copy (`worn`, or below an `OwnCopy`) whose full art
 * is owned. R1655000: an own copy's pick (`writeCardFullArt`, or a `FullArtShown`'s) decides first,
 * the Full-art cards setting otherwise. R1834004: a copy below a `TheirCopy` prints only where its
 * owner's loadout picks it (the server checked, R1834001) and the setting is on.
 */
export function useFullArt(defId: string, worn: boolean = false): boolean {
  const { fullArt } = useCardSettings();
  const allowed = useContext(FullArtAllowedContext);
  const owned = useContext(OwnershipContext);
  const ownCopy = useContext(OwnCopyContext);
  const theirs = useContext(TheirCopyContext);
  const shown = useContext(FullArtShownContext);
  const stored = useCardPicks(defId).fullArt;
  const pick = shown?.defId === defId ? shown.on : stored;
  const own = worn || ownCopy;
  if (!own && theirs !== null) return theirs.cards[defId]?.fullArt === true && fullArt && allowed;
  return (own ? (pick ?? fullArt) : fullArt) && allowed && ownsFullArt(owned, defId, own);
}
