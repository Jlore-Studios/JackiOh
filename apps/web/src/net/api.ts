// The client's one door onto `crates/server`'s REST surface.
//
// CLAUDE.md rule 7: the client sends intent and renders what comes back. Nothing here decides a
// rule — in particular no L1–L6 message is ever composed in the browser. The validator
// (`crates/engine/src/validator.rs`, the client's through WebAssembly) is
// "one validator module shared by client and server" (SPEC §9.4) and the server passes its issues
// through untouched, so a `LoadoutIssue.message` shown by the deckbuilder is either the validator's
// own sentence (client-side, as UX) or the server's relay of it (at save, as law). A second copy
// of a message would be a second source of truth.
//
// Every response shape below is the one `crates/server/src/api/*.rs` actually returns; every error
// is `{ error: { code, message, details? } }` from `crates/server/src/api/http.rs`.

import type { ReplayRefusal, ReplayStep } from "@jackioh/engine";
import type { GameOverReason, LeanPick, PlayerId, PortraitId, PrintedRarity, SetName } from "@jackioh/shared";

import { API_REQUEST_TIMEOUT_SECONDS } from "@jackioh/server-config";

import type { CosmeticRarity } from "../wire/generated/cosmetics.ts";
import { rememberRead, updateCachedRead } from "./readCache.ts";
import { whileSpending } from "./spending.ts";

const DEFAULT_HTTP_URL = "http://localhost:8787";

/**
 * `VITE_SERVER_HTTP_URL` is the public half of the environment contract
 * (`crates/server/src/env.rs` `PUBLIC_ENV_VARS`). The default is the port `crates/server` listens on,
 * which is what `e2e/support/config.ts` points at.
 */
export function apiBaseUrl(): string {
  const configured: unknown = import.meta.env.VITE_SERVER_HTTP_URL;
  const base = typeof configured === "string" && configured.length > 0 ? configured : DEFAULT_HTTP_URL;
  return base.replace(/\/+$/, "");
}

const DEFAULT_WS_URL = "ws://localhost:8787/ws/match";

/** `WS_PATH` in `crates/server/src/actor/ws_server.rs` is `/ws/match`; the default mirrors it. */
export function matchSocketUrl(): string {
  const configured: unknown = import.meta.env.VITE_SERVER_WS_URL;
  return typeof configured === "string" && configured.length > 0 ? configured : DEFAULT_WS_URL;
}

/** The `/ws/match` path the configured socket URL ends in. */
const MATCH_SOCKET_PATH = /\/ws\/match\/?$/u;

/**
 * A socket on the same server as `/ws/match` (#936: `/ws/duos`, `/ws/watch`): the configured URL
 * with its path swapped for `path`, so one environment variable still names the server.
 */
export function serverSocketUrl(path: string, base: string = matchSocketUrl()): string {
  try {
    const url = new URL(base);
    url.pathname = MATCH_SOCKET_PATH.test(url.pathname) ? url.pathname.replace(MATCH_SOCKET_PATH, path) : path;
    return url.toString();
  } catch {
    // A relative test address.
    return MATCH_SOCKET_PATH.test(base) ? base.replace(MATCH_SOCKET_PATH, path) : `${base.replace(/\/+$/u, "")}${path}`;
  }
}

/** `crates/server/src/api/http.rs` `ApiErrorCode`, as the client sees it: an opaque string. */
export type ApiErrorBody = { code: string; message: string; details?: unknown };

/** A refusal from the server, carrying its code and message verbatim. Never reworded here. */
export class ApiRequestError extends Error {
  readonly status: number;
  readonly code: string;
  readonly details: unknown;

  constructor(status: number, body: ApiErrorBody) {
    super(body.message);
    this.name = "ApiRequestError";
    this.status = status;
    this.code = body.code;
    this.details = body.details;
  }
}

/**
 * The transport failed: no response at all, or none within `API_REQUEST_TIMEOUT_SECONDS`. Distinct
 * from a refusal, which has a code. Its message is a sentence for the player; the browser's own
 * wording ("Failed to fetch", Safari's "Load failed") says nothing to them and is kept aside.
 */
export const API_UNREACHABLE_MESSAGE = "Couldn't reach JackiOh. Check your connection, then try again.";

export class ApiUnreachableError extends Error {
  /** What the transport said, for a debugger; never shown. */
  readonly transportError: unknown;

  constructor(transportError: unknown) {
    super(API_UNREACHABLE_MESSAGE);
    this.name = "ApiUnreachableError";
    this.transportError = transportError;
  }
}

/** Why a request was abandoned: it had not answered within `API_REQUEST_TIMEOUT_SECONDS`. */
class ApiTimeoutError extends Error {
  constructor() {
    super(`no answer within ${String(API_REQUEST_TIMEOUT_SECONDS)} s`);
    this.name = "ApiTimeoutError";
  }
}

export type RequestOptions = {
  method?: "GET" | "POST" | "PUT" | "DELETE";
  /** The bearer token. Omitted for the two `auth: "none"` routes. */
  token?: string | null;
  body?: unknown;
  signal?: AbortSignal;
  /** A spend's `Idempotency-Key` (R2124000): `spendRequest` below sets it, and nothing else does. */
  idempotencyKey?: string;
  /** R2119000: the request outlives the page (`fetch`'s `keepalive`), for one sent as it goes away. */
  keepalive?: boolean;
};

/** R2119000: how a held deletion goes; `keepalive` when the page is going away. */
export type SendOptions = { keepalive?: boolean };

function errorBodyOf(value: unknown, status: number): ApiErrorBody {
  if (typeof value === "object" && value !== null) {
    const error = (value as { error?: unknown }).error;
    if (typeof error === "object" && error !== null) {
      const code = (error as { code?: unknown }).code;
      const message = (error as { message?: unknown }).message;
      if (typeof code === "string" && typeof message === "string") {
        return { code, message, details: (error as { details?: unknown }).details };
      }
    }
  }
  return { code: "internal", message: `the server answered ${String(status)}` };
}

/**
 * One request. Resolves with the parsed body on 2xx and rejects with `ApiRequestError` otherwise,
 * so a caller never has to remember to check `response.ok`.
 */
export async function apiRequest<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = {};
  if (options.token !== undefined && options.token !== null && options.token.length > 0) {
    headers.authorization = `Bearer ${options.token}`;
  }
  if (options.body !== undefined) headers["content-type"] = "application/json";
  if (options.idempotencyKey !== undefined) headers["idempotency-key"] = options.idempotencyKey;

  // A request that never answers (a stalled mobile network, a dead socket) must not leave a screen
  // on "Checking your account…" or a button on "Redeeming…" for ever. The race is against our own
  // timer as well as the abort, so a `fetch` that ignores its signal still gives up.
  const controller = new AbortController();
  const outer = options.signal;
  if (outer !== undefined) {
    if (outer.aborted) controller.abort();
    else outer.addEventListener("abort", () => controller.abort(), { once: true });
  }
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timedOut = new Promise<never>((_resolve, reject) => {
    timer = setTimeout(() => {
      controller.abort();
      reject(new ApiTimeoutError());
    }, API_REQUEST_TIMEOUT_SECONDS * 1000);
  });

  let response: Response;
  let text: string;
  try {
    try {
      response = await Promise.race([
        fetch(`${apiBaseUrl()}${path}`, {
          method: options.method ?? "GET",
          headers,
          ...(options.body === undefined ? {} : { body: JSON.stringify(options.body) }),
          ...(options.keepalive === true ? { keepalive: true } : {}),
          signal: controller.signal,
        }),
        timedOut,
      ]);
    } catch (cause) {
      throw new ApiUnreachableError(cause);
    }
    try {
      text = await Promise.race([response.text(), timedOut]);
    } catch (cause) {
      throw new ApiUnreachableError(cause);
    }
  } finally {
    clearTimeout(timer);
  }

  let parsed: unknown = null;
  if (text.length > 0) {
    try {
      parsed = JSON.parse(text);
    } catch {
      parsed = null;
    }
  }
  if (!response.ok) throw new ApiRequestError(response.status, errorBodyOf(parsed, response.status));
  return parsed as T;
}

/**
 * R2125000: the menu reads whose last answer is kept (`net/readCache.ts`), each by its path. A screen
 * starts from `cachedRead(token, keptPaths.x)`; the statistics table's path is `cardStatsPath`, the
 * friends list's `FRIENDS_PATH` (`net/social.ts`). Nothing a match, a duos room, a watch or a replay
 * reads is here.
 */
export const keptPaths = {
  collection: "/api/collection",
  cosmetics: "/api/cosmetics",
  quests: "/api/quests",
  packs: "/api/packs",
  wallet: "/api/wallet",
  tracks: "/api/tracks",
  tokens: "/api/tokens",
  cosmeticPacks: "/api/cosmetic-packs",
  decks: "/api/decks",
  leaderboard: "/api/leaderboard",
} as const;

/** R2125000: a `GET` whose answer is kept for the next visit. A refusal or a failed read keeps what was there. */
async function keptGet<T>(path: string, token: string | null, signal?: AbortSignal): Promise<T> {
  const answer = await apiRequest<T>(path, { token, signal });
  rememberRead(token, path, answer);
  return answer;
}

