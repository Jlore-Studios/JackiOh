// `/collection`: the hub of what the account owns and makes (SPEC §14, #1307, #1579, R1579000): three
// doors. Cards and Cosmetics open in place by `?section=`, so Back and Forward move between them and
// the hub. Gated like `/account` (main.tsx). Every panel renders what the server answered and decides
// nothing (CLAUDE.md rule 7); each reads for itself, so one refused read never blanks the others (#1691).
// Until the economy goes live everyone owns every card (§14.12). R3335001: the Fleet track's panel is the tracks'
// read's `fleet`, shown once that answer holds one. R3335000: the Cosmetics section's Starship tab equips the looks
// every Starship the account launches wears.

import { useEffect, useState, type ReactElement } from "react";

import type { FleetTrackView } from "../wire/generated/FleetTrackView.ts";

import { AccountError } from "../auth/AccountError.tsx";
import OwnedCards from "../collection/OwnedCards.tsx";
import TracksPanel from "../collection/TracksPanel.tsx";
import WalletPanel from "../collection/WalletPanel.tsx";
import AccountCosmeticTiles from "../collection/AccountCosmeticTiles.tsx";
import FleetTrackPanel from "../collection/FleetTrackPanel.tsx";
import StarshipLooks from "../collection/StarshipLooks.tsx";
import { collectionSectionOf, collectionTestid, type CollectionPart } from "../collection/collection.ts";
import { t } from "../i18n/i18n.ts";
import {
  getCollection,
  getCosmeticPacks,
  getPacks,
  getTracks,
  getWallet,
  keptPaths,
  type CollectionResponse,
  type CosmeticPacksResponse,
  type PacksResponse,
  type TracksResponse,
  type WalletResponse,
} from "../net/api.ts";
import { readFailureOf, type ReadFailure } from "../net/gate.ts";
import { paths, useSearch } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { Tabs } from "../tabs/Tabs.tsx";
import { BackLink, followInApp } from "./nav.tsx";
import QuestsPanel from "./QuestsPanel.tsx";
import { Wordmark } from "./Wordmark.tsx";

import "../auth/tavern.css";
import "./account.css";
import "../collection/collection.css";

type WalletRead = { wallet: WalletResponse; packs: PacksResponse; cosmeticPacks: CosmeticPacksResponse };

async function readWallet(token: string): Promise<WalletRead> {
  const [wallet, packs, cosmeticPacks] = await Promise.all([
    getWallet(token),
    getPacks(token),
    getCosmeticPacks(token),
  ]);
  return { wallet, packs, cosmeticPacks };
}

/** R2125000: what the last visit read, for a panel to start from; undefined on a first visit. */
const keptCollection = (token: string): CollectionResponse | undefined =>
  cachedRead<CollectionResponse>(token, keptPaths.collection);
const keptTracks = (token: string): TracksResponse | undefined => cachedRead<TracksResponse>(token, keptPaths.tracks);

/** The wallet panel starts from its three answers or from none: half a panel is not one. */
function keptWallet(token: string): WalletRead | undefined {
  const wallet = cachedRead<WalletResponse>(token, keptPaths.wallet);
  const packs = cachedRead<PacksResponse>(token, keptPaths.packs);
  const cosmeticPacks = cachedRead<CosmeticPacksResponse>(token, keptPaths.cosmeticPacks);
  return wallet === undefined || packs === undefined || cosmeticPacks === undefined
    ? undefined
    : { wallet, packs, cosmeticPacks };
}

type Read<T> =
  | { kind: "loading" }
  | { kind: "ready"; value: T }
  | ({ kind: "error"; retry: () => void } & ReadFailure);

function startOf<T>(kept: (token: string) => T | undefined, token: string): Read<T> {
  const value = kept(token);
  return value === undefined ? { kind: "loading" } : { kind: "ready", value };
}

/**
 * One panel's read, when it mounts and each time Try again asks for it again. R2125000: a panel with
 * a kept answer starts on it, and a read that fails behind an answer keeps it and says nothing.
 */
function useRead<T>(
  token: string,
  load: (token: string) => Promise<T>,
  kept: (token: string) => T | undefined,
): Read<T> {
  // The read is held with the token it belongs to: after an account switch the other account's
  // answer is never drawn, and a failed read keeps only what this account's own cache holds.
  const [held, setHeld] = useState<{ token: string; read: Read<T> }>(() => ({ token, read: startOf(kept, token) }));
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let canceled = false;
    load(token)
      .then((value) => {
        if (!canceled) setHeld({ token, read: { kind: "ready", value } });
      })
      .catch((cause: unknown) => {
        if (canceled) return;
        setHeld((previous) => {
          const current = previous.token === token ? previous.read : startOf(kept, token);
          if (current.kind === "ready") return { token, read: current };
          return {
            token,
            read: {
              kind: "error",
              ...readFailureOf(cause),
              retry: () => {
                setHeld({ token, read: { kind: "loading" } });
                setAttempt((before) => before + 1);
              },
            },
          };
        });
      });
    return () => {
      canceled = true;
    };
  }, [token, load, kept, attempt]);

  return held.token === token ? held.read : startOf(kept, token);
}

/** A titled panel: its loading message, its error or its answer. */
function Section<T>({
  part,
  title,
  loading,
  read,
  children,
}: {
  part: CollectionPart;
  title: string;
  loading: string;
  read: Read<T>;
  children: (value: T) => ReactElement;
}): ReactElement {
  return (
    <section className="collection-section">
      <h2 className="account-heading">{title}</h2>
      {read.kind === "loading" ? (
        <Skeleton variant="rows" label={loading} testId={collectionTestid.loading(part)} />
      ) : read.kind === "error" ? (
        <AccountError
          message={read.message}
          retryAfterMs={read.retryAfterMs}
          onRetry={read.retry}
          testId={collectionTestid.error(part)}
          retryTestId={collectionTestid.retry(part)}
        />
      ) : (
        children(read.value)
      )}
    </section>
  );
}

