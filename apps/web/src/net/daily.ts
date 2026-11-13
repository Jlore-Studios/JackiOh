// Daily games' REST surface (#3727, SPEC §9.13), one wrapper per route the client calls. A daily game is
// played by correspondence: each action is one request, answered with the seat's whole read, so the board
// needs no socket and the browser runs no engine for it.
//
// CLAUDE.md rule 7: who holds the move, how long is left, what is legal and every refusal are the server's;
// the client sends intent and shows its answers. Every route is `AuthLevel::Active`, so no function takes a
// token: each reads the tab's session and renews it once on a 401 (R194), as `net/social.ts` does.

import type { ActionBody, GameOverReason, LeanPick, PlayerId, PlayerView, PortraitId } from "@jackioh/shared";

import type { DailyHold } from "../wire/generated/DailyHold.ts";
import { apiRequest, type RequestOptions } from "./api.ts";
import { withSession } from "./gate.ts";
import type { PlayerRef } from "./social.ts";

/** The modes a daily game is played in (§9.13): Best of 1 with a saved deck, or All random. */
export type DailyMode = "bo1" | "random";

export const DAILY_MODES: readonly DailyMode[] = ["bo1", "random"];

export function isDailyMode(value: unknown): value is DailyMode {
  return value === "bo1" || value === "random";
}

/** One game as the Daily screen lists it, from the caller's side (`me`, `them`). */
export type DailyGameSummary = {
  matchId: string;
  mode: DailyMode;
  perMoveDays: number;
  /** The caller's seat now (R677). */
  seat: PlayerId;
  opponent: PlayerRef;
  portraits: { me: PortraitId; them: PortraitId };
  turn: number;
  life: { me: number; them: number };
  /** The caller holds the move. */
  hasMove: boolean;
  /** The opponent holds it. */
  theirMove: boolean;
  /** Epoch ms the move held runs out: the caller's when `hasMove`, else the opponent's; null once over. */
  deadline: number | null;
  over: boolean;
  result: { outcome: "win" | "loss" | "draw"; reason: GameOverReason } | null;
  endedAt: number | null;
  createdAt: number;
};

/** A daily challenge (to a friend) or a seek (in the daily queue, `to` null). */
export type DailyInvite = {
  id: string;
  kind: "challenge" | "seek";
  from: PlayerRef;
  to: PlayerRef | null;
  mode: DailyMode;
  perMoveDays: number;
  expiresAt: number;
  createdAt: number;
};

/** `GET /api/daily`. `waiting` is how many games wait on the caller. */
export type DailyList = {
  games: DailyGameSummary[];
  challenges: { incoming: DailyInvite[]; outgoing: DailyInvite[] };
  seeks: DailyInvite[];
  waiting: number;
};

/** `GET /api/daily/games/:matchId`, and every action's answer: the caller's seat as it stands. */
export type DailyGame = {
  matchId: string;
  seat: PlayerId;
  view: PlayerView;
  /** The caller's legal actions; none while the other seat holds the move. */
  legal: ActionBody[];
  /** The last action's number: the next action is sent `after` it. */
  seq: number;
  holds: DailyHold[];
  hasMove: boolean;
  perMoveDays: number;
  mode: DailyMode;
  catalogVersion: string;
  /** R3727005: the game plays on the card version it started on, an older one than today's. */
  olderCatalog: boolean;
  opponent: PlayerRef;
  over: boolean;
  /** The game ended because a move ran out of time (R3727000). */
  lostOnTime: boolean;
  /** Another game waiting on the caller, least time left first. */
  nextGame: string | null;
};

/** `GET`/`PUT /api/daily/settings`: email on or off, and push, which needs the server's public key. */
export type DailySettings = {
  emailOn: boolean;
  /** The account has an email address to send to. */
  hasEmail: boolean;
  /** The VAPID public key a push subscription is made with; null when the server sends no push. */
  vapidPublicKey: string | null;
  pushDevices: number;
};

