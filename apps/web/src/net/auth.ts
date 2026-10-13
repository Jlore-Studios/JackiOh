// Client-side auth requests under SPEC §9.2; the server verifies returned tokens.
// `net/session.ts` owns storage (M8 e2e contract, A6); a verified account stays pending until §9.4's invite.
// Emailed links use PKCE when possible (R323, R324); the client never exposes provider text (R160).
// Auth flow rules: R192, R194, R663, R664, R665 and R666.
// BUILD M6-T1's server-only scope leaves client sign-in to this module.

import {
  AUTH_EMAIL_RESEND_COOLDOWN_SECONDS,
  AUTH_PROVIDER_TIMEOUT_SECONDS,
  AUTH_SESSION_REFRESH_MARGIN_SECONDS,
} from "@jackioh/server-config";
import { challengeForRequest, forgetVerifier, storedVerifiers, type PkceChallenge, type PkceFlow } from "../auth/pkce.ts";
import { ApiRequestError, apiRequest } from "./api.ts";
import { paths } from "./navigate.ts";
import {
  forgetPendingAddresses,
  readSession,
  rememberPendingEmail,
  rememberPendingReset,
  writeSession,
  type Session,
} from "./session.ts";

// Failures

/** Which provider call a refusal came from; the same answer means different things on each. */
export type AuthEndpoint =
  | "signIn"
  | "signUp"
  | "resend"
  | "recover"
  | "updatePassword"
  /** R663: an email change. */
  | "changeEmail"
  | "refresh"
  /** R664: mail a sign-in link and code. */
  | "otp"
  /** R664: a mailed sign-in code. */
  | "verifyOtp"
  /** R665: an authenticator app's code (enrolling, or signing in). */
  | "mfaVerify"
  /** R665: enrolling, listing or removing a factor. */
  | "mfa";

export type AuthFailure =
  | "credentials"
  /** R1443: a sign-in by username, refused. */
  | "usernameCredentials"
  /** R1443, R192: a sign-in by username, refused for going too fast, by name or by address. */
  | "usernameRateLimited"
  | "signUpRefused"
  | "rateLimited"
  | "emailRateLimited"
  | "weakPassword"
  | "weakPasswordLength"
  | "weakPasswordCharacters"
  | "weakPasswordPwned"
  | "samePassword"
  | "passwordTooLong"
  | "signUpInvalid"
  | "invalidEmail"
  | "emailChangeRefused"
  | "sameEmail"
  | "signupsClosed"
  | "linkExpired"
  | "sessionEnded"
  | "network"
  | "service"
  | "unconfigured"
  | "codeInvalid"
  | "mfaCodeInvalid"
  | "mfaUnavailable"
  | "oauthUnavailable"
  | "oauthFailed";

/** R160: account-dependent sign-in and sign-up outcomes use one refusal per endpoint. */
export const SIGN_IN_FAILED_MESSAGE = "That email and password do not match an account.";
/** R1443: the one refusal of a sign-in by username, as the server words it. */
export const USERNAME_SIGN_IN_FAILED_MESSAGE = "That username and password do not match an account.";
export const SIGN_UP_FAILED_MESSAGE = "Could not create that account.";

/** What a player reads when this build has no auth provider. The fix goes to the console instead. */
export const AUTH_UNCONFIGURED_MESSAGE = "Sign-in isn't available on this site right now.";

/** For whoever opens the console on a build with no auth provider; never shown on the page. */
const AUTH_UNCONFIGURED_DETAIL =
  "No auth provider configured: this build was made without VITE_SUPABASE_URL and " +
  "VITE_SUPABASE_PUBLISHABLE_KEY. Set both in the web app's .env (see its .env.example) and rebuild.";

/** R192: account-dependent failures use neutral sentences. */
export const AUTH_MESSAGES: Readonly<Record<AuthFailure, string>> = {
  credentials: SIGN_IN_FAILED_MESSAGE,
  usernameCredentials: USERNAME_SIGN_IN_FAILED_MESSAGE,
  usernameRateLimited:
    "Too many sign-in attempts with that username or from this network. Wait a few minutes, then try again.",
  signUpRefused: SIGN_UP_FAILED_MESSAGE,
  rateLimited: "Too many attempts from this network. Wait a few minutes, then try again.",
  emailRateLimited: "We couldn't send an email just now. Wait a few minutes, then try again.",
  // Provider policy can change, so this client promises no minimum.
  weakPassword: "That password is too weak. Choose a longer one that is harder to guess.",
  weakPasswordLength: "That password is too short. Choose a longer one.",
  weakPasswordCharacters:
    "That password needs more kinds of characters. Mix upper and lower case letters, digits and symbols.",
  weakPasswordPwned: "That password has appeared in a data breach. Choose one you haven't used anywhere else.",
  samePassword: "Choose a password different from your current one.",
  passwordTooLong: "That password is too long. Choose a shorter one.",
  signUpInvalid:
    "Check the email address and the password: one of them can't be used. A long password may be over the limit, so try a shorter one.",
  invalidEmail: "Enter a valid email address.",
  // R160, R663: address refusals must not reveal another account.
  emailChangeRefused: "That address can't be used for this account. Check it, or choose another.",
  sameEmail: "That is already your account's email address.",
  signupsClosed: "New accounts can't be created right now.",
  linkExpired: "That link has expired or was already used. Request a new one below.",
  sessionEnded: "Your session has ended. Sign in again.",
  network: "Couldn't reach the sign-in service. Check your connection and try again.",
  service: "The sign-in service had a problem. Try again in a minute.",
  unconfigured: AUTH_UNCONFIGURED_MESSAGE,
  // R664, R160: all email-code refusals use one sentence.
  codeInvalid: "That code didn't work. Check the newest email we sent, or ask for a new code.",
  mfaCodeInvalid: "That code didn't match. Type the 6-digit code your authenticator app shows now.",
  mfaUnavailable: "Two-step sign-in isn't available on this site right now.",
  oauthUnavailable: "That sign-in option doesn't work in this browser. Sign in with your email instead.",
  oauthFailed: "Signing in with that account didn't finish. Try again, or sign in with your email.",
};

