// Where the session lives: this tab's `sessionStorage` alone, never `localStorage`, so closing the tab
// forgets it. It is renewed before it expires (the server's AUTH_SESSION_REFRESH_MARGIN_SECONDS, and
// AUTH_SESSION_RENEWAL_FLOOR_SECONDS between two renewals, R194), and every request asks for it fresh.
//
// Development and e2e builds alone (`MODE` is not `production`) also take a session from
// `localStorage["jackioh.e2e.session"]`, `{ "accessToken": "…" }`, as apps/web/src/net/session.ts does:
// the e2e suite signs its fixture accounts in that way. Vite folds the `MODE` test away in a production
// build, and with it every line that reads the key (build.test.ts holds that).

import { useCallback, useEffect, useRef, useState } from "react";

import type { TokenSource } from "./api.ts";
import { AuthError, refreshSession, revokeSession, type Session } from "./auth.ts";
import { AUTH_SESSION_REFRESH_MARGIN_SECONDS, AUTH_SESSION_RENEWAL_FLOOR_SECONDS, MS_PER_SECOND } from "./wire.ts";

export const SESSION_STORAGE_KEY = "jackioh.admin.session";

/** The e2e suite's fixture session (e2e/support/config.ts `SESSION_STORAGE_KEY`); read by no production build. */
export const E2E_SESSION_STORAGE_KEY = "jackioh.e2e.session";

const REFRESH_MARGIN_MS = AUTH_SESSION_REFRESH_MARGIN_SECONDS * MS_PER_SECOND;
const RENEWAL_FLOOR_MS = AUTH_SESSION_RENEWAL_FLOOR_SECONDS * MS_PER_SECOND;

function storage(kind: "sessionStorage" | "localStorage"): Storage | null {
  try {
    return typeof window === "undefined" ? null : window[kind];
  } catch {
    return null;
  }
}

function read(kind: "sessionStorage" | "localStorage", key: string): string | null {
  try {
    return storage(kind)?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

function remove(kind: "sessionStorage" | "localStorage", key: string): void {
  try {
    storage(kind)?.removeItem(key);
  } catch {
    // Blocked storage held nothing.
  }
}

/** Untrusted storage never crashes the boot, and is a session only with a non-empty token. */
export function parseSession(raw: string | null): Session | null {
  if (raw === null) return null;
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    return null;
  }
  if (typeof value !== "object" || value === null) return null;
  const { accessToken, refreshToken, expiresAt } = value as Record<string, unknown>;
  if (typeof accessToken !== "string" || accessToken.length === 0) return null;
  return {
    accessToken,
    refreshToken: typeof refreshToken === "string" && refreshToken.length > 0 ? refreshToken : null,
    expiresAt: typeof expiresAt === "number" && Number.isFinite(expiresAt) ? expiresAt : null,
  };
}

/** The session the page opens with: the e2e fixture's (development and e2e builds only), else this tab's. */
export function bootSession(): Session | null {
  if (import.meta.env.MODE !== "production") {
    const fixture = parseSession(read("localStorage", E2E_SESSION_STORAGE_KEY));
    if (fixture !== null) return { accessToken: fixture.accessToken, refreshToken: null, expiresAt: null };
  }
  return parseSession(read("sessionStorage", SESSION_STORAGE_KEY));
}

export function storeSession(session: Session): void {
  try {
    storage("sessionStorage")?.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
  } catch {
    // The page keeps the session in memory until it is left.
  }
}

/** Sign-out's half on this device: the tab's session, and (development and e2e builds) the fixture's. */
export function forgetSession(): void {
  remove("sessionStorage", SESSION_STORAGE_KEY);
  if (import.meta.env.MODE !== "production") remove("localStorage", E2E_SESSION_STORAGE_KEY);
}

/** Whether a request should renew the session first: close to expiry, renewable, and not just renewed. */
export function dueForRenewal(session: Session, now: number, lastRenewal: number | null): boolean {
  if (session.refreshToken === null || session.expiresAt === null) return false;
  if (lastRenewal !== null && now - lastRenewal < RENEWAL_FLOOR_MS) return false;
  return session.expiresAt - now <= REFRESH_MARGIN_MS;
}

export type SessionKeeper = {
  session: Session | null;
  /** Why the last session ended, for the sign-in screen; `null` after a plain sign-out. */
  notice: string | null;
  adopt: (session: Session) => void;
  end: (notice: string | null) => void;
  token: TokenSource;
};

export function useSession(): SessionKeeper {
  const [session, setSession] = useState<Session | null>(bootSession);
  const [notice, setNotice] = useState<string | null>(null);
  const current = useRef(session);
  const lastRenewal = useRef<number | null>(null);

  const replace = useCallback((next: Session | null) => {
    current.current = next;
    setSession(next);
    if (next === null) forgetSession();
    else storeSession(next);
  }, []);

  const adopt = useCallback(
    (next: Session) => {
      setNotice(null);
      lastRenewal.current = null;
      replace(next);
    },
    [replace],
  );

  const end = useCallback(
    (message: string | null) => {
      const held = current.current;
      replace(null);
      setNotice(message);
      if (held !== null) void revokeSession(held);
    },
    [replace],
  );

  const renew = useCallback(async (): Promise<Session> => {
    const held = current.current;
    if (held === null || held.refreshToken === null) throw new AuthError("sessionEnded");
    try {
      const next = await refreshSession(held.refreshToken);
      lastRenewal.current = Date.now();
      if (current.current === held) replace(next);
      return current.current ?? next;
    } catch (cause) {
      if (cause instanceof AuthError && cause.failure === "sessionEnded" && current.current === held) end(cause.message);
      throw cause;
    }
  }, [replace, end]);

  const token = useCallback(async (): Promise<string> => {
    const held = current.current;
    if (held === null) throw new AuthError("sessionEnded");
    if (!dueForRenewal(held, Date.now(), lastRenewal.current)) return held.accessToken;
    try {
      return (await renew()).accessToken;
    } catch (cause) {
      // An unreachable provider leaves the held token for the server to judge; an ended session is ended.
      if (cause instanceof AuthError && cause.failure === "sessionEnded") throw cause;
      return held.accessToken;
    }
  }, [renew]);

  // Renew ahead of expiry, so a dashboard left open stays signed in.
  useEffect(() => {
    if (session === null || session.refreshToken === null || session.expiresAt === null) return;
    const floor = lastRenewal.current === null ? 0 : lastRenewal.current + RENEWAL_FLOOR_MS;
    const at = Math.max(session.expiresAt - REFRESH_MARGIN_MS, floor);
    const timer = setTimeout(
      () => {
        renew().catch(() => undefined);
      },
      Math.max(0, at - Date.now()),
    );
    return () => {
      clearTimeout(timer);
    };
  }, [session, renew]);

  return { session, notice, adopt, end, token };
}
