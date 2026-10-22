// `/collection`: the hub of what the account owns and makes (SPEC §14, #1307, #1579, R1579000): four
// doors. Decks (`/decks`) and the card builder (`/creator`) are screens of their own, so their doors
// are links; Cards and Cosmetics open in place, by `?section=`, so the browser's Back and Forward
// move between them and the hub. Cards is the cards the account owns per shipped set, its wallet,
// its set tracks and its quests; Cosmetics is where a cosmetic is equipped, and the only place.
// Gated like `/account`: an anonymous visitor goes to `/login` (main.tsx).
//
// Every panel renders what the server answered and decides nothing (CLAUDE.md rule 7): `GET
// /api/collection`, `/api/wallet`, `/api/packs`, `/api/cosmetic-packs`, `/api/tracks`, `/api/quests`
// and `/api/cosmetics`. Each panel reads for itself and says why in its place when its read fails,
// so one refused read never blanks the others. Until the economy goes live everyone owns every
// card (§14.12), so the screen shows a full collection. The page never scrolls: a section's body does.

import { useEffect, useState, type ReactElement } from "react";

import OwnedCards from "../collection/OwnedCards.tsx";
import TracksPanel from "../collection/TracksPanel.tsx";
import WalletPanel from "../collection/WalletPanel.tsx";
import OwnedCosmetics, { CardCosmeticsControls, DeckCosmeticsControls } from "../collection/CosmeticsPanel.tsx";
import { collectionSectionOf, collectionTestid, type CollectionPart } from "../collection/collection.ts";
import { t } from "../i18n/i18n.ts";
import {
  getCollection,
  getCosmeticPacks,
  getOwnedCosmetics,
  getPacks,
  getTracks,
  getWallet,
  type CosmeticPacksResponse,
  type PacksResponse,
  type WalletResponse,
} from "../net/api.ts";
import { paths, useSearch } from "../net/navigate.ts";
import { BackLink, followInApp } from "./nav.tsx";
import QuestsPanel from "./QuestsPanel.tsx";

import "../auth/tavern.css";
import "./account.css";
import "../collection/collection.css";

/** What the wallet panel shows: the balance and the two kinds of unopened pack. */
type WalletRead = { wallet: WalletResponse; packs: PacksResponse; cosmeticPacks: CosmeticPacksResponse };

async function readWallet(token: string): Promise<WalletRead> {
  const [wallet, packs, cosmeticPacks] = await Promise.all([
    getWallet(token),
    getPacks(token),
    getCosmeticPacks(token),
  ]);
  return { wallet, packs, cosmeticPacks };
}

type Read<T> = { kind: "loading" } | { kind: "ready"; value: T } | { kind: "error"; message: string };

/** One panel's read, once when it mounts. */
function useRead<T>(token: string, load: (token: string) => Promise<T>): Read<T> {
  const [read, setRead] = useState<Read<T>>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    load(token)
      .then((value) => {
        if (!cancelled) setRead({ kind: "ready", value });
      })
      .catch((cause: unknown) => {
        if (!cancelled) setRead({ kind: "error", message: cause instanceof Error ? cause.message : String(cause) });
      });
    return () => {
      cancelled = true;
    };
  }, [token, load]);

  return read;
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
      <h3 className="account-heading">{title}</h3>
      {read.kind === "loading" ? (
        <p className="notice" role="status" data-testid={collectionTestid.loading(part)}>
          {loading}
        </p>
      ) : read.kind === "error" ? (
        <p className="notice" role="alert" data-testid={collectionTestid.error(part)}>
          {t(read.message)}
        </p>
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
      <BackLink to={section === null ? paths.landing : paths.collection} />
      <section className="panel panel--auth">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
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

/** The four doors; each text is its own literal, so the translation guard sees it. */
function Hub(): ReactElement {
  return (
    <>
      <h2>{t("Your collection")}</h2>
      <nav className="collection-hub" aria-label={t("Collection")}>
        <a
          className="collection-door"
          href={paths.decks}
          data-testid={collectionTestid.door("decks")}
          onClick={followInApp(paths.decks)}
        >
          <span className="collection-door__title">{t("Decks")}</span>
          <span className="collection-door__hint">{t("Your decks and trios, and the deck builder.")}</span>
        </a>
        <a
          className="collection-door"
          href={paths.collectionSection("cards")}
          data-testid={collectionTestid.door("cards")}
          onClick={followInApp(paths.collectionSection("cards"))}
        >
          <span className="collection-door__title">{t("Cards")}</span>
          <span className="collection-door__hint">{t("The cards you own, your wallet, set tracks and quests.")}</span>
        </a>
        <a
          className="collection-door"
          href={paths.collectionSection("cosmetics")}
          data-testid={collectionTestid.door("cosmetics")}
          onClick={followInApp(paths.collectionSection("cosmetics"))}
        >
          <span className="collection-door__title">{t("Cosmetics")}</span>
          <span className="collection-door__hint">{t("The cosmetics you own, and the ones you wear.")}</span>
        </a>
        <a
          className="collection-door"
          href={paths.creator}
          data-testid={collectionTestid.door("builder")}
          onClick={followInApp(paths.creator)}
        >
          <span className="collection-door__title">{t("Card builder")}</span>
          <span className="collection-door__hint">{t("Make your own cards and try them against the AI.")}</span>
        </a>
      </nav>
    </>
  );
}

/** The owned cards, the wallet, the set tracks and the quests. */
function CardsSection({ token }: { token: string }): ReactElement {
  const cards = useRead(token, getCollection);
  const wallet = useRead(token, readWallet);
  const tracks = useRead(token, getTracks);

  return (
    <>
      <h2>{t("Cards")}</h2>
      <div className="collection-body" data-testid={collectionTestid.body}>
        <Section part="cards" title={t("Owned cards")} loading={t("Loading your cards…")} read={cards}>
          {(collection) => <OwnedCards collection={collection} />}
        </Section>
        <Section part="wallet" title={t("Wallet")} loading={t("Loading your wallet…")} read={wallet}>
          {(held) => <WalletPanel wallet={held.wallet} packs={held.packs} cosmeticPacks={held.cosmeticPacks} />}
        </Section>
        <Section part="tracks" title={t("Set tracks")} loading={t("Loading your tracks…")} read={tracks}>
          {(value) => <TracksPanel tracks={value.tracks} />}
        </Section>
        <QuestsPanel token={token} />
      </div>
    </>
  );
}

/** What is equipped, the only place to pick it (R1579000), and what the account owns. */
function CosmeticsSection({ token }: { token: string }): ReactElement {
  const owned = useRead(token, getOwnedCosmetics);

  return (
    <>
      <h2>{t("Cosmetics")}</h2>
      <div className="collection-body" data-testid={collectionTestid.body}>
        <section className="collection-section">
          <h3 className="account-heading">{t("Equipped")}</h3>
          <p>
            {t(
              "Your own cards and your side of the board wear these in every game. A deck can pick its own playmat, backdrop and card back in the deck builder.",
            )}
          </p>
          <CardCosmeticsControls />
          <DeckCosmeticsControls />
        </section>
        <Section part="cosmetics" title={t("Owned")} loading={t("Loading your cosmetics…")} read={owned}>
          {(value) => <OwnedCosmetics owned={value} />}
        </Section>
      </div>
    </>
  );
}