/** Unit conversion, not configuration. */
const SECONDS_PER_MINUTE = 60;

/** Format the provider's mail interval for notices. */
function mailInterval(): { span: string; wait: string } {
  const seconds = AUTH_EMAIL_RESEND_COOLDOWN_SECONDS;
  if (seconds % SECONDS_PER_MINUTE === 0) {
    const minutes = seconds / SECONDS_PER_MINUTE;
    if (minutes === 1) return { span: "minute", wait: "a minute" };
    return { span: `${String(minutes)} minutes`, wait: `${String(minutes)} minutes` };
  }
  return { span: `${String(seconds)} seconds`, wait: `${String(seconds)} seconds` };
}

const MAIL_INTERVAL = mailInterval();

/** R192: give every address the same mail-interval notice. */
const MAIL_INTERVAL_NOTE =
  `If one went out in the last ${MAIL_INTERVAL.span}, from any device, this one can't: ` +
  `wait ${MAIL_INTERVAL.wait}, then ask again.`;

/** Screen notices. */
export const AUTH_NOTICES: Readonly<{
  signUpSent: string;
  resendSent: string;
  resetSent: string;
  emailConfirmed: string;
  emailChangeSent: string;
  emailChanged: string;
  sessionExpired: string;
  confirmFirst: string;
  checkingLink: string;
  checkingSlow: string;
  linkUnchecked: string;
  recoveryUnchecked: string;
  recoveryElsewhere: string;
  recoveryClaim: string;
  recoveryClaimMismatch: string;
  inviteNeedsPassword: string;
  inviteOnly: string;
  emailCodeSent: string;
  mfaRequired: string;
  mfaEnrolled: string;
  mfaRemoved: string;
}> = {
  signUpSent:
    "Check your email for a confirmation link, then sign in. No email after a few minutes? You may already have an account: sign in, or reset your password.",
  resendSent: `If that address has an account waiting for confirmation, a new link is on its way. ${MAIL_INTERVAL_NOTE}`,
  resetSent: `If that address has an account, a reset link is on its way. ${MAIL_INTERVAL_NOTE}`,
  emailConfirmed: "Your email is confirmed. Sign in to continue.",
  // R663: secure email change may require confirmation from both addresses.
  emailChangeSent:
    "Check the new address for a confirmation link. If your current address gets one too, open both. Your email changes once the change is confirmed; until then, sign in with your current one.",
  emailChanged: "Your new email address is confirmed. Sign in with it to continue.",
  sessionExpired: "Your session ended. Sign in again to continue.",
  confirmFirst: "If you just created this account, open the confirmation link we emailed first.",
  checkingLink: "Checking your link…",
  checkingSlow:
    "This is taking a while. The server may be waking up, which can take up to a minute. Your link is still being checked, so keep this page open.",
  linkUnchecked:
    "We couldn't check that link just now. If your email is confirmed, sign in below; otherwise ask for a new link.",
  recoveryUnchecked:
    "We couldn't check your password reset link just now. Try again in a moment, or ask for a new reset link.",
  recoveryElsewhere:
    "That password reset link can't be used here. Each link works only once, so ask for a new one below.",
  // R193: claim a reset link from another browser without displaying its address.
  recoveryClaim:
    "This reset link was asked for on another device or browser. To use it here, type your account's email address. If you didn't ask to reset your password, don't use this link.",
  recoveryClaimMismatch: "That isn't the address this reset link was sent to. Type your account's email address.",
  inviteNeedsPassword:
    "You've been invited. Your account needs a password before you can sign in: ask for a link to set one below.",
  inviteOnly: "Online play needs an invite code, which you enter after confirming your email.",
  // R664, R192: the same notice serves every address.
  emailCodeSent:
    `If that address has a confirmed account, an email with a sign-in link and a code is on its way. ` +
    `Open the link in this browser, or type the code below. ${MAIL_INTERVAL_NOTE}`,
  mfaRequired: "This account uses two-step sign-in. Type the 6-digit code your authenticator app shows.",
  mfaEnrolled: "Two-step sign-in is on. Signing in from now on asks for a code from your authenticator app.",
  mfaRemoved: "Two-step sign-in is off. Signing in asks only for your email and password again.",
};

/** Further weak-password reasons follow the first. */
const WEAK_PASSWORD_ALSO: Readonly<Partial<Record<AuthFailure, string>>> = {
  weakPasswordCharacters: "It also needs more kinds of characters: mix upper and lower case letters, digits and symbols.",
  weakPasswordPwned: "It has also appeared in a data breach, so choose one you haven't used anywhere else.",
};

