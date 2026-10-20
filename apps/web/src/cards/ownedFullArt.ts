// R1313000: the full arts the signed-in viewer owns, read from `GET /api/cosmetics` once per session
// and mounted over a gated screen (main.tsx `Gated`) as `FullArtOwned`, so the viewer's own copies
// print the full arts they own (fullArt.ts). Only while ownership decides (`FULL_ART_GATED`, the
// economy live in a production build): before that the screen holds every full art, as before, and
// nothing is read. While the read is pending, or when it fails, nothing counts as owned. The answer
// is the caller's own, keyed on its token by the server; nobody else's ownership is read or guessed
// (CLAUDE.md rule 7).

import { createElement, useEffect, useState, type ReactElement, type ReactNode } from "react";

import { sessionIdFromToken } from "../auth/redirect.ts";
import { getOwnedCosmetics, type OwnedCosmeticsResponse } from "../net/api.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { FULL_ART_GATED, FullArtOwned, NO_FULL_ART } from "./fullArt.ts";

/** Each full art's card, to its cosmetic id (the registry's `fullArt` entries). */
const FULL_ART_OF_CARD: ReadonlyMap<string, string> = new Map(
  COSMETICS.flatMap((cosmetic): [string, string][] => (cosmetic.kind === "fullArt" ? [[cosmetic.card, cosmetic.id]] : [])),
);

/** The cards whose own full art `owned` holds: any other per-card cosmetic, and every account one, is not one. */
export function ownedFullArtCards(owned: OwnedCosmeticsResponse): ReadonlySet<string> {
  return new Set(owned.cards.filter((entry) => FULL_ART_OF_CARD.get(entry.cardId) === entry.cosmeticId).map((entry) => entry.cardId));
}

/** The provider session a token belongs to (its `session_id` claim), else the token itself. */
function sessionOf(token: string): string {
  return sessionIdFromToken(token) ?? token;
}

let cached: { session: string; read: Promise<ReadonlySet<string>> } | null = null;

/**
 * The full arts `token`'s account owns: one read per session, kept for the page (a renewed token of
 * the same session reuses it), and read again for another session or after a failure.
 */
export function readOwnedFullArt(
  token: string,
  fetchOwned: (token: string) => Promise<OwnedCosmeticsResponse> = getOwnedCosmetics,
): Promise<ReadonlySet<string>> {
  const session = sessionOf(token);
  if (cached !== null && cached.session === session) return cached.read;
  const entry = { session, read: fetchOwned(token).then(ownedFullArtCards) };
  cached = entry;
  entry.read.catch(() => {
    if (cached === entry) cached = null;
  });
  return entry.read;
}

/**
 * The full arts the faces below it print: `token`'s account's own once ownership decides (`gated`),
 * none for no token (a pending account's screen) or until the read answers, and every one otherwise.
 */
export function ViewerFullArt({
  token,
  gated = FULL_ART_GATED,
  read = readOwnedFullArt,
  children,
}: {
  token: string | null;
  gated?: boolean;
  read?: (token: string) => Promise<ReadonlySet<string>>;
  children?: ReactNode;
}): ReactElement {
  const session = token === null ? null : sessionOf(token);
  const [owned, setOwned] = useState<{ session: string; cards: ReadonlySet<string> } | null>(null);
  useEffect(() => {
    if (!gated || token === null) return;
    let current = true;
    read(token).then(
      (cards) => {
        if (current) setOwned({ session: sessionOf(token), cards });
      },
      () => {
        // Nothing counts as owned: the faces print their normal face, and the next screen reads again.
      },
    );
    return () => {
      current = false;
    };
  }, [gated, token, read]);
  // Another session's answer is never this one's.
  const cards = owned !== null && owned.session === session ? owned.cards : NO_FULL_ART;
  return createElement(FullArtOwned, { owned: gated ? cards : "every" }, children);
}

export function resetOwnedFullArtForTests(): void {
  cached = null;
}
