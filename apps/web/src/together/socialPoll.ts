// What a signed-in tab reads of the social layer on every screen, read once for all of them (#1831):
// the open challenges and the unread counts, both every `SOCIAL_POLL_MS`, in one `GET /api/social`
// (R3210000). The shell's toast and Play
// together poll (`useSocialPoll`); the Friends button's badge only reads what they published
// (`useSocialReads`). Nothing here is a rule (CLAUDE.md rule 7): a list is the server's answer, and a
// session it refuses outright is not asked again until the tab holds another one. R3205000: the poll
// sends nothing while the tab is hidden and reads once when it shows again, unless a challenge of the
// player's own is open or just sent: that room expires, and the follow that takes the challenger to
// the board rides this poll (`ChallengeToast.tsx`, `follow.ts`).
// R3214000: the poll also holds the tab's social socket (`socialSocket.ts`). While it is live (its first
// `social` frame came) the server pushes the same read whenever it changes, and the poll sends nothing;
// it reads at once when the socket drops and goes on at its pace until another is live. The socket's
// `chat` and `friends` pushes go to their listeners (`onChatPush`, `onFriendsPush`): an open chat and an
// open friends list read again.

import { useEffect, useSyncExternalStore } from "react";

import { pollWhileVisible } from "../game/net.ts";
import { readSession } from "../net/session.ts";
import { getSocial, type Challenge, type SocialRead } from "../net/social.ts";
import { SOCIAL_POLL_MS } from "./config.ts";
import { followingChallenge } from "./follow.ts";
import { isRefusedSession, unreadCountsOf } from "./session.ts";
import { openSocialSocket, socialSocketFactory, type SocialPush, type SocialSocket } from "./socialSocket.ts";
import { isQueueMode } from "../net/api.ts";

export type SocialReads = {
  readonly incoming: readonly Challenge[];
  /** Challenges this account sent that are still open. */
  readonly outgoing: readonly Challenge[];
  /** How many reads of the challenges have been answered: the shell's follow acts once per read. */
  readonly challengeReads: number;
  readonly unread: Readonly<Record<string, number>>;
  /** Incoming friend requests, as the friends list was last read. */
  readonly requests: number;
  /** The server refused the session outright: no signed-in social layer to show. */
  readonly refused: boolean;
  /** R3214000: the tab's social socket is live, so the server pushes these reads and no poll is sent. */
  readonly live: boolean;
};

const NO_CHALLENGES: readonly Challenge[] = Object.freeze([]);
const NO_UNREAD: Readonly<Record<string, number>> = Object.freeze({});

const EMPTY: SocialReads = Object.freeze({
  incoming: NO_CHALLENGES,
  outgoing: NO_CHALLENGES,
  challengeReads: 0,
  unread: NO_UNREAD,
  requests: 0,
  refused: false,
  live: false,
});

let reads: SocialReads = EMPTY;
const listeners = new Set<() => void>();
/** Screens polling now: the interval runs while there is one. */
let pollers = 0;
let stopPolling: (() => void) | null = null;
/** Counts the times polling started over, so an answer to a read made before is dropped. */
let generation = 0;
let refusedToken: string | null = null;
/** R3214000: the tab's social socket, while a screen polls and the socket is on. */
let socket: SocialSocket | null = null;
const chatListeners = new Set<(friendId: string) => void>();
const friendsListeners = new Set<() => void>();

function publish(next: SocialReads): void {
  if (next === reads) return;
  reads = next;
  for (const listener of listeners) listener();
}

/** A challenge as S7 shapes one, as far as the client reads it. */
function isChallenge(value: unknown): value is Challenge {
  if (typeof value !== "object" || value === null) return false;
  const { challengeId, roomCode, mode, from } = value as { challengeId?: unknown; roomCode?: unknown; mode?: unknown; from?: unknown };
  return (
    typeof challengeId === "string" &&
    typeof roomCode === "string" &&
    isQueueMode(mode) &&
    typeof from === "object" &&
    from !== null &&
    typeof (from as { username?: unknown }).username === "string"
  );
}

/** A read of the challenges and the unread counts, the poll's or the socket's: published as checked lists. */
function applySocial(answer: SocialRead): void {
  refusedToken = null;
  // Read as lists of challenges and counts whatever answered: the toast sits over every screen.
  publish({
    ...reads,
    incoming: Array.isArray(answer.incoming) ? answer.incoming.filter(isChallenge) : NO_CHALLENGES,
    outgoing: Array.isArray(answer.outgoing) ? answer.outgoing.filter(isChallenge) : NO_CHALLENGES,
    challengeReads: reads.challengeReads + 1,
    unread: unreadCountsOf(answer),
    refused: false,
  });
}