// ---------------------------------------------------------------------------------------------
// R2124000: a spend runs once. Every spend (a pack bought or opened, a token redeemed, a chase
// bought, an invite code redeemed) goes with an `Idempotency-Key`, a UUID minted when the player
// presses and sent again by that press's retries, "Try again" after a timeout included, so a spend
// the server finished after we gave up answers the same and spends nothing again. Any answer from
// the server, a refusal included but a 429, which ran nothing (R2378000), ends the press: the next
// one is a new key. A press with no answer (a timeout or a failed transport), a 429 or a server error
// keeps its key for the retry. A press is the spend's path and body: pressing again before an answer
// is the retry of the press that got none. Each spend is out (`net/spending.ts`) until it settles.
// ---------------------------------------------------------------------------------------------

/** The first status that is the server's own failure, not its answer to the spend. */
const FIRST_SERVER_ERROR_STATUS = 500;

/** R2378000: a refusal that ran nothing (a spend past the ones waiting their turn, or R109's limit): it ends no press. */
const RAN_NOTHING_STATUS = 429;

/** Each unanswered press's key, by `${path} ${body}`, until the server answers it. */
const unansweredKeys = new Map<string, string>();

function spendRequest<T>(path: string, token: string, body: unknown): Promise<T> {
  const press = `${path} ${JSON.stringify(body)}`;
  const idempotencyKey = unansweredKeys.get(press) ?? globalThis.crypto.randomUUID();
  unansweredKeys.set(press, idempotencyKey);
  return whileSpending(async () => {
    try {
      const answer = await apiRequest<T>(path, { method: "POST", token, body, idempotencyKey });
      unansweredKeys.delete(press);
      return answer;
    } catch (cause) {
      if (
        cause instanceof ApiRequestError &&
        cause.status < FIRST_SERVER_ERROR_STATUS &&
        cause.status !== RAN_NOTHING_STATUS
      ) {
        unansweredKeys.delete(press);
      }
      throw cause;
    }
  });
}

// ---------------------------------------------------------------------------------------------
// The endpoints, as the handlers in `crates/server/src/api/**` return them.
// ---------------------------------------------------------------------------------------------

/** `GET /api/auth/me` (`auth: "user"`): the code screen's own read (§9.4). */
export type MeResponse = {
  // R612: the hidden rating is never sent, not even to its owner. What the client shows is
  // `GET /api/ranked`'s visible rank, below.
  profile: { id: string; status: "pending" | "active" | "banned" };
  needsInviteCode: boolean;
  emailVerified: boolean;
  /** §9.5: the match this profile is in, or null. What `/play` waits on after it queues. */
  currentMatchId: string | null;
  /**
   * R330: the Conquest series this profile is in (not over), or null. Set between games too, when
   * `currentMatchId` is null, so `/play` can send a paired player to the series screen to pick.
   * Optional because a server from before R259 does not send it.
   */
  currentSeriesId?: string | null;
  /**
   * R1566000: the live co-op match this profile holds a seat in, or null; a co-op match sets no
   * `currentMatchId`. Optional because a server from before R1566000 (or a test stub) omits it.
   */
  currentDuosMatchId?: string | null;
  /** The address this account is tied to, so a player can see who they are signed in as. */
  email: string | null;
  /**
   * R1435: the caller's username, when the next change is allowed, and whether the prompt after
   * activation is still owed. Optional because a server from before R1435 (or a test stub) omits it.
   */
  username?: OwnUsername;
  /**
   * R757000: a pending account that has not answered the age screen. Optional because a server from
   * before R757000 (or a test stub) omits it; missing means nothing is owed.
   */
  ageCheckOwed?: boolean;
  /** R757001: the account has not agreed to the terms in force (`TERMS_VERSION`). Missing: not owed. */
  termsOwed?: boolean;
  /**
   * R897000: whether this profile holds an open queue ticket. Optional because a server from
   * before R897000 (or a test stub) omits it; missing never ends a wait.
   */
  queued?: boolean;
  /**
   * R1300000: whether the server's `DEV_PROFILE_IDS` lists this account, so `/account` shows the
   * Dev tools panel. The server checks the list again on every dev route. Missing: not a dev account.
   */
  isDev?: boolean;
};

/** The caller's own username, as `/api/auth/me` and every username route answer with it (R1435). */
export type OwnUsername = {
  /** The caller's username as shown: `Max` or `Max#3`. */
  name: string;
  /** Epoch ms when the next change is allowed, or null when a change is allowed now (R1435). */
  nextChangeAt: number | null;
  /** True while the username prompt shown after activation is still owed (R1435). */
  promptOwed: boolean;
};

/** `GET /api/profile`: the account screen's read — identity plus the ladder record. */
export type ProfileResponse = {
  id: string;
  email: string | null;
  status: "pending" | "active" | "banned";
  // R612: no hidden rating here either. The account screen reads the visible rank separately.
  record: { wins: number; losses: number; draws: number };
  /** 0..1, or null when nothing has been played. Computed server-side so the two cannot differ. */
  winRate: number | null;
};

export function getProfile(token: string): Promise<ProfileResponse> {
  return apiRequest<ProfileResponse>("/api/profile", { token });
}

export function getMe(token: string): Promise<MeResponse> {
  return apiRequest<MeResponse>("/api/auth/me", { token });
}

// ---------------------------------------------------------------------------------------------
// Usernames (R1435, R1436). The server validates, normalizes and tags a name; the client proposes
// one, shows the server's verdict as it stands, and saves exactly the username that verdict named,
// so a player never gets a tag they were not shown.
// ---------------------------------------------------------------------------------------------

/** Why the server refused a name. Its `message` is the sentence a player reads, shown as is. */
export type UsernameRefusal = "characters" | "mixed_scripts" | "too_short" | "too_long" | "not_allowed" | "unchanged";

/**
 * `GET /api/username/preview`: what a save of `name` would give. `base` is the stored form of the
 * base name, `username` the exact username a save would give (`Max` or `Max#3`), and `tagged` says
 * the base is taken, so the player would take a tag.
 */
export type UsernamePreview =
  | { ok: true; base: string; username: string; tagged: boolean }
  | { ok: false; reason: UsernameRefusal; message: string }
  | { ok: false; reason: "cooldown"; message: string; nextChangeAt: number };

export function previewUsername(token: string, name: string, signal?: AbortSignal): Promise<UsernamePreview> {
  return apiRequest<UsernamePreview>(`/api/username/preview?name=${encodeURIComponent(name)}`, { token, signal });
}

/**
 * `PUT /api/username` with a preview's `username`, exactly. Refusals: 400 `bad_request` whose
 * `details` is the refusing preview, and 409 `conflict` whose `details` is a fresh preview when the
 * outcome changed since the player looked (the base was taken meanwhile, the cooldown runs, or the
 * name is already theirs). `usernamePreviewOf` reads either.
 */
export async function saveUsername(token: string, username: string): Promise<OwnUsername> {
  return (await apiRequest<{ username: OwnUsername }>("/api/username", { method: "PUT", token, body: { username } }))
    .username;
}

/** `POST /api/username/skip`: answers the prompt after activation, keeping the default name. */
export async function skipUsernamePrompt(token: string): Promise<OwnUsername> {
  return (await apiRequest<{ username: OwnUsername }>("/api/username/skip", { method: "POST", token, body: {} }))
    .username;
}

/** The preview a username refusal carries in its `details`, or null for any other error. */
export function usernamePreviewOf(error: unknown): UsernamePreview | null {
  if (!(error instanceof ApiRequestError)) return null;
  const details = error.details;
  if (typeof details !== "object" || details === null) return null;
  const { ok } = details as { ok?: unknown };
  if (ok === true) {
    const { base, username, tagged } = details as { base?: unknown; username?: unknown; tagged?: unknown };
    return typeof base === "string" && typeof username === "string" && typeof tagged === "boolean"
      ? (details as UsernamePreview)
      : null;
  }
  if (ok !== false) return null;
  const { reason, message } = details as { reason?: unknown; message?: unknown };
  if (typeof reason !== "string" || typeof message !== "string") return null;
  if (reason === "cooldown" && typeof (details as { nextChangeAt?: unknown }).nextChangeAt !== "number") return null;
  return details as UsernamePreview;
}

/**
 * `POST /api/account/consent` (R757000, R757001): agreement to `termsVersion` and, while the age
 * check is owed, the month and year of birth, which the server counts and does not keep. Refusals:
 * 403 `under_age` (the account is deleted), 409 `update_required` (the terms changed since this
 * client loaded), 400 `bad_request` (the birth is missing or cannot be right).
 */
export function postConsent(
  token: string,
  body: { termsVersion: string; birthYear?: number; birthMonth?: number },
): Promise<{ ageCheckOwed: boolean; termsOwed: boolean }> {
  return apiRequest<{ ageCheckOwed: boolean; termsOwed: boolean }>("/api/account/consent", {
    method: "POST",
    token,
    body,
  });
}

/**
 * `DELETE /api/account`: deletes the caller's account at the server, which answers 204 with no
 * body. What it removes, and what it keeps for the other player's history, is the server's to
 * decide. A refusal rejects with `ApiRequestError`, as every call here does.
 */
export async function deleteAccount(token: string): Promise<void> {
  await apiRequest<null>("/api/account", { method: "DELETE", token });
}

// ---------------------------------------------------------------------------------------------
// The dev tools (SPEC §14.10, R1300000): a dev account's own economy, which the server alone
// changes. Each answers 403 `forbidden` to an account `DEV_PROFILE_IDS` does not list, and 400 to
// an amount that is not a positive whole number; the client sends what was typed and shows that.
// ---------------------------------------------------------------------------------------------

