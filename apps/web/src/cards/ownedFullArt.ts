// R1313000: the cosmetics the signed-in viewer owns, read from `GET /api/cosmetics` once per session
// and mounted over a gated screen (main.tsx `Gated`) as `FullArtOwned`, so the viewer's own copies
// print the full arts they own (fullArt.ts). R1655000: every per-card cosmetic the ledger lists, not
// the full arts alone, so a card's own foil, finish and border follow the same ownership
// (cosmetics.ts). Only while ownership decides (`FULL_ART_GATED`, the economy live in a production
// build): before that the screen holds every cosmetic, as before, and nothing is read. While the
// read is pending, or when it fails, nothing counts as owned past the defaults and starters. The
// answer is the caller's own, keyed on its token by the server; nobody else's ownership is read or
// guessed (CLAUDE.md rule 7).

import { createElement, useEffect, useState, type ReactElement, type ReactNode } from "react";

import { sessionIdFromToken } from "../auth/redirect.ts";
import { getOwnedCosmetics, type OwnedCosmeticsResponse } from "../net/api.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { CARD_SLOT_KINDS, type OwnedByCard } from "./cosmetics.ts";
import { FULL_ART_GATED, FullArtOwned, NO_FULL_ART } from "./fullArt.ts";

/** The kinds a card carries: the three slots and the full art. The account's kinds are no card's. */
const PER_CARD_KINDS: ReadonlySet<string> = new Set([...CARD_SLOT_KINDS, "fullArt"]);

/**
 * The per-card cosmetics `owned` holds, by card: a row naming a cosmetic the registry does not know,
 * an account cosmetic, or a cosmetic made for another card is not one.
 */
export function ownedCardCosmetics(owned: OwnedCosmeticsResponse): OwnedByCard {
  const byCard = new Map<string, Set<string>>();
  for (const { cardId, cosmeticId } of owned.cards) {
    const found = COSMETICS.find((cosmetic) => cosmetic.id === cosmeticId);
    if (found === undefined || !PER_CARD_KINDS.has(found.kind) || ("card" in found && found.card !== cardId)) continue;
    const ids = byCard.get(cardId) ?? new Set<string>();
    ids.add(cosmeticId);
    byCard.set(cardId, ids);
  }
  return byCard;
}

/** The provider session a token belongs to (its `session_id` claim), else the token itself. */
function sessionOf(token: string): string {
  return sessionIdFromToken(token) ?? token;
}

let cached: { session: string; read: Promise<OwnedByCard> } | null = null;

/**
 * The per-card cosmetics `token`'s account owns: one read per session, kept for the page (a renewed
 * token of the same session reuses it), and read again for another session or after a failure.
 */
export function readOwnedFullArt(
  token: string,
  fetchOwned: (token: string) => Promise<OwnedCosmeticsResponse> = getOwnedCosmetics,
): Promise<OwnedByCard> {
  const session = sessionOf(token);
  if (cached !== null && cached.session === session) return cached.read;
  const entry = { session, read: fetchOwned(token).then(ownedCardCosmetics) };
  cached = entry;
  entry.read.catch(() => {
    if (cached === entry) cached = null;
  });
  return entry.read;
}

/**
 * Drops the session's answer, so the next screen reads what the account owns again: called where an
 * answer may have granted a per-card cosmetic (a card pack, a cosmetic pack, a token, the Chase Shard).
 */
export function forgetOwnedCosmetics(): void {
  cached = null;
}

/**
 * The cosmetics the faces below it wear: `token`'s account's own once ownership decides (`gated`),
 * none past the defaults and starters for no token (a pending account's screen) or until the read
 * answers, and every one otherwise.
 */
export function ViewerFullArt({
  token,
  gated = FULL_ART_GATED,
  read = readOwnedFullArt,
  children,
}: {
  token: string | null;
  gated?: boolean;
  read?: (token: string) => Promise<OwnedByCard>;
  children?: ReactNode;
}): ReactElement {
  const session = token === null ? null : sessionOf(token);
  const [owned, setOwned] = useState<{ session: string; cards: OwnedByCard } | null>(null);
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
