// The client's one door onto `crates/server`'s REST surface.
//
// CLAUDE.md rule 7: nothing here decides a rule; no L1–L6 message is ever composed in the browser. A
// `LoadoutIssue.message` is the validator's own sentence (`crates/engine/src/validator.rs`, one module
// for client and server, SPEC §9.4) or the server's untouched relay of it: a second copy would be a
// second source of truth. Every shape below is what `crates/server/src/api/*.rs` returns; every error
// is `{ error: { code, message, details? } }` from `crates/server/src/api/http.rs`.

import type { ReplayRefusal, ReplayStep } from "@jackioh/engine";
import type { GameOverReason, LeanPick, PlayerId, PortraitId, PrintedRarity, SetName } from "@jackioh/shared";

import { API_REQUEST_TIMEOUT_SECONDS } from "@jackioh/server-config";

import type { CosmeticRarity } from "../wire/generated/cosmetics.ts";
import { rememberRead, updateCachedRead } from "./readCache.ts";
import { whileSpending } from "./spending.ts";

const DEFAULT_HTTP_URL = "http://localhost:8787";

/**
 * `VITE_SERVER_HTTP_URL` is public (`crates/server/src/env.rs` `PUBLIC_ENV_VARS`). The default is
 * the port `crates/server` listens on, which `e2e/support/config.ts` points at.
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

const MATCH_SOCKET_PATH = /\/ws\/match\/?$/u;

/** A socket on the same server as `/ws/match` (#936: `/ws/duos`, `/ws/watch`), so one variable names the server. */
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
 * The transport failed: no response, or none within `API_REQUEST_TIMEOUT_SECONDS`. The browser's own
 * wording ("Failed to fetch", Safari's "Load failed") says nothing to a player and is kept aside.
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

/** One request: the parsed body on 2xx, else `ApiRequestError`, so no caller checks `response.ok`. */
export async function apiRequest<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = {};
  if (options.token !== undefined && options.token !== null && options.token.length > 0) {
    headers.authorization = `Bearer ${options.token}`;
  }
  if (options.body !== undefined) headers["content-type"] = "application/json";
  if (options.idempotencyKey !== undefined) headers["idempotency-key"] = options.idempotencyKey;

  // A request that never answers must not leave a screen on "Redeeming…" for ever. The race is against
  // our own timer as well as the abort, so a `fetch` that ignores its signal still gives up.
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
 * R2125000: the menu reads whose last answer is kept (`net/readCache.ts`), by path; also `cardStatsPath`
 * and `FRIENDS_PATH` (`net/social.ts`). Nothing a match, a duos room, a watch or a replay reads is here.
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

// R2124000: a spend runs once. Every spend sends an `Idempotency-Key` minted at the press and resent by
// its retries, so a spend the server finished after we gave up spends nothing again. Any answer but a
// 429 (which ran nothing, R2378000) or a server error ends the press; a press is the spend's path and
// body. Each spend is out (`net/spending.ts`) until it settles.

const FIRST_SERVER_ERROR_STATUS = 500;

/** R2378000: ran nothing (a spend past the ones waiting their turn, or R109's limit). */
const RAN_NOTHING_STATUS = 429;

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

/** `GET /api/auth/me` (`auth: "user"`): the code screen's own read (§9.4). */
export type MeResponse = {
  // R612: the hidden rating is never sent, not even to its owner; the client shows `GET /api/ranked`'s rank.
  profile: { id: string; status: "pending" | "active" | "banned" };
  needsInviteCode: boolean;
  emailVerified: boolean;
  /** §9.5: the match this profile is in, or null. What `/play` waits on after it queues. */
  currentMatchId: string | null;
  /**
   * R330: the Conquest series this profile is in (not over), or null; set between games too, so `/play`
   * can send a paired player to pick. Optional because an older server (before R259) omits it.
   */
  currentSeriesId?: string | null;
  /** R1566000: the live co-op match this profile is seated in, or null (it sets no `currentMatchId`). Optional. */
  currentDuosMatchId?: string | null;
  /** So a player can see who they are signed in as. */
  email: string | null;
  /** R1435: the caller's username. Optional: an older server or a test stub omits it. */
  username?: OwnUsername;
  /** R757000: a pending account that has not answered the age screen. Missing means nothing is owed. */
  ageCheckOwed?: boolean;
  /** R757001: the account has not agreed to the terms in force (`TERMS_VERSION`). Missing: not owed. */
  termsOwed?: boolean;
  /** R897000: whether this profile holds an open queue ticket. Missing never ends a wait. */
  queued?: boolean;
  /** R1300000: the server's `DEV_PROFILE_IDS` lists this account (it checks again on every dev route). */
  isDev?: boolean;
};

/** The caller's own username, as `/api/auth/me` and every username route answer with it (R1435). */
export type OwnUsername = {
  /** `Max` or `Max#3`. */
  name: string;
  /** Epoch ms when the next change is allowed, or null when a change is allowed now. */
  nextChangeAt: number | null;
  /** True while the username prompt shown after activation is still owed. */
  promptOwed: boolean;
};

/** `GET /api/profile`: the account screen's read — identity plus the ladder record. */
export type ProfileResponse = {
  id: string;
  email: string | null;
  status: "pending" | "active" | "banned";
  // R612: no hidden rating here either.
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

// Usernames (R1435, R1436). The server validates, normalizes and tags a name; the client saves exactly
// the username the server's preview named, so a player never gets a tag they were not shown.

/** Why the server refused a name. Its `message` is the sentence a player reads, shown as is. */
export type UsernameRefusal = "characters" | "mixed_scripts" | "too_short" | "too_long" | "not_allowed" | "unchanged";

/** `GET /api/username/preview`: what a save of `name` would give; `tagged`: the base is taken. */
export type UsernamePreview =
  | { ok: true; base: string; username: string; tagged: boolean }
  | { ok: false; reason: UsernameRefusal; message: string }
  | { ok: false; reason: "cooldown"; message: string; nextChangeAt: number };

export function previewUsername(token: string, name: string, signal?: AbortSignal): Promise<UsernamePreview> {
  return apiRequest<UsernamePreview>(`/api/username/preview?name=${encodeURIComponent(name)}`, { token, signal });
}

/**
 * `PUT /api/username` with a preview's `username`, exactly. Refusals: 400 `bad_request` with the
 * refusing preview in `details`, 409 `conflict` with a fresh one when the outcome changed since the
 * player looked. `usernamePreviewOf` reads either.
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
 * `POST /api/account/consent` (R757000, R757001): agreement to `termsVersion` and, while the age check
 * is owed, the month and year of birth, which the server does not keep. Refusals: 403 `under_age` (the
 * account is deleted), 409 `update_required` (the terms changed), 400 `bad_request`.
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

/** `DELETE /api/account` (204, no body). What it removes and what it keeps is the server's to decide. */
export async function deleteAccount(token: string): Promise<void> {
  await apiRequest<null>("/api/account", { method: "DELETE", token });
}

// The dev tools (SPEC §14.10, R1300000): each answers 403 `forbidden` to an account `DEV_PROFILE_IDS`
// does not list, and 400 to an amount that is not a positive whole number; the client sends what was typed.

/** What `POST /api/dev/clear/:part` clears: one holding, or all five. */
export type DevClearPart = "cards" | "cosmetics" | "jlore" | "counters" | "tracks" | "everything";

/** A set track as the server holds it (SPEC §14.7). */
export type DevSetTrack = { setId: string; xp: number; level: number; rewardsClaimed: number };

/** XP to a shipped set's track, through the level curve. */
export function devGiveXp(token: string, setId: SetName, xp: number): Promise<{ track: DevSetTrack }> {
  return apiRequest<{ track: DevSetTrack }>("/api/dev/xp", { method: "POST", token, body: { setId, xp } });
}

export function devGiveJlore(token: string, jlore: number): Promise<{ jlore: number }> {
  return apiRequest<{ jlore: number }>("/api/dev/jlore", { method: "POST", token, body: { jlore } });
}

/** Every card and cosmetic, answering the ledger rows written. */
export function devFill(token: string): Promise<{ cards: number; cosmetics: number }> {
  return apiRequest<{ cards: number; cosmetics: number }>("/api/dev/fill", { method: "POST", token, body: {} });
}

export function devClear(token: string, part: DevClearPart): Promise<{ written: number }> {
  return apiRequest<{ written: number }>(`/api/dev/clear/${part}`, { method: "POST", token, body: {} });
}

export type DevBar = "card" | "cosmetic";

/** R1310003: `POST /api/dev/bar/:bar` (SPEC §14.5): fills that token bar so it pays its token at once. */
export function devFillBar(token: string, bar: DevBar): Promise<{ written: number }> {
  return apiRequest<{ written: number }>(`/api/dev/bar/${bar}`, { method: "POST", token, body: {} });
}

/** A quest at once (§14.9); 409 while the most are held. */
export function devGiveQuest(token: string): Promise<{ quest: QuestView }> {
  return apiRequest<{ quest: QuestView }>("/api/dev/quests", { method: "POST", token, body: {} });
}

/** Completes a held quest, answering what it paid; 404 for one not held. */
export function devCompleteQuest(token: string, questId: string): Promise<{ completed: string; jlore: number; xp: number }> {
  return apiRequest<{ completed: string; jlore: number; xp: number }>(`/api/dev/quests/${encodeURIComponent(questId)}/complete`, {
    method: "POST",
    token,
    body: {},
  });
}

// Sign-in and sign-up are NOT here: SPEC §9.2 sends the browser straight to the auth provider, from
// `net/auth.ts`. The server's `/api/auth/signin` is BUILD M8's fixture-account path, which answers 503 in
// a normal deployment, so a helper for it would be a sign-in that silently fails. The one sign-in that
// reaches this server is a username's (R1443), via `net/auth.ts`'s `signInWithUsername`.

/**
 * `GET /api/codes/status`: whether redemption is paused, and how many redemptions are left in §9.4's
 * window. `attemptsRemaining` is optional (an older server omits it; the screen then invents no
 * number) and advisory: the per-IP limit can still refuse first (the 429 covers it).
 */
export type CodeStatusResponse = {
  redemptionEnabled: boolean;
  retryAfterMs: number;
  attemptsRemaining?: number;
  /** With no tries left, how long until one comes back; 0 otherwise (R192). Optional, as above. */
  attemptsRetryAfterMs?: number;
};

/**
 * R192: how long a `rate_limited` refusal says to wait (`details.retryAfterMs`), else null. Never read
 * from any other code: R145's `invalid_code` carries no details on purpose.
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

// R1834000, R1834001: each card's picks on the account (R1655000). The device's copy is the one the
// faces read (`cards/cosmetics.ts`); `cards/cardPicksSync.ts` is the only caller of these three.

/** One card's picks as the account keeps them: an id this client does not know is kept, and `fullArt` may be false. */
export type CardPicksEntry = { cardId: string; foil?: string; finish?: string; border?: string; fullArt?: boolean };

export type CardPicksResponse = { cards: CardPicksEntry[] };

/** One card's change: a field absent keeps the account's pick, null clears it. */
export type CardPickPut = {
  cardId: string;
  foil?: string | null;
  finish?: string | null;
  border?: string | null;
  fullArt?: boolean | null;
};

export function getCardPicks(token: string): Promise<CardPicksResponse> {
  return apiRequest<CardPicksResponse>("/api/card-picks", { token });
}

/** `PUT /api/card-picks` (`active`): one card's change; 400 for an unknown card, or an id of the wrong kind or not owned. */
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
 * R1304000: `GET /api/quests`, oldest first, and when the next arrives (epoch ms); `null` while the
 * most are held, or while the economy is off (R1304002).
 */
export type QuestsResponse = {
  quests: QuestView[];
  nextDueAt: number | null;
  /** R2210000: the replaces left today; an older server (before #2210) sends none, which reads as none left. */
  replacesLeft?: number;
};

export function getQuests(token: string): Promise<QuestsResponse> {
  return keptGet<QuestsResponse>(keptPaths.quests, token);
}

/** `POST /api/quests/:questId/replace`: a held quest swapped for another; answers as `GET /api/quests`. */
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

// Card packs (SPEC §14.3, §14.4; #1301, #1308): the server holds the odds and counters and decides an
// opening slot by slot.

export type PackCounters = { epic: number; legendary: number; mythic: number };

/** One tiered roll's chance of each tier, fallback included (`TierOdds`, `crates/server/src/economy/tiers.rs`, R1339000). */
export type TierOdds = Record<"common" | "rare" | "epic" | "legendary" | "mythic", number>;

export type PacksResponse = {
  price: number;
  slots: number;
  pity: PackCounters;
  thresholds: PackCounters;
  /** R1661001 `globalCosmetic`: the share of a duplicate's cosmetic rolls that drop an unowned account cosmetic, and its rarity. */
  odds: {
    slot: TierOdds;
    flooredSlot: TierOdds;
    duplicateCosmetic: TierOdds;
    globalCosmetic: { chance: number; tiers: TierOdds };
  };
  /** One entry per kind of pack held (a shipped set's name, or `global`), the global pack first. */
  unopened: { pack: string; count: number }[];
};

/** What a slot became (R1301002). A cosmetic's rarity is its own scale, never the slot's card's (R1333000). */
export type PackSlotGrant =
  | { kind: "card" }
  | { kind: "cosmetic"; cosmeticId: string; cosmeticRarity: CosmeticRarity }
  | { kind: "bars"; cosmeticRarity: CosmeticRarity };

export type PackSlot = { rarity: PrintedRarity; cardId: string; grant: PackSlotGrant };

/** One token bar across an opening (`after` keeps a pay's overflow). Every slot adds points to both bars (R1999000). */
export type BarSettle = { before: number; after: number; size: number; pays: number };

/**
 * What an opening did to both token bars (R1305000, R1305001): `cosmeticPaid` is the steps the cosmetic
 * bar paid, in order (`cosmetic.pays` long), `slotPoints` each slot's points, by its rolled rarity (R1953000).
 */
export type OpeningBarsAnswer = {
  card: BarSettle;
  cosmetic: BarSettle;
  cosmeticPaid: CosmeticTokenStep[];
  slotPoints: number[];
};

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

/** `pack`'s count, set in place or appended; kinds with none left dropped. */
function withUnopened(unopened: PacksResponse["unopened"], pack: string, count: number): PacksResponse["unopened"] {
  const next = unopened.some((entry) => entry.pack === pack)
    ? unopened.map((entry) => (entry.pack === pack ? { pack, count } : entry))
    : [...unopened, { pack, count }];
  return next.filter((entry) => entry.count > 0);
}

function withItem<T extends { id: string }>(list: T[], item: T): T[] {
  return list.some((entry) => entry.id === item.id)
    ? list.map((entry) => (entry.id === item.id ? item : entry))
    : [...list, item];
}

export async function openPack(token: string, pack: string): Promise<OpenedPackResponse> {
  const answer = await spendRequest<OpenedPackResponse>("/api/packs/open", token, { pack });
  updateCachedRead<PacksResponse>(token, keptPaths.packs, (before) => ({
    ...before,
    pity: answer.pity,
    unopened: withUnopened(before.unopened, answer.pack, answer.unopened),
  }));
  return answer;
}

// The Jlore shop and the token picker (SPEC §14.1, §14.5, §14.6; #1309). Every number shown comes from
// these answers, never a client constant (CLAUDE.md rule 7). A refusal changes nothing.

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

export type BoughtPackResponse = { pack: string; jlore: number; unopened: number };

export async function buyPack(token: string, pack: string): Promise<BoughtPackResponse> {
  const answer = await spendRequest<BoughtPackResponse>("/api/packs/buy", token, { pack });
  rememberRead(token, keptPaths.wallet, { jlore: answer.jlore } satisfies WalletResponse);
  updateCachedRead<PacksResponse>(token, keptPaths.packs, (before) => ({
    ...before,
    unopened: withUnopened(before.unopened, answer.pack, answer.unopened),
  }));
  return answer;
}

export type TokenBarState = { points: number; size: number };

export type CosmeticTokenStep = "rare" | "epic" | "cosmeticPack";

/** `GET /api/tokens` (R1305004); both redeem routes answer the same after they spend. */
export type TokenBarsResponse = {
  cardBar: TokenBarState;
  cosmeticBar: TokenBarState;
  nextStep: CosmeticTokenStep;
  tokens: { card: number; cosmeticRare: number; cosmeticEpic: number; cosmeticPack: number };
};

export function getTokens(token: string): Promise<TokenBarsResponse> {
  return keptGet<TokenBarsResponse>(keptPaths.tokens, token);
}

export async function redeemCardToken(token: string, cardId: string): Promise<TokenBarsResponse> {
  const answer = await spendRequest<TokenBarsResponse>("/api/tokens/redeem-card", token, { cardId });
  rememberRead(token, keptPaths.tokens, answer);
  return answer;
}

/** A cosmetic token's tier, as `POST /api/tokens/redeem-cosmetic` parses `token` (R1305003). */
export type CosmeticTokenTier = "Rare" | "Epic";

/** A per-card cosmetic names its card; an account one sends none. */
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
  rememberRead(token, keptPaths.tokens, answer);
  return answer;
}

/** `GET /api/cosmetic-packs`: the Chase Shard bar, the packs unopened (R1306002) and the odds. */
export type CosmeticPacksResponse = {
  shardBar: TokenBarState;
  unopened: number;
  odds: { slot: TierOdds; flooredSlot: TierOdds };
};

export function getCosmeticPacks(token: string): Promise<CosmeticPacksResponse> {
  return keptGet<CosmeticPacksResponse>(keptPaths.cosmeticPacks, token);
}

/** What one cosmetic pack slot granted (R1306000, R1306001). */
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

export type CosmeticSlot = { rarity: CosmeticRarity; grant: CosmeticSlotGrant };

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
  updateCachedRead<CosmeticPacksResponse>(token, keptPaths.cosmeticPacks, (before) => ({
    ...before,
    shardBar: answer.shardBar,
    unopened: answer.unopened,
  }));
  return answer;
}

