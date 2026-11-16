// The notification feed's routes (R4039000): the rows the server kept for the signed-in account, newest first,
// a page at a time, and the mark that they were read. A row is written by the server when something happened
// to the player (a daily move, a challenge, a friend request, a lobby invite, a shared replay); the client
// only lists them (`inbox/feed.ts`) and never decides which exist. CLAUDE.md rule 7.

import { apiRequest, type RequestOptions } from "./api.ts";
import { withSession } from "./gate.ts";
import type { PlayerRef } from "./social.ts";
import { isRecord } from "../wire/json.ts";

/** What happened, as the server names it (its `notifications_kind_check`). */
export type FeedKind =
  | "dailyMove"
  | "dailyEnded"
  | "dailyChallenge"
  | "dailyAnswered"
  | "friendRequest"
  | "friendAccepted"
  | "lobbyInvite"
  | "replayShared";

const FEED_KINDS: readonly FeedKind[] = [
  "dailyMove",
  "dailyEnded",
  "dailyChallenge",
  "dailyAnswered",
  "friendRequest",
  "friendAccepted",
  "lobbyInvite",
  "replayShared",
];

/**
 * One row of the feed. `subject` names what it is about: the daily game (`dailyMove`, `dailyEnded`), the
 * daily challenge (`dailyChallenge`, `dailyAnswered`), the friend request (`friendRequest`, `friendAccepted`),
 * the lobby's code (`lobbyInvite`) or the shared replay's match (`replayShared`). `payload` is the kind's own:
 * `dailyEnded` `{outcome, reason}`, `dailyAnswered` `{answer, matchId?}`, `dailyChallenge` `{mode, perMoveDays}`,
 * `lobbyInvite` `{format}`, else `{}`.
 */
export type FeedNotification = {
  id: number;
  kind: FeedKind;
  /** Who did it; null when the server no longer knows them. */
  actor: PlayerRef | null;
  subject: string | null;
  payload: Record<string, unknown>;
  /** Epoch ms the server wrote it. */
  createdAt: number;
  read: boolean;
};

/** `GET /api/notifications`: a page, newest first; `nextBefore` asks for the page before it, null when none is left. */
export type NotificationsPage = { notifications: FeedNotification[]; nextBefore: number | null; unread: number };

function isFeedKind(value: unknown): value is FeedKind {
  return typeof value === "string" && (FEED_KINDS as readonly string[]).includes(value);
}

function isActor(value: unknown): value is PlayerRef | null {
  return value === null || (isRecord(value) && typeof value.profileId === "string" && typeof value.username === "string");
}

/** A row as the server shapes one, as far as the client reads it. */
export function isFeedNotification(value: unknown): value is FeedNotification {
  if (!isRecord(value)) return false;
  const { id, kind, actor, subject, payload, createdAt, read } = value;
  return (
    typeof id === "number" &&
    Number.isInteger(id) &&
    isFeedKind(kind) &&
    isActor(actor) &&
    (subject === null || typeof subject === "string") &&
    isRecord(payload) &&
    typeof createdAt === "number" &&
    typeof read === "boolean"
  );
}

function feed<T>(path: string, options: Omit<RequestOptions, "token"> = {}): Promise<T> {
  return withSession((token) => apiRequest<T>(path, { ...options, token }));
}

/** The newest page, or the one before `before` (a `nextBefore` as answered); rows the client cannot read are left out. */
export async function getNotifications(before?: number): Promise<NotificationsPage> {
  const query = before === undefined ? "" : `?${new URLSearchParams({ before: String(before) }).toString()}`;
  const answer = await feed<Partial<NotificationsPage>>(`/api/notifications${query}`);
  return {
    notifications: Array.isArray(answer.notifications) ? answer.notifications.filter(isFeedNotification) : [],
    nextBefore: typeof answer.nextBefore === "number" ? answer.nextBefore : null,
    unread: typeof answer.unread === "number" ? answer.unread : 0,
  };
}

/** Marks every unread row up to `upTo` (inclusive) read, or all of them; answers how many stay unread. */
export function markNotificationsRead(upTo?: number): Promise<{ unread: number }> {
  return feed<{ unread: number }>("/api/notifications/read", { method: "POST", body: upTo === undefined ? {} : { upTo } });
}
