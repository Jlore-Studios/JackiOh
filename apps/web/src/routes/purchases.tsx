// `/purchases`: the odds and the pity of every pack (#1665, part of #1651, SPEC §14.4, R1665000), off
// the Shop and the pack screen. Public like `/terms` (main.tsx serves it ungated): the disclaimer shows
// to anyone, and the odds and the account's counters show to a session, which this page reads for
// itself (`readSession`), so a visitor with none makes no request and is pointed at sign-in.
//
// The client computes nothing (CLAUDE.md rule 7): every percentage is the server's, from `GET
// /api/packs` and `GET /api/cosmetic-packs`, only formatted (`tierPercents`), the fallback tier
// included, and so is every counter and threshold. A read that fails says why in its place.

import { useEffect, useState, type ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { getCosmeticPacks, getPacks, type CosmeticPacksResponse, type PacksResponse } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import { ODDS_RARITIES, PITY_RARITIES, percent, tierPercents } from "../packs/packs.ts";
import { BackLink, followInApp } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";
import "./purchases.css";

export const purchasesTestid = {
  screen: "purchases-screen",
  disclaimer: "purchases-disclaimer",
  signIn: "purchases-sign-in",
  loading: (place: PurchasesPlace) => `purchases-${place}-loading`,
  loadError: (place: PurchasesPlace) => `purchases-${place}-load-error`,
  cardPacks: "purchases-card-packs",
  odds: "purchases-odds",
  oddsRow: (key: string) => `purchases-odds-${key}`,
  duplicate: "purchases-duplicate",
  globalCosmetic: "purchases-global-cosmetic",
  cosmeticPacks: "purchases-cosmetic-packs",
  cosmeticSlot: "purchases-cosmetic-slot",
  cosmeticFloored: "purchases-cosmetic-floored",
  pity: "purchases-pity",
  pityRow: (key: string) => `purchases-pity-${key}`,
  pityLine: "purchases-pity-line",
} as const;

/** Where a read's wait or failure shows: the card packs' and the pity's both read `GET /api/packs`. */
export type PurchasesPlace = "card-packs" | "cosmetic-packs" | "pity";

/** A read as the page shows it: waiting, answered, or refused with the words to show. */
type Read<T> = { kind: "loading" } | { kind: "ready"; value: T } | { kind: "failed"; message: string };

/** What a failed request says: the server's words, else the error's. */
function wordsOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/** `read(token)` once the page has a token; never, without one. */
function useRead<T>(token: string | null, read: (token: string) => Promise<T>): Read<T> {
  const [state, setState] = useState<Read<T>>({ kind: "loading" });
  useEffect(() => {
    if (token === null) return;
    let cancelled = false;
    read(token).then(
      (value) => {
        if (!cancelled) setState({ kind: "ready", value });
      },
      (cause: unknown) => {
        if (!cancelled) setState({ kind: "failed", message: wordsOf(cause) });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [token, read]);
  return state;
}

/** `ready` where the read answered, else what it is waiting for or why it failed, in the same place. */
function Answered<T>({
  read,
  place,
  loading,
  children,
}: {
  read: Read<T>;
  place: PurchasesPlace;
  loading: string;
  children: (value: T) => ReactElement;
}): ReactElement {
  switch (read.kind) {
    case "loading":
      return (
        <p className="notice" role="status" data-testid={purchasesTestid.loading(place)}>
          {loading}
        </p>
      );
    case "failed":
      return (
        <p className="notice" role="alert" data-testid={purchasesTestid.loadError(place)}>
          {read.message}
        </p>
      );
    case "ready":
      return children(read.value);
  }
}

/** A slot's odds and the guaranteed slot's, a rarity to a row, as `GET /api/packs` states them (R1339000). */
function SlotOdds({ odds }: { odds: PacksResponse["odds"] }): ReactElement {
  return (
    <table className="purchases-odds" data-testid={purchasesTestid.odds}>
      <thead>
        <tr>
          <th scope="col">{t("Rarity")}</th>
          <th scope="col">{t("Each slot")}</th>
          <th scope="col">{t("Guaranteed slot")}</th>
        </tr>
      </thead>
      <tbody>
        {ODDS_RARITIES.map(({ key, rarity }) => (
          <tr key={key} data-testid={purchasesTestid.oddsRow(key)}>
            <th scope="row">{t(rarity)}</th>
            <td>{percent(odds.slot[key])}</td>
            <td>{percent(odds.flooredSlot[key])}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** What a duplicate becomes: a cosmetic of the table's rarity, or, on the global roll, an account cosmetic (R1661001). */
function DuplicateOdds({ odds }: { odds: PacksResponse["odds"] }): ReactElement {
  const { chance, tiers } = odds.globalCosmetic;
  const global = tierPercents(tiers);
  return (
    <>
      <p data-testid={purchasesTestid.duplicate}>
        {t(
          "A duplicate becomes a cosmetic: Common {common}, Rare {rare}, Epic {epic}, Legendary {legendary}, Mythic {mythic}. With none of that rarity left to own, it fills both token bars.",
          tierPercents(odds.duplicateCosmetic),
        )}
      </p>
      <p data-testid={purchasesTestid.globalCosmetic}>
        {t(
          "While an account cosmetic is left to own, a duplicate becomes one {chance} of the time: Epic {epic}, Legendary {legendary}, Mythic {mythic}.",
          { chance: percent(chance), epic: global.epic, legendary: global.legendary, mythic: global.mythic },
        )}
      </p>
    </>
  );
}

/** A cosmetic pack's slot and its guaranteed slot, as `GET /api/cosmetic-packs` states them (R1339000). */
function CosmeticOdds({ odds }: { odds: CosmeticPacksResponse["odds"] }): ReactElement {
  return (
    <>
      <p data-testid={purchasesTestid.cosmeticSlot}>
        {t(
          "Each slot: Common {common}, Rare {rare}, Epic {epic}, Legendary {legendary}, Mythic {mythic}.",
          tierPercents(odds.slot),
        )}
      </p>
      <p data-testid={purchasesTestid.cosmeticFloored}>
        {t(
          "Guaranteed slot: Common {common}, Rare {rare}, Epic {epic}, Legendary {legendary}, Mythic {mythic}.",
          tierPercents(odds.flooredSlot),
        )}
      </p>
    </>
  );
}

/** How close each tracked rarity's pity is; one counter set counts every card pack (R1298000), so it shows once. */
function Pity({ info }: { info: PacksResponse }): ReactElement {
  const { pity, thresholds } = info;
  return (
    <>
      <p>{t("Every card pack counts toward the same pity.")}</p>
      <ul className="purchases-pity__list" data-testid={purchasesTestid.pity}>
        {PITY_RARITIES.map(({ key, rarity }) => (
          <li key={key} className="purchases-pity__row">
            <span>{t(rarity)}</span>
            <span data-testid={purchasesTestid.pityRow(key)}>
              {t("{count} / {threshold} packs", { count: pity[key], threshold: thresholds[key] })}
            </span>
          </li>
        ))}
      </ul>
      <p data-testid={purchasesTestid.pityLine}>
        {t(
          "Pity guarantees an Epic within {epic} packs, a Legendary within {legendary} packs and a Mythic within {mythic} packs.",
          thresholds,
        )}
      </p>
    </>
  );
}

/** The odds and the counters, for a session: the page's two reads, each answering the sections it holds. */
function Published({ token }: { token: string }): ReactElement {
  const packs = useRead(token, getPacks);
  const cosmetic = useRead(token, getCosmeticPacks);
  return (
    <>
      <section aria-labelledby="purchases-card-packs-title" data-testid={purchasesTestid.cardPacks}>
        <h3 id="purchases-card-packs-title">{t("Card packs")}</h3>
        <Answered read={packs} place="card-packs" loading={t("Loading your packs…")}>
          {({ odds }) => (
            <>
              <SlotOdds odds={odds} />
              <DuplicateOdds odds={odds} />
            </>
          )}
        </Answered>
      </section>
      <section aria-labelledby="purchases-cosmetic-packs-title" data-testid={purchasesTestid.cosmeticPacks}>
        <h3 id="purchases-cosmetic-packs-title">{t("Cosmetic packs")}</h3>
        <Answered read={cosmetic} place="cosmetic-packs" loading={t("Loading your cosmetic packs…")}>
          {({ odds }) => <CosmeticOdds odds={odds} />}
        </Answered>
      </section>
      <section aria-labelledby="purchases-pity-title">
        <h3 id="purchases-pity-title">{t("Pity")}</h3>
        <Answered read={packs} place="pity" loading={t("Loading your packs…")}>
          {(info) => <Pity info={info} />}
        </Answered>
      </section>
    </>
  );
}

export default function PurchasesRoute(): ReactElement {
  useLocale();
  const [token] = useState(() => readSession()?.accessToken ?? null);
  // One sentence, the link where the locale puts it.
  const [signInBefore = "", signInAfter = ""] = t("{signIn} to see the odds and your pity counters.").split("{signIn}");
  return (
    <div className="app-shell tavern privacy-screen purchases-screen" data-testid={purchasesTestid.screen}>
      <BackLink />

      <article className="panel panel--auth privacy-policy">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Purchases")}</h2>
        <p data-testid={purchasesTestid.disclaimer}>
          {t(
            "Jlore is earned by playing and never sold: nothing here is bought with money. What a pack holds is the server\u2019s seeded, logged roll, and every figure on this page is the server\u2019s.",
          )}
        </p>
        {token === null ? (
          <p data-testid={purchasesTestid.signIn}>
            {signInBefore}
            <a href={paths.login} onClick={followInApp(paths.login)}>
              {t("Sign in")}
            </a>
            {signInAfter}
          </p>
        ) : (
          <Published token={token} />
        )}
      </article>
    </div>
  );
}
