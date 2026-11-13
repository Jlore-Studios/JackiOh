// `/collection`: everything the account owns and makes (SPEC §14, #1307, #1579, R1579000; issue #3734, R3734000): four
// tabs, Cards, Decks, Cosmetics and Progress, by `?section=` (Cards when it names none), over the whole window.
// The route holds the Cards and Cosmetics tabs' search, filters and sort, so they survive a switch of tab. Gated like
// `/account` (main.tsx). Every panel renders what the server answered and decides nothing (CLAUDE.md rule 7); each
// reads for itself, so one refused read never blanks the others (#1691). Until the economy goes live everyone owns
// every card (§14.12). R3335001: the Fleet track's panel is the tracks' read's `fleet`, shown once that answer
// holds one. R3335000: the Cosmetics tab's Starship tab equips the looks every Starship the account launches wears.

import { useEffect, useState, type ReactElement } from "react";

import type { FleetTrackView } from "../wire/generated/FleetTrackView.ts";

import { AccountError } from "../auth/AccountError.tsx";
import DeckTiles from "../collection/DeckTiles.tsx";
import OwnedCards from "../collection/OwnedCards.tsx";
import TracksPanel from "../collection/TracksPanel.tsx";
import WalletPanel from "../collection/WalletPanel.tsx";
import AccountCosmeticTiles from "../collection/AccountCosmeticTiles.tsx";
import FleetTrackPanel from "../collection/FleetTrackPanel.tsx";
import StarshipLooks from "../collection/StarshipLooks.tsx";
import {
  COLLECTION_SECTIONS,
  DEFAULT_CARDS_BROWSE,
  collectionSectionOf,
  collectionTestid,
  type CardsBrowse,
  type CollectionPart,
  type CollectionSection,
} from "../collection/collection.ts";
import CosmeticsFilters from "../collection/CosmeticsFilters.tsx";
import { DEFAULT_COSMETICS_BROWSE, type CosmeticsBrowse } from "../collection/cosmeticsBrowse.ts";
import { t } from "../i18n/i18n.ts";
import {
  getCollection,
  getCosmeticPacks,
  getDecks,
  getPacks,
  getTracks,
  getWallet,
  keptPaths,
  type CollectionResponse,
  type CosmeticPacksResponse,
  type DecksResponse,
  type PacksResponse,
  type TracksResponse,
  type WalletResponse,
} from "../net/api.ts";
import { readFailureOf, type ReadFailure } from "../net/gate.ts";
import { navigate, paths, useSearch } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { Tabs } from "../tabs/Tabs.tsx";
import { BackLink } from "./nav.tsx";
import QuestsPanel from "./QuestsPanel.tsx";