/** What `POST /api/dev/clear/:part` clears: one holding, or all five. */
export type DevClearPart = "cards" | "cosmetics" | "jlore" | "counters" | "tracks" | "everything";

/** A set track as the server holds it (SPEC §14.7). */
export type DevSetTrack = { setId: string; xp: number; level: number; rewardsClaimed: number };

/** `POST /api/dev/xp`: XP to a shipped set's track, through the level curve. */
export function devGiveXp(token: string, setId: SetName, xp: number): Promise<{ track: DevSetTrack }> {
  return apiRequest<{ track: DevSetTrack }>("/api/dev/xp", { method: "POST", token, body: { setId, xp } });
}

/** `POST /api/dev/jlore`: Jlore, answering the new balance. */
export function devGiveJlore(token: string, jlore: number): Promise<{ jlore: number }> {
  return apiRequest<{ jlore: number }>("/api/dev/jlore", { method: "POST", token, body: { jlore } });
}

/** `POST /api/dev/fill`: every card and cosmetic, answering the ledger rows written. */
export function devFill(token: string): Promise<{ cards: number; cosmetics: number }> {
  return apiRequest<{ cards: number; cosmetics: number }>("/api/dev/fill", { method: "POST", token, body: {} });
}

/** `POST /api/dev/clear/:part`, answering the ledger rows written. */
export function devClear(token: string, part: DevClearPart): Promise<{ written: number }> {
  return apiRequest<{ written: number }>(`/api/dev/clear/${part}`, { method: "POST", token, body: {} });
}

/** R1310003: which token bar `POST /api/dev/bar/:bar` fills (SPEC §14.5). */
export type DevBar = "card" | "cosmetic";

/** R1310003: `POST /api/dev/bar/:bar`: fills that token bar so it pays its token at once, answering the ledger rows written. */
export function devFillBar(token: string, bar: DevBar): Promise<{ written: number }> {
  return apiRequest<{ written: number }>(`/api/dev/bar/${bar}`, { method: "POST", token, body: {} });
}

/** `POST /api/dev/quests`: a quest at once (§14.9), answering it; 409 while the most are held. */
export function devGiveQuest(token: string): Promise<{ quest: QuestView }> {
  return apiRequest<{ quest: QuestView }>("/api/dev/quests", { method: "POST", token, body: {} });
}

/** `POST /api/dev/quests/:questId/complete`: completes a held quest, answering what it paid; 404 for one not held. */
export function devCompleteQuest(token: string, questId: string): Promise<{ completed: string; jlore: number; xp: number }> {
  return apiRequest<{ completed: string; jlore: number; xp: number }>(`/api/dev/quests/${encodeURIComponent(questId)}/complete`, {
    method: "POST",
    token,
    body: {},
  });
}

// Sign-in and sign-up are NOT here. SPEC §9.2 draws the browser's arrow to the auth provider
// separately from its arrow to these API functions, so they live in `net/auth.ts` and go straight
// to the provider. `/api/auth/signin` does exist on the server, but it is BUILD M8's fixture-account
// path: without a publishable key configured there it answers 503 with "This server does not broker
// passwords", which is the normal deployment. A second client helper pointing at it would be a
// sign-in path that silently fails, so there is one and it is `net/auth.ts`. The one sign-in that
// reaches this server is a username's (R1443), since only the server may read the address a name
// signs in with: `net/auth.ts`'s `signInWithUsername` sends it to `/api/auth/username-signin`
// through `apiRequest`.

/**
 * `GET /api/codes/status`: so the code screen can say "paused" instead of guessing, and how many
 * redemptions this account has left in §9.4's window. `attemptsRemaining` is optional because a
 * server from before R192 does not send it; the screen then says nothing about tries rather than
 * inventing a number. It is advisory: the per-IP limit can still refuse first (the 429 covers it).
 */
export type CodeStatusResponse = {
  redemptionEnabled: boolean;
  retryAfterMs: number;
  attemptsRemaining?: number;
  /**
   * With no tries left, how long until this account's oldest counted attempt leaves the window and
   * a try comes back; 0 otherwise (R192). Optional for the same reason as `attemptsRemaining`.
   */
  attemptsRetryAfterMs?: number;
};

/**
 * R192: how long a `rate_limited` refusal says to wait, from its `details.retryAfterMs`. Null for
 * any other error, and for a rate limit that did not say (a finite, non-negative number or
 * nothing). Never read from any other code: R145's `invalid_code` carries no details on purpose.
 */
export function retryAfterMsOf(error: unknown): number | null {
  if (!(error instanceof ApiRequestError) || error.code !== "rate_limited") return null;
  const details = error.details;
  if (typeof details !== "object" || details === null) return null;
  const retryAfterMs = (details as { retryAfterMs?: unknown }).retryAfterMs;
  if (typeof retryAfterMs !== "number" || !Number.isFinite(retryAfterMs) || retryAfterMs < 0) return null;
  return retryAfterMs;
}

export function getCodeStatus(token: string): Promise<CodeStatusResponse> {
  return apiRequest<CodeStatusResponse>("/api/codes/status", { token });
}

/** `POST /api/codes/redeem`: flips pending to active (§9.4). */
export type RedeemResponse = { status: "active"; needsInviteCode: false };

export function redeemCode(token: string, code: string): Promise<RedeemResponse> {
  return spendRequest<RedeemResponse>("/api/codes/redeem", token, { code });
}

export type CollectionEntry = { cardId: string; quantity: number };
export type CollectionResponse = { catalogVersion: string; entries: CollectionEntry[] };

export function getCollection(token: string): Promise<CollectionResponse> {
  return keptGet<CollectionResponse>(keptPaths.collection, token);
}

/** R1313000: `GET /api/cosmetics`, the caller's own cosmetics, per card and on the account. */
export type OwnedCosmeticsResponse = { cards: { cardId: string; cosmeticId: string }[]; account: string[] };

export function getOwnedCosmetics(token: string): Promise<OwnedCosmeticsResponse> {
  return keptGet<OwnedCosmeticsResponse>(keptPaths.cosmetics, token);
}

// ---------------------------------------------------------------------------------------------
// R1834000, R1834001: each card's picks on the account (R1655000). The device's copy is the one the
// faces read (`cards/cosmetics.ts`); `cards/cardPicksSync.ts` is the only caller of these three.
// ---------------------------------------------------------------------------------------------

/**
 * One card's picks as the account keeps them: an id this client does not know is kept, and `fullArt`
 * may be false, true (the Epic) or a full art's id (R2655000).
 */
export type CardPicksEntry = { cardId: string; foil?: string; finish?: string; border?: string; fullArt?: boolean | string };

/** Every card's picks the account holds, by card id. */
export type CardPicksResponse = { cards: CardPicksEntry[] };

/** One card's change: a field absent keeps the account's pick, null clears it. */
export type CardPickPut = {
  cardId: string;
  foil?: string | null;
  finish?: string | null;
  border?: string | null;
  fullArt?: boolean | string | null;
};

/** `GET /api/card-picks` (`active`). */
export function getCardPicks(token: string): Promise<CardPicksResponse> {
  return apiRequest<CardPicksResponse>("/api/card-picks", { token });
}

/** `PUT /api/card-picks` (`active`): one card's change; 400 `bad_request` for an unknown card or an id of the wrong kind or not owned (R1834001). */
export function putCardPick(token: string, change: CardPickPut): Promise<CardPicksResponse> {
  return apiRequest<CardPicksResponse>("/api/card-picks", { method: "PUT", token, body: change });
}

/** `POST /api/card-picks/upload` (`active`): the device's picks, once; it never overwrites a card the account holds. */
export function uploadCardPicks(token: string, cards: CardPicksEntry[]): Promise<CardPicksResponse> {
  return apiRequest<CardPicksResponse>("/api/card-picks/upload", { method: "POST", token, body: { cards } });
}

/** §14.9: one quest the caller holds, its progress toward `target`, and what it pays. */
export type QuestView = {
  questId: string;
  title: string;
  progress: number;
  target: number;
  jlore: number;
  xp: number;
  arrivedAt: number;
};

/**
 * R1304000: `GET /api/quests`, the caller's held quests, oldest first, and when the next arrives
 * (epoch ms); `null` while it holds the most it may, or while the economy is off (R1304002).
 */
export type QuestsResponse = {
  quests: QuestView[];
  nextDueAt: number | null;
  /** R2210000: the replaces the account may still spend today; a server from before #2210 sends none, which reads as none left. */
  replacesLeft?: number;
};

export function getQuests(token: string): Promise<QuestsResponse> {
  return keptGet<QuestsResponse>(keptPaths.quests, token);
}

/** R2210000: `POST /api/quests/:questId/replace`, a held quest swapped for another; answers the quests as `GET /api/quests` does. */
export async function replaceQuest(token: string, questId: string): Promise<QuestsResponse> {
  const answer = await apiRequest<QuestsResponse>(`/api/quests/${encodeURIComponent(questId)}/replace`, {
    method: "POST",
    token,
    body: {},
  });
  // R2125000: the next visit shows what this answer left.
  rememberRead(token, keptPaths.quests, answer);
  return answer;
}

// ---------------------------------------------------------------------------------------------
// Card packs (SPEC §14.3, §14.4; #1301, #1308): the odds and the counters the server holds, and an
// opening it has already decided slot by slot.
// ---------------------------------------------------------------------------------------------

/** The three rarities a pity counter tracks. */
export type PackCounters = { epic: number; legendary: number; mythic: number };

