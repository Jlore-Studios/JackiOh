// `/replays` (R768000, #768): the player's recent games, each one's replay a click away. The
// practice games this device kept (the last `PRACTICE_REPLAYS_KEPT`, R768) and, signed in, the
// online matches the account played whose log the retention purge has not taken yet, newest first,
// a page at a time. A game this build cannot show says why, and opens to the same sentence.
//
// It holds no log, seed or state (CLAUDE.md rule 7): a row is what the practice worker or the server
// answers, and the player reads steps through a `ReplaySource`.

import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";

import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import type { CardLookup } from "../game/catalog.ts";
import { formatDateTime, t, useLocale } from "../i18n/i18n.ts";
import { getReplays } from "../net/api.ts";
import type { OnlineReplayListing, QueueMode } from "../net/api.ts";
import { useAccount } from "../net/gate.ts";
import { paths } from "../net/navigate.ts";
import { createPracticeHost } from "../practice/host.ts";
import type { PracticeHost } from "../practice/host.ts";
import type { PracticeReplayListing } from "../practice/protocol.ts";
import ReplayPlayer from "../replay/ReplayPlayer.tsx";
import { ReplayFailure, replayFailureText } from "../replay/source.ts";
import type { ReplaySource } from "../replay/source.ts";
import { onlineReplaySource, practiceReplaySource } from "../replay/sources.ts";
import { ShareWithFriend } from "../together/ShareReplay.tsx";
import { sharedWithMePath } from "../together/anchors.ts";
import { BackLink, followInApp } from "./nav.tsx";

import "./replays.css";

export const replaysTestid = {
  root: "replays",
  practice: (game: number): string => `replays-practice-${String(game)}`,
  online: (matchId: string): string => `replays-online-${matchId}`,
  noPractice: "replays-no-practice",
  noOnline: "replays-no-online",
  signIn: "replays-sign-in",
  more: "replays-more",
  /** #936 B59: Play together's list of the replays friends shared. */
  sharedWithMe: "replays-shared-with-me",
} as const;

type Listed<Row> =
  | { kind: "loading" }
  | { kind: "failed"; message: string }
  | { kind: "ready"; rows: Row[]; next: number | null; loadingMore: boolean };

/** The replay on screen; `matchId` for an online match's, which can be shared with a friend (#936 B59). */
type Open = { key: string; source: ReplaySource; matchId?: string };

const RESULT_WORDS: Readonly<Record<PracticeReplayListing["result"], string>> = {
  win: "Win",
  loss: "Loss",
  draw: "Draw",
};

const MODE_WORDS: Readonly<Record<QueueMode, string>> = {
  bo1: "Best of 1",
  bo3: "Conquest",
  random: "All Random",
};

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/** The worker's card data and the games it kept, asked in turn (its answers arrive in order). */
async function readDevice(host: PracticeHost): Promise<{ lookup: CardLookup; replays: PracticeReplayListing[] }> {
  const catalog = await host.request({ type: "catalog" });
  if (catalog.type !== "catalog") throw new Error(catalog.type === "failed" ? catalog.message : "the practice worker sent no card data");
  const replays = await host.request({ type: "replays" });
  if (replays.type !== "replays") throw new Error(replays.type === "failed" ? replays.message : "the practice worker sent no list");
  return { lookup: lookupFromDefs(catalog.defs), replays: replays.replays };
}

/** How long the game ran, in turns. */
function turnsText(turns: number): string {
  return turns === 1 ? t("1 turn") : t("{turns} turns", { turns });
}

/** The row's reason it cannot be shown, if any, worded as the player words it. */
function unavailableText(reason: PracticeReplayListing["unavailable"]): string | null {
  return reason === undefined ? null : replayFailureText(new ReplayFailure(reason));
}

