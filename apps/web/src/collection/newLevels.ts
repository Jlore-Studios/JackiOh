// R3349000: the variant levels the account earned but has not viewed yet, so the Collection's owned-cards
// grid carries a "new" mark on their cards until the card's view or preview opens. Only ids the server
// already granted this account are kept (CLAUDE.md rule 7): every entry is written at its grant's own
// success handler, never inferred. The store is per signed-in session, namespaced by its token, and
// degrades to memory where storage is blocked. Presentation only: nothing here enforces rules.

import { useEffect, useState } from "react";

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

function keyOf(account: string): string {
  return `${NEW_LEVELS_KEY}.${account}`;
}

function readAll(account: string): NewLevel[] {
  if (account === "") return [];
  try {
    const text = getStored(keyOf(account));
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

/** Keeps `levels` unseen for `account`, ignoring ids already kept; an empty account keeps nothing. */
export function markNewLevels(account: string, levels: readonly NewLevel[]): void {
  if (account === "") return;
  const kept = readAll(account);
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
  setStored(keyOf(account), JSON.stringify(next));
  notify();
}

/** The unseen level ids on `cardId`, in the order they were earned. */
export function unseenForCard(account: string, cardId: string): string[] {
  return readAll(account)
    .filter((entry) => entry.cardId === cardId)
    .map((entry) => entry.id);
}

/** Whether `cardId` carries the "new" mark. */
export function hasUnseen(account: string, cardId: string): boolean {
  return unseenForCard(account, cardId).length > 0;
}

/** Forgets every unseen level on `cardId`: its view or preview opened. */
export function clearSeenForCard(account: string, cardId: string): void {
  if (account === "") return;
  const kept = readAll(account);
  const next = kept.filter((entry) => entry.cardId !== cardId);
  if (next.length === kept.length) return;
  setStored(keyOf(account), JSON.stringify(next));
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
}
