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
// registry). Once the server's `ECONOMY_LIVE` is on, in a production build (`FULL_ART_GATED`), a copy
// prints full art only where it is the viewer's own (`worn`, or below an `OwnCopy`: the deck builder,
// an own card's inspect overlay) and the viewer owns its full art (`FullArtOwned`, which
// ownedFullArt.ts mounts on every gated screen from `GET /api/cosmetics`). Every other copy, the
// opponent's among them, prints its normal face: the client never infers another's ownership. With
// no `FullArtOwned` above it (practice, hotseat, the Almanac, the landing page's fan), and before
// the economy is live, every full art counts as owned, as before.
//
// Presentation only (CLAUDE.md rule 7): the face is drawn differently and plays the same.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import { ECONOMY_LIVE } from "@jackioh/server-config";
import { PORTRAITS } from "@jackioh/shared";

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
 * R1313000: the full arts the faces below a `FullArtOwned` may print: `"every"`, or the ids of the
 * cards whose full art the viewer owns.
 */
export type FullArtOwnership = "every" | ReadonlySet<string>;

/** R1313000: no full art owned: an account's while its ownership is unread, or a pending one's. */
export const NO_FULL_ART: ReadonlySet<string> = new Set();

/**
 * R1313000: whether ownership decides full art: once the economy is `live`, in a production build.
 * A dev or e2e build (`--mode development`) and the tests treat every full art as owned.
 */
export function fullArtGated(live: boolean, mode: string): boolean {
  return live && mode === "production";
}

/** R1313000: this build's `fullArtGated`. */
export const FULL_ART_GATED: boolean = fullArtGated(ECONOMY_LIVE, import.meta.env.MODE);

/** R1313000: the faces below it print only the full arts it holds; `"every"` with none above it. */
const FullArtOwnershipContext = createContext<FullArtOwnership>("every");

/** R1313000: the faces below it print the full arts of `owned`, and only on the viewer's own copies. */
export function FullArtOwned({ owned, children }: { owned: FullArtOwnership; children?: ReactNode }): ReactElement {
  return createElement(FullArtOwnershipContext.Provider, { value: owned }, children);
}

/** R1313000: whether the faces below it are the viewer's own copies. */
const OwnCopyContext = createContext(false);

/** R1313000: the faces below it are the viewer's own copies when `own`, or when an `OwnCopy` above it says so. */
export function OwnCopy({ own, children }: { own: boolean; children?: ReactNode }): ReactElement {
  const outer = useContext(OwnCopyContext);
  return createElement(OwnCopyContext.Provider, { value: own || outer }, children);
}

/** R1313000: whether a copy of `defId` may print its full art: every one may, else only an own copy whose full art is owned. */
export function ownsFullArt(owned: FullArtOwnership, defId: string, own: boolean): boolean {
  return owned === "every" || (own && owned.has(defId));
}

/**
 * R1108000: the Full-art cards setting, off below a `FullArtAllowed` of false. R1313000: and, for a
 * copy of `defId`, off where ownership decides and it is not the viewer's own (`worn`, or below an
 * `OwnCopy`) with its full art owned.
 */
export function useFullArt(defId: string, worn: boolean = false): boolean {
  const { fullArt } = useCardSettings();
  const allowed = useContext(FullArtAllowedContext);
  const owned = useContext(FullArtOwnershipContext);
  const ownCopy = useContext(OwnCopyContext);
  return fullArt && allowed && ownsFullArt(owned, defId, worn || ownCopy);
}
