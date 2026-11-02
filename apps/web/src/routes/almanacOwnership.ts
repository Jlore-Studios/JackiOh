// R1956000: the signed-in viewer's own cards in the Almanac (almanac.tsx), drawn as their own copies
// wearing their picks, behind the ownership the gated screens mount (cards/ownedFullArt.ts).
// Presentation only (CLAUDE.md rule 7).

import { useEffect, useState } from "react";

import { sessionIdFromToken } from "../auth/redirect.ts";
import type { CardOwnership, OwnedByCard } from "../cards/cosmetics.ts";
import { FULL_ART_GATED } from "../cards/fullArt.ts";
import { readOwnedFullArt } from "../cards/ownedFullArt.ts";
import type { OwnCopies } from "../game/deckbuilder/PoolGrid.tsx";
import { getCollection, type CollectionResponse } from "../net/api.ts";

/** The cards `collection` holds a copy of. */
export function ownedCardIds(collection: CollectionResponse): ReadonlySet<string> {
  return new Set(collection.entries.filter((entry) => entry.quantity > 0).map((entry) => entry.cardId));
}

/** What `useAlmanacOwnCopies` reads with: the build's gate and the two reads, which a test replaces. */
export type AlmanacReads = {
  gated?: boolean;
  readCollection?: (token: string) => Promise<CollectionResponse>;
  readOwned?: (token: string) => Promise<OwnedByCard>;
};

function sessionOf(token: string): string {
  return sessionIdFromToken(token) ?? token;
}

/**
 * The cards `token`'s account owns, as its own copies, and the cosmetics it owns on them; null for
 * no token, until both reads answer, and after either fails. A renewed token of the same session
 * keeps the answer on screen while it reads again, as `ViewerFullArt` does.
 */
export function useAlmanacOwnCopies(
  token: string | null,
  { gated = FULL_ART_GATED, readCollection = getCollection, readOwned = readOwnedFullArt }: AlmanacReads = {},
): OwnCopies | null {
  const session = token === null ? null : sessionOf(token);
  const [read, setRead] = useState<{ session: string; own: OwnCopies } | null>(null);
  useEffect(() => {
    if (token === null) return undefined;
    let current = true;
    const owned: Promise<CardOwnership> = gated ? readOwned(token) : Promise.resolve("every");
    Promise.all([readCollection(token), owned]).then(
      ([collection, cosmetics]) => {
        if (current) setRead({ session: sessionOf(token), own: { cards: ownedCardIds(collection), owned: cosmetics } });
      },
      () => {
        // No card is the viewer's: every face draws as before, and the next visit reads again.
      },
    );
    return () => {
      current = false;
    };
  }, [token, gated, readCollection, readOwned]);
  // Another session's answer is never this one's.
  return read !== null && read.session === session ? read.own : null;
}
