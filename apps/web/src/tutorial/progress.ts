// Device-local tutorial progress (SPEC §9.9, §9.10; R294, R320, R321, R322).
// `accountSync.ts` merges known completions and the newest Hide/Show choice through
// `adoptTutorialProgress`; unavailable or malformed storage means no saved progress.

import { useSyncExternalStore } from "react";

import { TUTORIAL_PROGRESS_KEY, TUTORIAL_PROGRESS_VERSION } from "./config.ts";
import { TUTORIAL_LESSONS, type TutorialLesson } from "./lessons.ts";

export type TutorialHiddenChoice = {
  readonly hidden: boolean;
  readonly at: number;
};

export type TutorialProgress = {
  readonly completed: readonly string[];
  readonly hiddenChoice: TutorialHiddenChoice | null;
};

export type LessonStatus = "locked" | "unlocked" | "completed";

const EMPTY: TutorialProgress = Object.freeze({
  completed: Object.freeze([]) as readonly string[],
  hiddenChoice: null,
});

let snapshot: TutorialProgress | null = null;

const listeners = new Set<() => void>();
let storageListening = false;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseChoice(raw: unknown): TutorialHiddenChoice | null {
  if (!isRecord(raw)) return null;
  const { hidden, at } = raw;
  if (typeof hidden !== "boolean" || typeof at !== "number" || !Number.isSafeInteger(at) || at < 0) return null;
  return Object.freeze({ hidden, at });
}

function normalise(ids: Iterable<unknown>, hiddenChoice: TutorialHiddenChoice | null = null): TutorialProgress {
  const wanted = new Set<string>();
  for (const id of ids) if (typeof id === "string") wanted.add(id);
  const completed = TUTORIAL_LESSONS.filter((lesson) => wanted.has(lesson.id)).map((lesson) => lesson.id);
  if (completed.length === 0 && hiddenChoice === null) return EMPTY;
  return Object.freeze({ completed: Object.freeze(completed), hiddenChoice });
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
    return normalise(value.completed as unknown[], parseChoice(value.hiddenChoice));
  } catch {
    return EMPTY;
  }
}

function storageOrNull(): Storage | null {
  try {
    return window.localStorage ?? null;
  } catch {
    return null;
  }
}

function loadStored(): TutorialProgress {
  try {
    const raw = storageOrNull()?.getItem(TUTORIAL_PROGRESS_KEY) ?? null;
    return raw === null ? EMPTY : parseTutorialProgress(raw);
  } catch {
    return EMPTY;
  }
}

function persist(progress: TutorialProgress): void {
  try {
    storageOrNull()?.setItem(
      TUTORIAL_PROGRESS_KEY,
      JSON.stringify({
        v: TUTORIAL_PROGRESS_VERSION,
        completed: progress.completed,
        // Omit an unmade choice to keep R294's stored shape.
        ...(progress.hiddenChoice === null ? {} : { hiddenChoice: progress.hiddenChoice }),
      }),
    );
  } catch {
    // Storage failure must not interrupt the current lesson.
  }
}

function sameChoice(a: TutorialHiddenChoice | null, b: TutorialHiddenChoice | null): boolean {
  return a === null || b === null ? a === b : a.hidden === b.hidden && a.at === b.at;
}

function sameProgress(a: TutorialProgress, b: TutorialProgress): boolean {
  return (
    a.completed.length === b.completed.length &&
    a.completed.every((id, at) => b.completed[at] === id) &&
    sameChoice(a.hiddenChoice, b.hiddenChoice)
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
  return commit(normalise([...current.completed, id], current.hiddenChoice));
}

/** R321, R322: stamp new choices after the old one even if this device clock steps back. */
export function setTutorialHidden(hidden: boolean, now: number = Date.now()): TutorialProgress {
  const current = readTutorialProgress();
  const at = Math.max(Math.floor(now), (current.hiddenChoice?.at ?? -1) + 1);
  return commit(normalise(current.completed, Object.freeze({ hidden, at })));
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
};

function newerChoice(a: TutorialHiddenChoice | null, b: TutorialHiddenChoice | null): TutorialHiddenChoice | null {
  if (b === null) return a;
  if (a === null || b.at > a.at) return b;
  return a;
}

/** R321: merging copies cannot lose a completion or let an older choice win. */
export function mergeTutorialProgress(a: TutorialProgressLike, b: TutorialProgressLike): TutorialProgress {
  return normalise([...a.completed, ...b.completed], newerChoice(parseChoice(a.hiddenChoice), parseChoice(b.hiddenChoice)));
}

/** R321: upload only missing completions or a later, conflicting choice. */
export function accountLacks(account: TutorialProgressLike, device: TutorialProgress): boolean {
  const listed = new Set(account.completed);
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
