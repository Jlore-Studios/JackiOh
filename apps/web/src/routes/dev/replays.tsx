// `/dev/replays` (#511): the replay player's dev list, and the only way to reach the player until
// the result screens and a recent-games list open it (#430). It lists the practice games kept on this
// device and, signed in, the account's online replays (first page), and opens one in the player.
//
// Like the hotseat, it exists only outside a production build (`main.tsx` imports it behind
// `DEV_ONLY`), so its text is plain English, not `t()`'s. It holds no log, seed or state: a listing
// is what the worker or the server answers, and the player reads steps through a `ReplaySource`.

import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";

import { CatalogContext, lookupFromDefs } from "../../game/catalog.ts";
import type { CardLookup } from "../../game/catalog.ts";
import { getReplays } from "../../net/api.ts";
import type { OnlineReplayListing } from "../../net/api.ts";
import { useAccount } from "../../net/gate.ts";
import { navigate, paths } from "../../net/navigate.ts";
import { createPracticeHost } from "../../practice/host.ts";
import type { PracticeHost } from "../../practice/host.ts";
import type { PracticeReplayListing } from "../../practice/protocol.ts";
import ReplayPlayer from "../../replay/ReplayPlayer.tsx";
import { ReplayFailure, replayFailureText } from "../../replay/source.ts";
import type { ReplaySource } from "../../replay/source.ts";
import { onlineReplaySource, practiceReplaySource } from "../../replay/sources.ts";
import { navTestid } from "../nav.tsx";

export const devReplaysTestid = {
  practice: (game: number): string => `dev-replays-practice-${String(game)}`,
  online: (matchId: string): string => `dev-replays-online-${matchId}`,
} as const;

type Listed<Row> = { kind: "loading" } | { kind: "failed"; message: string } | { kind: "ready"; rows: Row[] };

type Open = { key: string; source: ReplaySource };

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

export function DevReplaysRoute({ hostFactory = createPracticeHost }: { hostFactory?: () => PracticeHost }): ReactElement {
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
        setPractice({ kind: "ready", rows: read.replays });
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
        if (!cancelled) setOnline({ kind: "ready", rows: list.replays });
      },
      (cause: unknown) => {
        if (!cancelled) setOnline({ kind: "failed", message: messageOf(cause) });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [token]);

  const openPractice = (row: PracticeReplayListing): void => {
    if (host.current !== null) setOpen({ key: `practice-${String(row.game)}`, source: practiceReplaySource(host.current, row) });
  };
  const openOnline = (row: OnlineReplayListing): void => {
    if (token !== null) setOpen({ key: `online-${row.matchId}`, source: onlineReplaySource(token, row) });
  };

  if (open !== null) {
    return (
      <CatalogContext.Provider value={lookup}>
        <ReplayPlayer key={open.key} source={open.source} onLeave={() => setOpen(null)} />
      </CatalogContext.Provider>
    );
  }

  return (
    <div className="app-shell">
      <h1>JackiOh — replays</h1>
      <button type="button" className="link-button" data-testid={navTestid.back} onClick={() => navigate(paths.landing)}>
        ← Back
      </button>

      <section>
        <h2>Practice games on this device</h2>
        {practice.kind === "loading" ? <p>Reading the games kept on this device…</p> : null}
        {practice.kind === "failed" ? <p role="alert">{practice.message}</p> : null}
        {practice.kind === "ready" && practice.rows.length === 0 ? <p>No finished practice game is kept on this device.</p> : null}
        {practice.kind === "ready" ? (
          <ul>
            {practice.rows.map((row) => (
              <li key={row.game} data-testid={devReplaysTestid.practice(row.game)}>
                Game {row.game} · {row.result} · {row.turns} turns · {row.steps} steps
                {row.unavailable === undefined ? null : <> · {replayFailureText(new ReplayFailure(row.unavailable))}</>}{" "}
                <button type="button" onClick={() => openPractice(row)}>
                  Open
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </section>

      <section>
        <h2>Online games</h2>
        {account.kind === "loading" ? <p>Checking your account…</p> : null}
        {account.kind === "anonymous" ? <p>Sign in to list your online replays.</p> : null}
        {account.kind === "error" ? <p role="alert">{account.message}</p> : null}
        {token !== null && online.kind === "loading" ? <p>Reading your online replays…</p> : null}
        {token !== null && online.kind === "failed" ? <p role="alert">{online.message}</p> : null}
        {token !== null && online.kind === "ready" && online.rows.length === 0 ? <p>No online replay is kept for this account.</p> : null}
        {token !== null && online.kind === "ready" ? (
          <ul>
            {online.rows.map((row) => (
              <li key={row.matchId} data-testid={devReplaysTestid.online(row.matchId)}>
                vs {row.opponent.username === "" ? "a deleted account" : row.opponent.username} · {row.mode ?? "unknown mode"} ·{" "}
                {row.result} · {row.turns} turns
                {row.unavailable === undefined ? null : <> · {replayFailureText(new ReplayFailure(row.unavailable))}</>}{" "}
                <button type="button" onClick={() => openOnline(row)}>
                  Open
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </section>
    </div>
  );
}

export default DevReplaysRoute;