/** What a daily game is made with: the mode and its pick (a saved deck, or a lean for All random), and the time. */
export type DailyChoice = { mode: "bo1"; deckId: string; perMoveDays: number } | { mode: "random"; leanSet?: LeanPick; perMoveDays: number };

function daily<T>(path: string, options: Omit<RequestOptions, "token"> = {}): Promise<T> {
  return withSession((token) => apiRequest<T>(path, { ...options, token }));
}

const part = encodeURIComponent;

/** The Daily screen's list: games, challenges both ways, seeks and the recently finished. */
export function getDaily(): Promise<DailyList> {
  return daily<DailyList>("/api/daily");
}

/** Challenge any friend, online or not. 404 not a friend, 409 `already_challenged` or `daily_full`. */
export function createDailyChallenge(friendId: string, choice: DailyChoice): Promise<{ id: string }> {
  return daily<{ id: string }>("/api/daily/challenges", { method: "POST", body: { friendId, ...choice } });
}

/** Accept with the friend's own pick; the mode is the challenge's. */
export function acceptDailyChallenge(id: string, pick: { mode: DailyMode; deckId?: string; leanSet?: LeanPick }): Promise<{ matchId: string }> {
  return daily<{ matchId: string }>(`/api/daily/challenges/${part(id)}/accept`, { method: "POST", body: pick });
}

export async function declineDailyChallenge(id: string): Promise<void> {
  await daily<null>(`/api/daily/challenges/${part(id)}/decline`, { method: "POST" });
}

/** The challenger takes it back. */
export async function cancelDailyChallenge(id: string): Promise<void> {
  await daily<null>(`/api/daily/challenges/${part(id)}`, { method: "DELETE" });
}

/** The daily queue: a game at once when someone waits with the same mode and time, else a seek that waits. */
export function createDailySeek(choice: DailyChoice): Promise<{ matchId?: string; seekId?: string }> {
  return daily<{ matchId?: string; seekId?: string }>("/api/daily/seeks", { method: "POST", body: choice });
}

export async function cancelDailySeek(id: string): Promise<void> {
  await daily<null>(`/api/daily/seeks/${part(id)}`, { method: "DELETE" });
}

/** A board's read. */
export function getDailyGame(matchId: string): Promise<DailyGame> {
  return daily<DailyGame>(`/api/daily/games/${part(matchId)}`);
}

/**
 * One action, sent after the last one the board read (`after`), under a fresh nonce: the same nonce sent
 * again applies once. 409 `stale` (the board is behind) and `time_up`, 422 `not_your_move` and `illegal`.
 */
export function sendDailyAction(matchId: string, body: { after: number; nonce: string; action: ActionBody }): Promise<DailyGame> {
  return daily<DailyGame>(`/api/daily/games/${part(matchId)}/actions`, { method: "POST", body });
}

export function getDailySettings(): Promise<DailySettings> {
  return daily<DailySettings>("/api/daily/settings");
}

export function putDailySettings(emailOn: boolean): Promise<DailySettings> {
  return daily<DailySettings>("/api/daily/settings", { method: "PUT", body: { emailOn } });
}

/** This device's push subscription, as `PushSubscription.toJSON()` gives its address and keys. */
export type PushSubscriptionBody = { endpoint: string; keys: { p256dh: string; auth: string } };

export async function addPushSubscription(body: PushSubscriptionBody): Promise<void> {
  await daily<null>("/api/daily/push", { method: "POST", body });
}

export async function deletePushSubscription(endpoint: string): Promise<void> {
  await daily<null>("/api/daily/push", { method: "DELETE", body: { endpoint } });
}

/** At sign-out: the device's subscription goes with the session's own token, and outlives the page. */
export async function deletePushSubscriptionAs(token: string, endpoint: string): Promise<void> {
  await apiRequest<null>("/api/daily/push", { method: "DELETE", token, body: { endpoint }, keepalive: true });
}