/** `POST /api/cosmetic-packs/chase`: empties the full bar. `cardId` as in `redeemCosmeticToken`. */
export async function buyChase(token: string, cosmeticId: string, cardId?: string): Promise<CosmeticPacksResponse> {
  const answer = await spendRequest<CosmeticPacksResponse>(
    "/api/cosmetic-packs/chase",
    token,
    cardId === undefined ? { cosmeticId } : { cosmeticId, cardId },
  );
  rememberRead(token, keptPaths.cosmeticPacks, answer);
  return answer;
}

// Saved decks and trios (SPEC §9.4, R250–R256). Ids are minted by the client, so every save is an
// idempotent `PUT` that a retry after a dropped connection cannot turn into a second deck (R256).

export type SavedDeck = {
  id: string;
  name: string;
  cards: string[];
  catalogVersion: string;
  /** R641: the deck's hero portrait, `null` meaning `vanilla` (D5). */
  portrait: string | null;
  /** R1297000: the deck's own playmat, backdrop and card back, absent where it has none. */
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

/** `GET /api/decks`: oldest first, and the caps they live under. */
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
  /** Left out where the deck has none, which clears a stored pick. */
  tableCosmetic?: string | null;
  fieldCosmetic?: string | null;
  cardBackCosmetic?: string | null;
};