/**
 * One tiered roll's chance of each tier, the fallback included, as the server computes it from its
 * weight table (`TierOdds`, `crates/server/src/economy/tiers.rs`, R1339000).
 */
export type TierOdds = Record<"common" | "rare" | "epic" | "legendary" | "mythic", number>;

/** `GET /api/packs`: the pity counters, their thresholds, the odds and the unopened packs held. */
export type PacksResponse = {
  price: number;
  slots: number;
  pity: PackCounters;
  thresholds: PackCounters;
  /**
   * A slot's card rarity, the floored slot's, and a duplicate's cosmetic rarity; and a duplicate's
   * global roll (R1661001): `chance`, the share of its cosmetic rolls that drop an account cosmetic
   * while one is left unowned, and `tiers`, that cosmetic's rarity.
   */
  odds: {
    slot: TierOdds;
    flooredSlot: TierOdds;
    duplicateCosmetic: TierOdds;
    globalCosmetic: { chance: number; tiers: TierOdds };
  };
  /** One entry per kind of pack held (a shipped set's name, or `global`), the global pack first. */
  unopened: { pack: string; count: number }[];
};

/**
 * What a slot became (R1301002): a new card, a cosmetic for an owned card, or bar points. A
 * cosmetic's rarity is its own scale, never the slot's card's (R1333000).
 */
export type PackSlotGrant =
  | { kind: "card" }
  | { kind: "cosmetic"; cosmeticId: string; cosmeticRarity: CosmeticRarity }
  | { kind: "bars"; cosmeticRarity: CosmeticRarity };

export type PackSlot = { rarity: PrintedRarity; cardId: string; grant: PackSlotGrant };

/**
 * One token bar across an opening: its points before it and after it (the overflow a pay leaves
 * kept), its size and how many times it paid in this opening. Every opening moves both bars, since
 * every slot adds points (R1999000).
 */
export type BarSettle = { before: number; after: number; size: number; pays: number };

/**
 * What an opening did to both token bars (R1305000, R1305001): `cosmeticPaid` is the cycle's steps
 * the cosmetic bar paid, in order, so its length is `cosmetic.pays`, and `slotPoints` the points
 * each slot added to each bar, in slot order, every slot's its rolled rarity's (R1953000, R1999000).
 * The cosmetic bar's `size` is the step's it stands at after the opening (R2728000).
 */
export type OpeningBarsAnswer = {
  card: BarSettle;
  cosmetic: BarSettle;
  cosmeticPaid: CosmeticTokenStep[];
  /**
   * R2728000: the size of each step the cosmetic bar's sweep crosses: each step paid, then the step
   * it stands at after (`cosmetic.size`); `pays + 1` entries. Absent only from an answer kept before
   * #2728 (R2124000).
   */
  cosmeticSizes?: number[];
  slotPoints: number[];
};

/**
 * `POST /api/packs/open`: the slots in order, the pity counters after them, the packs of that kind
 * left and what the opening did to both token bars.
 */
export type OpenedPackResponse = {
  pack: string;
  slots: PackSlot[];
  pity: PackCounters;
  unopened: number;
  bars: OpeningBarsAnswer;
};

export function getPacks(token: string): Promise<PacksResponse> {
  return keptGet<PacksResponse>(keptPaths.packs, token);
}

/** R2125000: `pack`'s count, set in place or appended, and the kinds that have none left dropped. */
function withUnopened(unopened: PacksResponse["unopened"], pack: string, count: number): PacksResponse["unopened"] {
  const next = unopened.some((entry) => entry.pack === pack)
    ? unopened.map((entry) => (entry.pack === pack ? { pack, count } : entry))
    : [...unopened, { pack, count }];
  return next.filter((entry) => entry.count > 0);
}

/** R2125000: `item` in place of the entry with its id, or appended. */
function withItem<T extends { id: string }>(list: T[], item: T): T[] {
  return list.some((entry) => entry.id === item.id)
    ? list.map((entry) => (entry.id === item.id ? item : entry))
    : [...list, item];
}

export async function openPack(token: string, pack: string): Promise<OpenedPackResponse> {
  const answer = await spendRequest<OpenedPackResponse>("/api/packs/open", token, { pack });
  // R2125000: the next visit shows what this answer left.
  updateCachedRead<PacksResponse>(token, keptPaths.packs, (before) => ({
    ...before,
    pity: answer.pity,
    unopened: withUnopened(before.unopened, answer.pack, answer.unopened),
  }));
  return answer;
}

// ---------------------------------------------------------------------------------------------
// The Jlore shop and the token picker (SPEC §14.1, §14.5, §14.6; #1309): spending what the
// economy pays. Every number shown comes from these answers, never a client constant
// (CLAUDE.md rule 7): the wallet, the pack price, both bars and their sizes, the cycle's next
// step and the Chase Shard bar. A refusal rejects with `ApiRequestError` and changes nothing.
// ---------------------------------------------------------------------------------------------

/** `GET /api/wallet`: the caller's Jlore balance. */
export type WalletResponse = { jlore: number };

export function getWallet(token: string): Promise<WalletResponse> {
  return keptGet<WalletResponse>(keptPaths.wallet, token);
}

/** What reaching a set-track level pays (`LevelReward`, `crates/server/src/economy/track_rewards.rs`). */
export type TrackRewards = { jlore: number; setPacks: number; cardTokens: number };

/** One shipped set's track (§14.7): `xp` is its total; `levelXp`, `xpToNext` and `nextRewards` are null on the last level (R1302005). */
export type TrackView = {
  setId: SetName;
  level: number;
  xp: number;
  levelXp: number | null;
  xpToNext: number | null;
  nextRewards: TrackRewards | null;
  rewardsClaimed: number;
  unlocked: boolean;
};

/** `GET /api/tracks`: one track per shipped set, in catalog order (R1302003 unlocks them). */
export type TracksResponse = { tracks: TrackView[] };

export function getTracks(token: string): Promise<TracksResponse> {
  return keptGet<TracksResponse>(keptPaths.tracks, token);
}

/** `POST /api/packs/buy` with `{ pack }`: the pack, the wallet after it and the packs of its kind held. */
export type BoughtPackResponse = { pack: string; jlore: number; unopened: number };

export async function buyPack(token: string, pack: string): Promise<BoughtPackResponse> {
  const answer = await spendRequest<BoughtPackResponse>("/api/packs/buy", token, { pack });
  // R2125000: the next visit shows what this answer left.
  rememberRead(token, keptPaths.wallet, { jlore: answer.jlore } satisfies WalletResponse);
  updateCachedRead<PacksResponse>(token, keptPaths.packs, (before) => ({
    ...before,
    unopened: withUnopened(before.unopened, answer.pack, answer.unopened),
  }));
  return answer;
}

/** One token bar's points and size, as `GET /api/tokens` states them. */
export type TokenBarState = { points: number; size: number };

/** The cosmetic cycle's next step, as `nextStep` spells it: a tier token, or a cosmetic pack. */
export type CosmeticTokenStep = "rare" | "epic" | "legendary" | "cosmeticPack";

/**
 * `GET /api/tokens`: both bars' points and sizes, the cycle's next step and the tokens held
 * (R1305004); the cosmetic bar's size is its current step's (R2728000). `POST
 * /api/tokens/redeem-card` and `POST /api/tokens/redeem-cosmetic` answer the same after they spend.
 */
export type TokenBarsResponse = {
  cardBar: TokenBarState;
  cosmeticBar: TokenBarState;
  nextStep: CosmeticTokenStep;
  tokens: { card: number; cosmeticRare: number; cosmeticEpic: number; cosmeticLegendary: number; cosmeticPack: number };
};

export function getTokens(token: string): Promise<TokenBarsResponse> {
  return keptGet<TokenBarsResponse>(keptPaths.tokens, token);
}

/** `POST /api/tokens/redeem-card` with `{ cardId }`, answering the bars and tokens after it. */
export async function redeemCardToken(token: string, cardId: string): Promise<TokenBarsResponse> {
  const answer = await spendRequest<TokenBarsResponse>("/api/tokens/redeem-card", token, { cardId });
  // R2125000: the next visit shows what this answer left.
  rememberRead(token, keptPaths.tokens, answer);
  return answer;
}

/** A cosmetic token's tier, as `POST /api/tokens/redeem-cosmetic` parses `token` (R1305003, R2728000). */
export type CosmeticTokenTier = "Rare" | "Epic" | "Legendary";

/**
 * `POST /api/tokens/redeem-cosmetic` with `{ token, cosmeticId, cardId? }`, answering the bars
 * and tokens after it. A per-card cosmetic names its card; an account one sends none.
 */
export async function redeemCosmeticToken(
  token: string,
  cosmeticToken: CosmeticTokenTier,
  cosmeticId: string,
  cardId?: string,
): Promise<TokenBarsResponse> {
  const answer = await spendRequest<TokenBarsResponse>(
    "/api/tokens/redeem-cosmetic",
    token,
    cardId === undefined ? { token: cosmeticToken, cosmeticId } : { token: cosmeticToken, cosmeticId, cardId },
  );
  // R2125000: the next visit shows what this answer left.
  rememberRead(token, keptPaths.tokens, answer);
  return answer;
}

/**
 * `GET /api/cosmetic-packs`: the Chase Shard bar, the cosmetic packs unopened (R1306002) and a
 * slot's and the floored slot's odds (R1339000).
 */
export type CosmeticPacksResponse = {
  shardBar: TokenBarState;
  unopened: number;
  odds: { slot: TierOdds; flooredSlot: TierOdds };
};

