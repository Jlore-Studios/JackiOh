// Emailed auth links, read once and scrubbed (R193, R323, R324).
// `/login` exchanges a one-time PKCE code (R323); legacy sessions and errors arrive in fragments or queries (R324).
// Only `/login` and `/` treat code as auth; parsing never shows provider text and scrubbing happens before rendering.
// Recovery stays tab-local; email needs server acceptance; navigation after an auth link must use `paths`.

import { isAuthCode, revokeSignedOutSession } from "../net/auth.ts";
import { paths } from "../net/navigate.ts";
import type { Session } from "../net/session.ts";

export type SessionLinkType = "signup" | "invite" | "magiclink" | "email_change";

export type AuthRedirect =
  | { kind: "none" }
  | { kind: "session"; session: Session; email: string | null; linkType: SessionLinkType }
  | { kind: "recovery"; session: Session; email: string | null }
  /** R323: a PKCE link's one-time code, for `/login` to exchange. Never a session by itself. */
  | { kind: "code"; code: string }
  | { kind: "error"; failure: "linkExpired" | "linkDenied" };

const AUTH_PARAMETERS: readonly string[] = [
  "access_token",
  "refresh_token",
  "expires_at",
  "expires_in",
  "token_type",
  "type",
  "error",
  "error_code",
  "error_description",
];

const CODE_PARAMETER = "code";

const CODE_PATHS: ReadonlySet<string> = new Set([paths.login, paths.landing]);

function codeCounts(url: URL): boolean {
  return CODE_PATHS.has(url.pathname) && url.searchParams.has(CODE_PARAMETER);
}

const SESSION_TYPES: ReadonlySet<string> = new Set<SessionLinkType>(["signup", "invite", "magiclink", "email_change"]);

function isSessionLinkType(type: string): type is SessionLinkType {
  return SESSION_TYPES.has(type);
}

const NONE: AuthRedirect = { kind: "none" };

function fragmentParams(url: URL): URLSearchParams {
  return new URLSearchParams(url.hash.startsWith("#") ? url.hash.slice(1) : url.hash);
}

function hasAuthParameter(params: URLSearchParams): boolean {
  return AUTH_PARAMETERS.some((name) => params.has(name));
}

function positiveNumber(raw: string | null): number | null {
  if (raw === null || raw.trim().length === 0) return null;
  const value = Number(raw);
  return Number.isFinite(value) && value > 0 ? value : null;
}

function tokenClaim(token: string, name: "email" | "sub" | "session_id"): string | null {
  const parts = token.split(".");
  const payload = parts[1];
  if (payload === undefined || payload.length === 0) return null;
  try {
    const base64 = payload.replace(/-/gu, "+").replace(/_/gu, "/");
    const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    const claims: unknown = JSON.parse(new TextDecoder().decode(bytes));
    if (typeof claims !== "object" || claims === null) return null;
    const value = (claims as Record<string, unknown>)[name];
    return typeof value === "string" && value.length > 0 ? value : null;
  } catch {
    return null;
  }
}

/** Decodes an unverified email claim; `/login` acts only after server acceptance. */
export function emailFromToken(token: string): string | null {
  return tokenClaim(token, "email");
}

/** Decodes an unverified account claim to compare renewed and stored sessions. */
export function subjectFromToken(token: string): string | null {
  return tokenClaim(token, "sub");
}

/** Decodes an unverified session claim to distinguish a renewal from a new sign-in. */
export function sessionIdFromToken(token: string): string | null {
  return tokenClaim(token, "session_id");
}

function readParams(params: URLSearchParams): AuthRedirect {
  // An error outranks any token beside it. `error_description` is never read.
  const errorCode = params.get("error_code");
  if (params.has("error") || errorCode !== null) {
    return { kind: "error", failure: errorCode === "otp_expired" ? "linkExpired" : "linkDenied" };
  }

  const accessToken = params.get("access_token");
  const type = params.get("type");
  if (accessToken === null || accessToken.length === 0 || type === null) return NONE;
  if (type !== "recovery" && !isSessionLinkType(type)) return NONE;

  const refreshToken = params.get("refresh_token");
  const session: Session = {
    accessToken,
    refreshToken: refreshToken !== null && refreshToken.length > 0 ? refreshToken : null,
    expiresAt: null,
  };
  // Use this device's clock for `expires_in`; `expires_at` is the fallback.
  const expiresAt = positiveNumber(params.get("expires_at"));
  const expiresIn = positiveNumber(params.get("expires_in"));
  if (expiresIn !== null) {
    session.expiresAt = Date.now() + expiresIn * 1000;
  } else if (expiresAt !== null) {
    session.expiresAt = expiresAt * 1000;
  }

  const email = emailFromToken(accessToken);
  if (isSessionLinkType(type)) return { kind: "session", session, email, linkType: type };
  return { kind: "recovery", session, email };
}

