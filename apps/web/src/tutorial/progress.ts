// Device-local tutorial progress (SPEC §9.9, §9.10; R294, R320, R321, R322).
// `accountSync.ts` merges known completions and the newest Hide/Show choice through
// `adoptTutorialProgress`; unavailable or malformed storage means no saved progress.
// R2205002: the Field Guide's found entries ride in the same record on the device only; the account
// never sees them, and a lesson won anywhere counts its entries found.
// R2434000: an id of the lessons before #2434 reads as the lesson in its place on the path, so nothing
// won is un-won. R2434002: a lesson whose last game was conceded is kept as lost, on the device only,
// until it is won.

import { useSyncExternalStore } from "react";

import { TUTORIAL_PROGRESS_KEY, TUTORIAL_PROGRESS_VERSION } from "./config.ts";
import { FIELD_GUIDE } from "./discoveries.ts";
import { TUTORIAL_LESSONS, currentLessonId, type TutorialLesson } from "./lessons.ts";
import { isRecord } from "../wire/json.ts";
import { getStored, setStored } from "../net/storage.ts";

export type TutorialHiddenChoice = {
  readonly hidden: boolean;
  readonly at: number;
};

export type TutorialProgress = {
  readonly completed: readonly string[];
  readonly hiddenChoice: TutorialHiddenChoice | null;
  /** R2205002: the Field Guide entries found on this device, in the guide's order. */
  readonly found: readonly string[];
  /** R2434002: the lessons whose last game was conceded and that are not won, in path order; this device's alone. */
  readonly lost: readonly string[];
};

export type LessonStatus = "locked" | "unlocked" | "completed";

/** R2434002: whether this device keeps `lesson` as lost: its last game conceded, and not won since. */
export function lessonLost(progress: TutorialProgress, lesson: TutorialLesson): boolean {
  return progress.lost.includes(lesson.id);
}

const EMPTY: TutorialProgress = Object.freeze({
  completed: Object.freeze([]) as readonly string[],
  hiddenChoice: null,
  found: Object.freeze([]) as readonly string[],
  lost: Object.freeze([]) as readonly string[],
});

let snapshot: TutorialProgress | null = null;

const listeners = new Set<() => void>();
let storageListening = false;

function parseChoice(raw: unknown): TutorialHiddenChoice | null {
  if (!isRecord(raw)) return null;
  const { hidden, at } = raw;
  if (typeof hidden !== "boolean" || typeof at !== "number" || !Number.isSafeInteger(at) || at < 0) return null;
  return Object.freeze({ hidden, at });
}

function strings(ids: Iterable<unknown>): Set<string> {
  const wanted = new Set<string>();
  for (const id of ids) if (typeof id === "string") wanted.add(id);
  return wanted;
}

/** R2434000: the lesson ids `ids` name, an earlier id read as the lesson in its place. */
function lessonIds(ids: Iterable<unknown>): Set<string> {
  const wanted = new Set<string>();
  for (const id of strings(ids)) wanted.add(currentLessonId(id));
  return wanted;
}

function normalize(
  ids: Iterable<unknown>,
  hiddenChoice: TutorialHiddenChoice | null = null,
  foundIds: Iterable<unknown> = [],
  lostIds: Iterable<unknown> = [],
): TutorialProgress {
  const wanted = lessonIds(ids);
  const completed = TUTORIAL_LESSONS.filter((lesson) => wanted.has(lesson.id)).map((lesson) => lesson.id);
  const met = strings(foundIds);
  const found = FIELD_GUIDE.filter((entry) => met.has(entry.id)).map((entry) => entry.id);
  // A win clears a loss: a lesson is lost only while it is not won.
  const conceded = lessonIds(lostIds);
  const lost = TUTORIAL_LESSONS.filter((lesson) => conceded.has(lesson.id) && !wanted.has(lesson.id)).map((lesson) => lesson.id);
  if (completed.length === 0 && hiddenChoice === null && found.length === 0 && lost.length === 0) return EMPTY;
  return Object.freeze({ completed: Object.freeze(completed), hiddenChoice, found: Object.freeze(found), lost: Object.freeze(lost) });
}