function tick(): void {
  const token = readSession()?.accessToken ?? null;
  if (token === null) {
    refusedToken = null;
    publish(EMPTY);
    return;
  }
  if (token === refusedToken) {
    if (!reads.refused) publish({ ...reads, incoming: NO_CHALLENGES, outgoing: NO_CHALLENGES, unread: NO_UNREAD, refused: true });
    return;
  }
  // R3214000: a socket that is down (never opened, refused for an older token) opens again with this
  // one, and one opened for a token the session no longer holds is replaced.
  socket?.retry();
  // The live socket pushes what this would read.
  if (reads.live) return;
  const started = generation;
  getSocial().then(
    (answer) => {
      if (started !== generation || reads.live) return;
      applySocial(answer);
    },
    (cause: unknown) => {
      if (started !== generation || !isRefusedSession(cause)) return;
      refusedToken = token;
      publish({ ...reads, incoming: NO_CHALLENGES, outgoing: NO_CHALLENGES, unread: NO_UNREAD, refused: true });
    },
  );
}

/** The poll's own exception to R3205000: a challenge of the player's own is open or just sent. */
function keepPolling(): boolean {
  return reads.outgoing.length > 0 || followingChallenge();
}

/**
 * R3214000: the socket turned live or stopped being; down, the poll reads at once and goes on. R3205000:
 * a hidden tab's drop reads nothing (unless the poll would run hidden), since the poll reads when it shows.
 */
function onLive(live: boolean): void {
  publish({ ...reads, live });
  if (!live && (document.visibilityState !== "hidden" || keepPolling())) tick();
}

function onPush(push: SocialPush): void {
  if (readSession() === null) return;
  switch (push.kind) {
    case "social":
      applySocial(push.read);
      return;
    case "chat":
      for (const listener of [...chatListeners]) listener(push.friendId);
      return;
    case "friends":
      for (const listener of [...friendsListeners]) listener();
      return;
  }
}

function start(): void {
  const factory = socialSocketFactory();
  if (factory !== null) {
    socket = openSocialSocket({ token: () => readSession()?.accessToken ?? null, socketFactory: factory, onLive, onPush });
  }
  tick();
  stopPolling = pollWhileVisible(tick, SOCIAL_POLL_MS, keepPolling);
}

function stop(): void {
  socket?.close();
  socket = null;
  stopPolling?.();
  stopPolling = null;
  generation += 1;
  refusedToken = null;
  publish(EMPTY);
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function snapshot(): SocialReads {
  return reads;
}

/** What the tab last read, with no reading of its own. */
export function useSocialReads(): SocialReads {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}

/** Any number of screens may call it; there is one poll, started by the first and stopped by the last. */
export function useSocialPoll(): SocialReads {
  useEffect(() => {
    pollers += 1;
    if (pollers === 1) start();
    return () => {
      pollers -= 1;
      if (pollers === 0) stop();
    };
  }, []);
  return useSocialReads();
}

/** One of the player's own changes (a challenge answered, a chat read): the reads again at once, while polling. */
export function reloadSocial(): void {
  if (pollers > 0) tick();
}

/** R3214000: calls `listener` with the friend a pushed chat message came from or went to; returns its unsubscribe. */
export function onChatPush(listener: (friendId: string) => void): () => void {
  chatListeners.add(listener);
  return () => {
    chatListeners.delete(listener);
  };
}

/** R3214000: calls `listener` when the server says the friends list changed; returns its unsubscribe. */
export function onFriendsPush(listener: () => void): () => void {
  friendsListeners.add(listener);
  return () => {
    friendsListeners.delete(listener);
  };
}

/** The friends list as a screen last read it: how many requests wait for the player. */
export function noteFriends(answer: { incoming?: unknown }): void {
  const requests = Array.isArray(answer.incoming) ? answer.incoming.length : 0;
  if (requests !== reads.requests) publish({ ...reads, requests });
}

export function friendsBadgeCount(shown: SocialReads): number {
  let count = shown.requests + shown.incoming.length;
  for (const unread of Object.values(shown.unread)) count += unread;
  return count;
}