/** A provider refusal reduced to our own messages. */
export class AuthError extends Error {
  readonly failure: AuthFailure;

  constructor(failure: AuthFailure, alsoFailed: readonly AuthFailure[] = []) {
    const also = alsoFailed.map((reason) => WEAK_PASSWORD_ALSO[reason]).filter((line) => line !== undefined);
    super([AUTH_MESSAGES[failure], ...also].join(" "));
    this.name = "AuthError";
    this.failure = failure;
  }
}

// Classifying a refusal

/** A refresh token or access token the provider no longer honours. */
const SESSION_CODES: ReadonlySet<string> = new Set([
  "bad_jwt",
  "session_expired",
  "session_not_found",
  "refresh_token_not_found",
  "refresh_token_already_used",
  "invalid_grant",
]);

const SIGNUP_CLOSED_CODES: ReadonlySet<string> = new Set(["signup_disabled", "email_provider_disabled"]);

/** Read only machine-readable provider codes, never provider text. */
function providerCode(body: unknown): string | null {
  if (typeof body !== "object" || body === null) return null;
  const record = body as { error_code?: unknown; code?: unknown; error?: unknown };
  if (typeof record.error_code === "string" && record.error_code.length > 0) return record.error_code;
  if (typeof record.code === "string" && record.code.length > 0) return record.code;
  if (typeof record.error === "string" && record.error.length > 0) return record.error;
  return null;
}

/** GoTrue's `weak_password.reasons`, each as our failure, in this order of priority. */
const WEAK_PASSWORD_REASONS: ReadonlyArray<readonly [string, AuthFailure]> = [
  ["length", "weakPasswordLength"],
  ["characters", "weakPasswordCharacters"],
  ["pwned", "weakPasswordPwned"],
];

/** Recognized weak-password reasons, in display order. */
function weakPasswordFailures(body: unknown): AuthFailure[] {
  if (typeof body !== "object" || body === null) return [];
  const detail = (body as { weak_password?: unknown }).weak_password;
  if (typeof detail !== "object" || detail === null) return [];
  const reasons = (detail as { reasons?: unknown }).reasons;
  if (!Array.isArray(reasons)) return [];
  return WEAK_PASSWORD_REASONS.filter(([reason]) => reasons.includes(reason)).map(([, failure]) => failure);
}

/** The first weak-password reason; no reason, or none we know, is the plain `weakPassword`. */
function weakPasswordFailure(body: unknown): AuthFailure {
  return weakPasswordFailures(body)[0] ?? "weakPassword";
}

/** The endpoint's answer for "some other 4xx": its generic refusal. */
function genericRefusal(endpoint: AuthEndpoint): AuthFailure {
  switch (endpoint) {
    case "signIn":
      return "credentials";
    case "signUp":
      return "signUpRefused";
    case "resend":
    case "recover":
    case "updatePassword":
    case "changeEmail":
      return "service";
    case "refresh":
      return "sessionEnded";
    case "otp":
    case "mfa":
      return "service";
    case "verifyOtp":
      return "codeInvalid";
    case "mfaVerify":
      return "mfaCodeInvalid";
  }
}

/** GoTrue's answers when the project has not turned TOTP on (R665). */
const MFA_DISABLED_CODES: ReadonlySet<string> = new Set([
  "mfa_totp_enroll_not_enabled",
  "mfa_totp_verify_not_enabled",
]);

/** R160, R192: unknown responses may become less specific but must never expose account state. */
export function classifyProviderRefusal(endpoint: AuthEndpoint, status: number, body: unknown): AuthFailure {
  const code = providerCode(body);

  // R192: a rate limit is reported as a rate limit, never as the identical sign-in error.
  if (status === 429) {
    const mails = endpoint === "signUp" || endpoint === "changeEmail";
    return mails && code === "over_email_send_rate_limit" ? "emailRateLimited" : "rateLimited";
  }
  if (status >= 500) return "service";

  // R664, R160: every sign-in-code refusal is one sentence.
  if (endpoint === "verifyOtp") return "codeInvalid";
  if (endpoint === "mfaVerify" || endpoint === "mfa") {
    if (code !== null && MFA_DISABLED_CODES.has(code)) return "mfaUnavailable";
    if (status === 401 || status === 403 || (code !== null && SESSION_CODES.has(code))) return "sessionEnded";
    return genericRefusal(endpoint);
  }

  if (code === "weak_password") {
    if (endpoint === "signIn") return "credentials";
    if (endpoint === "signUp" || endpoint === "updatePassword") return weakPasswordFailure(body);
    return genericRefusal(endpoint);
  }
  if (code === "same_password") {
    if (endpoint === "signIn") return "credentials";
    if (endpoint === "signUp") return "signUpRefused";
    if (endpoint === "updatePassword") return "samePassword";
    return genericRefusal(endpoint);
  }
  if (code === "validation_failed") {
    if (endpoint === "updatePassword") return "passwordTooLong";
    if (endpoint === "signUp") return "signUpInvalid";
    return genericRefusal(endpoint);
  }
  if (endpoint === "changeEmail") {
    if (status === 401 || status === 403 || (code !== null && SESSION_CODES.has(code))) return "sessionEnded";
    if (status === 400 || status === 422) return "emailChangeRefused";
    return genericRefusal(endpoint);
  }
  if (code === "email_address_invalid") {
    if (endpoint === "signIn") return "credentials";
    if (endpoint === "signUp" || endpoint === "resend" || endpoint === "recover" || endpoint === "otp") {
      return "invalidEmail";
    }
    return genericRefusal(endpoint);
  }
  if (code !== null && SIGNUP_CLOSED_CODES.has(code)) {
    if (endpoint === "signIn") return "credentials";
    if (endpoint === "signUp") return "signupsClosed";
    if (endpoint === "resend" || endpoint === "recover") return "service";
    return genericRefusal(endpoint);
  }
  if (status === 401 || status === 403 || (code !== null && SESSION_CODES.has(code))) {
    if (endpoint === "updatePassword") return "linkExpired";
    return genericRefusal(endpoint);
  }
  return genericRefusal(endpoint);
}

