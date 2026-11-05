// The account's own sign-in, against its Supabase project directly (as apps/web/src/net/auth.ts does):
// email and password, or a code mailed to the address (so an account that signs in with Google alone
// can sign in here too), then the authenticator app's code when the account has one. None of this is
// the lock: the server checks the token, `DEV_PROFILE_IDS` and, when strict, aal2 on every route.
// The provider's own words are never shown (R160): each refusal is one of this module's sentences.

import { AUTH_PROVIDER_TIMEOUT_SECONDS, MS_PER_SECOND } from "./wire.ts";

export type Session = {
  accessToken: string;
  /** `null` for a session that cannot be renewed (the e2e fixture's, session.ts). */
  refreshToken: string | null;
  /** Epoch ms, by this device's clock; `null` when unknown. */
  expiresAt: number | null;
};

export type AuthFailure =
  | "credentials"
  | "codeInvalid"
  | "totpInvalid"
  | "rateLimited"
  | "sessionEnded"
  | "network"
  | "service"
  | "unconfigured";

export const AUTH_MESSAGES: Readonly<Record<AuthFailure, string>> = {
  credentials: "That email and password do not match an account.",
  codeInvalid: "That code didn't work. Check the newest email, or ask for a new code.",
  totpInvalid: "That code didn't match. Type the 6-digit code your authenticator app shows now.",
  rateLimited: "Too many attempts. Wait a few minutes, then try again.",
  sessionEnded: "Your session has ended. Sign in again.",
  network: "Couldn't reach the sign-in service. Check your connection and try again.",
  service: "The sign-in service had a problem. Try again in a minute.",
  unconfigured:
    "Sign-in isn't set up in this build: it was built without VITE_SUPABASE_URL and VITE_SUPABASE_PUBLISHABLE_KEY.",
};

/** R192: the same notice for every address, so the form tells no one which addresses have accounts. */
export const EMAIL_CODE_SENT =
  "If that address has an account, an email with a sign-in code is on its way. Type the code below.";

export class AuthError extends Error {
  readonly failure: AuthFailure;

  constructor(failure: AuthFailure) {
    super(AUTH_MESSAGES[failure]);
    this.name = "AuthError";
    this.failure = failure;
  }
}

export type AuthConfig = { url: string; publishableKey: string };

/** The build's Supabase project (.env.production, .env.staging); `null` in a build without one. */
export function authConfig(): AuthConfig | null {
  const url: unknown = import.meta.env.VITE_SUPABASE_URL;
  const publishableKey: unknown = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY;
  if (typeof url !== "string" || url.length === 0) return null;
  if (typeof publishableKey !== "string" || publishableKey.length === 0) return null;
  return { url: url.replace(/\/+$/u, ""), publishableKey };
}

function requireConfig(): AuthConfig {
  const config = authConfig();
  if (config === null) throw new AuthError("unconfigured");
  return config;
}

// One request

type ProviderRequest = { method: "GET" | "POST"; path: string; body?: Record<string, unknown>; bearer?: string };

const TOO_MANY_REQUESTS = 429;
const FIRST_SERVER_ERROR = 500;
const UNAUTHORIZED = 401;
const FORBIDDEN = 403;

const isSuccess = (status: number): boolean => status >= 200 && status < 300;

/** One call to the provider, bounded by the server's `AUTH_PROVIDER_TIMEOUT_SECONDS` even when a fetch ignores abort. */
async function send(config: AuthConfig, request: ProviderRequest): Promise<{ status: number; json: unknown }> {
  const headers: Record<string, string> = {
    apikey: config.publishableKey,
    authorization: `Bearer ${request.bearer ?? config.publishableKey}`,
  };
  if (request.body !== undefined) headers["content-type"] = "application/json";
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timedOut = new Promise<never>((_resolve, reject) => {
    timer = setTimeout(() => {
      controller.abort();
      reject(new AuthError("network"));
    }, AUTH_PROVIDER_TIMEOUT_SECONDS * MS_PER_SECOND);
  });
  try {
    let response: Response;
    try {
      response = await Promise.race([
        fetch(`${config.url}${request.path}`, {
          method: request.method,
          headers,
          ...(request.body === undefined ? {} : { body: JSON.stringify(request.body) }),
          signal: controller.signal,
        }),
        timedOut,
      ]);
    } catch {
      throw new AuthError("network");
    }
    let json: unknown = null;
    try {
      json = await Promise.race([response.json() as Promise<unknown>, timedOut]);
    } catch {
      // A malformed or stalled body: the status still says what happened.
    }
    return { status: response.status, json };
  } finally {
    clearTimeout(timer);
  }
}

