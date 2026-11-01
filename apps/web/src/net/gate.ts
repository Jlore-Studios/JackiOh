// Client view of SPEC §9.4's server gate (CLAUDE.md rule 7): `/api/auth/me` sends pending accounts
// to `/invite` and anonymous ones to `/login`. R194 renews once near expiry or after a 401. R632
// re-reads a tab-owned session after storage or back/forward-cache changes. R2630000: a new screen's
// gate starts from the page's last answer for an active account that owes nothing and reads again.

import { useEffect, useRef, useState } from "react";

import {
  AUTH_SESSION_REFRESH_MARGIN_SECONDS,
  AUTH_SESSION_RENEWAL_FLOOR_SECONDS,
} from "@jackioh/server-config";
import { sessionIdFromToken, subjectFromToken } from "../auth/redirect.ts";
import { ApiRequestError, getMe, retryAfterMsOf, type MeResponse } from "./api.ts";
import { AuthError, authConfig, refreshSession, revokeSession, sessionNearExpiry } from "./auth.ts";
import { cachedRead, forgetRead, rememberRead } from "./readCache.ts";
import {
  E2E_SESSION_STORAGE_KEY,
  SESSION_STORAGE_KEY,
  clearSession,
  readSession,
  writeSession,
  type Session,
} from "./session.ts";

export type Account =
  | { kind: "loading" }
  /** No session, or the server refused the token; `expired`: a refused renewal cleared it (R194). */
  | { kind: "anonymous"; reason?: "expired" }
  /**
   * Unreachable, or an unexpected answer. `retry` reads again without a page load (`gate-retry`);
   * `retryAfterMs` is a rate limit's wait (R192), counted down before `retry` is offered.
   */
  | { kind: "error"; message: string; retry?: () => void; retryAfterMs?: number }
  | {
      kind: "ready";
      token: string;
      me: MeResponse;
      /** When `token` expires (epoch ms, this device's clock), when the session says. */
      expiresAt?: number | null;
      /** The session has a refresh token, so the gate can keep `token` fresh (R194). */
      renewable?: boolean;
    };

function renewableToken(session: Session): string | null {
  const token = session.refreshToken;
  if (typeof token !== "string" || token.length === 0) return null;
  // A build with no provider configured cannot renew anything.
  if (authConfig() === null) return null;
  return token;
}

function isUnauthorized(cause: unknown): boolean {
  return cause instanceof ApiRequestError && cause.status === 401;
}

/** What the gate says for R109's API limit; the wait is counted down beside it (R192). */
export const GATE_RATE_LIMITED_MESSAGE = "Too many requests from this account or network. Wait a moment, then try again.";

/** The error's own sentence (a transport failure's is already plain); a rate limit gets ours and its wait (R192). */
function errorAccount(cause: unknown, retry: () => void): Account {
  if (cause instanceof ApiRequestError && (cause.status === 429 || cause.code === "rate_limited")) {
    const wait = retryAfterMsOf(cause);
    return {
      kind: "error",
      message: GATE_RATE_LIMITED_MESSAGE,
      retry,
      ...(wait === null ? {} : { retryAfterMs: wait }),
    };
  }
  return { kind: "error", message: cause instanceof Error ? cause.message : String(cause), retry };
}

/** A screen's own failed read: the error's own sentence, and the wait a rate limit stated (R192). */
export type ReadFailure = { message: string; retryAfterMs?: number };

export function readFailureOf(cause: unknown): ReadFailure {
  const wait = retryAfterMsOf(cause);
  return {
    message: cause instanceof Error ? cause.message : String(cause),
    ...(wait === null ? {} : { retryAfterMs: wait }),
  };
}

type Renewal = { ok: true; session: Session } | { ok: false; account: Account };

/** Two tokens that both name a user id, and not the same one. */
function differentAccounts(a: string, b: string): boolean {
  const first = subjectFromToken(a);
  const second = subjectFromToken(b);
  return first !== null && second !== null && first !== second;
}

/**
 * Renew only the session that started the read: a rotated refresh token must be retained, but a
 * changed store wins; sign-out revokes the result and an account mismatch ends the session.
 */