export function getCosmeticPacks(token: string): Promise<CosmeticPacksResponse> {
  return keptGet<CosmeticPacksResponse>(keptPaths.cosmeticPacks, token);
}

/** What one cosmetic pack slot granted (R1306000, R1306001): an unowned cosmetic, or bar points. */
export type CosmeticSlotGrant =
  | {
      kind: "cosmetic";
      cosmeticId: string;
      cosmeticRarity: CosmeticRarity;
      /** The card a per-card cosmetic goes on, null for an account cosmetic. */
      cardId: string | null;
      /** Whether the account lacked that card, so the slot grants it too. */
      cardGranted: boolean;
    }
  | { kind: "bars" };

/** One slot of an opened cosmetic pack: the rarity it rolled and what that granted. */
export type CosmeticSlot = { rarity: CosmeticRarity; grant: CosmeticSlotGrant };

/**
 * `POST /api/cosmetic-packs/open`: its slots in order, the shard bar after it, the packs left and
 * what the opening did to both token bars.
 */
export type OpenedCosmeticPackResponse = {
  slots: CosmeticSlot[];
  /** The Chase Shard bar's points the opening found, for the bar to fill from to `shardBar`. */
  shardBefore: number;
  shardBar: TokenBarState;
  unopened: number;
  bars: OpeningBarsAnswer;
};

export async function openCosmeticPack(token: string): Promise<OpenedCosmeticPackResponse> {
  const answer = await spendRequest<OpenedCosmeticPackResponse>("/api/cosmetic-packs/open", token, {});
  // R2125000: the next visit shows what this answer left.
  updateCachedRead<CosmeticPacksResponse>(token, keptPaths.cosmeticPacks, (before) => ({
    ...before,
    shardBar: answer.shardBar,
    unopened: answer.unopened,
  }));
  return answer;
}

/**
 * `POST /api/cosmetic-packs/chase` with `{ cosmeticId, cardId? }`, answering the bar and packs
 * after it emptied the full bar. A per-card cosmetic names its card; an account one sends none.
 */
export async function buyChase(token: string, cosmeticId: string, cardId?: string): Promise<CosmeticPacksResponse> {
  const answer = await spendRequest<CosmeticPacksResponse>(
    "/api/cosmetic-packs/chase",
    token,
    cardId === undefined ? { cosmeticId } : { cosmeticId, cardId },
  );
  // R2125000: the next visit shows what this answer left.
  rememberRead(token, keptPaths.cosmeticPacks, answer);
  return answer;
}

// ---------------------------------------------------------------------------------------------
// Saved decks and trios (SPEC §9.4, R250–R256). Ids are minted HERE, by the client
// (`crypto.randomUUID()`), so every save is an idempotent `PUT` that can be retried after a dropped
// connection without making a second deck (R256).
// ---------------------------------------------------------------------------------------------

export type SavedDeck = {
  id: string;
  name: string;
  cards: string[];
  catalogVersion: string;
  /** R641: the deck's hero portrait, `null` meaning `vanilla` (D5). */
  portrait: string | null;
  /** R1297000: the deck's own playmat, backdrop and card back, each an id of its kind and absent where the deck has none. */
  tableCosmetic?: string | null;
  fieldCosmetic?: string | null;
  cardBackCosmetic?: string | null;
  createdAt: number;
  updatedAt: number;
};

export type TrioSlots = [string | null, string | null, string | null];

export type SavedTrio = {
  id: string;
  name: string;
  deckIds: TrioSlots;
  createdAt: number;
  updatedAt: number;
};

/** `GET /api/decks`: the profile's decks and trios, oldest first, and the caps they live under. */
export type DecksResponse = {
  catalogVersion: string;
  decks: SavedDeck[];
  trios: SavedTrio[];
  limits: { decks: number; trios: number; nameLength: number };
};

export function getDecks(token: string): Promise<DecksResponse> {
  return keptGet<DecksResponse>(keptPaths.decks, token);
}

export type DeckInput = {
  name: string;
  cards: readonly string[];
  catalogVersion: string;
  portrait?: string | null;
  /** R1297000: the deck's playmat, backdrop and card back; left out where it has none, which clears a stored pick. */
  tableCosmetic?: string | null;
  fieldCosmetic?: string | null;
  cardBackCosmetic?: string | null;
};

/**
 * `PUT /api/decks/:id`: creates the deck when the id is new, else replaces its name and cards.
 * Refusals: 400 `bad_request` (a D1–D4 draft rule, the issues in `details`), 409 `conflict` with
 * `details.limit` when the profile already has `limits.decks` decks, 409 `update_required` for a
 * stale catalog, 404 when the id is another profile's.
 */
export async function putDeck(token: string, id: string, input: DeckInput): Promise<{ deck: SavedDeck }> {
  const answer = await apiRequest<{ deck: SavedDeck }>(`/api/decks/${encodeURIComponent(id)}`, {
    method: "PUT",
    token,
    body: input,
  });
  // R2125000: the next visit shows what this answer left.
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    decks: withItem(before.decks, answer.deck),
  }));
  return answer;
}

/** `DELETE /api/decks/:id`: idempotent; `deleted: false` when there was nothing to delete. */
export async function deleteDeck(token: string, id: string, options: SendOptions = {}): Promise<{ deleted: boolean }> {
  const answer = await apiRequest<{ deleted: boolean }>(`/api/decks/${encodeURIComponent(id)}`, {
    method: "DELETE",
    token,
    ...options,
  });
  // R2125000: the next visit shows what this answer left; a trio slot naming the deck empties (R252).
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    decks: before.decks.filter((deck) => deck.id !== id),
    trios: before.trios.map((trio) =>
      trio.deckIds.includes(id)
        ? { ...trio, deckIds: trio.deckIds.map((slot) => (slot === id ? null : slot)) as TrioSlots }
        : trio,
    ),
  }));
  return answer;
}

/** R2100000: a version a save of the deck kept: its name and cards then, and when (ms). */
export type DeckVersion = { id: string; name: string; cards: string[]; savedAt: number };

/** `GET /api/decks/:id/versions` (R2100000): the deck's kept versions, newest first; 404 when the deck is not the caller's. */
export function getDeckVersions(token: string, deckId: string): Promise<{ versions: DeckVersion[] }> {
  return apiRequest<{ versions: DeckVersion[] }>(`/api/decks/${encodeURIComponent(deckId)}/versions`, { token });
}

/**
 * `POST /api/decks/:id/versions/:versionId/restore` (R2100000): saves the version's name and cards as
 * the deck's, keeping its portrait and picks, and keeps that as a new version, so the one it replaced
 * stays listed. Refusals as `putDeck`'s (a draft rule, `update_required`), and 404 for a deck or
 * version that is not the caller's.
 */
export async function restoreDeckVersion(
  token: string,
  deckId: string,
  versionId: string,
  catalogVersion: string,
): Promise<{ deck: SavedDeck; versions: DeckVersion[] }> {
  const answer = await apiRequest<{ deck: SavedDeck; versions: DeckVersion[] }>(
    `/api/decks/${encodeURIComponent(deckId)}/versions/${encodeURIComponent(versionId)}/restore`,
    { method: "POST", token, body: { catalogVersion } },
  );
  // R2125000: the next visit shows the restored deck, as after any save.
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    decks: withItem(before.decks, answer.deck),
  }));
  return answer;
}

export type TrioInput = { name: string; deckIds: TrioSlots };

/**
 * `PUT /api/trios/:id`: as `putDeck`. A slot naming a deck the server does not have yet is a 409
 * `conflict` with `details.unknownDeck`, which a client syncing an offline draft answers by saving
 * the deck first.
 */
export async function putTrio(token: string, id: string, input: TrioInput): Promise<{ trio: SavedTrio }> {
  const answer = await apiRequest<{ trio: SavedTrio }>(`/api/trios/${encodeURIComponent(id)}`, {
    method: "PUT",
    token,
    body: input,
  });
  // R2125000: the next visit shows what this answer left.
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    trios: withItem(before.trios, answer.trio),
  }));
  return answer;
}

/** One deck of an imported trio: its id (minted here, R256), name and cards. */
export type ImportedDeckInput = { id: string; name: string; cards: readonly string[] };

/** `POST /api/trios/import`'s body (R341): the trio and its three slots, an empty one null. */
export type TrioImportInput = {
  catalogVersion: string;
  trio: { id: string; name: string };
  slots: [ImportedDeckInput | null, ImportedDeckInput | null, ImportedDeckInput | null];
};

/**
 * `POST /api/trios/import` (R340, R341): makes the decks and the trio naming them, all or nothing.
 * Refusals: 400 for a draft rule (the deck named in the message), 409 `conflict` with
 * `details.decksShort` / `details.triosShort` when the caps would be passed (R340), 409
 * `update_required` for a stale catalog. Sending the same ids again is safe.
 */
export async function importTrio(
  token: string,
  input: TrioImportInput,
): Promise<{ decks: SavedDeck[]; trio: SavedTrio }> {
  const answer = await apiRequest<{ decks: SavedDeck[]; trio: SavedTrio }>("/api/trios/import", {
    method: "POST",
    token,
    body: input,
  });
  // R2125000: the next visit shows what this answer left.
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    decks: answer.decks.reduce((decks, deck) => withItem(decks, deck), before.decks),
    trios: withItem(before.trios, answer.trio),
  }));
  return answer;
}