/** Turn a classified provider refusal into the caller's error. */
export function providerRefusal(endpoint: AuthEndpoint, status: number, body: unknown): AuthError {
  const failure = classifyProviderRefusal(endpoint, status, body);
  const reasons = weakPasswordFailures(body);
  if (failure !== reasons[0]) return new AuthError(failure);
  return new AuthError(failure, reasons.slice(1));
}

// Configuration

export type AuthConfig = { url: string; publishableKey: string };

/** Public auth configuration, or null for a deployment without auth. */
export function authConfig(): AuthConfig | null {
  const url: unknown = import.meta.env.VITE_SUPABASE_URL;
  const publishableKey: unknown = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY;
  if (typeof url !== "string" || url.length === 0) return null;
  if (typeof publishableKey !== "string" || publishableKey.length === 0) return null;
  return { url: url.replace(/\/+$/u, ""), publishableKey };
}

function requireConfig(): AuthConfig {
  const config = authConfig();
  if (config === null) {
    console.error(AUTH_UNCONFIGURED_DETAIL);
    throw new AuthError("unconfigured");
  }
  return config;
}

/** B35: redirect email links only to this origin's fixed login path. */
export function authRedirectUrl(): string {
  if (typeof window === "undefined") return "";
  return `${window.location.origin}${paths.login}`;
}

function withRedirect(path: string): string {
  const redirectTo = authRedirectUrl();
  return redirectTo === "" ? path : `${path}?redirect_to=${encodeURIComponent(redirectTo)}`;
}

/** R323, R324: omit PKCE fields only when this browser cannot retain a verifier. */
async function pkceFields(flow: PkceFlow, reuse = false): Promise<PkceChallenge | Record<string, never>> {
  try {
    return await challengeForRequest(flow, { reuse });
  } catch {
    return {};
  }
}

// One request

type ProviderRequest = {
  method: "GET" | "POST" | "PUT" | "DELETE";
  path: string;
  body?: Record<string, unknown>;
  /** A session call carries the access token; an anonymous one carries the publishable key. */
  bearer?: string;
  keepalive?: boolean;
};

/** Never expose provider text; bound requests even when a fetch ignores abort. */
async function send(config: AuthConfig, request: ProviderRequest): Promise<{ status: number; json: unknown }> {
  const headers: Record<string, string> = {
    apikey: config.publishableKey,
    Authorization: `Bearer ${request.bearer ?? config.publishableKey}`,
  };
  const controller = new AbortController();
  const init: RequestInit = { method: request.method, headers, signal: controller.signal };
  if (request.body !== undefined) {
    headers["Content-Type"] = "application/json";
    init.body = JSON.stringify(request.body);
  }
  if (request.keepalive === true) init.keepalive = true;

  let timer: ReturnType<typeof setTimeout> | undefined;
  const timedOut = new Promise<never>((_resolve, reject) => {
    timer = setTimeout(() => {
      controller.abort();
      reject(new AuthError("network"));
    }, AUTH_PROVIDER_TIMEOUT_SECONDS * 1000);
  });
  try {
    let response: Response;
    try {
      response = await Promise.race([fetch(`${config.url}${request.path}`, init), timedOut]);
    } catch {
      throw new AuthError("network");
    }
    let json: unknown = null;
    try {
      json = await Promise.race([response.json() as Promise<unknown>, timedOut]);
    } catch {
      // Ignore malformed or stalled bodies; status still classifies.
    }
    return { status: response.status, json };
  } finally {
    clearTimeout(timer);
  }
}

function isSuccess(status: number): boolean {
  return status >= 200 && status < 300;
}

type TokenResponse = {
  access_token?: unknown;
  refresh_token?: unknown;
  expires_at?: unknown;
  expires_in?: unknown;
  user?: { email_confirmed_at?: unknown; factors?: unknown } | null;
};

/** R665: derive a required factor from the token response without another request. */
function secondFactorIn(body: TokenResponse, accessToken: string): string | null {
  if (assuranceLevel(accessToken) === "aal2") return null;
  return factorsOf(body.user).find((factor) => factor.verified)?.id ?? null;
}

