// What Play together's screens share besides `net/social.ts`: a refused session told apart and a
// read that keeps itself fresh (a refusal's words are `net/errors.ts`'s `messageOf`). A route that takes a token (decks, replays, settings) is called with the
// tab's session through `net/gate.ts`'s `withSession`, as the social and Duos wrappers are.

import { useCallback, useEffect, useRef, useState } from "react";

import { ApiRequestError } from "../net/api.ts";
import { readFailureOf } from "../net/gate.ts";
import { attempt } from "../net/errors.ts";

/** The session refused a social route outright: signed out (401), or an account that is not active (403). */
export function isRefusedSession(cause: unknown): boolean {
  return cause instanceof ApiRequestError && (cause.status === 401 || cause.status === 403);
}

/**
 * The unread counts in a `GET /api/chat/unread` answer, by friend; anything that is not one reads as
 * none. The board rail reads it on every match screen, so an odd answer must not take a board down.
 */
export function unreadCountsOf(answer: unknown): Readonly<Record<string, number>> {
  if (typeof answer !== "object" || answer === null) return {};
  const unread = (answer as { unread?: unknown }).unread;
  if (typeof unread !== "object" || unread === null) return {};
  const counts: Record<string, number> = {};
  for (const [friendId, count] of Object.entries(unread)) {
    if (typeof count === "number" && Number.isFinite(count)) counts[friendId] = count;
  }
  return counts;
}

export type Polled<T> =
  | { kind: "loading" }
  | { kind: "ready"; value: T }
  | { kind: "failed"; message: string; retryAfterMs?: number };

/**
 * `read` now, and again every `everyMs` while mounted (never, for null), and whenever `reload` is
 * called. A failed read after a good one keeps the good one on screen; an answer older than one
 * already shown is dropped. After a failure, a later failed poll keeps the failure first shown, so
 * the wait it stated (R192) is counted once; `reload` after a failure shows loading (#1691). R2125000:
 * a mount that finds `kept` holding an answer starts on it, so a failed read leaves it too.
 */
export function usePolled<T>(
  read: () => Promise<T>,
  everyMs: number | null,
  kept?: () => T | undefined,
): [Polled<T>, () => void] {
  const [state, setState] = useState<Polled<T>>(() => {
    const value = kept?.();
    return value === undefined ? { kind: "loading" } : { kind: "ready", value };
  });
  const [round, setRound] = useState(0);
  const reader = useRef(read);
  reader.current = read;
  const asked = useRef(0);
  const shown = useRef(0);

  useEffect(() => {
    let canceled = false;
    const tick = (): void => {
      asked.current += 1;
      const ticket = asked.current;
      attempt(() => reader.current()).then(
        (value) => {
          if (canceled || ticket < shown.current) return;
          shown.current = ticket;
          setState({ kind: "ready", value });
        },
        (cause: unknown) => {
          if (canceled || ticket < shown.current) return;
          setState((previous) => (previous.kind === "loading" ? { kind: "failed", ...readFailureOf(cause) } : previous));
        },
      );
    };
    tick();
    if (everyMs === null) {
      return () => {
        canceled = true;
      };
    }
    const handle = window.setInterval(tick, everyMs);
    return () => {
      canceled = true;
      window.clearInterval(handle);
    };
  }, [everyMs, round]);

  const reload = useCallback((): void => {
    setState((previous) => (previous.kind === "failed" ? { kind: "loading" } : previous));
    setRound((count) => count + 1);
  }, []);
  return [state, reload];
}
