// R3349000: the variant levels the account earned but has not viewed yet, so the Collection's owned-cards
// grid carries a "new" mark on their cards until the card's view or preview opens. Only ids the server
// already granted this account are kept (CLAUDE.md rule 7): every entry is written at its grant's own
// success handler, never inferred. The store is per account, namespaced by the token's `sub` (a renewal
// keeps it; the token itself never reaches storage, R632), and degrades to memory where storage is blocked
// or the token carries no `sub`. Presentation only: nothing here enforces rules.

import { useEffect, useState } from "react";

import { tokenClaim } from "../auth/claims.ts";
import { getStored, setStored } from "../net/storage.ts";

/** One earned unseen level: the level and the card it goes on (null for an account cosmetic). */
export type NewLevel = { id: string; cardId: string | null };

/** The levels no card view claims, so marks can outlive the grant sites' own state. */
const NEW_LEVELS_KEY = "jackioh.new-levels.v1";

const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of [...listeners]) listener();
}

/** The store's own change signal; marks written elsewhere move the grid with them. */
export function subscribeNewLevels(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Marks of a token with no `sub`, or of storage that refuses writes: this page only, never stored. */
const memory = new Map<string, NewLevel[]>();

/** The non-secret account the marks belong to: the token's `sub`, null when it names none. */
function accountOf(token: string): string | null {
  return token === "" ? null : tokenClaim(token, "sub");
}

function keyOf(account: string): string {
  return `${NEW_LEVELS_KEY}.${account}`;
}

function parseAll(text: string | null): NewLevel[] {
  try {
    if (text === null) return [];
    const parsed: unknown = JSON.parse(text);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (entry): entry is NewLevel =>
        typeof entry === "object" &&
        entry !== null &&
        typeof (entry as { id?: unknown }).id === "string" &&
        (((entry as { cardId?: unknown }).cardId ?? null) === null ||
          typeof (entry as { cardId?: unknown }).cardId === "string"),
    );
  } catch {
    return [];
  }
}

function readAll(token: string): NewLevel[] {
  if (token === "") return [];
  const account = accountOf(token);
  const stored = account === null ? [] : parseAll(getStored(keyOf(account)));
  return stored.length > 0 ? stored : (memory.get(token) ?? []);
}

function writeAll(token: string, levels: NewLevel[]): void {
  const account = accountOf(token);
  if (account !== null && setStored(keyOf(account), JSON.stringify(levels))) {
    memory.delete(token);
    return;
  }
  memory.set(token, levels);
}

/** Keeps `levels` unseen for `token`, ignoring ids already kept; an empty token keeps nothing. */
export function markNewLevels(token: string, levels: readonly NewLevel[]): void {
  if (token === "") return;
  const kept = readAll(token);
  const known = new Set(kept.map((entry) => `${entry.id}\n${entry.cardId ?? ""}`));
  const next = [...kept];
  for (const level of levels) {
    const key = `${level.id}\n${level.cardId ?? ""}`;
    if (!known.has(key)) {
      known.add(key);
      next.push({ id: level.id, cardId: level.cardId ?? null });
    }
  }
  if (next.length === kept.length) return;
  writeAll(token, next);
  notify();
}

/** The unseen level ids on `cardId`, in the order they were earned. */
export function unseenForCard(token: string, cardId: string): string[] {
  return readAll(token)
    .filter((entry) => entry.cardId === cardId)
    .map((entry) => entry.id);
}

/** Whether `cardId` carries the "new" mark. */
export function hasUnseen(token: string, cardId: string): boolean {
  return unseenForCard(token, cardId).length > 0;
}

/** Forgets every unseen level on `cardId`: its view or preview opened. */
export function clearSeenForCard(token: string, cardId: string): void {
  if (token === "") return;
  const kept = readAll(token);
  const next = kept.filter((entry) => entry.cardId !== cardId);
  if (next.length === kept.length) return;
  writeAll(token, next);
  notify();
}

/** A render's subscription to the store: marks written elsewhere move the grid with them. */
export function useNewLevels(): void {
  const [, setVersion] = useState(0);
  useEffect(
    () =>
      subscribeNewLevels(() => {
        setVersion((version) => version + 1);
      }),
    [],
  );
}

/** Test seam: forgets everything this module's listeners track. */
export function resetNewLevelsForTests(): void {
  listeners.clear();
  memory.clear();
}
