// The Infinite Assault's device store: one open, versioned attempt, plus the recent days' records. A
// fight in progress is kept by the practice worker in a slot of its own (R4582001), never here. The
// Assault's rules, through the WebAssembly module, decide every transition (CLAUDE.md rule 7).

import { useMemo, useSyncExternalStore } from "react";

import type {
  AssaultAction,
  AssaultConstants,
  AssaultContent,
  AssaultDay,
  AssaultDeckSource,
  AssaultFile,
  AssaultRecord,
  AssaultSetup,
  AssaultState,
} from "@jackioh/run";

import { t } from "../i18n/i18n.ts";
import { messageOf } from "../net/errors.ts";
import { getStored, removeStored, setStored } from "../net/storage.ts";
import {
  assaultConstants,
  assaultContent,
  assaultContentAt,
  assaultCreate,
  assaultDay,
  assaultDayDeck,
  assaultFold,
  assaultRecord,
  assaultReduce,
  catalogVersion,
} from "../wasm/index.ts";
import { isCount, isRecord, isStringArray } from "../wire/json.ts";
import {
  ASSAULT_ATTEMPT_KEY,
  ASSAULT_ATTEMPT_STEP,
  ASSAULT_DATE_FORMAT,
  ASSAULT_FIRST_ATTEMPT,
  ASSAULT_FIRST_LEVEL,
  ASSAULT_HELD_KEY,
  ASSAULT_RECORDS_KEY,
  ASSAULT_STORAGE_VERSION,
} from "./config.ts";

export type AssaultStart = {
  date: string;
  deck: readonly string[];
  source: AssaultDeckSource;
  startLevel: number;
};

export type Assault = {
  constants: AssaultConstants;
  content: AssaultContent;
  day: AssaultDay;
  state: AssaultState | null;
  record: AssaultRecord;
  records: Readonly<Record<string, AssaultRecord>>;
  /** The refusal, word for word, or null. */
  start(start: AssaultStart): string | null;
  /** The refusal, word for word, or null. */
  act(action: AssaultAction): string | null;
  /**
   * R4733000: hold a finished fight's report beside the kept attempt, for the fight screen's
   * arrival effect. Silent unless an attempt stands on a fight: only the fight's own `fightOver`
   * is ever held.
   */
  hold(action: AssaultAction): void;
  /** A finished attempt's Try again (R4581000): the refusal, word for word, or null. */
  tryAgain(): string | null;
  /** Leave a finished attempt's result for today's day screen. */
  dismiss(): void;
};

type Core = {
  constants: AssaultConstants;
  content: AssaultContent;
  day: AssaultDay;
  state: AssaultState | null;
  file: AssaultFile | null;
  records: Record<string, AssaultRecord>;
  record: AssaultRecord;
};

function assaultToday(now: Date = new Date()): string {
  return now.toISOString().slice(0, ASSAULT_DATE_FORMAT.length);
}

function isPositiveCount(value: unknown): value is number {
  return isCount(value) && value >= ASSAULT_FIRST_LEVEL;
}

function parseSetup(raw: unknown): AssaultSetup | null {
  if (!isRecord(raw)) return null;
  const { date, deck, source, startLevel, attempt } = raw;
  if (typeof date !== "string" || !isStringArray(deck) || !isPositiveCount(startLevel) || !isPositiveCount(attempt)) return null;
  if (source !== "saved" && source !== "preset" && source !== "dayDeck") return null;
  return { date, deck, source, startLevel, attempt };
}

/** R4581001: a record no attempt could make, its best past the module's last level or a checkpoint
 *  that is none of the module's, is malformed: the module refuses a level past its own range, and a
 *  start from a checkpoint that is none. */
function parseRecord(raw: unknown, constants: AssaultConstants): AssaultRecord | null {
  if (!isRecord(raw)) return null;
  const { date, best, checkpoints, attempts, repelledOn } = raw;
  const isCheckpoint = (value: unknown): value is number => isPositiveCount(value) && constants.checkpoints.includes(value);
  if (typeof date !== "string" || !isCount(best) || best > constants.levels || !Array.isArray(checkpoints) || !checkpoints.every(isCheckpoint)) return null;
  if (!isCount(attempts) || (repelledOn !== null && !isPositiveCount(repelledOn))) return null;
  return { date, best, checkpoints, attempts, repelledOn };
}

function parseAction(raw: unknown): AssaultAction | null {
  if (!isRecord(raw)) return null;
  switch (raw.kind) {
    case "fight":
      return { kind: "fight" };
    case "fightOver":
      return typeof raw.won === "boolean" && isCount(raw.turns)
        ? { kind: "fightOver", won: raw.won, turns: raw.turns }
        : null;
    case "abandon":
      return { kind: "abandon" };
    default:
      return null;
  }
}

