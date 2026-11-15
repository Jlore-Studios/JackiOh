// What the inbox lists and counts, and in what order (R3729002). Pure: the lists the client already reads and
// a time in, items out. Nothing here is a rule (CLAUDE.md rule 7): a challenge's lapse, a daily game's move and
// a request's standing are the server's, and the client only drops what its own clock says has lapsed.
//
// The kinds come in the order of how soon each lapses or needs an answer: a live challenge (minutes), a lobby
// invite, a daily game on the player's move, a daily challenge, a friend request, a shared replay. Within a
// kind the order is the one its own screen uses, or the soonest to lapse, or the newest.

import { dailySections } from "../daily/order.ts";
import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { t } from "../i18n/i18n.ts";
import type { DailyGameSummary, DailyInvite, DailyList } from "../net/daily.ts";
import type { Challenge, FriendRequest, SharedReplay } from "../net/social.ts";
import { INBOX_CLOCK_DIGITS, INBOX_INVITE_KEEP_MS, SECONDS_PER_MINUTE } from "./config.ts";
import type { InboxInvite } from "./invites.ts";

export type InboxItem =
  | { kind: "challenge"; key: string; challenge: Challenge; expiresAt: number }
  | { kind: "invite"; key: string; invite: InboxInvite }
  | { kind: "dailyMove"; key: string; game: DailyGameSummary }
  | { kind: "dailyChallenge"; key: string; invite: DailyInvite }
  | { kind: "request"; key: string; request: FriendRequest }
  | { kind: "replay"; key: string; replay: SharedReplay };

export type InboxItemKind = InboxItem["kind"];

/** The kinds in the order the inbox lists them. */
export const INBOX_KINDS: readonly InboxItemKind[] = ["challenge", "invite", "dailyMove", "dailyChallenge", "request", "replay"];

export type InboxInput = {
  /** The live challenges to the player, as the social read holds them. */
  challenges: readonly Challenge[];
  invites: readonly InboxInvite[];
  /** The daily list, or null until it has been read. */
  daily: DailyList | null;
  /** The friend requests to the player. */
  requests: readonly FriendRequest[];
  shared: readonly SharedReplay[];
};

/** -1, 0 or 1: `a - b` would be NaN for two times that are both infinite. */
function compare(a: number, b: number): number {
  return a === b ? 0 : a < b ? -1 : 1;
}

/** An ISO time as epoch ms; one that does not parse sorts after every real time and never lapses. */
function millisOf(iso: string): number {
  const at = Date.parse(iso);
  return Number.isFinite(at) ? at : Number.POSITIVE_INFINITY;
}

/** A time that sorts a list oldest first or newest first: an unreadable one counts as the oldest. */
function sortTime(iso: string): number {
  const at = Date.parse(iso);
  return Number.isFinite(at) ? at : Number.NEGATIVE_INFINITY;
}

/** The identity of a shared replay: a match shared by two friends is two entries. */
export function replayKey(replay: SharedReplay): string {
  return `replay:${replay.matchId}:${replay.sharer.profileId}`;
}

/** Every thing waiting on the player at `now`, in the inbox's order. */
export function inboxItems(input: InboxInput, now: number): InboxItem[] {
  const challenges: InboxItem[] = input.challenges
    .map((challenge) => ({ challenge, expiresAt: millisOf(challenge.expiresAt) }))
    .filter(({ expiresAt }) => expiresAt > now)
    .sort((a, b) => compare(a.expiresAt, b.expiresAt))
    .map(({ challenge, expiresAt }) => ({ kind: "challenge", key: `challenge:${challenge.challengeId}`, challenge, expiresAt }));

  const invites: InboxItem[] = input.invites
    .filter((invite) => now - invite.receivedAt < INBOX_INVITE_KEEP_MS)
    .sort((a, b) => b.receivedAt - a.receivedAt)
    .map((invite) => ({ kind: "invite", key: `invite:${invite.code}`, invite }));

  const sections = input.daily === null ? null : dailySections(input.daily, now);
  const dailyMoves: InboxItem[] = (sections?.yourMove ?? []).map((game) => ({ kind: "dailyMove", key: `dailyMove:${game.matchId}`, game }));
  const dailyChallenges: InboxItem[] = (sections?.incoming ?? []).map((invite) => ({
    kind: "dailyChallenge",
    key: `dailyChallenge:${invite.id}`,
    invite,
  }));

  const requests: InboxItem[] = [...input.requests]
    .sort((a, b) => compare(sortTime(a.createdAt), sortTime(b.createdAt)))
    .map((request) => ({ kind: "request", key: `request:${request.requestId}`, request }));

  const replays: InboxItem[] = [...input.shared]
    .sort((a, b) => compare(sortTime(b.createdAt), sortTime(a.createdAt)))
    .map((replay) => ({ kind: "replay", key: replayKey(replay), replay }));

  return [...challenges, ...invites, ...dailyMoves, ...dailyChallenges, ...requests, ...replays];
}

/**
 * How many things need the player: every item but a shared replay already seen (`seen`, the keys of the replays
 * the panel has listed, kept by `store.ts`). A replay waits on nobody, so it counts until it has been looked at;
 * the rest count until they are answered or lapse, as the Friends badge counts what the last read held.
 */
export function inboxBadge(items: readonly InboxItem[], seen: ReadonlySet<string>): number {
  return items.filter((item) => item.kind !== "replay" || !seen.has(item.key)).length;
}

/** A challenge's time left as a clock: "1:59 left". It reads zero only once the challenge has lapsed. */
export function challengeLeftText(ms: number): string {
  const seconds = Math.max(0, Math.ceil(ms / MS_PER_SECOND));
  const minutes = Math.floor(seconds / SECONDS_PER_MINUTE);
  const rest = String(seconds % SECONDS_PER_MINUTE).padStart(INBOX_CLOCK_DIGITS, "0");
  return t("{time} left", { time: `${String(minutes)}:${rest}` });
}
