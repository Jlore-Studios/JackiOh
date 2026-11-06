// The menus' kept answers (#2125, R2125000): the last answer of each menu `GET`, in tab memory only,
// drawn at once while the screen reads again behind it; a failed read behind a kept answer keeps it, so
// an error or a skeleton shows only on a first visit (R2115000). An entry is keyed by its path and its
// account (the token's `sub`, so a renewal keeps it). Never kept: a match's, a series', a duos room's,
// a watch's or a replay's reads. R2630000: the gate keeps its `/api/auth/me` answer here too.
// R3206000: each entry carries the time its read finished. One younger than `READ_CACHE_FRESH_MS` is
// fresh: `freshRead` hands it out and the read behind it is not sent. An older one is only shown, as
// R2125000 has it. A fold of a write's answer keeps the time it found; `expireRead` ends freshness.
// The write count (`writeEpoch`) tells a read that a write landed while it was out.
// CLAUDE.md rule 7: only this account's own answers are kept; nothing here predicts or reveals game state.

import { tokenClaim } from "../auth/claims.ts";

/** The most answers kept; the one kept longest ago goes first (each statistics filter keeps its own). */
export const READ_CACHE_MAX_ENTRIES = 64;

/**
 * R3206000: how long a finished read stays fresh (ms): inside it a menu or the gate uses the kept answer
 * and sends no read behind it. The client's own number (CLAUDE.md rule 9), midway in the 30 to 60 s #3206
 * allowed: long enough that moving between screens stops re-reading, short enough that what another
 * device or the server changed shows within a minute. The server refuses what a stale view asks for.
 */
export const READ_CACHE_FRESH_MS = 45_000;

const PUBLIC = "";

type Entry = { value: unknown; at: number };

let owner: string | null = null;
const entries = new Map<string, Entry>();

/**
 * R3206000: counts the writes: a change to what is kept (a write's answer, a fold, a drop, a sign-out) and
 * a spend that settled (`noteWrite`). A read that finished after it moved is not kept, and a read asked
 * after it never joins one sent before it. A heartbeat or a chat message does not move it: a read that
 * straddles one would never be kept, and the landing sends one as it mounts.
 */
let epoch = 0;

/** What `writeEpoch` returned when a read was sent says whether a write has landed since. */
export function writeEpoch(): number {
  return epoch;
}

/** A spend settled, answer or refusal (`apiRequest`): reads out before it may show what it changed. */
export function noteWrite(): void {
  epoch += 1;
}

function accountOf(token: string | null): string {
  return token === null ? PUBLIC : (tokenClaim(token, "sub") ?? token);
}

function entryOf(token: string | null, path: string): Entry | undefined {
  const account = accountOf(token);
  if (account !== PUBLIC && account !== owner) return undefined;
  return entries.get(`${account}\n${path}`);
}

/** The answer `token`'s account (null: a public read) last had for `path`, or undefined; its age does not matter. */
export function cachedRead<T>(token: string | null, path: string): T | undefined {
  return entryOf(token, path)?.value as T | undefined;
}

/** R3206000: as `cachedRead`, but only an answer younger than `READ_CACHE_FRESH_MS`. */
export function freshRead<T>(token: string | null, path: string): T | undefined {
  const entry = entryOf(token, path);
  if (entry === undefined) return undefined;
  const age = Date.now() - entry.at;
  return age >= 0 && age < READ_CACHE_FRESH_MS ? (entry.value as T) : undefined;
}

function store(token: string | null, path: string, value: unknown, at: number): void {
  const account = accountOf(token);
  if (account !== PUBLIC && account !== owner) {
    entries.clear();
    owner = account;
  }
  const key = `${account}\n${path}`;
  entries.delete(key);
  entries.set(key, { value, at });
  if (entries.size > READ_CACHE_MAX_ENTRIES) {
    const oldest = entries.keys().next().value;
    if (oldest !== undefined) entries.delete(oldest);
  }
}

/**
 * Keeps `value`, a write's answer, as that account's answer for `path`, fresh from now; another
 * account's answers go first. R3206000: it counts as a write.
 */
export function rememberRead(token: string | null, path: string, value: unknown): void {
  epoch += 1;
  store(token, path, value, Date.now());
}

/**
 * R3206000: keeps `value`, what a finished read answered, fresh from now. Not a write: reads out beside
 * it are not told one landed.
 */
export function keepRead(token: string | null, path: string, value: unknown): void {
  store(token, path, value, Date.now());
}

/** Folds a write's answer into the kept answer for `path`, keeping its age; nothing when none is kept. */
export function updateCachedRead<T>(token: string | null, path: string, update: (before: T) => T): void {
  const before = entryOf(token, path);
  if (before === undefined) return;
  epoch += 1;
  // The fold is not a read: it adds to the answer and never makes it fresher than the read that made it.
  const key = `${accountOf(token)}\n${path}`;
  entries.delete(key);
  entries.set(key, { value: update(before.value as T), at: before.at });
}

/** Drops `token`'s account's kept answer for `path` (R2630000: the gate's, when the account changes). */
export function forgetRead(token: string | null, path: string): void {
  epoch += 1;
  entries.delete(`${accountOf(token)}\n${path}`);
}

/**
 * R3206000: ends the freshness of the kept answer for `path` and keeps it: shown as before, never
 * used in place of a read, so the next read is sent and a failed one still shows what was there.
 */
export function expireRead(token: string | null, path: string): void {
  const entry = entryOf(token, path);
  if (entry !== undefined) entry.at = Number.NEGATIVE_INFINITY;
}

/** Sign-out (`net/session.ts`) and every test (`test/setup.ts`). */
export function clearReadCache(): void {
  epoch += 1;
  entries.clear();
  owner = null;
}

/** Test support: how many answers are kept. */
export function __readCacheSizeForTests(): number {
  return entries.size;
}
