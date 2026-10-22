// Full-art faces (R1017001, a follow-up to issue #1017): the most iconic cards print their art across
// the whole face and no words, as Hearthstone's Signature cards do: the cost, the rarity gem opposite
// it, the name banner at the foot and a Unit's attack and health (cards.css, "full art"). Their whole text is
// one hover or click away (inspect/Extended.tsx, the detail view's rules). They are the cards the hero
// portraits are drawn from (R641's roster, `PORTRAITS` in src/wire/emotes.ts): a card that becomes a
// hero's face becomes a full-art card, by name, as the roster names them, so a portrait the roster
// adds needs no line here. The Full-art cards setting (settings.ts `fullArt`) turns them off, and they
// print their text like any card.
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
// Presentation only (CLAUDE.md rule 7): the face is drawn differently and plays the same.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import { ECONOMY_LIVE } from "@jackioh/server-config";
import { PORTRAITS } from "@jackioh/shared";

import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { OwnershipContext, ownsCosmetic, useCardPicks, type CardOwnership, type OwnedByCard } from "./cosmetics.ts";
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
 * Whether `face`'s full layout prints full art: a hero portrait's card while the Full-art cards
 * setting is on. A compact face and a board minion never do (CardFace, MinionFace).
 */
export function showsFullArt(face: FaceModel, fullArtOn: boolean = readCardSettings().fullArt): boolean {
  return fullArtOn && !isGlitch(face.defId) && isFullArt(face.englishName ?? face.name);
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

/** Each hero portrait's card, to the id of its full art (the registry's `fullArt` entries). */
export const FULL_ART_OF_CARD: ReadonlyMap<string, string> = new Map(
  COSMETICS.flatMap((cosmetic): [string, string][] => (cosmetic.kind === "fullArt" ? [[cosmetic.card, cosmetic.id]] : [])),
);

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
 * R1313000: whether a copy of `defId` may print its full art: every one may where every cosmetic is
 * owned, else only an own copy whose full art is owned (R1655000: on that card, as any cosmetic is).
 */
export function ownsFullArt(owned: FullArtOwnership, defId: string, own: boolean): boolean {
  if (owned === "every") return true;
  const id = FULL_ART_OF_CARD.get(defId);
  return own && id !== undefined && ownsCosmetic(owned, defId, id);
}

/**
 * R1108000: whether a copy of `defId` prints full art, which is off below a `FullArtAllowed` of
 * false. R1313000: and off where ownership decides and it is not the viewer's own (`worn`, or below
 * an `OwnCopy`) with its full art owned. R1655000: an own copy's own pick (`writeCardFullArt`)
 * decides first; the Full-art cards setting decides where the card has none, and for every copy
 * that is not the viewer's own.
 */
export function useFullArt(defId: string, worn: boolean = false): boolean {
  const { fullArt } = useCardSettings();
  const allowed = useContext(FullArtAllowedContext);
  const owned = useContext(OwnershipContext);
  const ownCopy = useContext(OwnCopyContext);
  const pick = useCardPicks(defId).fullArt;
  const own = worn || ownCopy;
  return (own ? (pick ?? fullArt) : fullArt) && allowed && ownsFullArt(owned, defId, own);
}