function parseFile(raw: unknown, constants: AssaultConstants): AssaultFile | null {
  if (!isRecord(raw) || typeof raw.version !== "string" || !Array.isArray(raw.actions)) return null;
  const setup = parseSetup(raw.setup);
  const record = parseRecord(raw.record, constants);
  const actions = raw.actions.map(parseAction);
  if (setup === null || record === null || actions.some((action) => action === null)) return null;
  return { version: raw.version, setup, record, actions: actions as AssaultAction[] };
}

function readJson(key: string): { present: boolean; value: unknown } {
  const raw = getStored(key);
  if (raw === null) return { present: false, value: null };
  try {
    return { present: true, value: JSON.parse(raw) as unknown };
  } catch {
    removeStored(key);
    return { present: true, value: null };
  }
}

function parseAttempt(raw: unknown, constants: AssaultConstants): AssaultFile | null {
  if (!isRecord(raw) || raw.v !== ASSAULT_STORAGE_VERSION) return null;
  return parseFile(raw.file, constants);
}

function readAttempt(constants: AssaultConstants): AssaultFile | null {
  const stored = readJson(ASSAULT_ATTEMPT_KEY);
  if (!stored.present) return null;
  const attempt = parseAttempt(stored.value, constants);
  if (attempt === null) removeStored(ASSAULT_ATTEMPT_KEY);
  return attempt;
}

/**
 * R4733000: a finished fight's report, held beside the kept attempt keyed to its fight: the
 * fight's own `fightOver` with the attempt and the level.
 */
type HeldAssault = { attempt: number; level: number; action: AssaultAction };

function parseHeld(raw: unknown): HeldAssault | null {
  if (!isRecord(raw)) return null;
  const { attempt, level, action } = raw;
  if (!isCount(attempt) || !isCount(level)) return null;
  const parsed = parseAction(action);
  if (parsed === null || parsed.kind !== "fightOver") return null;
  return { attempt, level, action: parsed };
}

function readHeld(): HeldAssault | null {
  const stored = readJson(ASSAULT_HELD_KEY);
  if (!stored.present) return null;
  const held = parseHeld(stored.value);
  if (held === null) removeStored(ASSAULT_HELD_KEY);
  return held;
}

function writeHeld(held: HeldAssault): void {
  writeJson(ASSAULT_HELD_KEY, held);
}

function clearHeld(): void {
  removeStored(ASSAULT_HELD_KEY);
}

function parseRecords(raw: unknown, constants: AssaultConstants): Record<string, AssaultRecord> | null {
  if (!isRecord(raw) || raw.v !== ASSAULT_STORAGE_VERSION || !isRecord(raw.days)) return null;
  const days: Record<string, AssaultRecord> = {};
  for (const [date, value] of Object.entries(raw.days)) {
    const record = parseRecord(value, constants);
    if (record === null || record.date !== date) return null;
    days[date] = record;
  }
  return days;
}

function readRecords(constants: AssaultConstants): Record<string, AssaultRecord> {
  const stored = readJson(ASSAULT_RECORDS_KEY);
  if (!stored.present) return {};
  const records = parseRecords(stored.value, constants);
  if (records === null) {
    removeStored(ASSAULT_RECORDS_KEY);
    return {};
  }
  return records;
}

function writeJson(key: string, value: unknown): void {
  setStored(key, JSON.stringify(value));
}

function emptyRecord(date: string): AssaultRecord {
  return { date, best: 0, checkpoints: [], attempts: 0, repelledOn: null };
}

function trimRecords(
  records: Readonly<Record<string, AssaultRecord>>,
  constants: AssaultConstants,
): Record<string, AssaultRecord> {
  const kept = Object.keys(records).sort().slice(-constants.historyDays);
  return Object.fromEntries(kept.map((date) => [date, records[date]]).filter((entry) => entry[1] !== undefined));
}

function persistRecords(records: Readonly<Record<string, AssaultRecord>>): void {
  writeJson(ASSAULT_RECORDS_KEY, { v: ASSAULT_STORAGE_VERSION, days: records });
}

function persistAttempt(file: AssaultFile): void {
  writeJson(ASSAULT_ATTEMPT_KEY, { v: ASSAULT_STORAGE_VERSION, file });
}

function terminal(state: AssaultState): boolean {
  return state.phase === "lost" || state.phase === "repelled" || state.phase === "abandoned";
}

/** R4581000: an open attempt is folded on the content version and UTC day it started on. */
function foldAttempt(file: AssaultFile, currentContent: AssaultContent): {
  content: AssaultContent;
  day: AssaultDay;
  state: AssaultState;
} | null {
  try {
    const content = file.version === catalogVersion() ? currentContent : assaultContentAt(file.version);
    if (content === null) return null;
    const day = assaultDay(file.setup.date, content);
    const folded = assaultFold(file, day);
    return "error" in folded ? null : { content, day, state: folded.state };
  } catch {
    return null;
  }
}

