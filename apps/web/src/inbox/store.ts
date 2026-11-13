// What the inbox reads beside the social read (R3729002): the daily list, the friends list (its incoming
// requests) and the replays friends shared, read together by one `refreshInbox`, and the shared replays the
// player has seen listed. The live challenges are `together/socialPoll.ts`'s, and the lobby invites are
// `invites.ts`'s: this adds no server state and no route, only reads the client already makes.
//
// A read that fails keeps the answer before it (an error never empties the inbox). A refresh within
// `INBOX_MIN_GAP_MS` of the last is skipped unless forced; the polls and a social read that lands ask
// unforced, and the player's own changes force. The friends read goes through `noteFriends`, so the Friends
// badge counts the requests this read held, as a screen that read the list would (CLAUDE.md rule 7: the
// lists are the server's; nothing here decides what is waiting).

import { useEffect, useSyncExternalStore } from "react";

import { tokenClaim } from "../auth/claims.ts";
import { getDaily, type DailyList } from "../net/daily.ts";
import { attempt } from "../net/errors.ts";
import { readSession } from "../net/session.ts";
import { getFriends, getSharedReplays, type FriendsList, type SharedReplay } from "../net/social.ts";
import { getStored, setStored } from "../net/storage.ts";
import { pollWhileVisible } from "../game/net.ts";
import { noteFriends, onFriendsPush } from "../together/socialPoll.ts";
import { INBOX_MIN_GAP_MS, INBOX_POLL_MS, INBOX_SEEN_KEY, INBOX_SEEN_MAX } from "./config.ts";
import { forgetInvites } from "./invites.ts";

export type InboxReads = {
  /** The daily list, or null until a read has answered. */
  readonly daily: DailyList | null;
  /** The friends list, or null until a read has answered. */
  readonly friends: FriendsList | null;
  /** The shared replays, or null until a read has answered. */
  readonly shared: readonly SharedReplay[] | null;
  /** The keys of the shared replays the panel has listed (`markSeen`). */
  readonly seen: ReadonlySet<string>;
};

/** The shared replays this browser has seen, oldest mark first; anything but a list of text reads as none. */
function storedSeen(): string[] {
  try {
    const raw = getStored(INBOX_SEEN_KEY);
    const parsed: unknown = raw === null ? [] : JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((key): key is string => typeof key === "string") : [];
  } catch {
    return [];
  }
}

function freshReads(): InboxReads {
  return { daily: null, friends: null, shared: null, seen: new Set(storedSeen()) };
}

let reads: InboxReads = freshReads();
const listeners = new Set<() => void>();
/** The account `reads` belong to; another account's are dropped before this one's are read. */
let owner: string | null = null;
/** When the last refresh started; unforced refreshes inside `INBOX_MIN_GAP_MS` of it are skipped. */
let lastAt = Number.NEGATIVE_INFINITY;
/** Counts the refreshes started and the clearings made, so an answer to an older one is dropped. */
let generation = 0;
/** Screens polling now: the interval and the push listener run while there is one. */
let pollers = 0;
let stopPolling: (() => void) | null = null;
let stopPushing: (() => void) | null = null;

function publish(next: InboxReads): void {
  if (next === reads) return;
  reads = next;
  for (const listener of listeners) listener();
}

/** Everything read goes (a sign-out, a refused session): it was another account's. */
export function clearInbox(): void {
  generation += 1;
  owner = null;
  lastAt = Number.NEGATIVE_INFINITY;
  forgetInvites();
  if (reads.daily !== null || reads.friends !== null || reads.shared !== null) publish({ ...reads, daily: null, friends: null, shared: null });
}

/**
 * Reads the daily list, the friends list and the shared replays, each on its own: one that fails keeps what
 * it held. Resolves once all three have answered or failed. With no session it clears instead.
 */
export async function refreshInbox(force = false): Promise<void> {
  const token = readSession()?.accessToken ?? null;
  if (token === null) {
    clearInbox();
    return;
  }
  const account = tokenClaim(token, "sub") ?? token;
  if (owner !== null && owner !== account) clearInbox();
  owner = account;
  const now = Date.now();
  if (!force && now - lastAt < INBOX_MIN_GAP_MS) return;
  lastAt = now;
  generation += 1;
  const started = generation;
  const [daily, friends, shared] = await Promise.allSettled([attempt(getDaily), attempt(getFriends), attempt(getSharedReplays)]);
  if (started !== generation) return;
  if (friends.status === "fulfilled") noteFriends(friends.value);
  publish({
    ...reads,
    daily: daily.status === "fulfilled" ? daily.value : reads.daily,
    friends: friends.status === "fulfilled" ? friends.value : reads.friends,
    shared: shared.status === "fulfilled" && Array.isArray(shared.value.shared) ? shared.value.shared : reads.shared,
  });
}

/** The shared replays with these keys are seen: the badge stops counting them. The newest `INBOX_SEEN_MAX` marks are kept. */
export function markSeen(keys: readonly string[]): void {
  const fresh = keys.filter((key) => !reads.seen.has(key));
  if (fresh.length === 0) return;
  const kept = [...storedSeen().filter((key) => !fresh.includes(key)), ...fresh].slice(-INBOX_SEEN_MAX);
  // Remembering what was seen is a convenience: a browser that refuses the write counts them again next time.
  setStored(INBOX_SEEN_KEY, JSON.stringify(kept));
  publish({ ...reads, seen: new Set(kept) });
}

/** Forgets what was seen and read (tests start from nothing; so does a sign-out's next account). */
export function resetInboxForTests(): void {
  generation += 1;
  owner = null;
  lastAt = Number.NEGATIVE_INFINITY;
  forgetInvites();
  publish(freshReads());
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function snapshot(): InboxReads {
  return reads;
}

/** What the inbox last read, with no reading of its own. */
export function useInboxReads(): InboxReads {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}

function startPolling(): void {
  stopPolling = pollWhileVisible(() => {
    void refreshInbox();
  }, INBOX_POLL_MS);
  // R3214000: the server says the friends list changed (a request came or went).
  stopPushing = onFriendsPush(() => {
    void refreshInbox(true);
  });
}

function stopPollingNow(): void {
  stopPolling?.();
  stopPolling = null;
  stopPushing?.();
  stopPushing = null;
}

/**
 * Keeps the inbox's lists read while `active` (the bell shown): once on start, every `INBOX_POLL_MS` while the
 * tab is visible, when the friends list changes, and whenever a social read lands (`socialReads`, its count),
 * which is how a daily game passed to the player shows. Any number of bells may call it: there is one interval,
 * started by the first and stopped by the last. Turning inactive clears the inbox.
 */
export function useInboxPoll(active: boolean, socialReads: number): void {
  useEffect(() => {
    if (!active) {
      clearInbox();
      return undefined;
    }
    pollers += 1;
    if (pollers === 1) startPolling();
    return () => {
      pollers -= 1;
      if (pollers === 0) stopPollingNow();
    };
  }, [active]);

  useEffect(() => {
    if (active) void refreshInbox();
  }, [active, socialReads]);
}