export async function deleteTrio(token: string, id: string, options: SendOptions = {}): Promise<{ deleted: boolean }> {
  const answer = await apiRequest<{ deleted: boolean }>(`/api/trios/${encodeURIComponent(id)}`, {
    method: "DELETE",
    token,
    ...options,
  });
  // R2125000: the next visit shows what this answer left.
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    trios: before.trios.filter((trio) => trio.id !== id),
  }));
  return answer;
}

// ---------------------------------------------------------------------------------------------
// Tutorial progress on the account (SPEC §9.10, R320). The device's copy is the one the page
// renders (`tutorial/progress.ts`); `tutorial/accountSync.ts` is the only caller of these two.
// ---------------------------------------------------------------------------------------------

/** `TutorialProgressView` in `crates/server/src/api/tutorial.rs`. */
export type TutorialAccountProgress = {
  /** Lesson ids, each once, in code-point order; an id this client does not know may be among them. */
  completed: string[];
  /** The newest Hide/Show choice the account has seen (epoch ms), or null. */
  hiddenChoice: { hidden: boolean; at: number } | null;
};

/** `GET /api/tutorial` (`active`): the account's copy, empty before its first write. */
export async function getTutorialProgress(token: string): Promise<TutorialAccountProgress> {
  return (await apiRequest<{ progress: TutorialAccountProgress }>("/api/tutorial", { token })).progress;
}

/**
 * `PUT /api/tutorial` (`active`): merges this device's progress into the account's and answers with
 * the result — the union of the lessons and the newer choice (R320). It never removes anything, so
 * sending the same progress twice is harmless.
 */
export async function putTutorialProgress(
  token: string,
  progress: TutorialAccountProgress,
): Promise<TutorialAccountProgress> {
  return (
    await apiRequest<{ progress: TutorialAccountProgress }>("/api/tutorial", {
      method: "PUT",
      token,
      body: { completed: progress.completed, hiddenChoice: progress.hiddenChoice },
    })
  ).progress;
}

// ---------------------------------------------------------------------------------------------
// Game settings on the account (SPEC §9.1, R633, R634). The device's copy is the one the game
// reads (`settings/`); `settings/accountSync.ts` is the only caller of these two.
// ---------------------------------------------------------------------------------------------

/** One setting's value, as the server keeps it. */
export type PlayerSettingValue = boolean | number | string;

/** One group of settings (a store of the client) and when it last changed (epoch ms, the device's clock). */
export type PlayerSettingsGroup = { at: number; values: Record<string, PlayerSettingValue> };

/**
 * `PlayerSettingsView` in `crates/server/src/api/settings.rs`: group id to group, empty before the
 * first write. It carries no watching switch: friends may always watch (R2615000).
 */
export type PlayerSettingsAccountCopy = { groups: Record<string, PlayerSettingsGroup> };

/** `GET /api/settings` (`active`): the account's copy, empty before its first write. */
export async function getPlayerSettings(token: string): Promise<PlayerSettingsAccountCopy> {
  return (await apiRequest<{ settings: PlayerSettingsAccountCopy }>("/api/settings", { token })).settings;
}

/**
 * `PUT /api/settings` (`active`): merges the groups this device changed into the account's and
 * answers with the result. A group replaces the stored one only when its time is strictly later
 * (R634), so sending the same groups twice, or a stale one, is harmless.
 */
export async function putPlayerSettings(
  token: string,
  groups: Record<string, PlayerSettingsGroup>,
): Promise<PlayerSettingsAccountCopy> {
  return (
    await apiRequest<{ settings: PlayerSettingsAccountCopy }>("/api/settings", {
      method: "PUT",
      token,
      body: { groups },
    })
  ).settings;
}

// ---------------------------------------------------------------------------------------------
// Queue modes (SPEC §9.5, R257–R258, R264)
// ---------------------------------------------------------------------------------------------

export type QueueMode = "bo1" | "bo3" | "random";

/**
 * What a queue ticket or a room is made with: a deck, a trio, or nothing at all. All Random's
 * `leanSet` is the player's "More cards from" pick for their own dealt deck (R1372, R924000), sent
 * only when there is one: the server resolves "newest" and deals the deck (CLAUDE.md rule 7).
 */
export type ModeChoice =
  | { mode: "bo1"; deckId: string }
  | { mode: "bo3"; trioId: string }
  | { mode: "random"; leanSet?: LeanPick };

/** `POST /api/queue`. `matchId` for a paired Best-of-1 or All Random game, `seriesId` for Conquest. */
export type EnqueueResponse = {
  ticketId: string;
  status: "open" | "matched" | "cancelled";
  matchId: string | null;
  seriesId: string | null;
  mode: QueueMode;
};

export function enqueue(token: string, choice: ModeChoice): Promise<EnqueueResponse> {
  return apiRequest<EnqueueResponse>("/api/queue", { method: "POST", token, body: choice });
}

export function dequeue(token: string): Promise<{ cancelled: boolean; ticketId?: string }> {
  return apiRequest<{ cancelled: boolean; ticketId?: string }>("/api/queue", { method: "DELETE", token });
}

/**
 * `GET /api/queue/population`: the queue readout (R640002), every number counted over the last hour:
 * the games that ended in it, and per mode (R257) the expected wait in seconds, `null` when too few
 * tickets were claimed to say. Both are optional because a server from before R640002 sends neither.
 */
export type PopulationResponse = {
  gamesLastHour?: number;
  expectedWaitSeconds?: Record<QueueMode, number | null>;
};

export function getPopulation(token: string): Promise<PopulationResponse> {
  return apiRequest<PopulationResponse>("/api/queue/population", { token });
}

/** `POST /api/rooms` / `POST /api/rooms/:code/join` (§9.5, R264). */
export type CreateRoomResponse = { code: string; expiresAt: number; mode: QueueMode };

/**
 * A join answers with the match (Best-of-1, All Random) or the series (Conquest) it made. A joiner
 * whose choice is in another mode than the room's is refused with 409 `conflict` and
 * `details.mode`, the room's mode, so the lobby can ask for the right deck or trio.
 */
export type JoinRoomResponse = {
  matchId: string | null;
  seriesId: string | null;
  code: string;
  seat: "p1" | "p2";
  mode: QueueMode;
};

export function createRoom(token: string, choice: ModeChoice): Promise<CreateRoomResponse> {
  return apiRequest<CreateRoomResponse>("/api/rooms", { method: "POST", token, body: choice });
}

export function joinRoom(token: string, code: string, choice: ModeChoice): Promise<JoinRoomResponse> {
  return apiRequest<JoinRoomResponse>(`/api/rooms/${encodeURIComponent(code)}/join`, {
    method: "POST",
    token,
    body: choice,
  });
}

/** `GET /api/rooms` (R1075000): the caller's rooms nobody has joined yet, newest first. */
export type OpenRoomsResponse = { rooms: CreateRoomResponse[] };

export function getOpenRooms(token: string): Promise<OpenRoomsResponse> {
  return apiRequest<OpenRoomsResponse>("/api/rooms", { token });
}

/** `DELETE /api/rooms/:code` (R1075000): `cancelled: false` when no unjoined room of the caller's held it. */
export function cancelRoom(token: string, code: string): Promise<{ cancelled: boolean }> {
  return apiRequest<{ cancelled: boolean }>(`/api/rooms/${encodeURIComponent(code)}`, { method: "DELETE", token });
}

/** The room's mode from a mode-mismatch refusal (R264), or null for any other error. */
export function roomModeOf(error: unknown): QueueMode | null {
  if (!(error instanceof ApiRequestError) || error.code !== "conflict") return null;
  const details = error.details;
  if (typeof details !== "object" || details === null) return null;
  const mode = (details as { mode?: unknown }).mode;
  return mode === "bo1" || mode === "bo3" || mode === "random" ? mode : null;
}

// ---------------------------------------------------------------------------------------------
// The Conquest series (SPEC §9.5, R330–R336). The server's projection for the caller: the other
// side's pick is never in it before both have picked, and the other side's deck names never are.
// ---------------------------------------------------------------------------------------------

export type SeriesEnd = "decided" | "exhausted" | "forfeit" | "abandoned";

export type SeriesView = {
  id: string;
  status: "picking" | "playing" | "over";
  /** The game being picked for or played, or the last one played once the series is over. */
  gameNo: number;
  /** R330: game wins that take the series, one with each deck of the trio. */
  winsNeeded: number;
  /** R334: the most games the series plays, drawn games included. */
  maxGames: number;
  /** Epoch ms the pick clock runs out (R333), or null outside the pick phase. */
  pickDeadline: number | null;
  /** The server's clock when it answered, so a countdown does not depend on this device's. */
  now: number;
  /** The match to open while `status` is `playing`. */
  currentMatchId: string | null;
  /**
   * R604: the queue paired this series, so it is rated; a room's is not. Known from the start —
   * the player chose the mode — so it rides on the view, not only on `result`.
   */
  ranked: boolean;
  you: {
    seat: "p1" | "p2";
    wins: number;
    trioName: string;
    /** `won`: the deck has won a game in this series and is locked (R330); `games`: games it played. */
    decks: { slot: number; name: string; cards: string[]; won: boolean; games: number }[];
    /** Your sealed pick for the next game, or null (R331). */
    pick: number | null;
    /** R332: your pick was made for you, because one deck is left that has not won. */
    autoPick: boolean;
  };
  opponent: {
    wins: number;
    /** Only which slots have won: names, cards and picks stay hidden (R336). */
    decks: { slot: number; won: boolean }[];
    /** Whether their pick is in; never what (R331). */
    picked: boolean;
  };
  games: {
    gameNo: number;
    matchId: string;
    yourSlot: number;
    opponentSlot: number;
    youWentFirst: boolean;
    result: "win" | "loss" | "draw" | null;
    reason: GameOverReason | null;
  }[];
  /**
   * Null until the series is over. `ranked`: the series moved your rank (R604); the rating it
   * moved is never sent (R612), and the rank it left is `GET /api/ranked`'s.
   */
  result: {
    outcome: "win" | "loss" | "draw" | "abandoned";
    endReason: SeriesEnd;
    ranked: boolean;
  } | null;
};