function loadCore(): Core {
  const constants = assaultConstants();
  const currentContent = assaultContent();
  const storedRecords = readRecords(constants);
  let records = trimRecords(storedRecords, constants);
  if (Object.keys(records).length !== Object.keys(storedRecords).length) persistRecords(records);
  const file = readAttempt(constants);
  if (file !== null) {
    const folded = foldAttempt(file, currentContent);
    if (folded !== null) {
      const before = records[file.setup.date] ?? file.record;
      const record = assaultRecord(before, folded.state);
      records = trimRecords({ ...records, [record.date]: record }, constants);
      persistRecords(records);
      let state = folded.state;
      let kept = file;
      let foldedRecord = record;
      // R4733000: a held report for this fight is the fight's result, left before Continue sent
      // it: apply it once, as Continue would, in the same tick that clears the hold. A refused
      // or mismatched hold is dropped, and the fight is dealt fresh from its seed as today.
      const held = readHeld();
      if (held !== null) {
        if (state.phase === "fight" && state.setup.attempt === held.attempt && state.level === held.level) {
          const base: Core = {
            constants,
            content: folded.content,
            day: folded.day,
            state,
            file: kept,
            records,
            record: foldedRecord,
          };
          const applied = commitAction(base, held.action);
          if (typeof applied !== "string") {
            state = applied.state as AssaultState;
            kept = applied.file as AssaultFile;
            records = applied.records;
            foldedRecord = applied.record;
          }
        }
        // The hold is consumed either way: applied once, or dropped with the fight dealt fresh.
        clearHeld();
      }
      if (terminal(state)) removeStored(ASSAULT_ATTEMPT_KEY);
      return {
        constants,
        content: folded.content,
        day: folded.day,
        state,
        file: kept,
        records,
        record: foldedRecord,
      };
    }
    removeStored(ASSAULT_ATTEMPT_KEY);
    // R4733000: no attempt stands on the held report's fight, so the hold goes with it.
    clearHeld();
  }
  const date = assaultToday();
  const day = assaultDay(date, currentContent);
  return {
    constants,
    content: currentContent,
    day,
    state: null,
    file: null,
    records,
    record: records[date] ?? emptyRecord(date),
  };
}

let core: Core | null = null;
const listeners = new Set<() => void>();
let pageListening = false;
/** An Assault page was open and left: the cache may be older than what the device holds. */
let pageWasOpen = false;

function current(): Core {
  core ??= loadCore();
  return core;
}

function commit(next: Core): void {
  core = next;
  for (const listener of [...listeners]) listener();
}

function reloadCore(): void {
  // Another tab changed what is kept: read it all again, but for a finished attempt this tab still shows.
  const kept = core;
  const next = loadCore();
  commit(
    next.state === null && kept !== null && kept.state !== null && terminal(kept.state)
      ? { ...next, content: kept.content, day: kept.day, state: kept.state, file: kept.file, record: next.records[kept.record.date] ?? kept.record }
      : next,
  );
}

function onStorage(event: StorageEvent): void {
  // R4733000: another tab holding or clearing a report reloads this tab too, so a report is
  // applied once no matter which tab left the fight.
  if (event.key !== null && event.key !== ASSAULT_ATTEMPT_KEY && event.key !== ASSAULT_RECORDS_KEY && event.key !== ASSAULT_HELD_KEY) return;
  reloadCore();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (!pageListening && typeof window !== "undefined") {
    window.addEventListener("storage", onStorage);
    pageListening = true;
    // The page opens again after leaving: start from what the device holds now, as another tab
    // may have written while nothing listened. A first load already read it. A finished attempt
    // this tab still shows is kept.
    if (pageWasOpen) reloadCore();
  }
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) pageWasOpen = true;
    if (listeners.size === 0 && pageListening) {
      window.removeEventListener("storage", onStorage);
      pageListening = false;
    }
  };
}