async function renew(stale: Session, refreshToken: string, retry: () => void): Promise<Renewal> {
  const stored = readSession();
  if (stored === null) return { ok: false, account: { kind: "anonymous" } };
  if (stored.accessToken !== stale.accessToken) return { ok: true, session: stored };
  try {
    const session = await refreshSession(refreshToken);
    const now = readSession();
    if (now === null) {
      void revokeSession(session.accessToken);
      return { ok: false, account: { kind: "anonymous" } };
    }
    if (now.accessToken !== stale.accessToken) return { ok: true, session: now };
    if (differentAccounts(stale.accessToken, session.accessToken)) {
      clearSession();
      void revokeSession(session.accessToken);
      return { ok: false, account: { kind: "anonymous", reason: "expired" } };
    }
    writeSession(session);
    return { ok: true, session };
  } catch (cause) {
    if (cause instanceof AuthError && cause.failure === "sessionEnded") {
      // Only the session this renewal was for: one signed in meanwhile is not this one's to end.
      const now = readSession();
      if (now === null || now.accessToken === stale.accessToken) clearSession();
      return { ok: false, account: { kind: "anonymous", reason: "expired" } };
    }
    return { ok: false, account: errorAccount(cause, retry) };
  }
}

/** The account a session and its `/api/auth/me` answer make. */
/** R2630000: the path the gate's last answer is kept under, among the menus' (`readCache.ts`). */
const ME_PATH = "/api/auth/me";

/**
 * R2630000: an answer a new screen may open on before the server answers again: an active account
 * that owes nothing. A redirect, the banned panel and a prompt never rest on a kept answer.
 */
function opensAtOnce(me: MeResponse): boolean {
  return (
    me.profile.status === "active" &&
    me.ageCheckOwed !== true &&
    me.termsOwed !== true &&
    me.username?.promptOwed !== true
  );
}

/** R2630000: what a gate starts from: the page's last answer for this tab's session, or `loading`. */
function startingAccount(): Account {
  const session = readSession();
  if (session === null) return { kind: "loading" };
  const me = cachedRead<MeResponse>(session.accessToken, ME_PATH);
  return me !== undefined && opensAtOnce(me) ? readyAccount(session, me) : { kind: "loading" };
}

function readyAccount(session: Session, me: MeResponse): Account {
  return {
    kind: "ready",
    token: session.accessToken,
    me,
    expiresAt: typeof session.expiresAt === "number" && Number.isFinite(session.expiresAt) ? session.expiresAt : null,
    renewable: renewableToken(session) !== null,
  };
}

async function resolveAccount(retry: () => void): Promise<Account> {
  let session = readSession();
  if (session === null) return { kind: "anonymous" };

  let renewed = false;
  const early = renewableToken(session);
  if (early !== null && sessionNearExpiry(session, Date.now())) {
    const renewal = await renew(session, early, retry);
    if (!renewal.ok) return renewal.account;
    session = renewal.session;
    renewed = true;
  }

  try {
    const me = await getMe(session.accessToken);
    return readyAccount(session, me);
  } catch (cause) {
    if (!isUnauthorized(cause)) return errorAccount(cause, retry);
  }

  // `/api/auth/me` said 401. A token the provider no longer accepts is the same as having none,
  // unless there is a refresh token that has not been spent yet on this read.
  const late = renewed ? null : renewableToken(session);
  if (late === null) return { kind: "anonymous" };

  const renewal = await renew(session, late, retry);
  if (!renewal.ok) return renewal.account;
  try {
    const me = await getMe(renewal.session.accessToken);
    return readyAccount(renewal.session, me);
  } catch (cause) {
    // Renewed once already; a second 401 is the ordinary "sign in again".
    if (isUnauthorized(cause)) return { kind: "anonymous" };
    return errorAccount(cause, retry);
  }
}

const ACCOUNT_CHANGED = "jackioh:account-changed";

/**
 * Tells every `useAccount` in this tab to read again: a screen's own request renewed the session, or
 * the server answered as if the account's status had moved on (another tab already redeemed).
 */
export function announceAccountChange(): void {
  if (typeof window === "undefined") return;
  // R2630000: the next screen's gate waits for the account as it now is.
  forgetRead(readSession()?.accessToken ?? null, ME_PATH);
  window.dispatchEvent(new Event(ACCOUNT_CHANGED));
}

export type UnauthorizedOutcome =
  /** The session was renewed (here or already elsewhere): retry once with `token`. */
  | { kind: "renewed"; token: string }
  /** The session is over: sign in again. `expired` when the provider refused the renewal (R194). */
  | { kind: "signedOut"; expired: boolean }
  /** Another tab signed in as another account: a request made for one is never sent again as another. */
  | { kind: "switched" }
  /** The provider could not be reached; the session is kept. */
  | { kind: "failed" };