/**
 * `PUT /api/decks/:id`: creates or replaces. Refusals: 400 `bad_request` (a D1–D4 draft rule, issues in
 * `details`), 409 `conflict` with `details.limit` at `limits.decks`, 409 `update_required` for a stale
 * catalog, 404 when the id is another profile's.
 */
export async function putDeck(token: string, id: string, input: DeckInput): Promise<{ deck: SavedDeck }> {
  const answer = await apiRequest<{ deck: SavedDeck }>(`/api/decks/${encodeURIComponent(id)}`, {
    method: "PUT",
    token,
    body: input,
  });
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
  // A trio slot naming the deck empties (R252).
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

/** `GET /api/decks/:id/versions`: newest first; 404 when the deck is not the caller's. */
export function getDeckVersions(token: string, deckId: string): Promise<{ versions: DeckVersion[] }> {
  return apiRequest<{ versions: DeckVersion[] }>(`/api/decks/${encodeURIComponent(deckId)}/versions`, { token });
}

/**
 * `POST /api/decks/:id/versions/:versionId/restore`: saves the version's name and cards as the deck's,
 * keeping its portrait and picks, as a new version, so the one it replaced stays listed. Refusals as
 * `putDeck`'s.
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
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    decks: withItem(before.decks, answer.deck),
  }));
  return answer;
}

export type TrioInput = { name: string; deckIds: TrioSlots };

/**
 * `PUT /api/trios/:id`: as `putDeck`. A slot naming a deck the server does not have yet is a 409
 * `conflict` with `details.unknownDeck`: save the deck first.
 */
export async function putTrio(token: string, id: string, input: TrioInput): Promise<{ trio: SavedTrio }> {
  const answer = await apiRequest<{ trio: SavedTrio }>(`/api/trios/${encodeURIComponent(id)}`, {
    method: "PUT",
    token,
    body: input,
  });
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    trios: withItem(before.trios, answer.trio),
  }));
  return answer;
}