/** Use `expires_in` against this device's clock; provider epoch time is only a fallback. */
function sessionFromTokens(body: TokenResponse, fallbackRefreshToken: string | null): Session | null {
  const accessToken = body.access_token;
  if (typeof accessToken !== "string" || accessToken.length === 0) return null;
  const session: Session = { accessToken };
  session.refreshToken =
    typeof body.refresh_token === "string" && body.refresh_token.length > 0
      ? body.refresh_token
      : fallbackRefreshToken;
  if (typeof body.expires_in === "number" && Number.isFinite(body.expires_in) && body.expires_in > 0) {
    session.expiresAt = Date.now() + body.expires_in * 1000;
  } else if (typeof body.expires_at === "number" && Number.isFinite(body.expires_at)) {
    session.expiresAt = body.expires_at * 1000;
  } else {
    session.expiresAt = null;
  }
  return session;
}

// Sign in, sign up

export type SignInResult = {
  session: Session;
  emailVerified: boolean;
  /** R665: the authenticator app whose code is still needed before the session may be kept. */
  secondFactor: string | null;
};

/** Password grant; the server, not this client, decides account status. */
export async function signIn(email: string, password: string): Promise<SignInResult> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "POST",
    path: "/auth/v1/token?grant_type=password",
    body: { email, password },
  });
  // R160: every account-dependent refusal is one message.
  if (!isSuccess(status)) throw new AuthError(classifyProviderRefusal("signIn", status, json));

  const body = (json ?? {}) as TokenResponse;
  const session = sessionFromTokens(body, null);
  if (session === null) throw new AuthError("credentials");

  forgetPendingAddresses();

  return {
    session,
    // §9.4: only the server trusts verification; this is screen wording.
    emailVerified:
      typeof body.user?.email_confirmed_at === "string" && body.user.email_confirmed_at.length > 0,
    secondFactor: secondFactorIn(body, session.accessToken),
  };
}

/** `POST /api/auth/username-signin`'s answer (`crates/server/src/api/auth.rs`, R1443). */
type UsernameSignInResponse = {
  session?: { accessToken?: unknown; refreshToken?: unknown; expiresIn?: unknown } | null;
  emailVerified?: unknown;
  secondFactor?: unknown;
};

/** The failure a refused username sign-in reads as: our own sentences, never the server's text. */
function usernameRefusal(cause: unknown): AuthFailure {
  if (!(cause instanceof ApiRequestError)) return "network";
  if (cause.status === 401) return "usernameCredentials";
  if (cause.status === 429) return "usernameRateLimited";
  return "service";
}

/**
 * R1443: a sign-in by username. The provider signs in by email alone and this client never learns
 * another player's address, so our server makes the password grant for the account that holds the
 * name and answers the session it got, never the address. What follows is a password sign-in's:
 * the session is the provider's, renewed and ended against it.
 */
export async function signInWithUsername(username: string, password: string): Promise<SignInResult> {
  requireConfig();
  let body: UsernameSignInResponse | null;
  try {
    body = await apiRequest<UsernameSignInResponse | null>("/api/auth/username-signin", {
      method: "POST",
      body: { username, password },
    });
  } catch (cause) {
    throw new AuthError(usernameRefusal(cause));
  }
  const answered = body?.session;
  const accessToken = answered?.accessToken;
  if (typeof accessToken !== "string" || accessToken.length === 0) throw new AuthError("service");
  const session: Session = { accessToken };
  session.refreshToken =
    typeof answered?.refreshToken === "string" && answered.refreshToken.length > 0 ? answered.refreshToken : null;
  const expiresIn = answered?.expiresIn;
  session.expiresAt =
    typeof expiresIn === "number" && Number.isFinite(expiresIn) && expiresIn > 0 ? Date.now() + expiresIn * 1000 : null;

  forgetPendingAddresses();

  const secondFactor = body?.secondFactor;
  return {
    session,
    // §9.4: only the server trusts verification; this is screen wording.
    emailVerified: body?.emailVerified === true,
    secondFactor: typeof secondFactor === "string" && secondFactor.length > 0 ? secondFactor : null,
  };
}

export type SignUpResult = { needsEmailConfirmation: boolean };

/** R160, R193, §9.4: confirmation-on sign-up preserves neutral account handling. */
export async function signUp(email: string, password: string): Promise<SignUpResult> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "POST",
    path: withRedirect("/auth/v1/signup"),
    body: { email, password, ...(await pkceFields("signup")) },
  });
  if (!isSuccess(status)) throw providerRefusal("signUp", status, json);

  rememberPendingEmail(email);
  return { needsEmailConfirmation: true };
}

// The two mailers

/** R192: mailers report only malformed addresses, never account-dependent provider answers. */
function mailerRefusal(endpoint: "resend" | "recover" | "otp", status: number, json: unknown): AuthError | null {
  if (isSuccess(status)) return null;
  const failure = classifyProviderRefusal(endpoint, status, json);
  return failure === "invalidEmail" ? new AuthError(failure) : null;
}

/** R192: resend confirmation without disclosing account state. */
export async function resendConfirmation(email: string): Promise<void> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "POST",
    path: withRedirect("/auth/v1/resend"),
    // Reuse the verifier so the first email's link still exchanges.
    body: { type: "signup", email, ...(await pkceFields("signup", true)) },
  });
  const refusal = mailerRefusal("resend", status, json);
  if (refusal !== null) throw refusal;
  rememberPendingEmail(email);
}

/** R192, R193: reset links remain neutral and require their browser's pending address. */
export async function requestPasswordReset(email: string): Promise<void> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "POST",
    path: withRedirect("/auth/v1/recover"),
    body: { email, ...(await pkceFields("recovery")) },
  });
  const refusal = mailerRefusal("recover", status, json);
  if (refusal !== null) throw refusal;
  rememberPendingReset(email);
}

