// `/decks`: §9.4 workshop gate and I/O (R250–R256, R341).
// Redirecting pending accounts is UX; active-only endpoints enforce the gate.
// Reads run once per profile; writes use the current token only while it remains that profile (R194, R256).
// The collection is optional: without L5 quantities, ownership is unknown.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { CardDefsProvider } from "../cards/index.ts";
import DeckWorkshop from "../game/deckbuilder/DeckWorkshop.tsx";
import { collectionFrom } from "../game/deckbuilder/loadout.ts";
import type { DeckSyncApi } from "../game/deckbuilder/sync.ts";
import { DECKBUILDER_ERROR, DECKBUILDER_LOADING } from "../game/deckbuilder/testids.ts";
import {
  ApiUnreachableError,
  deleteDeck,
  deleteTrio,
  getCatalog,
  getCollection,
  getDecks,
  importTrio,
  putDeck,
  putTrio,
  type DecksResponse,
} from "../net/api.ts";
import { useAccount } from "../net/gate.ts";
import { navigate, paths } from "../net/navigate.ts";

type Loaded = {
  catalog: CatalogSnapshot;
  collection: Collection | null;
  decks: DecksResponse;
};

type Screen =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; profileId: string; data: Loaded };

function Shell({ children }: { children: ReactNode }) {
  return (
    <div className="app-shell">
      <h1>JackiOh — decks</h1>
      {children}
    </div>
  );
}

export default function DecksRoute() {
  const account = useAccount();
  const [screen, setScreen] = useState<Screen>({ kind: "loading" });

  // §9.4 redirect visible to Cypress specs 09 and 10.
  useEffect(() => {
    if (account.kind === "anonymous") navigate(paths.login, { replace: true });
    else if (account.kind === "ready" && account.me.needsInviteCode) {
      navigate(paths.invite, { replace: true });
    }
  }, [account]);

  const token = account.kind === "ready" ? account.token : null;
  const profileId = account.kind === "ready" ? account.me.profile.id : null;
  const blocked = account.kind === "ready" && account.me.needsInviteCode;

  // Keep the token and profile together so renewals differ from account changes (R194).
  const sessionRef = useRef<{ token: string | null; profileId: string | null }>({ token, profileId });
  useLayoutEffect(() => {
    sessionRef.current = { token, profileId };
  }, [token, profileId]);

  const workshopProfile = screen.kind === "ready" ? screen.profileId : null;
  const api = useMemo<DeckSyncApi>(() => {
    const current = async (): Promise<string> => {
      const session = sessionRef.current;
      if (session.token === null || session.profileId !== workshopProfile) {
        throw new ApiUnreachableError(new Error("signed in as another account"));
      }
      return session.token;
    };
    return {
      putDeck: async (id, input) => putDeck(await current(), id, input),
      deleteDeck: async (id) => deleteDeck(await current(), id),
      putTrio: async (id, input) => putTrio(await current(), id, input),
      deleteTrio: async (id) => deleteTrio(await current(), id),
      importTrio: async (input) => importTrio(await current(), input),
    };
  }, [workshopProfile]);

  const hasToken = token !== null;
  useEffect(() => {
    if (!hasToken || profileId === null || blocked) return;
    const readToken = sessionRef.current.token ?? "";
    let cancelled = false;
    setScreen({ kind: "loading" });

    Promise.all([
      getDecks(readToken),
      getCatalog(),
      getCollection(readToken).then(
        (response) => response,
        () => null,
      ),
    ])
      .then(([decks, catalog, collection]) => {
        if (cancelled) return;
        setScreen({
          kind: "ready",
          profileId,
          data: {
            catalog: { version: catalog.version, cards: catalog.defs },
            collection: collection === null ? null : collectionFrom(collection.entries),
            decks,
          },
        });
      })
      .catch((cause: unknown) => {
        if (cancelled) return;
        setScreen({
          kind: "error",
          message: cause instanceof Error ? cause.message : String(cause),
        });
      });

    return () => {
      cancelled = true;
    };
  }, [hasToken, profileId, blocked]);

  if (account.kind === "error") {
    return (
      <Shell>
        <p className="notice" data-testid={DECKBUILDER_ERROR}>
          {account.message}
        </p>
      </Shell>
    );
  }

  if (account.kind === "loading" || account.kind === "anonymous" || blocked) {
    return (
      <Shell>
        <p data-testid={DECKBUILDER_LOADING}>Loading…</p>
      </Shell>
    );
  }

  if (account.kind === "ready" && account.me.profile.status === "banned") {
    // §9.4 has no banned screen: the server refuses reads and redemption is pending → active.
    return (
      <Shell>
        <p className="notice" data-testid={DECKBUILDER_ERROR}>
          This account cannot edit decks.
        </p>
      </Shell>
    );
  }

  if (screen.kind === "error") {
    return (
      <Shell>
        <p className="notice" data-testid={DECKBUILDER_ERROR}>
          {screen.message}
        </p>
      </Shell>
    );
  }

  if (screen.kind === "loading") {
    return (
      <Shell>
        <p data-testid={DECKBUILDER_LOADING}>Loading…</p>
      </Shell>
    );
  }

  // R279: a reference in a card's text shows the card the catalog names.
  return (
    <CardDefsProvider defs={screen.data.catalog.cards}>
      <DeckWorkshop
        // One store per profile keeps another account's mirror separate.
        key={screen.profileId}
        catalog={screen.data.catalog}
        collection={screen.data.collection}
        data={screen.data.decks}
        profileId={screen.profileId}
        api={api}
      />
    </CardDefsProvider>
  );
}