/** Untrusted stored data must parse to the current shape or become no progress. */
export function parseTutorialProgress(raw: unknown): TutorialProgress {
  try {
    let value: unknown = raw;
    if (typeof value === "string") {
      try {
        value = JSON.parse(value) as unknown;
      } catch {
        return EMPTY;
      }
    }
    if (!isRecord(value) || value.v !== TUTORIAL_PROGRESS_VERSION || !Array.isArray(value.completed)) return EMPTY;
    const found = Array.isArray(value.found) ? (value.found as unknown[]) : [];
    const lost = Array.isArray(value.lost) ? (value.lost as unknown[]) : [];
    return normalize(value.completed as unknown[], parseChoice(value.hiddenChoice), found, lost);
  } catch {
    return EMPTY;
  }
}

function loadStored(): TutorialProgress {
  try {
    const raw = getStored(TUTORIAL_PROGRESS_KEY);
    return raw === null ? EMPTY : parseTutorialProgress(raw);
  } catch {
    return EMPTY;
  }
}

function persist(progress: TutorialProgress): void {
  // A storage failure must not interrupt the current lesson.
  setStored(
    TUTORIAL_PROGRESS_KEY,
    JSON.stringify({
      v: TUTORIAL_PROGRESS_VERSION,
      completed: progress.completed,
      // Omit an unmade choice to keep R294's stored shape.
      ...(progress.hiddenChoice === null ? {} : { hiddenChoice: progress.hiddenChoice }),
      ...(progress.found.length === 0 ? {} : { found: progress.found }),
      ...(progress.lost.length === 0 ? {} : { lost: progress.lost }),
    }),
  );
}

function sameChoice(a: TutorialHiddenChoice | null, b: TutorialHiddenChoice | null): boolean {
  return a === null || b === null ? a === b : a.hidden === b.hidden && a.at === b.at;
}

function sameList(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((id, at) => b[at] === id);
}

function sameProgress(a: TutorialProgress, b: TutorialProgress): boolean {
  return (
    sameList(a.completed, b.completed) &&
    sameChoice(a.hiddenChoice, b.hiddenChoice) &&
    sameList(a.found, b.found) &&
    sameList(a.lost, b.lost)
  );
}

function notify(): void {
  for (const listener of [...listeners]) listener();
}

export function readTutorialProgress(): TutorialProgress {
  if (snapshot === null) snapshot = loadStored();
  return snapshot;
}

function commit(next: TutorialProgress): TutorialProgress {
  const current = readTutorialProgress();
  const settled = sameProgress(current, next) ? current : next;
  snapshot = settled;
  persist(settled);
  if (settled !== current) notify();
  return settled;
}

export function markLessonComplete(id: string): TutorialProgress {
  const current = readTutorialProgress();
  return commit(normalize([...current.completed, id], current.hiddenChoice, current.found, current.lost));
}

/** R2434002: a conceded lesson is lost: kept so on this device until the lesson is won. */
export function markLessonLost(id: string): TutorialProgress {
  const current = readTutorialProgress();
  return commit(normalize(current.completed, current.hiddenChoice, current.found, [...current.lost, id]));
}

/** R2205002: entries the game showed found, kept on this device only. */
export function markFound(ids: readonly string[]): TutorialProgress {
  const current = readTutorialProgress();
  return commit(normalize(current.completed, current.hiddenChoice, [...current.found, ...ids], current.lost));
}

/**
 * R2205002: what the Field Guide shows as met: the entries found on this device and every entry of a
 * lesson won (on any device), since a win names whatever its game did not show found (R314).
 */
export function fieldGuideOf(progress: TutorialProgressLike, lessons: readonly TutorialLesson[] = TUTORIAL_LESSONS): readonly string[] {
  const met = new Set(progress.found ?? []);
  for (const lesson of lessons) {
    if (progress.completed.includes(lesson.id)) for (const id of lesson.discoveries ?? []) met.add(id);
  }
  return FIELD_GUIDE.filter((entry) => met.has(entry.id)).map((entry) => entry.id);
}

