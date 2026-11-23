// The inbox's Recent section (R4039001): the notification feed the server keeps for the player (R4039000), read
// a page at a time. Opening the panel reads the newest page and marks every row up to the newest one shown as
// read (the rows keep the dots they came with until the next read), which the bell's dot follows through the
// social read. A read that fails keeps the rows before it. The rows are another account's once the session
// changes hands, so they go before that account's are read, as `store.ts`'s lists do.
//
// `feedLine` and `feedPath` say a row in words and where its Open goes; nothing here decides what happened
// (CLAUDE.md rule 7): a row is the server's, and the screen it opens reads its own state.

import { useSyncExternalStore } from "react";

import { tokenClaim } from "../auth/claims.ts";
import { t } from "../i18n/i18n.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { paths } from "../net/navigate.ts";
import { getNotifications, markNotificationsRead, type FeedNotification } from "../net/notifications.ts";
import { readSession } from "../net/session.ts";
import { friendsSectionPath, sharedWithMePath } from "../together/anchors.ts";
import { reloadSocial } from "../together/socialPoll.ts";

export type FeedReads = {
  /** The rows read, newest first. */
  readonly rows: readonly FeedNotification[];
  /** The id the page before the last one read starts under; null when no older row is left. */
  readonly nextBefore: number | null;
  /** The last read's failure, in the server's words; null after a read that answered. */
  readonly error: string | null;
  /** A page has answered since the rows were last cleared. */
  readonly loaded: boolean;
};

const EMPTY: FeedReads = Object.freeze({ rows: Object.freeze([]), nextBefore: null, error: null, loaded: false });

let reads: FeedReads = EMPTY;
const listeners = new Set<() => void>();
/** The account `reads` belong to. */
let owner: string | null = null;
/** Counts the reads started and the clearings made, so an answer to an older one is dropped. */
let generation = 0;
/** An older page's read is out. */
let loadingOlder = false;

function publish(next: FeedReads): void {
  if (next === reads) return;
  reads = next;
  for (const listener of [...listeners]) listener();
}

/** The session's account, after dropping another account's rows; null with no session. */
function account(): string | null {
  const token = readSession()?.accessToken ?? null;
  const who = token === null ? null : (tokenClaim(token, "sub") ?? token);
  if (who !== owner) {
    generation += 1;
    owner = who;
    publish(EMPTY);
  }
  return who;
}

/**
 * Reads the newest page. The rows older than it that a "Show older" read already holds stay under it. When a
 * row on it, or any row of the account's, is unread, every row up to its newest is marked read, and the social
 * read is asked again.
 */
export async function openFeed(): Promise<void> {
  if (account() === null) return;
  generation += 1;
  const started = generation;
  const page = await attempt(() => getNotifications()).catch((cause: unknown) => {
    if (started === generation) publish({ ...reads, error: messageOf(cause) });
    return null;
  });
  if (page === null || started !== generation) return;
  const last = page.notifications.at(-1)?.id;
  const older = page.nextBefore === null || last === undefined ? [] : reads.rows.filter((row) => row.id < last);
  publish({
    rows: [...page.notifications, ...older],
    nextBefore: older.length > 0 ? reads.nextBefore : page.nextBefore,
    error: null,
    loaded: true,
  });
  const newest = page.notifications[0];
  // R4039001: every row up to the newest is marked, an older unread one below a page the acts it is about
  // already read (R4233000) among them, so the page's own rows are not the only test.
  if (newest === undefined || (page.unread === 0 && page.notifications.every((row) => row.read))) return;
  // A mark that fails leaves them unread at the server, and the next opening marks them again.
  const marked = await attempt(() => markNotificationsRead(newest.id)).then(
    () => true,
    () => false,
  );
  if (marked) reloadSocial();
}

/** Reads the page before the oldest row held and adds it under the rows. */
export async function loadOlderFeed(): Promise<void> {
  const who = account();
  const before = reads.nextBefore;
  if (who === null || before === null || loadingOlder) return;
  loadingOlder = true;
  try {
    const page = await attempt(() => getNotifications(before));
    // Another account's, or the rows it would go under were read again without it.
    if (owner !== who || reads.nextBefore !== before) return;
    const oldest = reads.rows.at(-1)?.id ?? Number.POSITIVE_INFINITY;
    publish({ ...reads, rows: [...reads.rows, ...page.notifications.filter((row) => row.id < oldest)], nextBefore: page.nextBefore, error: null });
  } catch (cause) {
    if (owner === who) publish({ ...reads, error: messageOf(cause) });
  } finally {
    loadingOlder = false;
  }
}

/** Forgets every row read (tests start from nothing). */
export function resetFeedForTests(): void {
  generation += 1;
  owner = null;
  publish(EMPTY);
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function snapshot(): FeedReads {
  return reads;
}

/** The feed as last read, with no reading of its own. */
export function useFeed(): FeedReads {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}

/** Who did it: the actor's username, or "Someone" when the server no longer knows them. */
export function feedName(row: FeedNotification): string {
  return row.actor?.username ?? t("Someone");
}

/** The row's sentence with `{name}` left in it, for `together/words.tsx`'s `Named` to draw the username (R1436). */
export function feedSentence(row: FeedNotification): string {
  const { payload } = row;
  switch (row.kind) {
    case "dailyMove":
      return t("Your move against {name}");
    case "dailyEnded": {
      const onTime = payload.reason === "disconnect";
      if (payload.outcome === "win") return onTime ? t("Your daily game with {name} ended: you won on time") : t("Your daily game with {name} ended: you won");
      if (payload.outcome === "loss") return onTime ? t("Your daily game with {name} ended: you lost on time") : t("Your daily game with {name} ended: you lost");
      if (payload.outcome === "draw") return t("Your daily game with {name} ended: it was a draw");
      return t("Your daily game with {name} ended");
    }
    case "dailyChallenge":
      return t("{name} challenged you to a daily game");
    case "dailyAnswered":
      return payload.answer === "accepted" ? t("{name} accepted your daily challenge") : t("{name} declined your daily challenge");
    case "friendRequest":
      return t("{name} sent you a friend request");
    case "friendAccepted":
      return t("{name} accepted your friend request");
    case "lobbyInvite":
      return t("{name} invited you to their lobby");
    case "replayShared":
      return t("{name} shared a replay with you");
  }
}

/** The row in words, the name filled in: a toast's and a desktop notification's text. */
export function feedLine(row: FeedNotification): string {
  return feedSentence(row).split("{name}").join(feedName(row));
}

/** Where the row's Open goes; null when it names nowhere to go. */
export function feedPath(row: FeedNotification): string | null {
  const { subject, payload } = row;
  switch (row.kind) {
    case "dailyMove":
    case "dailyEnded":
      return subject === null ? paths.daily : paths.dailyGame(subject);
    case "dailyAnswered":
      return payload.answer === "accepted" && typeof payload.matchId === "string" ? paths.dailyGame(payload.matchId) : paths.daily;
    case "dailyChallenge":
      return paths.daily;
    case "friendRequest":
      return friendsSectionPath("requests");
    case "friendAccepted":
      return friendsSectionPath("friends");
    case "lobbyInvite":
      return subject === null ? null : paths.lobby(subject);
    case "replayShared":
      return subject === null ? sharedWithMePath() : sharedWithMePath(subject);
  }
}
