// Client-side online-game follow (SPEC §9.5, R765); the server decides the account state
// (CLAUDE.md rule 7) through `/api/auth/me` (`currentMatchId`, §9.5; `currentSeriesId`, R259).
// Queue follow is per-tab session storage; storage failures mean no follow, never a failed queue.

import { useEffect, useRef, useState, useSyncExternalStore } from "react";

import { MATCH_FOUND_NAV_DELAY_MS, SERIES_POLL_SECONDS } from "@jackioh/server-config";
import { getMe, type MeResponse, type QueueMode } from "./api.ts";
import { callWithRenewal, type Account } from "./gate.ts";
import { navigate, paths } from "./navigate.ts";
import { readSession } from "./session.ts";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;

/** What a screen says the moment a pairing lands, before it takes the player there. */
export const MATCH_FOUND_STATUS = "Match found! Taking you to your game…";

// Live game

/** A running match or a Conquest series between games. */
export type LiveGame = { kind: "match" | "series"; id: string; path: string };

/**
 * Where a profile's game is: the board (`currentMatchId`) or, between the games of a series, the
 * series screen (`currentSeriesId`), or nowhere. The board wins when both are set.
 */
export function liveGameOf(me: { currentMatchId: string | null; currentSeriesId?: string | null }): LiveGame | null {
  if (typeof me.currentMatchId === "string" && me.currentMatchId.length > 0) {
    return { kind: "match", id: me.currentMatchId, path: paths.match(me.currentMatchId) };
  }
  if (typeof me.currentSeriesId === "string" && me.currentSeriesId.length > 0) {
    return { kind: "series", id: me.currentSeriesId, path: paths.series(me.currentSeriesId) };
  }
  return null;
}

/** Turns a synchronous throw into a rejected promise. */
function attempt<T>(call: () => Promise<T>): Promise<T> {
  return Promise.resolve().then(call);
}

/**
 * R765: the live game `account` names, kept current: read again every `SERIES_POLL_SECONDS` for as
 * long as there is one, and null once a read says it is over. An account that is not ready has none.
 */
export function useLiveGame(account: Account, read: (token: string) => Promise<MeResponse> = getMe): LiveGame | null {
  const token = account.kind === "ready" ? account.token : null;
  const me = account.kind === "ready" ? account.me : null;
  /** Pair a re-read with its account read so a fresh account read wins. */
  const [polled, setPolled] = useState<{ after: MeResponse; game: LiveGame | null } | null>(null);
  const reader = useRef(read);
  reader.current = read;

  const game = me === null ? null : polled !== null && polled.after === me ? polled.game : liveGameOf(me);
  const watched = game?.path ?? null;

  useEffect(() => {
    if (token === null || me === null || watched === null) return;
    let cancelled = false;
    const handle = setInterval(() => {
      attempt(() => reader.current(token)).then(
        (next) => {
          if (!cancelled) setPolled({ after: me, game: liveGameOf(next) });
        },
        () => undefined,
      );
    }, SERIES_POLL_SECONDS * MS_PER_SECOND);
    return () => {
      cancelled = true;
      clearInterval(handle);
    };
  }, [token, me, watched]);

  return game;
}

// Queue joined by this tab

/** Where this tab remembers the mode it queued in, while it waits to be paired. */
export const QUEUE_STORAGE_KEY = "jackioh.play.queued";

/** Fired for this tab's writes, which `storage` events do not cover. */
const QUEUE_CHANGED = "jackioh:queue-changed";

const QUEUE_MODES: readonly QueueMode[] = ["bo1", "bo3", "random"];

function tabStorage(): Storage | null {
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}

/** The mode this tab is queued in, or null. */
export function readQueued(): QueueMode | null {
  try {
    const raw = tabStorage()?.getItem(QUEUE_STORAGE_KEY) ?? null;
    return (QUEUE_MODES as readonly (string | null)[]).includes(raw) ? (raw as QueueMode) : null;
  } catch {
    return null;
  }
}

function changed(): void {
  window.dispatchEvent(new Event(QUEUE_CHANGED));
}

/** R765: remember this tab's queue so a pairing can follow it. */
export function rememberQueued(mode: QueueMode): void {
  try {
    tabStorage()?.setItem(QUEUE_STORAGE_KEY, mode);
  } catch {
    // Blocked storage: only `/play` follows the queue (R765).
  }
  changed();
}

/** Forget a left or paired queue. */
export function forgetQueued(): void {
  try {
    tabStorage()?.removeItem(QUEUE_STORAGE_KEY);
  } catch {
    // Blocked storage has nothing to forget.
  }
  changed();
}

function subscribeQueued(onChange: () => void): () => void {
  window.addEventListener(QUEUE_CHANGED, onChange);
  return () => {
    window.removeEventListener(QUEUE_CHANGED, onChange);
  };
}

/** The mode this tab is queued in, re-rendering the caller whenever it changes. */
export function useQueued(): QueueMode | null {
  return useSyncExternalStore(subscribeQueued, readQueued, readQueued);
}

/**
 * R765: while this tab is queued and the player is anywhere but `/play`, read the account every
 * `SERIES_POLL_SECONDS`, and once it names a game, forget the queue, announce the pairing for
 * `MATCH_FOUND_NAV_DELAY_MS` and take the player there. Returns the game being announced, or null.
 * A session that has ended is out of the queue as far as this tab can follow it.
 */
export function useQueueFollow(path: string, read: (token: string) => Promise<MeResponse> = getMe): LiveGame | null {
  const queued = useQueued();
  const [found, setFound] = useState<LiveGame | null>(null);
  const reader = useRef(read);
  reader.current = read;
  const watching = queued !== null && path !== paths.play && found === null;

  useEffect(() => {
    if (!watching) return;
    let cancelled = false;
    const check = (): void => {
      const session = readSession();
      if (session === null) {
        forgetQueued();
        return;
      }
      attempt(() => callWithRenewal(session.accessToken, (token) => reader.current(token))).then(
        (answer) => {
          if (cancelled) return;
          if (answer.kind !== "ok") {
            if (answer.kind === "signedOut") forgetQueued();
            return;
          }
          const game = liveGameOf(answer.value);
          if (game === null) return;
          forgetQueued();
          setFound(game);
        },
        () => undefined,
      );
    };
    check();
    const handle = setInterval(check, SERIES_POLL_SECONDS * MS_PER_SECOND);
    return () => {
      cancelled = true;
      clearInterval(handle);
    };
  }, [watching]);

  useEffect(() => {
    if (found === null) return;
    const timer = setTimeout(() => {
      setFound(null);
      navigate(found.path);
    }, MATCH_FOUND_NAV_DELAY_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [found]);

  return found;
}