/** R321, R322: stamp new choices after the old one even if this device clock steps back. */
export function setTutorialHidden(hidden: boolean, now: number = Date.now()): TutorialProgress {
  const current = readTutorialProgress();
  const at = Math.max(Math.floor(now), (current.hiddenChoice?.at ?? -1) + 1);
  return commit(normalize(current.completed, Object.freeze({ hidden, at }), current.found, current.lost));
}

export function isTutorialHidden(progress: TutorialProgress): boolean {
  return progress.hiddenChoice?.hidden === true;
}

export function resetTutorialProgress(): TutorialProgress {
  return commit(EMPTY);
}

export type TutorialProgressLike = {
  readonly completed: readonly string[];
  readonly hiddenChoice: TutorialHiddenChoice | null;
  /** The device's alone: the account's copy never carries it (R2205002). */
  readonly found?: readonly string[];
  /** The device's alone, as `found` is (R2434002). */
  readonly lost?: readonly string[];
};

function newerChoice(a: TutorialHiddenChoice | null, b: TutorialHiddenChoice | null): TutorialHiddenChoice | null {
  if (b === null) return a;
  if (a === null || b.at > a.at) return b;
  return a;
}

/** R321: merging copies cannot lose a completion or let an older choice win. */
export function mergeTutorialProgress(a: TutorialProgressLike, b: TutorialProgressLike): TutorialProgress {
  return normalize(
    [...a.completed, ...b.completed],
    newerChoice(parseChoice(a.hiddenChoice), parseChoice(b.hiddenChoice)),
    [...(a.found ?? []), ...(b.found ?? [])],
    [...(a.lost ?? []), ...(b.lost ?? [])],
  );
}

/** R321: upload only missing completions or a later, conflicting choice. */
export function accountLacks(account: TutorialProgressLike, device: TutorialProgress): boolean {
  const listed = new Set([...account.completed, ...lessonIds(account.completed)]);
  if (device.completed.some((id) => !listed.has(id))) return true;
  const mine = device.hiddenChoice;
  const theirs = parseChoice(account.hiddenChoice);
  if (mine === null) return false;
  return theirs === null || (mine.hidden !== theirs.hidden && mine.at > theirs.at);
}

export function adoptTutorialProgress(other: TutorialProgressLike): TutorialProgress {
  return commit(mergeTutorialProgress(readTutorialProgress(), other));
}

function onStorage(event: StorageEvent): void {
  if (typeof event.key === "string" && event.key !== TUTORIAL_PROGRESS_KEY) return;
  const next = typeof event.newValue === "string" ? parseTutorialProgress(event.newValue) : loadStored();
  const current = snapshot;
  if (current !== null && sameProgress(current, next)) return;
  snapshot = next;
  notify();
}

export function subscribeTutorialProgress(listener: () => void): () => void {
  const subscription = (): void => {
    listener();
  };
  listeners.add(subscription);
  if (!storageListening) {
    window.addEventListener("storage", onStorage);
    storageListening = true;
  }
  return () => {
    listeners.delete(subscription);
    if (listeners.size === 0 && storageListening) {
      window.removeEventListener("storage", onStorage);
      storageListening = false;
    }
  };
}

export function useTutorialProgress(): TutorialProgress {
  return useSyncExternalStore(subscribeTutorialProgress, readTutorialProgress, readTutorialProgress);
}

/** Lesson 1 is always open; lesson N opens once lesson N-1 is completed; completed stays completed. */
export function lessonStatus(progress: TutorialProgress, lesson: TutorialLesson): LessonStatus {
  if (progress.completed.includes(lesson.id)) return "completed";
  if (lesson.number <= 1) return "unlocked";
  const previous = TUTORIAL_LESSONS.find((candidate) => candidate.number === lesson.number - 1);
  return previous !== undefined && progress.completed.includes(previous.id) ? "unlocked" : "locked";
}

export function nextLessonToPlay(progress: TutorialProgress): TutorialLesson | undefined {
  return TUTORIAL_LESSONS.find((lesson) => lessonStatus(progress, lesson) === "unlocked");
}

export function __resetTutorialProgressForTests(): void {
  snapshot = null;
}
