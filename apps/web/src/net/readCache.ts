// The menus' kept answers (#2125, R2125000): the last answer of each menu `GET`, in tab memory only.
// A screen with a kept answer draws it at once, reads again behind it and takes the new answer; a failed read
// behind a kept answer keeps it and says nothing, so an error or a skeleton shows only on a first visit (R2115000).
// An entry is keyed by its path (query included) and its account (the token's `sub`, so a renewal keeps it).
// Sign-out and an account switch empty the cache; a mutating answer that carries a new value updates its entry.
// Never kept: a match's, a series', a duos room's, a watch's or a replay's reads, and anything that isn't a `GET`.
// R2630000: the gate keeps its `/api/auth/me` answer here too, for the next screen to open on.
// CLAUDE.md rule 7: only this account's own answers from the server are kept; nothing here predicts, enforces
// or reveals game state.

import { tokenClaim } from "../auth/claims.ts";

/** The most answers kept; the one kept longest ago goes first (each statistics filter keeps its own). */
export const READ_CACHE_MAX_ENTRIES = 64;

const PUBLIC = "";

let owner: string | null = null;
const entries = new Map<string, unknown>();

function accountOf(token: string | null): string {
  return token === null ? PUBLIC : (tokenClaim(token, "sub") ?? token);
}

/** The answer `token`'s account (null: a public read) last had for `path`, or undefined. */
export function cachedRead<T>(token: string | null, path: string): T | undefined {
  const account = accountOf(token);
  if (account !== PUBLIC && account !== owner) return undefined;
  return entries.get(`${account}\n${path}`) as T | undefined;
}

/** Keeps `value` as that account's answer for `path`; another account's answers go first. */
export function rememberRead(token: string | null, path: string, value: unknown): void {
  const account = accountOf(token);
  if (account !== PUBLIC && account !== owner) {
    entries.clear();
    owner = account;
  }
  const key = `${account}\n${path}`;
  entries.delete(key);
  entries.set(key, value);
  if (entries.size > READ_CACHE_MAX_ENTRIES) {
    const oldest = entries.keys().next().value;
    if (oldest !== undefined) entries.delete(oldest);
  }
}

/** Folds a write's answer into the kept answer for `path`; nothing when none is kept. */
export function updateCachedRead<T>(token: string | null, path: string, update: (before: T) => T): void {
  const before = cachedRead<T>(token, path);
  if (before !== undefined) rememberRead(token, path, update(before));
}

/** Drops `token`'s account's kept answer for `path` (R2630000: the gate's, when the account changes). */
export function forgetRead(token: string | null, path: string): void {
  entries.delete(`${accountOf(token)}\n${path}`);
}

/** Sign-out (`net/session.ts`) and every test (`test/setup.ts`). */
export function clearReadCache(): void {
  entries.clear();
  owner = null;
}

/** Test support: how many answers are kept. */
export function __readCacheSizeForTests(): number {
  return entries.size;
}
