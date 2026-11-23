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
  assaultFold,
  assaultRecord,
  assaultReduce,
  catalogVersion,
} from "../wasm/index.ts";
import { isRecord, isStringArray } from "../wire/json.ts";
import {
  ASSAULT_ATTEMPT_KEY,
  ASSAULT_ATTEMPT_STEP,
  ASSAULT_DATE_FORMAT,
  ASSAULT_FIRST_ATTEMPT,
  ASSAULT_FIRST_LEVEL,
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

export function assaultToday(now: Date = new Date()): string {
  return now.toISOString().slice(0, ASSAULT_DATE_FORMAT.length);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
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

function parseRecord(raw: unknown): AssaultRecord | null {
  if (!isRecord(raw)) return null;
  const { date, best, checkpoints, attempts, repelledOn } = raw;
  if (typeof date !== "string" || !isCount(best) || !Array.isArray(checkpoints) || !checkpoints.every(isPositiveCount)) return null;
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

function parseFile(raw: unknown): AssaultFile | null {
  if (!isRecord(raw) || typeof raw.version !== "string" || !Array.isArray(raw.actions)) return null;
  const setup = parseSetup(raw.setup);
  const record = parseRecord(raw.record);
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

function parseAttempt(raw: unknown): AssaultFile | null {
  if (!isRecord(raw) || raw.v !== ASSAULT_STORAGE_VERSION) return null;
  return parseFile(raw.file);
}

function readAttempt(): AssaultFile | null {
  const stored = readJson(ASSAULT_ATTEMPT_KEY);
  if (!stored.present) return null;
  const attempt = parseAttempt(stored.value);
  if (attempt === null) removeStored(ASSAULT_ATTEMPT_KEY);
  return attempt;
}

function parseRecords(raw: unknown): Record<string, AssaultRecord> | null {
  if (!isRecord(raw) || raw.v !== ASSAULT_STORAGE_VERSION || !isRecord(raw.days)) return null;
  const days: Record<string, AssaultRecord> = {};
  for (const [date, value] of Object.entries(raw.days)) {
    const record = parseRecord(value);
    if (record === null || record.date !== date) return null;
    days[date] = record;
  }
  return days;
}

function readRecords(): Record<string, AssaultRecord> {
  const stored = readJson(ASSAULT_RECORDS_KEY);
  if (!stored.present) return {};
  const records = parseRecords(stored.value);
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
  const storedRecords = readRecords();
  let records = trimRecords(storedRecords, constants);
  if (Object.keys(records).length !== Object.keys(storedRecords).length) persistRecords(records);
  const file = readAttempt();
  if (file !== null) {
    const folded = foldAttempt(file, currentContent);
    if (folded !== null) {
      const before = records[file.setup.date] ?? file.record;
      const record = assaultRecord(before, folded.state);
      records = trimRecords({ ...records, [record.date]: record }, constants);
      persistRecords(records);
      if (terminal(folded.state)) removeStored(ASSAULT_ATTEMPT_KEY);
      return {
        constants,
        content: folded.content,
        day: folded.day,
        state: folded.state,
        file,
        records,
        record,
      };
    }
    removeStored(ASSAULT_ATTEMPT_KEY);
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

function current(): Core {
  core ??= loadCore();
  return core;
}

function commit(next: Core): void {
  core = next;
  for (const listener of [...listeners]) listener();
}

function onStorage(event: StorageEvent): void {
  if (event.key !== null && event.key !== ASSAULT_ATTEMPT_KEY && event.key !== ASSAULT_RECORDS_KEY) return;
  commit(loadCore());
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (!pageListening && typeof window !== "undefined") {
    window.addEventListener("storage", onStorage);
    pageListening = true;
  }
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && pageListening) {
      window.removeEventListener("storage", onStorage);
      pageListening = false;
    }
  };
}

function startAttempt(start: AssaultStart): string | null {
  const from = current();
  if (from.state !== null && !terminal(from.state)) return t("Finish or abandon the attempt in progress first.");
  try {
    const day = from.day.date === start.date ? from.day : assaultDay(start.date, from.content);
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
    persistRecords(records);
    commit({ ...from, day, state: created.state, file, records, record: nextRecord });
    return null;
  } catch (cause) {
    return messageOf(cause);
  }
}

function actOn(action: AssaultAction): string | null {
  const from = current();
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
    commit({ ...from, state: reduced.state, file, records, record });
    return null;
  } catch (cause) {
    return messageOf(cause);
  }
}

function dismissResult(): void {
  const from = current();
  if (from.state !== null && !terminal(from.state)) return;
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
}