/** One deck of an imported trio; its id is minted here (R256). */
export type ImportedDeckInput = { id: string; name: string; cards: readonly string[] };

/** `POST /api/trios/import`'s body (R341): an empty slot is null. */
export type TrioImportInput = {
  catalogVersion: string;
  trio: { id: string; name: string };
  slots: [ImportedDeckInput | null, ImportedDeckInput | null, ImportedDeckInput | null];
};

/**
 * `POST /api/trios/import` (R340): the decks and the trio, all or nothing; the same ids again is safe.
 * Refusals: 400 for a draft rule, 409 `conflict` with `details.decksShort` / `details.triosShort` past
 * the caps, 409 `update_required` for a stale catalog.
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
  updateCachedRead<DecksResponse>(token, keptPaths.decks, (before) => ({
    ...before,
    trios: before.trios.filter((trio) => trio.id !== id),
  }));
  return answer;
}

// Tutorial progress on the account (SPEC §9.10, R320). The device's copy is the one the page
// renders (`tutorial/progress.ts`); `tutorial/accountSync.ts` is the only caller of these two.

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

/** `PUT /api/tutorial` (`active`): merges (the union of lessons, the newer choice) and answers the result; never removes. */
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

// Game settings on the account (SPEC §9.1, R633, R634). The device's copy is the one the game
// reads (`settings/`); `settings/accountSync.ts` is the only caller of these two.

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
 * `PUT /api/settings` (`active`): merges and answers the result. A group replaces the stored one only
 * when its time is strictly later (R634), so a repeat or a stale one is harmless.
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