/** Reads the fragment before the query; errors win, and R323 codes count only on `/login` and `/`. */
export function parseAuthRedirect(url: URL): AuthRedirect {
  const fromFragment = readParams(fragmentParams(url));
  if (fromFragment.kind !== "none") return fromFragment;
  const fromQuery = readParams(url.searchParams);
  if (fromQuery.kind !== "none" || !codeCounts(url)) return fromQuery;
  const code = url.searchParams.get(CODE_PARAMETER) ?? "";
  return isAuthCode(code) ? { kind: "code", code } : NONE;
}

function carriesAuth(url: URL): boolean {
  return hasAuthParameter(fragmentParams(url)) || hasAuthParameter(url.searchParams) || codeCounts(url);
}

let consumed: AuthRedirect | null = null;

/** R193: scrub auth URL data before rendering and cache the result for StrictMode. */
export function consumeAuthRedirect(): AuthRedirect {
  if (consumed !== null) return consumed;

  const url = new URL(window.location.href);
  if (!carriesAuth(url)) return NONE;

  const result = parseAuthRedirect(url);
  try {
    window.history.replaceState(null, "", url.pathname);
  } catch {
    // A sandboxed frame may refuse; the tokens are still never rendered or stored from here.
  }
  consumed = result;
  return result;
}

export function clearConsumedAuthRedirect(): void {
  consumed = null;
}

/** R193: consume links at boot before lazy login leaves a token in the URL, then move non-login links. */
export function adoptAuthRedirect(loginPathname: string): boolean {
  if (typeof window === "undefined") return false;
  const url = new URL(window.location.href);
  if (!carriesAuth(url)) return false;
  consumeAuthRedirect();
  if (url.pathname === loginPathname) return false; // already scrubbed; `/login` reads the cache
  try {
    window.history.replaceState(null, "", loginPathname);
  } catch {
    return false;
  }
  return true;
}

// Recovery is tab-local (`sessionStorage`, never `localStorage`), so an unrepeatable link survives reload.

export const RECOVERY_STORAGE_KEY = "jackioh.auth.recovery";

export type HeldRecovery = { session: Session; email: string | null };

let recovery: HeldRecovery | null = null;

function writeRecoveryMirror(value: HeldRecovery | null): void {
  if (typeof window === "undefined") return;
  try {
    if (value === null) window.sessionStorage.removeItem(RECOVERY_STORAGE_KEY);
    else window.sessionStorage.setItem(RECOVERY_STORAGE_KEY, JSON.stringify(value));
  } catch {
    // Blocked storage: the memory copy still serves this page; a reload asks for a new link.
  }
}

/** Treat the storage mirror as untrusted: only a session with an access token survives. */
function readRecoveryMirror(): HeldRecovery | null {
  if (typeof window === "undefined") return null;
  let raw: string | null;
  try {
    raw = window.sessionStorage.getItem(RECOVERY_STORAGE_KEY);
  } catch {
    return null;
  }
  if (raw === null) return null;
  try {
    const value = JSON.parse(raw) as { session?: unknown; email?: unknown } | null;
    const session = value?.session as { accessToken?: unknown; refreshToken?: unknown; expiresAt?: unknown } | undefined;
    if (typeof session?.accessToken !== "string" || session.accessToken.length === 0) return null;
    return {
      session: {
        accessToken: session.accessToken,
        refreshToken: typeof session.refreshToken === "string" ? session.refreshToken : null,
        expiresAt: typeof session.expiresAt === "number" ? session.expiresAt : null,
      },
      email: typeof value?.email === "string" ? value.email : null,
    };
  } catch {
    return null;
  }
}

export function holdRecoverySession(session: Session, email: string | null): void {
  recovery = { session, email };
  writeRecoveryMirror(recovery);
}

/** An expired recovery session is usable only with a refresh token (R194). */
export function recoverySession(): HeldRecovery | null {
  const held = recovery ?? readRecoveryMirror();
  if (held === null) return null;
  const expiresAt = held.session.expiresAt;
  const renewable = typeof held.session.refreshToken === "string" && held.session.refreshToken.length > 0;
  if (!renewable && typeof expiresAt === "number" && Number.isFinite(expiresAt) && expiresAt <= Date.now()) {
    releaseRecoverySession();
    return null;
  }
  recovery = held;
  return held;
}

export function releaseRecoverySession(): void {
  cancelScheduledAbandon();
  recovery = null;
  writeRecoveryMirror(null);
}

/** On unsaved exit, revoke recovery. Renew expired access first (R194); best effort. */
export function abandonRecoverySession(): void {
  const held = recovery ?? readRecoveryMirror();
  releaseRecoverySession();
  if (held !== null) void revokeSignedOutSession(held.session);
}

let scheduledAbandon: ReturnType<typeof setTimeout> | null = null;

/** Defer abandonment so StrictMode's development remount can retain the session. */
export function abandonRecoverySessionSoon(): void {
  cancelScheduledAbandon();
  scheduledAbandon = setTimeout(() => {
    scheduledAbandon = null;
    abandonRecoverySession();
  }, 0);
}

export function keepRecoverySession(): void {
  cancelScheduledAbandon();
}

function cancelScheduledAbandon(): void {
  if (scheduledAbandon === null) return;
  clearTimeout(scheduledAbandon);
  scheduledAbandon = null;
}