export function ReplaysRoute({ hostFactory = createPracticeHost }: { hostFactory?: () => PracticeHost }): ReactElement {
  useLocale();
  const account = useAccount();
  const token = account.kind === "ready" ? account.token : null;
  const [practice, setPractice] = useState<Listed<PracticeReplayListing>>({ kind: "loading" });
  const [online, setOnline] = useState<Listed<OnlineReplayListing>>({ kind: "loading" });
  const [lookup, setLookup] = useState<CardLookup | null>(null);
  const [open, setOpen] = useState<Open | null>(null);
  /** The worker that kept the practice games: one for this mount, which a practice source asks pages of. */
  const host = useRef<PracticeHost | null>(null);

  useEffect(() => {
    const made = hostFactory();
    host.current = made;
    let cancelled = false;
    readDevice(made).then(
      (read) => {
        if (cancelled) return;
        setLookup(() => read.lookup);
        setPractice({ kind: "ready", rows: read.replays, next: null, loadingMore: false });
      },
      (cause: unknown) => {
        if (!cancelled) setPractice({ kind: "failed", message: messageOf(cause) });
      },
    );
    return () => {
      cancelled = true;
      host.current = null;
      made.dispose();
    };
  }, [hostFactory]);

  useEffect(() => {
    if (token === null) return;
    let cancelled = false;
    getReplays(token).then(
      (list) => {
        if (!cancelled) setOnline({ kind: "ready", rows: list.replays, next: list.nextOffset, loadingMore: false });
      },
      (cause: unknown) => {
        if (!cancelled) setOnline({ kind: "failed", message: messageOf(cause) });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [token]);

  const loadMore = (): void => {
    if (token === null || online.kind !== "ready" || online.next === null || online.loadingMore) return;
    const shown = online;
    setOnline({ ...shown, loadingMore: true });
    getReplays(token, shown.next ?? 0).then(
      (list) => {
        setOnline({ kind: "ready", rows: [...shown.rows, ...list.replays], next: list.nextOffset, loadingMore: false });
      },
      (cause: unknown) => {
        setOnline({ kind: "failed", message: messageOf(cause) });
      },
    );
  };

  const openPractice = (row: PracticeReplayListing): void => {
    if (host.current !== null) setOpen({ key: `practice-${String(row.game)}`, source: practiceReplaySource(host.current, row) });
  };
  const openOnline = (row: OnlineReplayListing): void => {
    if (token !== null) setOpen({ key: `online-${row.matchId}`, source: onlineReplaySource(token, row), matchId: row.matchId });
  };

  if (open !== null) {
    return (
      <CatalogContext.Provider value={lookup}>
        <ReplayPlayer
          key={open.key}
          source={open.source}
          onLeave={() => setOpen(null)}
          {...(open.matchId === undefined ? {} : { actions: <ShareWithFriend matchId={open.matchId} /> })}
        />
      </CatalogContext.Provider>
    );
  }

  return (
    <div className="app-shell replays" data-testid={replaysTestid.root}>
      <BackLink />
      <h1>{t("Replays")}</h1>
      <p className="auth-hint">{t("Watch your recent games again, step by step, and share a turn as a GIF.")}</p>

      <section className="replays__section" aria-labelledby="replays-practice-heading">
        <h2 id="replays-practice-heading">{t("Practice games on this device")}</h2>
        {practice.kind === "loading" ? <p role="status">{t("Reading the games kept on this device…")}</p> : null}
        {practice.kind === "failed" ? <p role="alert">{practice.message}</p> : null}
        {practice.kind === "ready" && practice.rows.length === 0 ? (
          <p data-testid={replaysTestid.noPractice}>{t("Finish a practice game and it is kept here.")}</p>
        ) : null}
        {practice.kind === "ready" && practice.rows.length > 0 ? (
          <ul className="replays__list">
            {practice.rows.map((row) => {
              const why = unavailableText(row.unavailable);
              return (
                <li key={row.game} className="replays__row" data-testid={replaysTestid.practice(row.game)}>
                  <span className="replays__what">
                    <strong>{t(RESULT_WORDS[row.result])}</strong> · {t("Practice")} ·{" "}
                    {turnsText(row.turns)} · {formatDateTime(row.endedAt, { dateStyle: "medium", timeStyle: "short" })}
                    {why === null ? null : <span className="replays__why"> · {why}</span>}
                  </span>
                  <button type="button" onClick={() => openPractice(row)} disabled={why !== null}>
                    {t("Watch")}
                  </button>
                </li>
              );
            })}
          </ul>
        ) : null}
      </section>

      <section className="replays__section" aria-labelledby="replays-online-heading">
        <h2 id="replays-online-heading">{t("Online games")}</h2>
        {account.kind === "loading" ? <p role="status">{t("Checking your account…")}</p> : null}
        {account.kind === "anonymous" ? (
          <p data-testid={replaysTestid.signIn}>
            <a href={paths.login} onClick={followInApp(paths.login)}>
              {t("Sign in")}
            </a>{" "}
            {t("to watch your online games again.")}
          </p>
        ) : null}
        {account.kind === "error" ? <p role="alert">{account.message}</p> : null}
        {token === null ? null : (
          // #936 B59: the replays friends shared with this account are listed on Play together.
          <p>
            <a href={sharedWithMePath()} data-testid={replaysTestid.sharedWithMe} onClick={followInApp(sharedWithMePath())}>
              {t("Shared with me →")}
            </a>
          </p>
        )}
        {token !== null && online.kind === "loading" ? <p role="status">{t("Reading your online games…")}</p> : null}
        {token !== null && online.kind === "failed" ? <p role="alert">{online.message}</p> : null}
        {token !== null && online.kind === "ready" && online.rows.length === 0 ? (
          <p data-testid={replaysTestid.noOnline}>{t("No online game of yours is kept. A game is kept for a while after it ends.")}</p>
        ) : null}
        {token !== null && online.kind === "ready" && online.rows.length > 0 ? (
          <>
            <ul className="replays__list">
              {online.rows.map((row) => {
                const why = unavailableText(row.unavailable);
                const opponent = row.opponent.username === "" ? t("a deleted account") : row.opponent.username;
                return (
                  <li key={row.matchId} className="replays__row" data-testid={replaysTestid.online(row.matchId)}>
                    <span className="replays__what">
                      <strong>{t(RESULT_WORDS[row.result])}</strong> · {t("vs {opponent}", { opponent })}
                      {row.mode === null ? null : <> · {t(MODE_WORDS[row.mode])}</>} · {turnsText(row.turns)} ·{" "}
                      {formatDateTime(row.endedAt, { dateStyle: "medium", timeStyle: "short" })}
                      {why === null ? null : <span className="replays__why"> · {why}</span>}
                    </span>
                    <button type="button" onClick={() => openOnline(row)} disabled={why !== null}>
                      {t("Watch")}
                    </button>
                    {/* #936 B59: a game that can be watched can be shared with a friend. */}
                    {why === null ? <ShareWithFriend matchId={row.matchId} /> : null}
                  </li>
                );
              })}
            </ul>
            {online.next === null ? null : (
              <button type="button" data-testid={replaysTestid.more} onClick={loadMore} disabled={online.loadingMore}>
                {online.loadingMore ? t("Loading…") : t("Show older games")}
              </button>
            )}
          </>
        ) : null}
      </section>
    </div>
  );
}

export default ReplaysRoute;