// Queue modes (SPEC §9.5, R257–R258, R264)

export type QueueMode = "bo1" | "bo3" | "random";

/**
 * What a queue ticket or a room is made with. `leanSet` is the "More cards from" pick for the dealt
 * deck (R1372, R924000), sent only when there is one: the server resolves "newest" and deals.
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
 * `GET /api/queue/population` (R640002), over the last hour: the games ended, and per mode (R257) the
 * expected wait in seconds, `null` when too few tickets were claimed. An older server sends neither.
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
 * A join answers with the match or the series (Conquest) it made. A choice in another mode than the
 * room's is a 409 `conflict` with the room's `details.mode`, so the lobby can ask for the right one.
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

/** `DELETE /api/rooms/:code`: `cancelled: false` when no unjoined room of the caller's held it. */
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

// The Conquest series (SPEC §9.5, R330–R336). The server's projection for the caller: the other
// side's pick is never in it before both have picked, and the other side's deck names never are.

export type SeriesEnd = "decided" | "exhausted" | "forfeit" | "abandoned";

export type SeriesView = {
  id: string;
  status: "picking" | "playing" | "over";
  /** The game being picked for or played, or the last one once the series is over. */
  gameNo: number;
  /** R330: game wins that take the series, one with each deck of the trio. */
  winsNeeded: number;
  /** R334: the most games the series plays, drawn games included. */
  maxGames: number;
  /** Epoch ms the pick clock runs out (R333), or null outside the pick phase. */
  pickDeadline: number | null;
  /** The server's clock when it answered, so a countdown does not depend on this device's. */
  now: number;
  currentMatchId: string | null;
  /** R604: the queue paired this series, so it is rated (a room's is not); known from the start, not only on `result`. */
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
  /** Null until the series is over. `ranked`: it moved your rank; the rating is never sent. */
  result: {
    outcome: "win" | "loss" | "draw" | "abandoned";
    endReason: SeriesEnd;
    ranked: boolean;
  } | null;
};