export function getSeries(token: string, seriesId: string): Promise<SeriesView> {
  return apiRequest<SeriesView>(`/api/series/${encodeURIComponent(seriesId)}`, { token });
}

/** `GET /api/matches/:id/series`: the series a match belongs to, for the board's series banner. */
export function getSeriesForMatch(token: string, matchId: string): Promise<{ series: SeriesView | null }> {
  return apiRequest<{ series: SeriesView | null }>(`/api/matches/${encodeURIComponent(matchId)}/series`, { token });
}

// ---------------------------------------------------------------------------------------------
// The ranked ladder (SPEC §9.12, R612). Every shape below is `crates/server/src/api/ranked.rs`'s,
// and none of them carries the hidden rating — the client shows the visible rank only.
// ---------------------------------------------------------------------------------------------

export type GrapeTier = "rotten" | "normal" | "large" | "golden" | "mythic";

/** What a player is shown as (R605, R608, R612): never the rating. */
export type VisibleRank =
  | { tier: "raisin"; placementsPlayed: number; placementGames: number }
  | { tier: GrapeTier; division: number; pips: number; pipsPerDivision: number; floor: GrapeTier }
  | { tier: "jlorious"; position: number };

/** A season's best, as the profile's badge shows it (R607). */
export type PeakBadge =
  | { seasonId: string; tier: "jlorious"; position: number }
  | { seasonId: string; tier: GrapeTier; division: number };

/**
 * `GET /api/ranked`: the caller's own season, username, rank, streak, record and badges. The
 * profile id is for keys and requests; a player is only ever shown by `username` (R1436).
 */
export type OwnRankResponse = {
  season: string;
  profileId: string;
  username: string;
  rank: VisibleRank;
  streak: number;
  record: { games: number; wins: number; losses: number; draws: number };
  badges: PeakBadge[];
};

export function getOwnRank(token: string): Promise<OwnRankResponse> {
  return apiRequest<OwnRankResponse>("/api/ranked", { token });
}

/** `GET /api/leaderboard` (R608, R612): Jlorious #1–#100, then every other tier, then the Raisins. */
export type LeaderboardResponse = {
  season: string;
  jlorious: { position: number; profileId: string; username: string; you: boolean }[];
  tiers: {
    tier: GrapeTier;
    count: number;
    players: { profileId: string; username: string; division: number; pips: number; you: boolean }[];
  }[];
  raisins: number;
  you: VisibleRank;
};

export function getLeaderboard(token: string): Promise<LeaderboardResponse> {
  return keptGet<LeaderboardResponse>(keptPaths.leaderboard, token);
}

/**
 * `GET /api/matches/:id/ranks`: both seats' ranks and win streaks for the match screen (R604, R612, R2101000).
 * R2388000: once a ranked match's rated record is written, each seat's rank before and after it; absent until then, on an unranked match and on a series' earlier game.
 */
export type MatchRanksResponse = {
  ranked: boolean;
  seats: Record<
    "p1" | "p2",
    { profileId: string; username: string; rank: VisibleRank; streak: number; you: boolean; rankBefore?: VisibleRank; rankAfter?: VisibleRank }
  >;
};

export function getMatchRanks(token: string, matchId: string): Promise<MatchRanksResponse> {
  return apiRequest<MatchRanksResponse>(`/api/matches/${encodeURIComponent(matchId)}/ranks`, { token });
}

// ---------------------------------------------------------------------------------------------
// Online replays (SPEC §9.3, R768, #510): the shapes of `crates/server/src/api/replays.rs`. A step is
// the view the caller's seat held; no seed, log, state or deck crosses (CLAUDE.md rule 7).
// ---------------------------------------------------------------------------------------------

/** One finished match in `GET /api/replays`; `unavailable` is part 1's reason this build cannot show it. */
export type OnlineReplayListing = {
  matchId: string;
  /** Null for a match nothing records the mode of. */
  mode: QueueMode | null;
  endedAt: number;
  /** The seat the caller began in, whose view is step 0. */
  seat: PlayerId;
  result: "win" | "loss" | "draw";
  turns: number;
  /** Step 0 and one per logged action. */
  steps: number;
  portraits: { p1: PortraitId; p2: PortraitId };
  /** Both empty for a seat a deleted account left. */
  opponent: { profileId: string; username: string };
  unavailable?: ReplayRefusal;
};

/** `GET /api/replays`: one page, newest first, and where the next starts (null on the last). */
export type OnlineReplayList = { replays: OnlineReplayListing[]; nextOffset: number | null };

/**
 * `GET /api/replays/:matchId`: steps `[from, from + steps.length)` of `total`. R1834003: and both seats'
 * frozen loadouts (all defaults for a match from before R1834002), with the picks of the cards this
 * page's steps show; read as untrusted (`cards/loadouts.ts`).
 */
export type OnlineReplaySteps = {
  matchId: string;
  from: number;
  steps: ReplayStep[];
  total: number;
  loadouts?: unknown;
  cardLoadouts?: unknown;
};

export function getReplays(token: string, offset = 0): Promise<OnlineReplayList> {
  return apiRequest<OnlineReplayList>(`/api/replays?offset=${String(offset)}`, { token });
}

/** A page of a replay: 410 `gone` once its log is purged, 422 `replay_unavailable` (with `details.reason`) when this build cannot show it. */
export function getReplaySteps(token: string, matchId: string, from: number, count: number): Promise<OnlineReplaySteps> {
  return apiRequest<OnlineReplaySteps>(
    `/api/replays/${encodeURIComponent(matchId)}?from=${String(from)}&count=${String(count)}`,
    { token },
  );
}

// ---------------------------------------------------------------------------------------------
// Rematch offers after a finished non-series match (SPEC §9.5, R672). Only offer stakes,
// presence booleans and ids cross here — never decks, hands or ratings (CLAUDE.md rule 7).
// ---------------------------------------------------------------------------------------------

/** A rematch's stakes: 1 is a normal game, 2 is double-or-nothing (ranked only). */
export type RematchStakes = 1 | 2;

/** `POST /api/matches/:id/rematch`: the created game, or null while the seats disagree. */
export type RematchOfferResponse = { matchId: string | null };

/**
 * `GET /api/matches/:id/rematch`: both seats' offers, the opponent's presence, the created game, and
 * the mode a rematch plays (R1372: All Random's offers carry "More cards from the newest set"). A
 * server from before the mode answers without it.
 */
export type RematchStatusResponse = {
  youOffered: RematchStakes | null;
  opponentOffer: RematchStakes | null;
  opponentHere: boolean;
  matchId: string | null;
  mode?: QueueMode;
};

/**
 * `POST /api/matches/:id/rematch`. `lean` (R1372, R924000) is this seat's "More cards from" pick for
 * an All Random rematch's dealt deck, sent only when there is one; a Best-of-1 rematch replays its
 * decks and the server reads no lean.
 */
export function rematchOffer(
  token: string,
  matchId: string,
  stakes: RematchStakes,
  lean: LeanPick | null = null,
): Promise<RematchOfferResponse> {
  return apiRequest<RematchOfferResponse>(`/api/matches/${encodeURIComponent(matchId)}/rematch`, {
    method: "POST",
    token,
    body: lean === null ? { stakes } : { stakes, leanSet: lean },
  });
}

export function rematchStatus(token: string, matchId: string): Promise<RematchStatusResponse> {
  return apiRequest<RematchStatusResponse>(`/api/matches/${encodeURIComponent(matchId)}/rematch`, { token });
}

/**
 * `POST /api/series/:id/pick` with a trio slot (0-based) and the game it is for. Answers with the
 * new projection. A pick is sealed (R331): another slot afterwards is a 409, the same slot again
 * answers as success, and a pick for a game that has already begun is never applied to the next.
 */
export function pickSeriesDeck(token: string, seriesId: string, slot: number, gameNo: number): Promise<SeriesView> {
  return apiRequest<SeriesView>(`/api/series/${encodeURIComponent(seriesId)}/pick`, {
    method: "POST",
    token,
    body: { slot, gameNo },
  });
}

/** `POST /api/series/:id/forfeit`: between games only (R334). */
export function forfeitSeries(token: string, seriesId: string): Promise<SeriesView> {
  return apiRequest<SeriesView>(`/api/series/${encodeURIComponent(seriesId)}/forfeit`, {
    method: "POST",
    token,
  });
}

// ---------------------------------------------------------------------------------------------
// Public card and player statistics (SPEC §9.11, R654)
// ---------------------------------------------------------------------------------------------