// An emailed link's code (R323, R324)

/** GoTrue's answer when a verifier is not the one the code's challenge was made from. */
const BAD_CODE_VERIFIER = "bad_code_verifier";

/** A GoTrue auth code is a UUID; anything else in `?code=` is not one, and is never sent. */
const AUTH_CODE_PATTERN = /^[A-Za-z0-9-]{1,128}$/u;

export function isAuthCode(code: string): boolean {
  return AUTH_CODE_PATTERN.test(code);
}

export type CodeExchange =
  /** The code and a verifier here matched: the link's session, and which kind of link it was. */
  | { kind: "session"; session: Session; flow: PkceFlow; secondFactor: string | null }
  /** R324: a code without a local verifier came from elsewhere. */
  | { kind: "elsewhere" }
  /** The provider refused the code itself: spent, expired, or never real. */
  | { kind: "refused" };

/** R323, R324: try local verifiers newest first without spending a refused code. */
export async function exchangeAuthCode(code: string): Promise<CodeExchange> {
  if (!isAuthCode(code)) return { kind: "refused" };
  const verifiers = storedVerifiers();
  if (verifiers.length === 0) return { kind: "elsewhere" };
  const config = requireConfig();
  for (const { flow, verifier } of verifiers) {
    const { status, json } = await send(config, {
      method: "POST",
      path: "/auth/v1/token?grant_type=pkce",
      body: { auth_code: code, code_verifier: verifier },
    });
    if (isSuccess(status)) {
      const body = (json ?? {}) as TokenResponse;
      const session = sessionFromTokens(body, null);
      if (session === null) return { kind: "refused" };
      forgetVerifier(flow);
      return { kind: "session", session, flow, secondFactor: secondFactorIn(body, session.accessToken) };
    }
    if (status >= 500) throw new AuthError("service");
    if (status === 429) throw new AuthError("rateLimited");
    if (providerCode(json) === BAD_CODE_VERIFIER) continue;
    return { kind: "refused" };
  }
  return { kind: "elsewhere" };
}

// Session calls

/** R663, R323, R160: request a confirmed email change with a neutral address refusal. */
export async function requestEmailChange(accessToken: string, email: string): Promise<void> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "PUT",
    path: withRedirect("/auth/v1/user"),
    body: { email, ...(await pkceFields("email_change")) },
    bearer: accessToken,
  });
  if (!isSuccess(status)) throw providerRefusal("changeEmail", status, json);
}

/** Set a new password with the recovery link's access token (`/reset-password`). */
export async function updatePassword(accessToken: string, password: string): Promise<void> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "PUT",
    path: "/auth/v1/user",
    body: { password },
    bearer: accessToken,
  });
  if (!isSuccess(status)) throw providerRefusal("updatePassword", status, json);
}

/** R194: share concurrent refreshes so token rotation cannot race. */
const refreshesInFlight = new Map<string, Promise<Session>>();

async function renewOnce(refreshToken: string): Promise<Session> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "POST",
    path: "/auth/v1/token?grant_type=refresh_token",
    body: { refresh_token: refreshToken },
  });
  if (!isSuccess(status)) throw new AuthError(classifyProviderRefusal("refresh", status, json));
  const session = sessionFromTokens((json ?? {}) as TokenResponse, refreshToken);
  if (session === null) throw new AuthError("service");
  return session;
}

/** R194: callers sharing a refresh token share its request. */
export function refreshSession(refreshToken: string): Promise<Session> {
  const existing = refreshesInFlight.get(refreshToken);
  if (existing !== undefined) return existing;
  const request = renewOnce(refreshToken).finally(() => {
    refreshesInFlight.delete(refreshToken);
  });
  refreshesInFlight.set(refreshToken, request);
  return request;
}

/** R194: best-effort revocation may finish while the page unloads. */
export async function revokeSession(accessToken: string): Promise<void> {
  try {
    const config = authConfig();
    if (config === null || accessToken.length === 0) return;
    await logout(config, accessToken);
  } catch {
    // Nothing to do: see above.
  }
}

async function logout(config: AuthConfig, accessToken: string): Promise<number> {
  const { status } = await send(config, {
    method: "POST",
    path: "/auth/v1/logout?scope=local",
    bearer: accessToken,
    keepalive: true,
  });
  return status;
}

/** R194: revoke a replaced session so its refresh token cannot outlive sign-out. */
export function adoptSession(next: Session): void {
  const previous = readSession();
  writeSession(next);
  if (previous !== null && previous.accessToken !== next.accessToken) void revokeSignedOutSession(previous);
}

/** R194: a session this close to expiring (or past it) is renewed before it is used. */
export function sessionNearExpiry(session: Session, now: number): boolean {
  const expiresAt = session.expiresAt;
  if (typeof expiresAt !== "number" || !Number.isFinite(expiresAt)) return false;
  return expiresAt - now <= AUTH_SESSION_REFRESH_MARGIN_SECONDS * 1000;
}