export function getSeries(token: string, seriesId: string): Promise<SeriesView> {
  return apiRequest<SeriesView>(`/api/series/${encodeURIComponent(seriesId)}`, { token });
}

/** The series a match belongs to, for the board's series banner. */
export function getSeriesForMatch(token: string, matchId: string): Promise<{ series: SeriesView | null }> {
  return apiRequest<{ series: SeriesView | null }>(`/api/matches/${encodeURIComponent(matchId)}/series`, { token });
}

// The ranked ladder (SPEC §9.12, R612): `crates/server/src/api/ranked.rs`'s shapes, none with the hidden rating.

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

/** `GET /api/ranked`. The profile id is for keys and requests; a player is only ever shown by `username` (R1436). */
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
 * `GET /api/matches/:id/ranks` (R604, R2101000). R2388000: `rankBefore`/`rankAfter` once a ranked
 * match's rated record is written; absent until then, unranked, and on a series' earlier game.
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

// Online replays (SPEC §9.3, R768, #510): `crates/server/src/api/replays.rs`'s shapes. A step is the view
// the caller's seat held; no seed, log, state or deck crosses.

/** One finished match in `GET /api/replays`; `unavailable`: why this build cannot show it. */
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
 * `GET /api/replays/:matchId`. R1834003: with both seats' frozen loadouts (all defaults before
 * R1834002) and this page's cards' picks, read as untrusted (`cards/loadouts.ts`).
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

