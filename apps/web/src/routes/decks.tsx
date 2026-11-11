// `/decks`: §9.4 workshop gate and I/O (R250–R256, R341, R2100000's history and restore). Redirecting
// pending accounts is UX; active-only endpoints enforce the gate. Writes use the current token only while
// it remains that profile (R194, R256). Without the collection's L5 quantities, ownership is unknown.

import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { AccountError } from "../auth/AccountError.tsx";
import { FULL_ART_GATED, OwnCopy } from "../cards/fullArt.ts";
import { CardDefsProvider } from "../cards/index.ts";
import { ownedPortraits, PortraitsOwned, type PortraitOwnership } from "../emotes/ownedPortraits.ts";
import { t } from "../i18n/i18n.ts";
import { currentCatalog } from "../game/currentCatalog.ts";
import DeckWorkshop from "../game/deckbuilder/DeckWorkshop.tsx";
import { collectionFrom } from "../game/deckbuilder/loadout.ts";
import type { DeckSyncApi } from "../game/deckbuilder/sync.ts";
import { DECKBUILDER_ERROR, DECKBUILDER_LOADING, DECKBUILDER_RETRY } from "../game/deckbuilder/testids.ts";
import { WorkshopFrame } from "../game/deckbuilder/WorkshopFrame.tsx";
import {
  ApiUnreachableError,
  getCollection,
  getDeckVersions,
  getDecks,
  getOwnedCosmetics,
  importTrio,
  keptPaths,
  restoreDeckVersion,
  syncDecks,
  type CollectionResponse,
  type DecksResponse,
  type MeResponse,
  type OwnedCosmeticsResponse,
} from "../net/api.ts";
import { useAccount, type Account } from "../net/gate.ts";
import { navigate, paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { messageOf } from "../net/errors.ts";

type Loaded = {
  catalog: CatalogSnapshot;
  collection: Collection | null;
  decks: DecksResponse;
  portraits: PortraitOwnership;
};

/**
 * R2125000: what the last visit read, or null on a first visit. A collection or cosmetics list not kept
 * reads as a failed read of it does: ownership unknown, only the starters' portraits.
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

function sameDecks(a: DecksResponse, b: DecksResponse): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

type Screen =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  /** `generation` counts the workshop's reopenings (R2125000): its store is made once, from the decks it opens on. */
  | { kind: "ready"; profileId: string; data: Loaded; generation: number };

export type DecksRouteProps = {
  /** R3207003: the account the gate read for this screen, so it reads none of its own. */
  account?: { token: string; me: MeResponse };
  portraitsGated?: boolean;
};

/**
 * `portraitsGated`: whether ownership decides the picker's portraits (R1827000), as for full art
 * (`FULL_ART_GATED`, R1310004); otherwise every portrait is offered and nothing is read.
 * R3207003: `account` is what the gate read (`main.tsx`); without one the screen reads it itself.
 */
export default function DecksRoute({ account, portraitsGated = FULL_ART_GATED }: DecksRouteProps = {}) {
  if (account !== undefined) return <HandedDecks account={account} portraitsGated={portraitsGated} />;
  return <ReadingDecks portraitsGated={portraitsGated} />;
}

function ReadingDecks({ portraitsGated }: { portraitsGated: boolean }) {
  return <DecksScreen account={useAccount()} portraitsGated={portraitsGated} />;
}

function HandedDecks({
  account: { token, me },
  portraitsGated,
}: {
  account: { token: string; me: MeResponse };
  portraitsGated: boolean;
}) {
  const account = useMemo<Account>(() => ({ kind: "ready", token, me }), [token, me]);
  return <DecksScreen account={account} portraitsGated={portraitsGated} />;
}

/**
 * R3228000: every state of `/decks` (the account's read, the decks' read, an error, a ban) draws the
 * workshop's own frame, gear included, so the bar and the title stay where they are when the decks land
 * and only the body changes. A settings dialog opened during the wait closes when they do (#1524).
 */
function DecksScreen({ account, portraitsGated }: { account: Account; portraitsGated: boolean }) {
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
      syncDecks: async (input, options) => syncDecks(await current(), input, options),
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
    const kept = keptLoaded(readToken, portraitsGated);
    setScreen(kept === null ? { kind: "loading" } : { kind: "ready", profileId, data: kept, generation: 0 });

    Promise.all([
      getDecks(readToken),
      getCollection(readToken).then(
        (response) => response,
        () => null,
      ),
      // R1827000: a failed read offers only the starters (and each deck's own pick).
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
          // The workshop reopens only on decks that differ from the ones it holds.
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
      <WorkshopFrame>
        <AccountError
          message={account.message}
          retryAfterMs={account.retryAfterMs}
          onRetry={() => {
            account.retry?.();
          }}
          testId={DECKBUILDER_ERROR}
          retryTestId={DECKBUILDER_RETRY}
        />
      </WorkshopFrame>
    );
  }

  if (account.kind === "loading" || account.kind === "anonymous" || blocked) {
    return (
      <WorkshopFrame>
        <Skeleton variant="rows" label={t("Loading…")} testId={DECKBUILDER_LOADING} />
      </WorkshopFrame>
    );
  }

  if (account.kind === "ready" && account.me.profile.status === "banned") {
    // §9.4 has no banned screen: the server refuses reads and redemption is pending → active.
    return (
      <WorkshopFrame>
        <p className="notice" data-testid={DECKBUILDER_ERROR}>
          {t("This account cannot edit decks.")}
        </p>
      </WorkshopFrame>
    );
  }

  if (screen.kind === "error") {
    return (
      <WorkshopFrame>
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
      </WorkshopFrame>
    );
  }

  if (screen.kind === "loading") {
    return (
      <WorkshopFrame>
        <Skeleton variant="rows" label={t("Loading…")} testId={DECKBUILDER_LOADING} />
      </WorkshopFrame>
    );
  }

  // R279: a reference in a card's text shows the card the catalog names.
  const { catalog, collection } = screen.data;
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
