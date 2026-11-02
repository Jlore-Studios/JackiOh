// What a signed-in tab reads of the social layer on every screen, read once for all of them (#1831):
// the open challenges and the unread counts, each every `SOCIAL_POLL_MS`. The shell's toast and Play
// together poll (`useSocialPoll`); the Friends button's badge only reads what they published
// (`useSocialReads`). Nothing here is a rule (CLAUDE.md rule 7): a list is the server's answer, and a
// session it refuses outright is not asked again until the tab holds another one.

import { useEffect, useSyncExternalStore } from "react";

import { readSession } from "../net/session.ts";
import { getChallenges, getUnread, type Challenge } from "../net/social.ts";
import { SOCIAL_POLL_MS } from "./config.ts";
import { isRefusedSession, unreadCountsOf } from "./session.ts";
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
});

let reads: SocialReads = EMPTY;
const listeners = new Set<() => void>();
/** Screens polling now: the interval runs while there is one. */
let pollers = 0;
let handle: number | null = null;
/** Counts the times polling started over, so an answer to a read made before is dropped. */
let generation = 0;
let refusedToken: string | null = null;

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
  const started = generation;
  getChallenges().then(
    (answer) => {
      if (started !== generation) return;
      refusedToken = null;
      // Read as lists of challenges whatever answered: the toast sits over every screen, and a server
      // from before #936 has no such route.
      publish({
        ...reads,
        incoming: Array.isArray(answer.incoming) ? answer.incoming.filter(isChallenge) : NO_CHALLENGES,
        outgoing: Array.isArray(answer.outgoing) ? answer.outgoing.filter(isChallenge) : NO_CHALLENGES,
        challengeReads: reads.challengeReads + 1,
        refused: false,
      });
    },
    (cause: unknown) => {
      if (started !== generation || !isRefusedSession(cause)) return;
      refusedToken = token;
      publish({ ...reads, incoming: NO_CHALLENGES, outgoing: NO_CHALLENGES, unread: NO_UNREAD, refused: true });
    },
  );
  getUnread().then(
    (answer) => {
      if (started === generation && token !== refusedToken) publish({ ...reads, unread: unreadCountsOf(answer) });
    },
    () => undefined,
  );
}

function start(): void {
  tick();
  handle = window.setInterval(tick, SOCIAL_POLL_MS);
}

function stop(): void {
  if (handle !== null) window.clearInterval(handle);
  handle = null;
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
