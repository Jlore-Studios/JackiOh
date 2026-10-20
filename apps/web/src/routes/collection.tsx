// `/collection`: the account's collection (SPEC §14, #1307): the cards it owns per shipped set, its
// wallet, its set tracks and its quests. Gated like `/account`: an anonymous visitor goes to
// `/login` (main.tsx).
//
// Every panel renders what the server answered and decides nothing (CLAUDE.md rule 7): `GET
// /api/collection`, `/api/wallet`, `/api/packs`, `/api/cosmetic-packs`, `/api/tracks` and
// `/api/quests`. Each panel reads for itself and says why in its place when its read fails, so one
// refused read never blanks the others. Until the economy goes live everyone owns every card
// (§14.12), so the screen shows a full collection.

import { useEffect, useState, type ReactElement } from "react";

import OwnedCards from "../collection/OwnedCards.tsx";
import TracksPanel from "../collection/TracksPanel.tsx";
import WalletPanel from "../collection/WalletPanel.tsx";
import { collectionTestid, type CollectionPart } from "../collection/collection.ts";
import { t } from "../i18n/i18n.ts";
import {
  getCollection,
  getCosmeticPacks,
  getPacks,
  getTracks,
  getWallet,
  type CosmeticPacksResponse,
  type PacksResponse,
  type WalletResponse,
} from "../net/api.ts";
import { BackLink } from "./nav.tsx";
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

/** One panel's read, once when it mounts, like `QuestsPanel`'s. */
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
  const cards = useRead(token, getCollection);
  const wallet = useRead(token, readWallet);
  const tracks = useRead(token, getTracks);

  return (
    <div className="app-shell tavern collection-screen" data-testid={collectionTestid.screen}>
      <BackLink />
      <section className="panel panel--auth">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Your collection")}</h2>

        <Section part="cards" title={t("Cards")} loading={t("Loading your cards…")} read={cards}>
          {(collection) => <OwnedCards collection={collection} />}
        </Section>
        <Section part="wallet" title={t("Wallet")} loading={t("Loading your wallet…")} read={wallet}>
          {(held) => <WalletPanel wallet={held.wallet} packs={held.packs} cosmeticPacks={held.cosmeticPacks} />}
        </Section>
        <Section part="tracks" title={t("Set tracks")} loading={t("Loading your tracks…")} read={tracks}>
          {(value) => <TracksPanel tracks={value.tracks} />}
        </Section>
        <QuestsPanel token={token} />
      </section>
    </div>
  );
}