/** R194: renew an expired token once before retrying its best-effort revocation. */
export async function revokeSignedOutSession(session: Session): Promise<void> {
  try {
    const config = authConfig();
    if (config === null || session.accessToken.length === 0) return;
    const refreshToken =
      typeof session.refreshToken === "string" && session.refreshToken.length > 0 ? session.refreshToken : null;
    let accessToken = session.accessToken;
    let renewed = false;
    const renew = async (): Promise<boolean> => {
      if (refreshToken === null || renewed) return false;
      renewed = true;
      try {
        accessToken = (await refreshSession(refreshToken)).accessToken;
        return true;
      } catch {
        return false;
      }
    };

    if (sessionNearExpiry(session, Date.now())) await renew();
    const status = await logout(config, accessToken);
    if ((status === 401 || status === 403) && (await renew())) await logout(config, accessToken);
  } catch {
    // Nothing to do: the device's copy is already gone (see `revokeSession`).
  }
}

// The email sign-in link and code (R664)

/** R664, R193, R192, R323: mail only confirmed existing accounts without making an oracle. */
export async function requestEmailSignIn(email: string): Promise<void> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "POST",
    path: withRedirect("/auth/v1/otp"),
    body: { email, create_user: false, ...(await pkceFields("magiclink")) },
  });
  const refusal = mailerRefusal("otp", status, json);
  if (refusal !== null) throw refusal;
}

/** GoTrue's sign-in codes are digits; anything else is never sent. */
const EMAIL_CODE_PATTERN = /^[0-9]{6,10}$/u;

/** A typed code with its spaces and dashes taken out (a code is often pasted as `123 456`). */
export function normalizeOneTimeCode(raw: string): string {
  return raw.replace(/[\s-]/gu, "");
}

/** R664, R160, R192: every email-code refusal is `codeInvalid` except rate limits and faults. */
export async function verifyEmailCode(
  email: string,
  code: string,
): Promise<{ session: Session; secondFactor: string | null }> {
  const token = normalizeOneTimeCode(code);
  if (!EMAIL_CODE_PATTERN.test(token)) throw new AuthError("codeInvalid");
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "POST",
    path: "/auth/v1/verify",
    body: { type: "email", email, token },
  });
  if (!isSuccess(status)) throw new AuthError(classifyProviderRefusal("verifyOtp", status, json));
  const body = (json ?? {}) as TokenResponse;
  const session = sessionFromTokens(body, null);
  if (session === null) throw new AuthError("codeInvalid");
  forgetPendingAddresses();
  return { session, secondFactor: secondFactorIn(body, session.accessToken) };
}

// Two-step sign-in: an authenticator app (R665)

/** One of the account's TOTP factors, as GoTrue lists them on `GET /auth/v1/user`. */
export type TotpFactor = { id: string; verified: boolean };

/** The assurance level an access token's own payload claims (`aal`), read and never verified. */
export function assuranceLevel(accessToken: string): string | null {
  const payload = accessToken.split(".")[1];
  if (payload === undefined || payload.length === 0) return null;
  try {
    const base64 = payload.replace(/-/gu, "+").replace(/_/gu, "/");
    const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
    const bytes = Uint8Array.from(atob(padded), (character) => character.charCodeAt(0));
    const claims: unknown = JSON.parse(new TextDecoder().decode(bytes));
    if (typeof claims !== "object" || claims === null) return null;
    const aal = (claims as Record<string, unknown>)["aal"];
    return typeof aal === "string" ? aal : null;
  } catch {
    return null;
  }
}

function factorsOf(body: unknown): TotpFactor[] {
  if (typeof body !== "object" || body === null) return [];
  const factors = (body as { factors?: unknown }).factors;
  if (!Array.isArray(factors)) return [];
  return factors.flatMap((entry: unknown): TotpFactor[] => {
    if (typeof entry !== "object" || entry === null) return [];
    const record = entry as { id?: unknown; factor_type?: unknown; status?: unknown };
    if (typeof record.id !== "string" || record.id.length === 0 || record.factor_type !== "totp") return [];
    return [{ id: record.id, verified: record.status === "verified" }];
  });
}

/** The account's TOTP factors (R665). Throws `sessionEnded` for a token the provider refuses. */
export async function listTotpFactors(accessToken: string): Promise<TotpFactor[]> {
  const config = requireConfig();
  const { status, json } = await send(config, { method: "GET", path: "/auth/v1/user", bearer: accessToken });
  if (!isSuccess(status)) throw new AuthError(classifyProviderRefusal("mfa", status, json));
  return factorsOf(json);
}

/** R665: an `aal1` session with a verified factor must complete that factor. */
export async function secondFactorFor(session: Session): Promise<string | null> {
  if (assuranceLevel(session.accessToken) === "aal2") return null;
  const factors = await listTotpFactors(session.accessToken);
  return factors.find((factor) => factor.verified)?.id ?? null;
}

/** A factor id is GoTrue's UUID; anything else is never put in a path. */
const FACTOR_ID_PATTERN = /^[A-Za-z0-9-]{1,128}$/u;

function factorPath(factorId: string, suffix = ""): string {
  if (!FACTOR_ID_PATTERN.test(factorId)) throw new AuthError("mfaUnavailable");
  return `/auth/v1/factors/${encodeURIComponent(factorId)}${suffix}`;
}

/** An authenticator app's codes are six digits. */
const TOTP_CODE_PATTERN = /^[0-9]{6}$/u;