/** A refusal as one of this module's failures: `refused` is what a 4xx means on that endpoint. */
function refusal(status: number, refused: AuthFailure): AuthError {
  if (status === TOO_MANY_REQUESTS) return new AuthError("rateLimited");
  if (status >= FIRST_SERVER_ERROR) return new AuthError("service");
  return new AuthError(refused);
}

/** On a call made with the session's own token, a 401 or 403 means the provider no longer honors it. */
function sessionRefusal(status: number, refused: AuthFailure): AuthError {
  return status === UNAUTHORIZED || status === FORBIDDEN ? new AuthError("sessionEnded") : refusal(status, refused);
}

type TokenResponse = { access_token?: unknown; refresh_token?: unknown; expires_in?: unknown; expires_at?: unknown };

/** The session a token answer holds, `expires_in` read against this device's clock; `null` without a token. */
export function sessionFromTokens(body: unknown, fallbackRefreshToken: string | null, now = Date.now()): Session | null {
  const tokens = (typeof body === "object" && body !== null ? body : {}) as TokenResponse;
  if (typeof tokens.access_token !== "string" || tokens.access_token.length === 0) return null;
  const refreshToken =
    typeof tokens.refresh_token === "string" && tokens.refresh_token.length > 0 ? tokens.refresh_token : fallbackRefreshToken;
  let expiresAt: number | null = null;
  if (typeof tokens.expires_in === "number" && Number.isFinite(tokens.expires_in) && tokens.expires_in > 0) {
    expiresAt = now + tokens.expires_in * MS_PER_SECOND;
  } else if (typeof tokens.expires_at === "number" && Number.isFinite(tokens.expires_at)) {
    expiresAt = tokens.expires_at * MS_PER_SECOND;
  }
  return { accessToken: tokens.access_token, refreshToken, expiresAt };
}

// Signing in

/** Email and password (`grant_type=password`). */
export async function signInWithPassword(email: string, password: string): Promise<Session> {
  const { status, json } = await send(requireConfig(), {
    method: "POST",
    path: "/auth/v1/token?grant_type=password",
    body: { email, password },
  });
  if (!isSuccess(status)) throw refusal(status, "credentials");
  const session = sessionFromTokens(json, null);
  if (session === null) throw new AuthError("service");
  return session;
}

/**
 * Mails a sign-in code to an existing account (`create_user: false`). Every answer but a rate limit or a
 * fault reads as sent (R192), so the form is no way to learn which addresses have accounts.
 */
export async function requestEmailCode(email: string): Promise<void> {
  const { status } = await send(requireConfig(), {
    method: "POST",
    path: "/auth/v1/otp",
    body: { email, create_user: false },
  });
  if (status === TOO_MANY_REQUESTS || status >= FIRST_SERVER_ERROR) throw refusal(status, "service");
}

/** A code is often pasted as `123 456` or `123-456`. */
export function normalizeCode(raw: string): string {
  return raw.replace(/[\s-]/gu, "");
}

/** The mailed code (`POST /auth/v1/verify`, type `email`). */
export async function verifyEmailCode(email: string, code: string): Promise<Session> {
  const { status, json } = await send(requireConfig(), {
    method: "POST",
    path: "/auth/v1/verify",
    body: { type: "email", email, token: normalizeCode(code) },
  });
  if (!isSuccess(status)) throw refusal(status, "codeInvalid");
  const session = sessionFromTokens(json, null);
  if (session === null) throw new AuthError("codeInvalid");
  return session;
}

// The second step: an authenticator app (TOTP)

/** Base64 encodes in blocks of this many characters, which `atob` wants whole (a JWT drops the padding). */
const BASE64_BLOCK = 4;