// Rematch offers after a finished non-series match (SPEC §9.5, R672): never decks, hands or ratings.

/** A rematch's stakes: 1 is a normal game, 2 is double-or-nothing (ranked only). */
export type RematchStakes = 1 | 2;

/** `POST /api/matches/:id/rematch`: the created game, or null while the seats disagree. */
export type RematchOfferResponse = { matchId: string | null };

/** `GET /api/matches/:id/rematch`. `mode`: what a rematch plays (R1372); an older server omits it. */
export type RematchStatusResponse = {
  youOffered: RematchStakes | null;
  opponentOffer: RematchStakes | null;
  opponentHere: boolean;
  matchId: string | null;
  mode?: QueueMode;
};

/** `lean`: as `ModeChoice`'s `leanSet`, for an All Random rematch; a Best-of-1 one replays its decks. */
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
 * `POST /api/series/:id/pick`, a 0-based trio slot. A pick is sealed (R331): another slot is a 409, the
 * same one again succeeds, and a pick for a game already begun is never applied to the next.
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

// Public card and player statistics (SPEC §9.11, R654)

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
  /** R2427000: the decks that held the card and never drew it, and the drawn delta; an older server omits them. */
  notDrawnGames?: number;
  notDrawnWinRate?: number | null;
  drawnDelta?: number | null;
  playRate: number;
  hasEnoughGames: boolean;
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
 * R2089000: a Created card's figures, over the seats that made a copy of it during a game. Used: the
 * seats that used one (split by Radiant at its first use); Not used, the delta's baseline: those that used none.
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
  /** The Created cards under `cards`' filters: enough games first, then most games, then name. Absent from an older cache. */
  created?: PublicCreatedStat[];
  /**
   * R1973002: every patch the page can show, newest first, each window's versions oldest first (its
   * figures are asked for at the last). An older server omits it.
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

/** The path of one filter's table, query included; its kept answer is a public one. */
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
 * R1994000: how a patch's games went. Shares are 0..1; `firstShare` is the first seat's share of the
 * decided games, null when every game was drawn (R2567000), and `firstLow` and `firstHigh` bound it at
 * 95% (Wilson). The fatigue, trap, position and Radiant figures count only the `counted` games.
 */
export type PublicGameFigures = {
  games: number;
  draws: number;
  firstWins: number;
  firstShare: number | null;
  firstLow: number;
  firstHigh: number;
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