/** `renewed`, unless the session to retry with belongs to another account than `token` did. */
function renewedAs(token: string, next: string): UnauthorizedOutcome {
  return differentAccounts(token, next) ? { kind: "switched" } : { kind: "renewed", token: next };
}

/**
 * R194 for a screen's own request (the gate covers `/api/auth/me`) refused as unauthorised: renew the
 * stored session once, sharing the provider call with any renewal already in flight.
 */
export async function renewAfterUnauthorized(token: string): Promise<UnauthorizedOutcome> {
  const session = readSession();
  if (session === null) return { kind: "signedOut", expired: false };
  // Someone renewed while this request was out: retry with what storage holds now.
  if (session.accessToken !== token) return renewedAs(token, session.accessToken);
  const refreshToken = renewableToken(session);
  if (refreshToken === null) return { kind: "signedOut", expired: false };
  const renewal = await renew(session, refreshToken, () => undefined);
  if (renewal.ok) return renewedAs(token, renewal.session.accessToken);
  if (renewal.account.kind === "anonymous") {
    return { kind: "signedOut", expired: renewal.account.reason === "expired" };
  }
  return { kind: "failed" };
}

/** `switched`: the device moved to another account, and every screen has been told to read it again. */
export type RenewedCall<T> =
  | { kind: "ok"; value: T }
  | { kind: "signedOut"; expired: boolean }
  | { kind: "switched" };

/**
 * R194: `call` with `token`; on a 401 renew once, tell every screen in this tab, and call again. A
 * refusal that is not a 401, or a renewal that could not reach the provider, is the call's own error.
 */
export async function callWithRenewal<T>(token: string, call: (token: string) => Promise<T>): Promise<RenewedCall<T>> {
  try {
    return { kind: "ok", value: await call(token) };
  } catch (cause) {
    if (!isUnauthorized(cause)) throw cause;
    const renewal = await renewAfterUnauthorized(token);
    if (renewal.kind === "signedOut") return { kind: "signedOut", expired: renewal.expired };
    if (renewal.kind === "failed") throw cause;
    announceAccountChange();
    if (renewal.kind === "switched") return { kind: "switched" };
    return { kind: "ok", value: await call(renewal.token) };
  }
}

/** What a call made with no usable session gets: the refusal the server gives a missing token. */
const SIGNED_OUT = { code: "unauthorized", message: "Sign in to continue." } as const;

/**
 * `call` as the tab's signed-in account (#936: the social and Duos wrappers, whose S8 signatures
 * take no token), read at each call so a renewal made meanwhile is the one sent.
 */
export async function withSession<T>(call: (token: string) => Promise<T>): Promise<T> {
  const session = readSession();
  if (session === null) throw new ApiRequestError(401, SIGNED_OUT);
  const answer = await callWithRenewal(session.accessToken, call);
  if (answer.kind === "ok") return answer.value;
  throw new ApiRequestError(401, SIGNED_OUT);
}

/** Two tokens of the same provider session (a renewal rotated it), when both name one. */
function sameSession(a: string, b: string): boolean {
  const first = sessionIdFromToken(a);
  return first !== null && first === sessionIdFromToken(b);
}

function tokenNearExpiry(account: Extract<Account, { kind: "ready" }>, now: number): boolean {
  const expiresAt = account.expiresAt;
  return typeof expiresAt === "number" && expiresAt - now <= AUTH_SESSION_REFRESH_MARGIN_SECONDS * 1000;
}

/**
 * What a read of the account leaves on screen, given what is there now. A first read and the
 * player's own retry start from `loading`, so they always show what they found.
 */
export function settleAccount(previous: Account, next: Account, now: number): Account {
  if (previous.kind !== "ready") return next;
  if (next.kind === "error") {
    // A background re-read failed. The screen stays for the account it was opened for, unless the
    // device now holds another account's session (or none, which reads as anonymous, not error).
    const stored = readSession();
    if (stored !== null && !differentAccounts(previous.token, stored.accessToken)) return previous;
    return next;
  }
  if (
    next.kind === "ready" &&
    next.token !== previous.token &&
    next.me.profile.id === previous.me.profile.id &&
    sameSession(previous.token, next.token) &&
    !tokenNearExpiry(previous, now)
  ) {
    // The same session, renewed elsewhere: the token the screen holds is as good for a while yet.
    return { ...next, token: previous.token, expiresAt: previous.expiresAt ?? null };
  }
  return next;
}