/** The assurance level an access token's own payload claims (`aal`), read and never verified. */
export function assuranceLevel(accessToken: string): string | null {
  const payload = accessToken.split(".")[1];
  if (payload === undefined || payload.length === 0) return null;
  try {
    const base64 = payload.replace(/-/gu, "+").replace(/_/gu, "/");
    const padded = base64.padEnd(Math.ceil(base64.length / BASE64_BLOCK) * BASE64_BLOCK, "=");
    const bytes = Uint8Array.from(atob(padded), (character) => character.charCodeAt(0));
    const claims: unknown = JSON.parse(new TextDecoder().decode(bytes));
    if (typeof claims !== "object" || claims === null) return null;
    const aal = (claims as Record<string, unknown>)["aal"];
    return typeof aal === "string" ? aal : null;
  } catch {
    return null;
  }
}

/**
 * The verified TOTP factor an `aal1` session must still complete, or `null` when it need not: the
 * session is past `aal1`, or the account has none. The factors are the user object's (`GET /auth/v1/user`).
 */
export async function pendingSecondFactor(session: Session): Promise<string | null> {
  if (assuranceLevel(session.accessToken) !== "aal1") return null;
  const { status, json } = await send(requireConfig(), {
    method: "GET",
    path: "/auth/v1/user",
    bearer: session.accessToken,
  });
  if (!isSuccess(status)) throw sessionRefusal(status, "service");
  const factors = typeof json === "object" && json !== null ? (json as { factors?: unknown }).factors : undefined;
  if (!Array.isArray(factors)) return null;
  for (const entry of factors as unknown[]) {
    if (typeof entry !== "object" || entry === null) continue;
    const { id, factor_type: type, status: state } = entry as { id?: unknown; factor_type?: unknown; status?: unknown };
    if (typeof id === "string" && id.length > 0 && type === "totp" && state === "verified") return id;
  }
  return null;
}

function factorPath(factorId: string, suffix: "/challenge" | "/verify"): string {
  return `/auth/v1/factors/${encodeURIComponent(factorId)}${suffix}`;
}

/** Challenges the factor and verifies the app's code: the answer is the session at `aal2`. */
export async function verifySecondFactor(session: Session, factorId: string, code: string): Promise<Session> {
  const config = requireConfig();
  const challenge = await send(config, {
    method: "POST",
    path: factorPath(factorId, "/challenge"),
    bearer: session.accessToken,
  });
  if (!isSuccess(challenge.status)) throw sessionRefusal(challenge.status, "service");
  const challengeId = (challenge.json as { id?: unknown } | null)?.id;
  if (typeof challengeId !== "string" || challengeId.length === 0) throw new AuthError("service");
  const { status, json } = await send(config, {
    method: "POST",
    path: factorPath(factorId, "/verify"),
    body: { challenge_id: challengeId, code: normalizeCode(code) },
    bearer: session.accessToken,
  });
  if (!isSuccess(status)) throw sessionRefusal(status, "totpInvalid");
  const next = sessionFromTokens(json, session.refreshToken);
  if (next === null) throw new AuthError("service");
  return next;
}

// Keeping and ending a session

/** One renewal per refresh token at a time: the provider rotates it, so two would race (R194). */
const renewing = new Map<string, Promise<Session>>();

/** `grant_type=refresh_token`; an `aal2` session stays `aal2`. Throws `sessionEnded` when the provider refuses it. */
export function refreshSession(refreshToken: string): Promise<Session> {
  const running = renewing.get(refreshToken);
  if (running !== undefined) return running;
  const request = (async () => {
    const { status, json } = await send(requireConfig(), {
      method: "POST",
      path: "/auth/v1/token?grant_type=refresh_token",
      body: { refresh_token: refreshToken },
    });
    if (!isSuccess(status)) throw refusal(status, "sessionEnded");
    const session = sessionFromTokens(json, refreshToken);
    if (session === null) throw new AuthError("service");
    return session;
  })().finally(() => {
    renewing.delete(refreshToken);
  });
  renewing.set(refreshToken, request);
  return request;
}

/** Best effort: revokes this session alone (`scope=local`), so the dev's other sign-ins stay. Never throws. */
export async function revokeSession(session: Session): Promise<void> {
  const config = authConfig();
  if (config === null) return;
  try {
    await send(config, { method: "POST", path: "/auth/v1/logout?scope=local", bearer: session.accessToken });
  } catch {
    // The device's copy is gone already; an unrevoked token expires on its own.
  }
}