/** R665: verify a factor's challenge and replace the session with its `aal2` tokens. */
export async function verifySecondFactor(session: Session, factorId: string, code: string): Promise<Session> {
  const typed = normalizeOneTimeCode(code);
  if (!TOTP_CODE_PATTERN.test(typed)) throw new AuthError("mfaCodeInvalid");
  const config = requireConfig();
  const challenge = await send(config, {
    method: "POST",
    path: factorPath(factorId, "/challenge"),
    bearer: session.accessToken,
  });
  if (!isSuccess(challenge.status)) {
    throw new AuthError(classifyProviderRefusal("mfa", challenge.status, challenge.json));
  }
  const challengeId = (challenge.json as { id?: unknown } | null)?.id;
  if (typeof challengeId !== "string" || challengeId.length === 0) throw new AuthError("service");
  const { status, json } = await send(config, {
    method: "POST",
    path: factorPath(factorId, "/verify"),
    body: { challenge_id: challengeId, code: typed },
    bearer: session.accessToken,
  });
  if (!isSuccess(status)) throw new AuthError(classifyProviderRefusal("mfaVerify", status, json));
  const next = sessionFromTokens((json ?? {}) as TokenResponse, session.refreshToken ?? null);
  if (next === null) throw new AuthError("service");
  return next;
}

/** What enrolling shows the player: the QR code, and the same key to type by hand. */
export type TotpEnrolment = { factorId: string; qrCode: string | null; secret: string };

/** The issuer an authenticator app files the account under. */
const TOTP_ISSUER = "JackiOh";

/** Re-encode only SVG data URIs; a raw `#` would terminate one. */
function qrImage(raw: unknown): string | null {
  if (typeof raw !== "string") return null;
  const match = /^data:image\/svg\+xml(?:;[a-z0-9=-]+)*,(.*)$/isu.exec(raw);
  const svg = match?.[1];
  if (svg === undefined || svg.length === 0) return null;
  let decoded = svg;
  try {
    decoded = decodeURIComponent(svg);
  } catch {
    // Raw SVG needs no decoding.
  }
  if (!decoded.trimStart().startsWith("<svg")) return null;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(decoded)}`;
}

/** R665: remove unfinished factors before enrolment; they count only after verification. */
export async function enrollTotp(session: Session): Promise<TotpEnrolment> {
  const config = requireConfig();
  for (const factor of await listTotpFactors(session.accessToken)) {
    if (!factor.verified) await removeFactor(session, factor.id);
  }
  const { status, json } = await send(config, {
    method: "POST",
    path: "/auth/v1/factors",
    body: { factor_type: "totp", issuer: TOTP_ISSUER },
    bearer: session.accessToken,
  });
  if (!isSuccess(status)) throw new AuthError(classifyProviderRefusal("mfa", status, json));
  const body = (json ?? {}) as { id?: unknown; totp?: { qr_code?: unknown; secret?: unknown } | null };
  const secret = body.totp?.secret;
  if (typeof body.id !== "string" || body.id.length === 0 || typeof secret !== "string" || secret.length === 0) {
    throw new AuthError("service");
  }
  return { factorId: body.id, qrCode: qrImage(body.totp?.qr_code), secret };
}

/** R665: remove a factor. The provider asks an `aal2` session to remove a verified one. */
export async function removeFactor(session: Session, factorId: string): Promise<void> {
  const config = requireConfig();
  const { status, json } = await send(config, {
    method: "DELETE",
    path: factorPath(factorId),
    bearer: session.accessToken,
  });
  if (!isSuccess(status)) throw new AuthError(classifyProviderRefusal("mfa", status, json));
}

// OAuth providers (R666)

/** Providers this client can name; unknown configuration is ignored. */
export const OAUTH_PROVIDER_LABELS = {
  apple: "Apple",
  azure: "Microsoft",
  discord: "Discord",
  facebook: "Facebook",
  github: "GitHub",
  gitlab: "GitLab",
  google: "Google",
  twitch: "Twitch",
} as const;

export type OAuthProvider = keyof typeof OAUTH_PROVIDER_LABELS;

function isOAuthProvider(name: string): name is OAuthProvider {
  return Object.hasOwn(OAUTH_PROVIDER_LABELS, name);
}

/** R666: offer configured known providers only when auth itself is configured. */
export function oauthProviders(raw: unknown = import.meta.env.VITE_AUTH_OAUTH_PROVIDERS): OAuthProvider[] {
  if (authConfig() === null || typeof raw !== "string") return [];
  const named = raw
    .split(",")
    .map((name) => name.trim().toLowerCase())
    .filter(isOAuthProvider);
  return [...new Set(named)];
}

/** R666, R323: OAuth requires a locally retained PKCE verifier, never the implicit flow. */
export async function oauthAuthorizeUrl(provider: OAuthProvider): Promise<string> {
  const config = requireConfig();
  let challenge: PkceChallenge;
  try {
    challenge = await challengeForRequest("oauth");
  } catch {
    throw new AuthError("oauthUnavailable");
  }
  const params = new URLSearchParams({
    provider,
    redirect_to: authRedirectUrl(),
    code_challenge: challenge.code_challenge,
    code_challenge_method: challenge.code_challenge_method,
  });
  return `${config.url}/auth/v1/authorize?${params.toString()}`;
}

/** R666: leave for the provider's sign-in page. A navigation, so the CSP's `connect-src` is not involved. */
export async function startOAuthSignIn(provider: OAuthProvider): Promise<void> {
  const url = await oauthAuthorizeUrl(provider);
  window.location.assign(url);
}