export default function CollectionRoute({ token }: { token: string }): ReactElement {
  const section = collectionSectionOf(useSearch());

  return (
    <div
      className="app-shell tavern collection-screen"
      data-testid={collectionTestid.screen}
      data-section={section ?? "hub"}
    >
      <BackLink
        to={section === null ? paths.landing : paths.collection}
        title={section === null ? t("Your collection") : section === "cards" ? t("Cards") : t("Cosmetics")}
      />
      <section className="panel panel--auth">
        <Wordmark />
        {section === "cards" ? (
          <CardsSection token={token} />
        ) : section === "cosmetics" ? (
          <CosmeticsSection token={token} />
        ) : (
          <Hub />
        )}
      </section>
    </div>
  );
}

/** The three doors; each text is its own literal, so the translation guard sees it. */
function Hub(): ReactElement {
  return (
    <>
      <nav className="collection-hub" aria-label={t("Collection")}>
        <a
          className="collection-door"
          href={paths.decks}
          data-testid={collectionTestid.door("decks")}
          onClick={followInApp(paths.decks)}
        >
          <span className="collection-door__title">{t("Decks")}</span>
        </a>
        <a
          className="collection-door"
          href={paths.collectionSection("cards")}
          data-testid={collectionTestid.door("cards")}
          onClick={followInApp(paths.collectionSection("cards"))}
        >
          <span className="collection-door__title">{t("Cards")}</span>
        </a>
        <a
          className="collection-door"
          href={paths.collectionSection("cosmetics")}
          data-testid={collectionTestid.door("cosmetics")}
          onClick={followInApp(paths.collectionSection("cosmetics"))}
        >
          <span className="collection-door__title">{t("Cosmetics")}</span>
        </a>
      </nav>
    </>
  );
}

/** R3335001: the Fleet track's part of the tracks' read: ready once the answer holds it, else none (a server from before #3335). */
function fleetOf(tracks: Read<TracksResponse>): Read<FleetTrackView> | null {
  return tracks.kind === "ready" && tracks.value.fleet !== undefined ? { kind: "ready", value: tracks.value.fleet } : null;
}

/** The owned cards, the wallet, the set tracks, the Fleet track and the quests. */
function CardsSection({ token }: { token: string }): ReactElement {
  const cards = useRead(token, getCollection, keptCollection);
  const wallet = useRead(token, readWallet, keptWallet);
  const tracks = useRead(token, getTracks, keptTracks);
  const fleet = fleetOf(tracks);

  return (
    <>
      <div className="collection-body" data-testid={collectionTestid.body}>
        <Section part="cards" title={t("Owned cards")} loading={t("Loading your cards…")} read={cards}>
          {(collection) => <OwnedCards collection={collection} token={token} />}
        </Section>
        <Section part="wallet" title={t("Wallet")} loading={t("Loading your wallet…")} read={wallet}>
          {(held) => <WalletPanel wallet={held.wallet} packs={held.packs} cosmeticPacks={held.cosmeticPacks} />}
        </Section>
        <Section part="tracks" title={t("Set tracks")} loading={t("Loading your tracks…")} read={tracks}>
          {(value) => <TracksPanel tracks={value.tracks} />}
        </Section>
        {fleet === null ? null : (
          <Section part="fleet" title={t("Fleet track")} loading={t("Loading your tracks…")} read={fleet}>
            {(value) => <FleetTrackPanel fleet={value} />}
          </Section>
        )}
        <QuestsPanel token={token} />
      </div>
    </>
  );
}

/** R3335000: the Cosmetics section's tabs: the board's slots, and the Starship's. */
type CosmeticsTab = "board" | "starship";

/**
 * What is equipped, the only place to pick it (R1579000). R1655000: a card's own picks are set from the
 * card in the Cards section. R1953001: a card with no pick of its own wears its kind's default, and a
 * locked tile offers a held cosmetic token in place. R2731000: no list of what the account owns, which
 * the tiles and the cards already show.
 */
function CosmeticsSection({ token }: { token: string }): ReactElement {
  const [tab, setTab] = useState<CosmeticsTab>("board");
  return (
    <>
      <div className="collection-body" data-testid={collectionTestid.body}>
        <section className="collection-section">
          <h2 className="account-heading">{t("Equipped")}</h2>
          <Tabs
            label={t("Cosmetics")}
            idPrefix="collection-cosmetics"
            value={tab}
            onChange={setTab}
            items={[
              {
                id: "board",
                label: t("Board"),
                testId: collectionTestid.cosmeticsTab("board"),
                panel: (
                  <>
                    <p>
                      {t(
                        "Your side of the board wears these in every game. A deck can pick its own playmat, backdrop and card back in the deck builder.",
                      )}
                    </p>
                    <AccountCosmeticTiles token={token} />
                  </>
                ),
              },
              {
                id: "starship",
                label: t("Starship"),
                testId: collectionTestid.cosmeticsTab("starship"),
                panel: (
                  <>
                    <p>{t("Every Starship you launch wears these in every game. A Dormant Starship keeps its shipyard.")}</p>
                    <StarshipLooks />
                  </>
                ),
              },
            ]}
          />
        </section>
      </div>
    </>
  );
}
