// `/purchases`: the odds and the pity of every pack (#1665, part of #1651, SPEC §14.4, R1665000), every
// figure in a table (#3188). Public like `/terms` (main.tsx serves it ungated); it reads the session
// itself (`readSession`), so a visitor with none makes no request. The client computes nothing (CLAUDE.md
// rule 7): every figure is the server's, only formatted, and a rate of 0 shows as a dash. A failed read
// offers Try again in its place (#1774), as elsewhere (#1691). R3015003: both odds list the chance a pull
// grants a variant's levels out of order (`levelSkip`).

import { COSMETIC_PACK_FLOOR_RARITY, COSMETIC_PACK_SLOTS, PACK_FLOOR_RARITY } from "@jackioh/server-config";
import { useEffect, useState, type ReactElement } from "react";

import { AccountError } from "../auth/AccountError.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import {
  getCosmeticPacks,
  getPacks,
  type CosmeticPacksResponse,
  type PacksResponse,
  type TierOdds,
} from "../net/api.ts";
import { readFailureOf, type ReadFailure } from "../net/gate.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import { ODDS_RARITIES, PITY_RARITIES, percent } from "../packs/packs.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import type { SkeletonVariant } from "../skeleton/Skeleton.tsx";
import { BackLink, followInApp } from "./nav.tsx";
import { Wordmark } from "./Wordmark.tsx";

import "../auth/tavern.css";
import "./privacy.css";
import "./purchases.css";

export const purchasesTestid = {
  screen: "purchases-screen",
  disclaimer: "purchases-disclaimer",
  signIn: "purchases-sign-in",
  loading: (place: PurchasesPlace) => `purchases-${place}-loading`,
  loadError: (place: PurchasesPlace) => `purchases-${place}-load-error`,
  retry: (place: PurchasesPlace) => `purchases-${place}-retry`,
  cardPacks: "purchases-card-packs",
  cardPacksSummary: "purchases-card-packs-summary",
  odds: "purchases-odds",
  oddsRow: (key: string) => `purchases-odds-${key}`,
  duplicates: "purchases-duplicates",
  duplicate: "purchases-duplicate",
  duplicateRow: (key: string) => `purchases-duplicate-${key}`,
  duplicateNote: "purchases-duplicate-note",
  levelSkip: "purchases-level-skip",
  cosmeticLevelSkip: "purchases-cosmetic-level-skip",
  cosmeticPacks: "purchases-cosmetic-packs",
  cosmeticPacksSummary: "purchases-cosmetic-packs-summary",
  cosmetic: "purchases-cosmetic",
  cosmeticRow: (key: string) => `purchases-cosmetic-${key}`,
  pity: "purchases-pity",
  pityRow: (key: string) => `purchases-pity-${key}`,
} as const;

/** Where a read's wait or failure shows: the card packs', the duplicates' and the pity's all read `GET /api/packs`. */
export type PurchasesPlace = "card-packs" | "duplicates" | "cosmetic-packs" | "pity";

type Read<T> =
  | { kind: "loading" }
  | { kind: "ready"; value: T }
  | ({ kind: "failed"; retry: () => void } & ReadFailure);

/** `read(token)` once the page has a token, and again each time Try again asks; never, without one. */
function useRead<T>(token: string | null, read: (token: string) => Promise<T>): Read<T> {
  const [state, setState] = useState<Read<T>>({ kind: "loading" });
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    if (token === null) return;
    let canceled = false;
    read(token).then(
      (value) => {
        if (!canceled) setState({ kind: "ready", value });
      },
      (cause: unknown) => {
        if (canceled) return;
        setState({
          kind: "failed",
          ...readFailureOf(cause),
          retry: () => {
            setState({ kind: "loading" });
            setAttempt((before) => before + 1);
          },
        });
      },
    );
    return () => {
      canceled = true;
    };
  }, [token, read, attempt]);
  return state;
}