function startAttempt(start: AssaultStart): string | null {
  const from = current();
  if (from.state !== null && !terminal(from.state)) return t("Finish or abandon the attempt in progress first.");
  // R4581000: every attempt started is dealt on the current UTC day, so a Start from a screen left open
  // past UTC midnight, which names the day before, is refused and the screen shows the new day (#5005).
  if (start.date !== assaultToday()) {
    reloadCore();
    return t("A new day's Assault has begun. Look it over and start again.");
  }
  try {
    // R4581000: a finished attempt may have been folded on an older catalog's content; the next one
    // is dealt on the current content, the version its file names.
    const content = from.state === null ? from.content : assaultContent();
    const day = content === from.content && from.day.date === start.date ? from.day : assaultDay(start.date, content);
    const record = from.records[start.date] ?? emptyRecord(start.date);
    const setup: AssaultSetup = {
      date: start.date,
      deck: [...start.deck],
      source: start.source,
      startLevel: start.startLevel,
      attempt: Math.max(ASSAULT_FIRST_ATTEMPT, record.attempts + ASSAULT_ATTEMPT_STEP),
    };
    const created = assaultCreate(setup, record, day);
    if ("error" in created) return created.error;
    const file: AssaultFile = { version: catalogVersion(), setup, record, actions: [] };
    const nextRecord = assaultRecord(record, created.state);
    const records = trimRecords({ ...from.records, [start.date]: nextRecord }, from.constants);
    persistAttempt(file);
    // R4733000: a new attempt stands on no fight the hold names.
    clearHeld();
    persistRecords(records);
    commit({ ...from, content, day, state: created.state, file, records, record: nextRecord });
    return null;
  } catch (cause) {
    return messageOf(cause);
  }
}

/** R4581000: Try again starts today's next attempt, even when the finished one was an earlier day's,
 *  from the highest checkpoint today's record holds (IA7), with the finished attempt's deck, or today's
 *  Assault deck when it played the day's. */
function tryAgain(): string | null {
  const from = current();
  if (from.state === null) return t("No attempt is in progress.");
  const { deck, source } = from.state.setup;
  const date = assaultToday();
  try {
    return startAttempt({
      date,
      deck: source === "dayDeck" ? assaultDayDeck(date) : deck,
      source,
      startLevel: Math.max(ASSAULT_FIRST_LEVEL, ...(from.records[date]?.checkpoints ?? [])),
    });
  } catch (cause) {
    return messageOf(cause);
  }
}

/**
 * R4733000: the attempt after `action`, kept on the device, or the attempt's own refusal. The
 * state in memory is the caller's to commit. A taken action clears any hold for its fight in the
 * same tick that appends it, so Continue-then-leave and two tabs apply nothing twice.
 */
function commitAction(from: Core, action: AssaultAction): Core | string {
  if (from.state === null || from.file === null) return t("No attempt is in progress.");
  if (terminal(from.state)) return t("The attempt is over.");
  try {
    const reduced = assaultReduce(from.state, from.day, action);
    if ("error" in reduced) return reduced.error;
    const file: AssaultFile = { ...from.file, actions: [...from.file.actions, action] };
    const record = assaultRecord(from.record, reduced.state);
    const records = trimRecords({ ...from.records, [record.date]: record }, from.constants);
    persistRecords(records);
    if (terminal(reduced.state)) removeStored(ASSAULT_ATTEMPT_KEY);
    else persistAttempt(file);
    clearHeld();
    return { ...from, state: reduced.state, file, records, record };
  } catch (cause) {
    return messageOf(cause);
  }
}

function actOn(action: AssaultAction): string | null {
  const next = commitAction(current(), action);
  if (typeof next === "string") return next;
  commit(next);
  return null;
}

/**
 * R4733000: hold a finished fight's report beside the kept attempt, keyed to its fight. Only the
 * fight's own `fightOver` is ever held, and only while the attempt still stands on a fight.
 */
function holdAttemptReport(action: AssaultAction): void {
  const from = current();
  if (from.state === null || from.state.phase !== "fight") return;
  if (action.kind !== "fightOver") return;
  writeHeld({ attempt: from.state.setup.attempt, level: from.state.level, action });
}

function dismissResult(): void {
  const from = current();
  if (from.state !== null && !terminal(from.state)) return;
  // R4733000: leaving a finished attempt's result keeps no hold for its fight.
  clearHeld();
  const content = assaultContent();
  const date = assaultToday();
  const day = assaultDay(date, content);
  commit({
    ...from,
    content,
    day,
    state: null,
    file: null,
    record: from.records[date] ?? emptyRecord(date),
  });
}

function snapshot(): Core {
  return current();
}

export function useAssault(): Assault {
  const value = useSyncExternalStore(subscribe, snapshot, snapshot);
  return useMemo(
    () => ({
      constants: value.constants,
      content: value.content,
      day: value.day,
      state: value.state,
      record: value.record,
      records: value.records,
      start: startAttempt,
      act: actOn,
      hold: holdAttemptReport,
      tryAgain,
      dismiss: dismissResult,
    }),
    [value],
  );
}

/** Tests: forget every in-memory read; storage is left exactly as the test set it. */
export function __resetAssaultForTests(): void {
  core = null;
  listeners.clear();
  if (pageListening && typeof window !== "undefined") window.removeEventListener("storage", onStorage);
  pageListening = false;
  pageWasOpen = false;
}