/** `setTimeout`'s own ceiling (a signed 32-bit millisecond count); a longer wait fires at once. */
const LONGEST_TIMER_MS = 2_147_483_647;

/**
 * R194: re-read near expiry and when a sleeping page wakes, never more often than
 * `AUTH_SESSION_RENEWAL_FLOOR_SECONDS`, so short-lived tokens cannot loop renewals.
 */
function useRenewalBeforeExpiry(account: Account, reread: () => void): void {
  const renewAt =
    account.kind === "ready" && account.renewable === true && typeof account.expiresAt === "number"
      ? account.expiresAt - AUTH_SESSION_REFRESH_MARGIN_SECONDS * 1000
      : null;
  const lastRenewal = useRef<number | null>(null);

  useEffect(() => {
    if (renewAt === null) return;
    const earliest = (): number =>
      Math.max(renewAt, lastRenewal.current === null ? 0 : lastRenewal.current + AUTH_SESSION_RENEWAL_FLOOR_SECONDS * 1000);
    // No once-only guard: a renewal that failed keeps the account (and so this effect) as it was,
    // and the page coming back into view may try again, never sooner than the floor allows.
    let timer: number | undefined;
    const schedule = (): void => {
      window.clearTimeout(timer);
      timer = window.setTimeout(fire, Math.min(Math.max(0, earliest() - Date.now()), LONGEST_TIMER_MS));
    };
    // A timer runs on the event loop's cached clock, which a long task leaves behind the wall clock,
    // so it can come due before `Date.now()` says: then it waits out the rest rather than drop the renewal.
    function fire(): void {
      if (Date.now() < earliest()) {
        schedule();
        return;
      }
      lastRenewal.current = Date.now();
      reread();
    }
    const onWake = (): void => {
      if (document.visibilityState !== "hidden") fire();
    };
    schedule();
    document.addEventListener("visibilitychange", onWake);
    window.addEventListener("focus", onWake);
    window.addEventListener("pageshow", onWake);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener("visibilitychange", onWake);
      window.removeEventListener("focus", onWake);
      window.removeEventListener("pageshow", onWake);
    };
  }, [renewAt, reread]);
}

/** Reads and, when needed, renews the session before `/api/auth/me`. */
export function useAccount(): Account {
  const [attempt, setAttempt] = useState(0);
  const [account, setAccount] = useState<Account>(startingAccount);
  const [reread] = useState(() => (): void => {
    setAttempt((count) => count + 1);
  });

  useRenewalBeforeExpiry(account, reread);

  // R2630000: each answer is kept for the next screen's gate, by account, in the tab's memory.
  useEffect(() => {
    if (account.kind === "ready") rememberRead(account.token, ME_PATH, account.me);
  }, [account]);

  useEffect(() => {
    let canceled = false;
    const retry = (): void => {
      setAccount({ kind: "loading" });
      setAttempt((count) => count + 1);
    };

    resolveAccount(retry)
      .then((next) => {
        if (!canceled) setAccount((previous) => settleAccount(previous, next, Date.now()));
      })
      .catch((cause: unknown) => {
        if (!canceled) setAccount((previous) => settleAccount(previous, errorAccount(cause, retry), Date.now()));
      });

    // Another tab signed out, signed in or renewed: read the account again. What is on screen
    // stays until the new answer arrives, so a renewal elsewhere does not flash "Checking…".
    const onStorage = (event: StorageEvent): void => {
      if (event.key !== null && event.key !== SESSION_STORAGE_KEY && event.key !== E2E_SESSION_STORAGE_KEY) return;
      setAttempt((count) => count + 1);
    };
    const onChanged = (): void => {
      setAttempt((count) => count + 1);
    };
    // Restored from the back/forward cache: the account read before it was frozen may be gone.
    const onPageShow = (event: PageTransitionEvent): void => {
      if (event.persisted) setAttempt((count) => count + 1);
    };
    window.addEventListener("storage", onStorage);
    window.addEventListener(ACCOUNT_CHANGED, onChanged);
    window.addEventListener("pageshow", onPageShow);

    return () => {
      canceled = true;
      window.removeEventListener("storage", onStorage);
      window.removeEventListener(ACCOUNT_CHANGED, onChanged);
      window.removeEventListener("pageshow", onPageShow);
    };
  }, [attempt]);

  return account;
}