import "../auth/tavern.css";
import "../game/deckbuilder/deckbuilder.css";
import "./account.css";
import "../collection/collection.css";
import "../collection/browse.css";

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
const keptDecks = (token: string): DecksResponse | undefined => cachedRead<DecksResponse>(token, keptPaths.decks);

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
  className,
  children,
}: {
  part: CollectionPart;
  title: string;
  loading: string;
  read: Read<T>;
  className?: string;
  children: (value: T) => ReactElement;
}): ReactElement {
  return (
    <section className={className === undefined ? "collection-section" : `collection-section ${className}`}>
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

/** R3335000: the Cosmetics tab's tabs: the board's slots, and the Starship's. */
type CosmeticsTab = "board" | "starship";

export default function CollectionRoute({ token }: { token: string }): ReactElement {
  const section = collectionSectionOf(useSearch());
  // Held here, not in a tab, so a switch of tab leaves them as they are.
  const [cardsBrowse, setCardsBrowse] = useState(DEFAULT_CARDS_BROWSE);
  const [cosmeticsBrowse, setCosmeticsBrowse] = useState<CosmeticsBrowse>(DEFAULT_COSMETICS_BROWSE);
  const [cosmeticsTab, setCosmeticsTab] = useState<CosmeticsTab>("board");

  return (
    <div
      className="app-shell app-shell--wide tavern deckbuilder collection-screen one-screen"
      data-testid={collectionTestid.screen}
      data-section={section}
    >
      <BackLink to={paths.landing} title={t("Your collection")} />
      <Tabs<CollectionSection>
        label={t("Collection")}
        idPrefix="collection"
        className="collection-tabs"
        tablistClassName="collection-tablist"
        panelsClassName="collection-panels"
        value={section}
        onChange={(next) => {
          navigate(paths.collectionSection(next), { replace: true });
        }}
        items={COLLECTION_SECTIONS.map((id) => ({
          id,
          label: sectionLabel(id),
          testId: collectionTestid.tab(id),
          panelClassName: "collection-panel",
          panel:
            id === "cards" ? (
              <CardsSection token={token} browse={cardsBrowse} onBrowse={setCardsBrowse} />
            ) : id === "decks" ? (
              <DecksSection token={token} />
            ) : id === "cosmetics" ? (
              <CosmeticsSection token={token} browse={cosmeticsBrowse} onBrowse={setCosmeticsBrowse} tab={cosmeticsTab} onTab={setCosmeticsTab} />
            ) : (
              <ProgressSection token={token} />
            ),
        }))}
      />
    </div>
  );
}

/** Each text is its own literal, so the translation guard sees it. */
function sectionLabel(section: CollectionSection): string {
  switch (section) {
    case "cards":
      return t("Cards");
    case "decks":
      return t("Decks");
    case "cosmetics":
      return t("Cosmetics");
    case "progress":
      return t("Progress");
  }
}

/** R3335001: the Fleet track's part of the tracks' read: ready once the answer holds it, else none (a server from before #3335). */
function fleetOf(tracks: Read<TracksResponse>): Read<FleetTrackView> | null {
  return tracks.kind === "ready" && tracks.value.fleet !== undefined ? { kind: "ready", value: tracks.value.fleet } : null;
}

/** The whole catalog as one grid, owned as the collection says. */
function CardsSection({
  token,
  browse,
  onBrowse,
}: {
  token: string;
  browse: CardsBrowse;
  onBrowse: (next: CardsBrowse) => void;
}): ReactElement {
  const cards = useRead(token, getCollection, keptCollection);
  return (
    <Section part="cards" title={t("Owned cards")} loading={t("Loading your cards…")} read={cards} className="collection-section--fill">
      {(collection) => <OwnedCards collection={collection} token={token} browse={browse} onBrowse={onBrowse} />}
    </Section>
  );
}

/** Issue #3734: the decks as tiles, each with the validator's verdict on it. A collection that cannot be read leaves ownership unknown. */
function DecksSection({ token }: { token: string }): ReactElement {
  const decks = useRead(token, getDecks, keptDecks);
  const cards = useRead(token, getCollection, keptCollection);
  return (
    <div className="collection-body" data-testid={collectionTestid.body}>
      <Section part="decks" title={t("Your decks")} loading={t("Loading your decks…")} read={decks}>
        {(value) => <DeckTiles data={value} collection={cards.kind === "ready" ? cards.value : null} />}
      </Section>
    </div>
  );
}

/** Issue #3734: the wallet, the set tracks, the Fleet track and the quests, out from under the card list. */
function ProgressSection({ token }: { token: string }): ReactElement {
  const wallet = useRead(token, readWallet, keptWallet);
  const tracks = useRead(token, getTracks, keptTracks);
  const fleet = fleetOf(tracks);

  return (
    <div className="collection-body" data-testid={collectionTestid.body}>
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
  );
}

/**
 * What is equipped, the only place to pick it (R1579000). R1655000: a card's own picks are set from the
 * card in the Cards section. R1953001: a card with no pick of its own wears its kind's default, and a
 * locked tile offers a held cosmetic token in place. R2731000: no list of what the account owns, which
 * the tiles and the cards already show.
 */
function CosmeticsSection({
  token,
  browse,
  onBrowse,
  tab,
  onTab,
}: {
  token: string;
  browse: CosmeticsBrowse;
  onBrowse: (next: CosmeticsBrowse) => void;
  tab: CosmeticsTab;
  onTab: (next: CosmeticsTab) => void;
}): ReactElement {
  return (
    <div className="collection-cards collection-cards--cosmetics">
      <CosmeticsFilters browse={browse} onBrowse={onBrowse} />
      <div className="collection-scroll" data-testid={collectionTestid.body}>
        <section className="collection-section">
          <h2 className="account-heading">{t("Equipped")}</h2>
          <Tabs
            label={t("Cosmetics")}
            idPrefix="collection-cosmetics"
            value={tab}
            onChange={onTab}
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
                    <AccountCosmeticTiles token={token} browse={browse} />
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
                    <StarshipLooks browse={browse} />
                  </>
                ),
              },
            ]}
          />
        </section>
      </div>
    </div>
  );
}
