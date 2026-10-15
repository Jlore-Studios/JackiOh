// What Play together's screens share besides `net/social.ts`: a refusal in its own words and a read
// that keeps itself fresh. A route that takes a token (decks, replays, settings) is called with the
// tab's session through `net/gate.ts`'s `withSession`, as the social and Duos wrappers are.

import { useCallback, useEffect, useRef, useState } from "react";

import { ApiRequestError } from "../net/api.ts";

/** A refusal's sentence: the server's own for `ApiRequestError`, the transport's plain one otherwise. */
export function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

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

/** Turns a synchronous throw into a rejected promise. */
export function attempt<T>(call: () => Promise<T>): Promise<T> {
  return Promise.resolve().then(call);
}

export type Polled<T> =
  | { kind: "loading" }
  | { kind: "ready"; value: T }
  | { kind: "failed"; message: string };

/**
 * `read` now, and again every `everyMs` while mounted (never, for null), and whenever `reload` is
 * called. A failed read after a good one keeps the good one on screen; an answer older than one
 * already shown is dropped.
 */
export function usePolled<T>(read: () => Promise<T>, everyMs: number | null): [Polled<T>, () => void] {
  const [state, setState] = useState<Polled<T>>({ kind: "loading" });
  const [round, setRound] = useState(0);
  const reader = useRef(read);
  reader.current = read;
  const asked = useRef(0);
  const shown = useRef(0);

  useEffect(() => {
    let cancelled = false;
    const tick = (): void => {
      asked.current += 1;
      const ticket = asked.current;
      attempt(() => reader.current()).then(
        (value) => {
          if (cancelled || ticket < shown.current) return;
          shown.current = ticket;
          setState({ kind: "ready", value });
        },
        (cause: unknown) => {
          if (cancelled || ticket < shown.current) return;
          setState((previous) => (previous.kind === "ready" ? previous : { kind: "failed", message: messageOf(cause) }));
        },
      );
    };
    tick();
    if (everyMs === null) {
      return () => {
        cancelled = true;
      };
    }
    const handle = window.setInterval(tick, everyMs);
    return () => {
      cancelled = true;
      window.clearInterval(handle);
    };
  }, [everyMs, round]);

  const reload = useCallback((): void => {
    setRound((count) => count + 1);
  }, []);
  return [state, reload];
}