export type PublicCardStat = {
  id: string;
  name: string;
  cost: number;
  rarity: string;
  set: string;
  games: number;
  winRate: number | null;
  drawnGames: number;
  drawnWinRate: number | null;
  playedGames: number;
  playedWinRate: number | null;
  playedRadiantGames?: number;
  playedRadiantWinRate?: number | null;
  playedNormalGames?: number;
  playedNormalWinRate?: number | null;
  drawnNotPlayedGames: number;
  drawnNotPlayedWinRate: number | null;
  playedDelta: number | null;
  /** R2427000: the decks that held the card and never drew it, and the drawn delta; absent from a server deployed before them. */
  notDrawnGames?: number;
  notDrawnWinRate?: number | null;
  drawnDelta?: number | null;
  playRate: number;
  hasEnoughGames: boolean;
  /** R1367000: the card's window, and the versions its figures count. */
  window?: PublicCardWindow;
};

/** R1367000: a card's window, and the versions its figures count. */
export type PublicCardWindow = {
  from: string;
  liveGames: number;
  source: PublicStatsSource;
  counted: string[];
};

/**
 * R2089000: a Created card's figures, apart from the deck cards': the seats that made a copy of it
 * during a game (a token, a generated or Discovered card, a copy, a transform's result, The Coin).
 * Used counts the seats that used a copy, split by whether one was Radiant at its first use; Not used
 * those that made copies and used none, the baseline the delta is taken from.
 */
export type PublicCreatedStat = {
  id: string;
  name: string;
  cost: number;
  rarity: string;
  set: string;
  /** The seats counted: `createdPlayedGames` and `createdNotPlayedGames`. */
  games: number;
  createdPlayedGames: number;
  createdPlayedWinRate: number | null;
  createdNotPlayedGames: number;
  createdNotPlayedWinRate: number | null;
  createdPlayedRadiantGames: number;
  createdPlayedRadiantWinRate: number | null;
  createdPlayedNormalGames: number;
  createdPlayedNormalWinRate: number | null;
  /** Used's win rate less Not used's, as a fraction; null until both sides have `minSample` games. */
  createdPlayedDelta: number | null;
  hasEnoughGames: boolean;
  window: PublicCardWindow;
};

/** R654, R1367000: which records a window's figures count. */
export type PublicStatsSource = "provisional" | "live" | "previous" | "dev";

export type PublicStatsCardsResponse = {
  patch: string;
  previousPatch: string | null;
  gate: {
    cleared: boolean;
    liveGames: number;
    minLiveGames: number;
  };
  source: PublicStatsSource;
  sourceLabel: string;
  pilot: "merged" | "human" | "ai";
  pilotLabel: string;
  mode: "all" | "bo1" | "bo3" | "random";
  modeLabel: string;
  minSample: number;
  totalGames: number;
  cards: PublicCardStat[];
  /**
   * R2089000: the Created cards, tokens among them, under the same filters as `cards`: enough games
   * first, then the most games, then by name. Absent from a response cached before them.
   */
  created?: PublicCreatedStat[];
  /**
   * R1973002: every patch the page can show, newest first: the window the cards share, its versions
   * oldest first (its figures are asked for at the last), its games and its live games. Absent from
   * a server older than it.
   */
  patches?: { versions: string[]; games: number; liveGames: number }[];
  summary: {
    totalGames: number;
    liveGames: number;
    activePatch: string;
    source: PublicStatsSource;
    pilot: "merged" | "human" | "ai";
    mode: "all" | "bo1" | "bo3" | "random";
    bestCard: { id: string; name: string; winRate: number; games: number } | null;
    worstCard: { id: string; name: string; winRate: number; games: number } | null;
  };
};

export type CardDrillDownResponse = {
  card: {
    id: string;
    name: string;
    cost: number;
    rarity: string;
  };
  patches: { patch: string; games: number; winRate: number | null }[];
  byTurn: { turn: number; games: number; winRate: number | null }[];
  coPlayed: { id: string; name: string; games: number; winRate: number | null }[];
};

export type CardStatsOptions = {
  source?: string;
  pilot?: string;
  patch?: string;
  set?: string;
  rarity?: string;
  cost?: number;
  card?: string;
  mode?: string;
  signal?: AbortSignal;
};

/** R2125000: the path of one filter's table, query included; its kept answer is a public one (`net/readCache.ts`). */
export function cardStatsPath(options: CardStatsOptions = {}): string {
  const params = new URLSearchParams();
  if (options.source) params.set("source", options.source);
  if (options.pilot) params.set("pilot", options.pilot);
  if (options.patch) params.set("patch", options.patch);
  if (options.set) params.set("set", options.set);
  if (options.rarity) params.set("rarity", options.rarity);
  if (options.cost !== undefined) params.set("cost", String(options.cost));
  if (options.card) params.set("card", options.card);
  if (options.mode) params.set("mode", options.mode);
  const query = params.toString();
  return `/api/stats/cards${query ? `?${query}` : ""}`;
}

export function getCardStats(options: CardStatsOptions = {}): Promise<PublicStatsCardsResponse> {
  return keptGet<PublicStatsCardsResponse>(cardStatsPath(options), null, options.signal);
}

export function getCardDrillDown(
  cardId: string,
  mode?: string,
  signal?: AbortSignal,
): Promise<CardDrillDownResponse> {
  const query = mode ? `?mode=${encodeURIComponent(mode)}` : "";
  return apiRequest<CardDrillDownResponse>(`/api/stats/cards/${encodeURIComponent(cardId)}${query}`, {
    signal,
  });
}

/**
 * R1994000: how a patch's games went, over the games whose records carry the figures. Shares are
 * 0..1 to three places; `firstShare` is the first seat's share of the decided games (`games - draws`),
 * null when every game was drawn (R2567000); `firstLow` and `firstHigh` bound it at 95% (Wilson), 0 and 1
 * when it is null.
 * The fatigue, trap, position and Radiant figures count only the `counted` games whose records keep
 * them, and are null when there are none.
 */
export type PublicGameFigures = {
  games: number;
  draws: number;
  firstWins: number;
  firstShare: number | null;
  firstLow: number;
  firstHigh: number;
  /** Every way a game ended ("hero-death", "turn-cap", …) to its games. */
  reasons: Record<string, number>;
  /** The turn counter at the end: Q1, median, Q3. */
  turnQuartiles: [number, number, number];
  counted: number;
  fatigueGames: number | null;
  fatigueShare: number | null;
  fatigueDamageMean: number | null;
  trapsSet: number | null;
  trapsFired: number | null;
  trapsFiredPerSet: number | null;
  positionSwitches: number | null;
  positionSwitchesMean: number | null;
  radiantUpgrades: number | null;
  radiantUpgradesMean: number | null;
};

/** R1994000: `GET /api/stats/games`. The client shows neither `source` nor `sourceLabel` (R661). */
export type PublicStatsGamesResponse = {
  patch: string;
  source: PublicStatsSource;
  sourceLabel: string;
  mode: "all" | "bo1" | "bo3" | "random";
  modeLabel: string;
  minSample: number;
  /** Every game the patch's window counted, those recorded before these figures were kept among them. */
  totalGames: number;
  /** Null when no game on the patch carries the figures. */
  figures: PublicGameFigures | null;
  /** The same figures for each pair of seat pilots. */
  pairs: { pilots: ["human" | "ai", "human" | "ai"]; figures: PublicGameFigures }[];
};

export type GameStatsOptions = {
  patch?: string;
  mode?: string;
  signal?: AbortSignal;
};

export function getGameStats(options: GameStatsOptions = {}): Promise<PublicStatsGamesResponse> {
  const params = new URLSearchParams();
  if (options.patch) params.set("patch", options.patch);
  if (options.mode) params.set("mode", options.mode);
  const query = params.toString();
  return apiRequest<PublicStatsGamesResponse>(`/api/stats/games${query ? `?${query}` : ""}`, {
    signal: options.signal,
  });
}

/** R1583000: `GET /api/stats/play-rates`: each listed card's share of the human live seats that played it. */
export type PlayRatesResponse = {
  decks: number;
  cards: { id: string; rate: number }[];
};

export function getPlayRates(signal?: AbortSignal): Promise<PlayRatesResponse> {
  return apiRequest<PlayRatesResponse>("/api/stats/play-rates", { signal });
}

export type PlayerStatsAccountResponse = {
  stats: Record<string, unknown>;
  isPrivate: boolean;
  updatedAt: number | null;
};

export function putPlayerStats(
  token: string,
  data: { stats?: Record<string, unknown>; isPrivate?: boolean },
): Promise<PlayerStatsAccountResponse> {
  return apiRequest<PlayerStatsAccountResponse>("/api/stats/player", {
    method: "PUT",
    token,
    body: data,
  });
}

export type PublicPlayerSummary = {
  /** For keys only: a player is shown by `username`, never by anything made from this (R1436). */
  profileId: string;
  username: string;
  games: number;
  wins: number;
  losses: number;
  draws: number;
  winRate: number | null;
  favouriteCards: readonly { id: string; count: number }[];
  funStats: {
    nemesisCardId: string | null;
    totalDestroyed: number;
    totalDefeated: number;
  };
  updatedAt: number;
};

export type PublicPlayersResponse = {
  players: PublicPlayerSummary[];
  page: number;
  limit: number;
};

/** `GET /api/stats/players`. `search` matches usernames, folded and compared by the server, not here. */
export function getPublicPlayers(options: { search?: string; page?: number; signal?: AbortSignal } = {}): Promise<PublicPlayersResponse> {
  const params = new URLSearchParams();
  if (options.search) params.set("search", options.search);
  if (options.page !== undefined) params.set("page", String(options.page));
  const query = params.toString();
  return apiRequest<PublicPlayersResponse>(`/api/stats/players${query ? `?${query}` : ""}`, {
    signal: options.signal,
  });
}

