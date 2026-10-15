// `/decks`: §9.4 workshop gate and I/O (R250–R256, R341).
// Redirecting pending accounts is UX; active-only endpoints enforce the gate.
// Reads run once per profile; writes use the current token only while it remains that profile (R194, R256).
// The collection is optional: without L5 quantities, ownership is unknown.
// R915005: the account's custom cards join the catalog the workshop reads (set `Custom`, drawn from
// their frames by `customDef`) and are owned once each; the validator's L7 marks a deck naming one.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type { CardDefs, CustomCard } from "@jackioh/shared";
import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { CardDefsProvider } from "../cards/index.ts";
import { customDefsOf, useCustomCards } from "../creator/useCustomCards.ts";
import { t } from "../i18n/i18n.ts";
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

const NO_CUSTOM: readonly CustomCard[] = [];

/** R915005: the catalog and collection with the account's custom cards, each owned once. */
function withCustom(data: Loaded, customDefs: CardDefs): { catalog: CatalogSnapshot; collection: Collection | null } {
  const ids = Object.keys(customDefs);
  if (ids.length === 0) return { catalog: data.catalog, collection: data.collection };
  return {
    catalog: { ...data.catalog, cards: { ...data.catalog.cards, ...customDefs } },
    collection: data.collection === null ? null : { ...data.collection, ...Object.fromEntries(ids.map((id) => [id, 1])) },
  };
}

type Screen =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "ready"; profileId: string; data: Loaded };

function Shell({ children }: { children: ReactNode }) {
  return (
    <div className="app-shell">
      <h1>{t("JackiOh — decks")}</h1>
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
  // §12.9: an active account's custom cards, beside the catalog.
  const custom = useCustomCards(account.kind === "ready" && account.me.profile.status === "active" && !blocked ? token : null);
  const customCards = custom.state.kind === "ready" ? custom.state.cards : NO_CUSTOM;
  const customDefs = useMemo(() => customDefsOf(customCards), [customCards]);
  const readyData = screen.kind === "ready" ? screen.data : null;
  const shelf = useMemo(() => (readyData === null ? null : withCustom(readyData, customDefs)), [readyData, customDefs]);

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
          {t(account.message)}
        </p>
      </Shell>
    );
  }

  if (account.kind === "loading" || account.kind === "anonymous" || blocked) {
    return (
      <Shell>
        <p data-testid={DECKBUILDER_LOADING}>{t("Loading…")}</p>
      </Shell>
    );
  }

  if (account.kind === "ready" && account.me.profile.status === "banned") {
    // §9.4 has no banned screen: the server refuses reads and redemption is pending → active.
    return (
      <Shell>
        <p className="notice" data-testid={DECKBUILDER_ERROR}>
          {t("This account cannot edit decks.")}
        </p>
      </Shell>
    );
  }

  if (screen.kind === "error") {
    return (
      <Shell>
        <p className="notice" data-testid={DECKBUILDER_ERROR}>
          {t(screen.message)}
        </p>
      </Shell>
    );
  }

  if (screen.kind === "loading") {
    return (
      <Shell>
        <p data-testid={DECKBUILDER_LOADING}>{t("Loading…")}</p>
      </Shell>
    );
  }

  // R279: a reference in a card's text shows the card the catalog names.
  const { catalog, collection } = shelf ?? { catalog: screen.data.catalog, collection: screen.data.collection };
  return (
    <CardDefsProvider defs={catalog.cards}>
      <DeckWorkshop
        // One store per profile keeps another account's mirror separate.
        key={screen.profileId}
        catalog={catalog}
        collection={collection}
        data={screen.data.decks}
        profileId={screen.profileId}
        api={api}
      />
    </CardDefsProvider>
  );
}