/** The read's answer, or its wait or its failure with Try again, in the same place. */
function Answered<T>({
  read,
  place,
  loading,
  skeleton,
  children,
}: {
  read: Read<T>;
  place: PurchasesPlace;
  loading: string;
  skeleton: SkeletonVariant;
  children: (value: T) => ReactElement;
}): ReactElement {
  switch (read.kind) {
    case "loading":
      return <Skeleton variant={skeleton} label={loading} testId={purchasesTestid.loading(place)} />;
    case "failed":
      return (
        <AccountError
          message={read.message}
          retryAfterMs={read.retryAfterMs}
          onRetry={read.retry}
          testId={purchasesTestid.loadError(place)}
          retryTestId={purchasesTestid.retry(place)}
        />
      );
    case "ready":
      return children(read.value);
  }
}

/** What a rate with no chance shows in a table (a rarity a roll never lands on). */
const NO_ODDS = "—";

function oddsCell(rate: number): string {
  return rate === 0 ? NO_ODDS : percent(rate);
}

/** A table of two rolls' odds, a rarity to a row: Common to Mythic, each roll's chance in its own column. */
function RarityTable({
  testId,
  rowTestId,
  first,
  second,
}: {
  testId: string;
  rowTestId: (key: string) => string;
  first: { head: string; rates: TierOdds };
  second: { head: string; rates: TierOdds };
}): ReactElement {
  return (
    <table className="purchases-odds" data-testid={testId}>
      <thead>
        <tr>
          <th scope="col">{t("Rarity")}</th>
          <th scope="col">{first.head}</th>
          <th scope="col">{second.head}</th>
        </tr>
      </thead>
      <tbody>
        {ODDS_RARITIES.map(({ key, rarity }) => (
          <tr key={key} data-testid={rowTestId(key)}>
            <th scope="row">{t(rarity)}</th>
            <td>{oddsCell(first.rates[key])}</td>
            <td>{oddsCell(second.rates[key])}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** A card pack slot's odds and the guaranteed slot's, as `GET /api/packs` states them (R1339000). */
function SlotOdds({ odds }: { odds: PacksResponse["odds"] }): ReactElement {
  return (
    <RarityTable
      testId={purchasesTestid.odds}
      rowTestId={purchasesTestid.oddsRow}
      first={{ head: t("Each card"), rates: odds.slot }}
      second={{ head: t("Guaranteed card"), rates: odds.flooredSlot }}
    />
  );
}

/**
 * R3015003: how a pull grants a variant's levels: its next level, and `chance` of the time, where a higher one is
 * left, that one and every level below it instead.
 */
function LevelSkip({ chance, testid }: { chance: number; testid: string }): ReactElement {
  return (
    <p data-testid={testid}>
      {t(
        "A variant drops its next level. {chance} of the time, when a higher level is left, it drops that level and every level below it instead.",
        { chance: percent(chance) },
      )}
    </p>
  );
}

/** What a duplicate becomes: a cosmetic of the table's rarity, or, on the global roll, an account cosmetic (R1661001). */
function DuplicateOdds({ odds }: { odds: PacksResponse["odds"] }): ReactElement {
  const { chance, tiers } = odds.globalCosmetic;
  return (
    <>
      <p>{t("A card you already own becomes a cosmetic.")}</p>
      <RarityTable
        testId={purchasesTestid.duplicate}
        rowTestId={purchasesTestid.duplicateRow}
        first={{ head: t("Cosmetic"), rates: odds.duplicateCosmetic }}
        second={{ head: t("Account cosmetic ({chance} of duplicates)", { chance: percent(chance) }), rates: tiers }}
      />
      <p data-testid={purchasesTestid.duplicateNote}>
        {t("If you already own every cosmetic of that rarity, the duplicate fills both token bars instead.")}
      </p>
      <LevelSkip chance={odds.levelSkip} testid={purchasesTestid.levelSkip} />
    </>
  );
}

/** A cosmetic pack's slot and its guaranteed slot, as `GET /api/cosmetic-packs` states them (R1339000). */
function CosmeticOdds({ odds }: { odds: CosmeticPacksResponse["odds"] }): ReactElement {
  return (
    <>
      <RarityTable
        testId={purchasesTestid.cosmetic}
        rowTestId={purchasesTestid.cosmeticRow}
        first={{ head: t("Each slot"), rates: odds.slot }}
        second={{ head: t("Guaranteed slot"), rates: odds.flooredSlot }}
      />
      <LevelSkip chance={odds.levelSkip} testid={purchasesTestid.cosmeticLevelSkip} />
    </>
  );
}

/** How close each tracked rarity's pity is; one counter set counts every card pack (R1298000), so it shows once. */
function Pity({ info }: { info: PacksResponse }): ReactElement {
  const { pity, thresholds } = info;
  return (
    <>
      <p>{t("All card packs count toward the same pity.")}</p>
      <table className="purchases-odds" data-testid={purchasesTestid.pity}>
        <thead>
          <tr>
            <th scope="col">{t("Rarity")}</th>
            <th scope="col">{t("Guaranteed within")}</th>
            <th scope="col">{t("Your progress")}</th>
          </tr>
        </thead>
        <tbody>
          {PITY_RARITIES.map(({ key, rarity }) => (
            <tr key={key} data-testid={purchasesTestid.pityRow(key)}>
              <th scope="row">{t(rarity)}</th>
              <td>{t("{count} packs", { count: thresholds[key] })}</td>
              <td>{`${String(pity[key])} / ${String(thresholds[key])}`}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}

/** The odds and the counters, for a session. */
function Published({ token }: { token: string }): ReactElement {
  const packs = useRead(token, getPacks);
  const cosmetic = useRead(token, getCosmeticPacks);
  return (
    <>
      <section aria-labelledby="purchases-card-packs-title" data-testid={purchasesTestid.cardPacks}>
        <h2 id="purchases-card-packs-title" className="section-heading">{t("Card packs")}</h2>
        <Answered read={packs} place="card-packs" loading={t("Loading your packs…")} skeleton="table">
          {({ odds, price, slots }) => (
            <>
              <p data-testid={purchasesTestid.cardPacksSummary}>
                {t("{price} Jlore · {slots} cards · one card is always {rarity} or better", {
                  price,
                  slots,
                  rarity: t(PACK_FLOOR_RARITY),
                })}
              </p>
              <SlotOdds odds={odds} />
            </>
          )}
        </Answered>
      </section>
      <section aria-labelledby="purchases-duplicates-title" data-testid={purchasesTestid.duplicates}>
        <h2 id="purchases-duplicates-title" className="section-heading">{t("Duplicates")}</h2>
        <Answered read={packs} place="duplicates" loading={t("Loading your packs…")} skeleton="table">
          {({ odds }) => <DuplicateOdds odds={odds} />}
        </Answered>
      </section>
      <section aria-labelledby="purchases-cosmetic-packs-title" data-testid={purchasesTestid.cosmeticPacks}>
        <h2 id="purchases-cosmetic-packs-title" className="section-heading">{t("Cosmetic packs")}</h2>
        <Answered read={cosmetic} place="cosmetic-packs" loading={t("Loading your cosmetic packs…")} skeleton="panel">
          {({ odds }) => (
            <>
              <p data-testid={purchasesTestid.cosmeticPacksSummary}>
                {t("{slots} cosmetics · one is always {rarity} or better", {
                  slots: COSMETIC_PACK_SLOTS,
                  rarity: t(COSMETIC_PACK_FLOOR_RARITY),
                })}
              </p>
              <CosmeticOdds odds={odds} />
            </>
          )}
        </Answered>
      </section>
      <section aria-labelledby="purchases-pity-title">
        <h2 id="purchases-pity-title" className="section-heading">{t("Pity")}</h2>
        <Answered read={packs} place="pity" loading={t("Loading your packs…")} skeleton="rows">
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
      <BackLink title={t("Purchases")} />

      <article className="panel panel--auth privacy-policy">
        <Wordmark />
        <p data-testid={purchasesTestid.disclaimer}>
          {t("You buy packs with Jlore, which you earn by playing. Jlore is never sold for real money.")}
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
