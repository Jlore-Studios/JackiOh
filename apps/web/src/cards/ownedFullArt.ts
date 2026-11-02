// R1313000: the cosmetics the signed-in viewer owns, read from `GET /api/cosmetics` once per session
// and mounted over a gated screen (main.tsx `Gated`) as `FullArtOwned`; R1655000: every per-card
// cosmetic the ledger lists, not the full arts alone (cosmetics.ts). Read only while ownership decides
// (`FULL_ART_GATED`). The answer is the caller's own, keyed on its token; nobody else's is read (CLAUDE.md rule 7).
//
// R1833000: the same answer's account list is mounted as `AccountCosmeticsOwned`
// (`ViewerAccountCosmetics`) for the playmat, backdrop and card-back pickers and the viewer's side.

import { createElement, useEffect, useState, useSyncExternalStore, type ReactElement, type ReactNode } from "react";

import { sessionIdFromToken } from "../auth/redirect.ts";
import { getOwnedCosmetics, type OwnedCosmeticsResponse } from "../net/api.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { AccountCosmeticsOwned, CARD_SLOT_KINDS, type OwnedByCard } from "./cosmetics.ts";
import { FULL_ART_GATED, FullArtOwned, NO_FULL_ART } from "./fullArt.ts";

/** The kinds a card carries; the account's kinds are no card's. */
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

/**
 * R1833000: the account cosmetics `owned` holds: a row naming a cosmetic the registry does not know,
 * or a per-card one, is not one.
 */
export function ownedAccountCosmetics(owned: OwnedCosmeticsResponse): ReadonlySet<string> {
  return new Set(
    owned.account.filter((id) => {
      const found = COSMETICS.find((cosmetic) => cosmetic.id === id);
      return found !== undefined && !PER_CARD_KINDS.has(found.kind);
    }),
  );
}

/** R1833000: no account cosmetic owned past the defaults and starters: an account's while its ownership is unread, or a pending one's. */
export const NO_ACCOUNT_COSMETICS: ReadonlySet<string> = new Set();

/** The provider session a token belongs to (its `session_id` claim), else the token itself. */
function sessionOf(token: string): string {
  return sessionIdFromToken(token) ?? token;
}

let cached: { session: string; read: Promise<OwnedCosmeticsResponse> } | null = null;

/** How many times the session's answer was dropped; a mounted provider reads again on each. */
let forgotten = 0;
const forgottenListeners = new Set<() => void>();

function subscribeForgotten(listener: () => void): () => void {
  forgottenListeners.add(listener);
  return () => {
    forgottenListeners.delete(listener);
  };
}

function forgottenCount(): number {
  return forgotten;
}

/**
 * What `token`'s account owns, as `GET /api/cosmetics` answers: one read per session, kept for the
 * page (a renewed token of the same session reuses it), and read again for another session or after
 * a failure.
 */
function readOwnedCosmetics(
  token: string,
  fetchOwned: (token: string) => Promise<OwnedCosmeticsResponse>,
): Promise<OwnedCosmeticsResponse> {
  const session = sessionOf(token);
  if (cached !== null && cached.session === session) return cached.read;
  const entry = { session, read: fetchOwned(token) };
  cached = entry;
  entry.read.catch(() => {
    if (cached === entry) cached = null;
  });
  return entry.read;
}

/** The per-card cosmetics `token`'s account owns, by card, from the session's one read. */
export function readOwnedFullArt(
  token: string,
  fetchOwned: (token: string) => Promise<OwnedCosmeticsResponse> = getOwnedCosmetics,
): Promise<OwnedByCard> {
  return readOwnedCosmetics(token, fetchOwned).then(ownedCardCosmetics);
}

export function readOwnedAccountCosmetics(
  token: string,
  fetchOwned: (token: string) => Promise<OwnedCosmeticsResponse> = getOwnedCosmetics,
): Promise<ReadonlySet<string>> {
  return readOwnedCosmetics(token, fetchOwned).then(ownedAccountCosmetics);
}

/**
 * Drops the session's answer and has every mounted `ViewerFullArt` and `ViewerAccountCosmetics` read
 * again (R1955000): called where an answer may have granted a cosmetic (a pack, a token, the Chase Shard).
 */
export function forgetOwnedCosmetics(): void {
  cached = null;
  forgotten += 1;
  for (const listener of [...forgottenListeners]) listener();
}

/**
 * R1953001: `forgetOwnedCosmetics` for a cosmetic granted on a screen that stays up (a token spent in
 * place, collection/TokenOffer.tsx), so the locked entry it bought shows owned.
 */
export function rereadOwnedCosmetics(): void {
  forgetOwnedCosmetics();
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
  const forgets = useSyncExternalStore(subscribeForgotten, forgottenCount, forgottenCount);
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
  }, [gated, token, read, forgets]);
  // Another session's answer is never this one's.
  const cards = owned !== null && owned.session === session ? owned.cards : NO_FULL_ART;
  return createElement(FullArtOwned, { owned: gated ? cards : "every" }, children);
}

/** R1833000: `ViewerFullArt` for the account cosmetics the pickers and the viewer's side below it own. */
export function ViewerAccountCosmetics({
  token,
  gated = FULL_ART_GATED,
  read = readOwnedAccountCosmetics,
  children,
}: {
  token: string | null;
  gated?: boolean;
  read?: (token: string) => Promise<ReadonlySet<string>>;
  children?: ReactNode;
}): ReactElement {
  const session = token === null ? null : sessionOf(token);
  const [owned, setOwned] = useState<{ session: string; ids: ReadonlySet<string> } | null>(null);
  const forgets = useSyncExternalStore(subscribeForgotten, forgottenCount, forgottenCount);
  useEffect(() => {
    if (!gated || token === null) return;
    let current = true;
    read(token).then(
      (ids) => {
        if (current) setOwned({ session: sessionOf(token), ids });
      },
      () => {
        // Nothing counts as owned past the starters, and the next screen reads again.
      },
    );
    return () => {
      current = false;
    };
  }, [gated, token, read, forgets]);
  const ids = owned !== null && owned.session === session ? owned.ids : NO_ACCOUNT_COSMETICS;
  return createElement(AccountCosmeticsOwned, { owned: gated ? ids : "every" }, children);
}

export function resetOwnedFullArtForTests(): void {
  cached = null;
}
