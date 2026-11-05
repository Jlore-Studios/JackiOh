// SPEC §14.14, R3013001, R3013002: card proficiency's stamps. The server counts each card's plays and the level
// they reach (R3013000); level n unlocks stamp n, of PROFICIENCY_LEVELS, and the account picks at most one per card.
// This module holds the signed-in account's own copy (`proficiencySync.ts` loads it and sends each pick), which
// the viewer's own faces wear (`worn`), and reads another seat's stamp from its frozen loadout (`TheirCopy`,
// loadouts.ts), which the server sends only once that card has been shown (R1834003). A face that is a token or a
// Created card wears none: proficiency counts neither. Drawn state only (CLAUDE.md rule 7): nothing here reaches the
// engine, an action or the view, and the client checks no pick: the server refuses a locked one.

import { useContext, useSyncExternalStore } from "react";

import { PROFICIENCY_LEVELS, PROFICIENCY_LEVEL_PLAYS } from "@jackioh/server-config";
import { isRecord } from "../wire/json.ts";
import { TheirCopyContext } from "./cosmetics.ts";
import type { FaceModel } from "./model.ts";

/** R3013000: one card's proficiency as the account holds it. */
export type OwnProficiency = { plays: number; level: number; stamp: number | null };

/** R3013001: a stamp a card can wear, a whole number of `1..PROFICIENCY_LEVELS`; level n's stamp is n. */
export function isStampLevel(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= PROFICIENCY_LEVELS;
}

/** Every stamp, lowest first: the Collection's picker lists them all, the locked ones too. */
export const STAMP_LEVELS: readonly number[] = Array.from({ length: PROFICIENCY_LEVELS }, (_, at) => at + 1);

/** R3013000: the plays that reach `level` (`PROFICIENCY_LEVEL_PLAYS`), null past the top. */
export function playsForLevel(level: number): number | null {
  return PROFICIENCY_LEVEL_PLAYS[level - 1] ?? null;
}

let own: ReadonlyMap<string, OwnProficiency> = new Map();
const listeners = new Set<() => void>();
const writeListeners = new Set<(defId: string, stamp: number | null) => void>();

function notify(): void {
  for (const listener of [...listeners]) listener();
}

/** One entry of the account's answer, or null when it is not one. A stamp past the row's level is worn as none. */
function entryOf(raw: unknown): [string, OwnProficiency] | null {
  if (!isRecord(raw) || typeof raw.cardId !== "string") return null;
  const { plays, level } = raw;
  if (typeof plays !== "number" || !Number.isInteger(plays) || typeof level !== "number" || !Number.isInteger(level)) return null;
  const stamp = isStampLevel(raw.stamp) && raw.stamp <= level ? raw.stamp : null;
  return [raw.cardId, { plays, level, stamp }];
}

/** R3013001: the account's copy, whole, as `GET /api/proficiency` answers it; an entry of another shape is left out. */
export function replaceOwnProficiency(cards: readonly unknown[]): void {
  const next = new Map<string, OwnProficiency>();
  for (const raw of cards) {
    const entry = entryOf(raw);
    if (entry !== null) next.set(entry[0], entry[1]);
  }
  own = next;
  notify();
}

/** The account's proficiency of `defId`, or null for a card it has not played in a game that counts. */
export function readOwnProficiency(defId: string): OwnProficiency | null {
  return own.get(defId) ?? null;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** `readOwnProficiency`, re-rendering on every change. */
export function useOwnProficiency(defId: string): OwnProficiency | null {
  const read = (): OwnProficiency | null => readOwnProficiency(defId);
  return useSyncExternalStore(subscribe, read, read);
}

/**
 * R3013001: picks the stamp `defId` wears, or none with null, here at once and on the account through
 * `subscribeStampWrites`. A stamp the card's level has not unlocked writes nothing and is false, and so is a card
 * the account has never played, which has no stamp to pick or clear.
 */
export function writeOwnStamp(defId: string, stamp: number | null): boolean {
  const held = own.get(defId);
  if (held === undefined) return false;
  if (stamp !== null && (!isStampLevel(stamp) || stamp > held.level)) return false;
  if (held.stamp === stamp) return true;
  own = new Map(own).set(defId, { ...held, stamp });
  notify();
  for (const listener of [...writeListeners]) listener(defId, stamp);
  return true;
}

/** R3013001: calls `listener` with the card and its new stamp after every pick written on this page. */
export function subscribeStampWrites(listener: (defId: string, stamp: number | null) => void): () => void {
  writeListeners.add(listener);
  return () => {
    writeListeners.delete(listener);
  };
}

/**
 * R3013002: the stamp `face` wears: on the viewer's own copy (`worn`) the account's pick; below a `TheirCopy`
 * the seat's frozen one, once the server has sent it; else none. A token and a Created card wear none.
 */
export function useCardStamp(face: FaceModel, worn: boolean): number | null {
  const mine = useOwnProficiency(face.defId);
  const theirs = useContext(TheirCopyContext);
  if (face.token === true || face.created === true) return null;
  const stamp = worn ? (mine?.stamp ?? null) : (theirs?.stamps?.[face.defId] ?? null);
  return isStampLevel(stamp) ? stamp : null;
}

/** Test seam: no account copy, no listeners. */
export function resetProficiencyForTests(): void {
  own = new Map();
  listeners.clear();
  writeListeners.clear();
}
