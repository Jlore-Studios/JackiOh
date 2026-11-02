// `/decks`: §9.4 workshop gate and I/O (R250–R256, R341, R2100000's history and restore).
// Redirecting pending accounts is UX; active-only endpoints enforce the gate.
// Reads run once per profile, and again on Try again; writes use the current token only while it remains that profile (R194, R256).
// The collection is optional: without L5 quantities, ownership is unknown.
// R915005: the account's custom cards join the catalog the workshop reads (set `Custom`, drawn from
// their frames by `customDef`) and are owned once each; the validator's L7 marks a deck naming one.
// The catalog is the one the client ships (game/currentCatalog.ts, §9.4); only the decks and the collection are read.
// R1827000: and, while ownership decides, the account's cosmetics, for the portraits its picker offers.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type { CardDefs, CustomCard } from "@jackioh/shared";
import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { AccountError } from "../auth/AccountError.tsx";
import { FULL_ART_GATED, OwnCopy } from "../cards/fullArt.ts";
import { CardDefsProvider } from "../cards/index.ts";
import { customDefsOf, useCustomCards } from "../creator/useCustomCards.ts";
import { ownedPortraits, PortraitsOwned, type PortraitOwnership } from "../emotes/ownedPortraits.ts";
import { t } from "../i18n/i18n.ts";
import { currentCatalog } from "../game/currentCatalog.ts";
import DeckWorkshop from "../game/deckbuilder/DeckWorkshop.tsx";
import { collectionFrom } from "../game/deckbuilder/loadout.ts";
import type { DeckSyncApi } from "../game/deckbuilder/sync.ts";
import { DECKBUILDER_ERROR, DECKBUILDER_LOADING, DECKBUILDER_RETRY } from "../game/deckbuilder/testids.ts";
import {
  ApiUnreachableError,
  deleteDeck,
  deleteTrio,
  getCollection,
  getDeckVersions,
  getDecks,
  getOwnedCosmetics,
  importTrio,
  keptPaths,
  putDeck,
  putTrio,
  restoreDeckVersion,
  type CollectionResponse,
  type DecksResponse,
  type OwnedCosmeticsResponse,
} from "../net/api.ts";
import { useAccount } from "../net/gate.ts";
import { navigate, paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { BackLink } from "./nav.tsx";
import { messageOf } from "../net/errors.ts";

type Loaded = {
  catalog: CatalogSnapshot;
  collection: Collection | null;
  decks: DecksResponse;
  /** R1827000: the portraits the picker offers. */
  portraits: PortraitOwnership;
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

/**
 * R2125000: what the last visit read, or null on a first visit (no decks kept). The workshop opens on it
 * at once; the answer that lands behind it replaces it. A collection or cosmetics list not kept reads as
 * a failed read of it does: ownership unknown, only the starters' portraits.
 */
function keptLoaded(token: string, portraitsGated: boolean): Loaded | null {
  const decks = cachedRead<DecksResponse>(token, keptPaths.decks);
  if (decks === undefined) return null;
  const collection = cachedRead<CollectionResponse>(token, keptPaths.collection);
  const owned = cachedRead<OwnedCosmeticsResponse>(token, keptPaths.cosmetics);
  return {
    catalog: currentCatalog(),
    collection: collection === undefined ? null : collectionFrom(collection.entries),
    decks,
    portraits: portraitsGated ? ownedPortraits(owned?.account ?? []) : "every",
  };
}

/** Whether the server's decks and trios are what the workshop already holds. */
function sameDecks(a: DecksResponse, b: DecksResponse): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

type Screen =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  /** `generation` counts the workshop's reopenings (R2125000): its store is made once, from the decks it opens on. */
  | { kind: "ready"; profileId: string; data: Loaded; generation: number };

/** `settings` is off on the Loading… screens: the workshop replaces them with its own gear, so a dialog opened from theirs would close when the decks arrive (#1524). */
function Shell({ children, settings = true }: { children: ReactNode; settings?: boolean }) {
  return (
    <div className="app-shell">
      <BackLink showSettings={settings} to={paths.collection} />
      <h1>{t("JackiOh — decks")}</h1>
      {children}
    </div>
  );
}

/**
 * `portraitsGated`: whether ownership decides which portraits the picker offers (R1827000). It
 * decides here as it does for full art (`FULL_ART_GATED`, R1310004): once the economy is live in a
 * production build. Elsewhere every portrait is offered and nothing is read.
 */
export default function DecksRoute({ portraitsGated = FULL_ART_GATED }: { portraitsGated?: boolean } = {}) {
  const account = useAccount();
  const [screen, setScreen] = useState<Screen>({ kind: "loading" });
  const [attempt, setAttempt] = useState(0);

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
      deleteDeck: async (id, options) => deleteDeck(await current(), id, options),
      putTrio: async (id, input) => putTrio(await current(), id, input),
      deleteTrio: async (id, options) => deleteTrio(await current(), id, options),
      importTrio: async (input) => importTrio(await current(), input),
      getDeckVersions: async (id) => getDeckVersions(await current(), id),
      restoreDeckVersion: async (id, versionId, catalogVersion) => restoreDeckVersion(await current(), id, versionId, catalogVersion),
    };
  }, [workshopProfile]);

  const hasToken = token !== null;
  useEffect(() => {
    if (!hasToken || profileId === null || blocked) return;
    const readToken = sessionRef.current.token ?? "";
    let canceled = false;
    // R2125000: a revisit opens on the decks the last visit kept and reads again behind them.
    const kept = keptLoaded(readToken, portraitsGated);
    setScreen(kept === null ? { kind: "loading" } : { kind: "ready", profileId, data: kept, generation: 0 });

    Promise.all([
      getDecks(readToken),
      getCollection(readToken).then(
        (response) => response,
        () => null,
      ),
      // R1827000: a failed read offers only the starters (and each deck's own pick), as full art
      // shows only the defaults and starters until its read answers.
      portraitsGated
        ? getOwnedCosmetics(readToken).then(
            (owned) => ownedPortraits(owned.account),
            () => ownedPortraits([]),
          )
        : Promise.resolve<PortraitOwnership>("every"),
    ])
      .then(([decks, collection, portraits]) => {
        if (canceled) return;
        setScreen((previous) => ({
          kind: "ready",
          profileId,
          data: {
            catalog: currentCatalog(),
            collection: collection === null ? null : collectionFrom(collection.entries),
            decks,
            portraits,
          },
          // The workshop reopens on the server's decks only when they are not the ones it holds.
          generation:
            previous.kind === "ready"
              ? sameDecks(previous.data.decks, decks)
                ? previous.generation
                : previous.generation + 1
              : 0,
        }));
      })
      .catch((cause: unknown) => {
        // A read that fails behind kept decks leaves them and says nothing.
        if (canceled || kept !== null) return;
        setScreen({
          kind: "error",
          message: messageOf(cause),
        });
      });

    return () => {
      canceled = true;
    };
  }, [hasToken, profileId, blocked, attempt, portraitsGated]);

  if (account.kind === "error") {
    return (
      <Shell>
        <AccountError
          message={account.message}
          retryAfterMs={account.retryAfterMs}
          onRetry={() => {
            account.retry?.();
          }}
          testId={DECKBUILDER_ERROR}
          retryTestId={DECKBUILDER_RETRY}
        />
      </Shell>
    );
  }

  if (account.kind === "loading" || account.kind === "anonymous" || blocked) {
    return (
      <Shell settings={false}>
        <Skeleton variant="rows" label={t("Loading…")} testId={DECKBUILDER_LOADING} />
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
        <div className="row">
          <button
            type="button"
            className="button-primary"
            data-testid={DECKBUILDER_RETRY}
            onClick={() => {
              setAttempt((before) => before + 1);
            }}
          >
            {t("Try again")}
          </button>
        </div>
      </Shell>
    );
  }

  if (screen.kind === "loading") {
    return (
      <Shell settings={false}>
        <Skeleton variant="rows" label={t("Loading…")} testId={DECKBUILDER_LOADING} />
      </Shell>
    );
  }

  // R279: a reference in a card's text shows the card the catalog names.
  const { catalog, collection } = shelf ?? { catalog: screen.data.catalog, collection: screen.data.collection };
  return (
    <CardDefsProvider defs={catalog.cards}>
      {/* R1313000: every face here is the viewer's own, and prints the full arts they own. */}
      <OwnCopy own>
        {/* R1827000: the picker offers the portraits the account owns; the server refuses any other. */}
        <PortraitsOwned owned={screen.data.portraits}>
          <DeckWorkshop
            // One store per profile keeps another account's mirror separate; another generation reopens
            // it on the decks that landed behind the kept ones (R2125000).
            key={`${screen.profileId}:${String(screen.generation)}`}
            catalog={catalog}
            collection={collection}
            data={screen.data.decks}
            profileId={screen.profileId}
            api={api}
          />
        </PortraitsOwned>
      </OwnCopy>
    </CardDefsProvider>
  );
}
