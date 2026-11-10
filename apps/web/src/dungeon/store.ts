// The Dungeon run's store (DR14 part D, #2705): the player's progress, the run in progress and the
// finished runs, kept on this device (DR1: the Sandbox is offline and its stats stay on the device),
// as `tutorial/progress.ts` keeps its own: one snapshot, `useSyncExternalStore`, unavailable or
// malformed storage meaning nothing stored.
//
// The run's rules are `crates/run`'s, which this module reaches only through `wasm/index.ts`
// (CLAUDE.md rule 7): `runCreate` deals a run, `runReduce` applies an action or refuses it, `runFold`
// replays a kept run, `runScore` and `runEmbers` price a finished one. This store holds their results.
//
// A run's time is the time it was played, not the calendar's (R3502000): the session on screen is banked into
// the kept total on each save and whenever the page hides or leaves, so a run picked up the next day is not timed
// by the hours away.
//
// A run is kept as its `RunFile`: its setup, the unlocks it started from and every action taken (B19).
// A reload folds it back. A run that reaches `won` or
// `dead` settles once (B20): a `RunRecord` joins the history, what it extracted joins the unlocks and its
// Embers join the balance, and the kept run is cleared. The finished state stays in memory, for the
// run-over screen (part F), until `abandon()` leaves it; a reload finds the start screen.

import { useMemo, useSyncExternalStore } from "react";

import type { RunAction, RunContent, RunFile, RunSetup, RunState, RunStats, RunUnlocks } from "@jackioh/run";

import { t } from "../i18n/i18n.ts";
import { messageOf } from "../net/errors.ts";
import { getStored, removeStored, setStored } from "../net/storage.ts";
import { clearStoredRunFight } from "../practice/saveStore.ts";
import {
  catalogVersion,
  runCheckSetup,
  runContent,
  runContentAt,
  runCreate,
  runDailySeed,
  runEmbers,
  runFold,
  runLevelCost,
  runReduce,
  runScore,
} from "../wasm/index.ts";
import { isRecord } from "../wire/json.ts";
import {
  DUNGEON_ELAPSED_MIN_MS,
  DUNGEON_ELAPSED_START_MS,
  DUNGEON_FIRST_LEVEL,
  DUNGEON_FIRST_RUN,
  DUNGEON_HISTORY_KEY,
  DUNGEON_LEVEL_STEP,
  DUNGEON_PROGRESS_KEY,
  DUNGEON_ROW_OFFSET,
  DUNGEON_RUN_KEY,
  DUNGEON_RUN_STEP,
  DUNGEON_STORAGE_VERSION,
} from "./config.ts";
import { todayDate } from "./seed.ts";

export type DungeonMode = "sandbox" | "campaign";

export type DungeonProgress = { unlocks: RunUnlocks; embers: number };

/** DR9: what the stats and the leaderboards read of a finished run. */
export type RunRecord = {
  seed: string;
  daily: boolean;
  mode: DungeonMode;
  curses: string[];
  cursePoints: number;
  rowReached: number;
  won: boolean;
  score: number;
  embers: number;
  timeMs: number;
  endedAt: string;
  stats: RunStats;
};

export type Dungeon = {
  content: RunContent;
  progress: DungeonProgress;
  run: RunState | null;
  /** R3502000: the run's time played in ms as of `resumedAt`, for the timer; null with no run. */
  elapsedMs: number | null;
  /** R3502000: `Date.now()` when the session now running began, or null while the run is paused; null with no run. */
  resumedAt: number | null;
  /** The finished run's time in ms, as its record holds it (B20); null while the run is on its way. */
  timeMs: number | null;
  /** R3496000: why the kept run ended without being played, shown once on the start screen. */
  notice: string | null;
  /** The refusal, or null. */
  start(setup: RunSetup): string | null;
  /** The refusal, or null. */
  act(action: RunAction): string | null;
  abandon(): void;
  /** The refusal, or null. */
  buyLevel(): string | null;
};

// What the device keeps

/** The day the daily run was last started on, and how many of that day's daily runs were started. */
type DailyCount = { date: string; started: number };

/**
 * The run on its way, as the device keeps it. R3496000: `catalog` is the catalog version the run
 * started on, null when written before versions were kept; the extractions are what its last fold
 * showed, banked when it cannot be folded at all. R3502000: `elapsedMs` is the time played as of
 * the last save; `startedAt` stays as when the run began and no longer times it.
 */
type KeptRun = {
  file: RunFile;
  startedAt: number;
  daily: boolean;
  catalog: string | null;
  extractedCards: string[];
  extractedArtifacts: string[];
  elapsedMs: number;
};

/** The run the page plays: the kept one folded, with the content it folded on. */
type Live = KeptRun & {
  run: RunState;
  content: RunContent;
  /** The settled run's time, as its record holds it; null while the run is on its way. */
  timeMs: number | null;
  /** R3502000: `Date.now()` when the session now running began; null while paused. Never kept. */
  resumedAt: number | null;
};

type Core = {
  progress: DungeonProgress;
  daily: DailyCount | null;
  history: RunRecord[];
  live: Live | null;
  notice: string | null;
};

/** Nothing yet: the first level, no extracted card or relic, no Embers, and no account's collection to hold to. */
function emptyProgress(): DungeonProgress {
  return { unlocks: { level: DUNGEON_FIRST_LEVEL, cards: [], artifacts: [], owned: null }, embers: 0 };
}

/** A run is finished when it is won or dead, the two phases nothing leaves. */
export function runFinished(run: RunState): boolean {
  return run.phase.phase === "won" || run.phase.phase === "dead";
}

/**
 * R3502000: a run's time played at `now`: the time banked as of `resumedAt` plus the session since, a paused run
 * (`resumedAt` null) holding what it banked. A clock set back inside a session adds nothing, never less.
 */
export function playedMs(elapsedMs: number, resumedAt: number | null, now: number): number {
  return elapsedMs + (resumedAt === null ? 0 : Math.max(DUNGEON_ELAPSED_MIN_MS, now - resumedAt));
}

// Reading what the device holds: untrusted data must parse to the current shape or become nothing.

function isStrings(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function parseUnlocks(raw: unknown): RunUnlocks | null {
  if (!isRecord(raw)) return null;
  const { level, cards, artifacts, owned } = raw;
  if (!isCount(level) || level < DUNGEON_FIRST_LEVEL || !isStrings(cards) || !isStrings(artifacts)) return null;
  if (owned !== null && !isStrings(owned)) return null;
  return { level, cards, artifacts, owned };
}

function parseSetup(raw: unknown): RunSetup | null {
  if (!isRecord(raw)) return null;
  const { seed, starter, artifacts, curses } = raw;
  if (typeof seed !== "string" || !isStrings(starter) || !isStrings(artifacts) || !isStrings(curses)) return null;
  return { seed, starter, artifacts, curses };
}

/** The shape of a file, not its rules: a log `runFold` refuses is dealt with when it is folded. */
function parseFile(raw: unknown): RunFile | null {
  if (!isRecord(raw) || !Array.isArray(raw.log)) return null;
  const setup = parseSetup(raw.setup);
  const unlocks = parseUnlocks(raw.unlocks);
  if (setup === null || unlocks === null) return null;
  if (!raw.log.every((action) => isRecord(action) && typeof action.type === "string")) return null;
  return { setup, unlocks, log: raw.log as RunAction[] };
}

function parseKept(raw: unknown): KeptRun | null {
  if (!isRecord(raw) || raw.v !== DUNGEON_STORAGE_VERSION) return null;
  const file = parseFile(raw.file);
  const { startedAt, daily, elapsedMs } = raw;
  if (file === null || !isCount(startedAt) || typeof daily !== "boolean") return null;
  // R3496000: a run kept before versions were kept has no version: it folds as today's.
  const catalog = raw.catalog === undefined || raw.catalog === null ? null : raw.catalog;
  if (catalog !== null && typeof catalog !== "string") return null;
  const extractedCards = raw.extractedCards === undefined ? [] : raw.extractedCards;
  const extractedArtifacts = raw.extractedArtifacts === undefined ? [] : raw.extractedArtifacts;
  if (!isStrings(extractedCards) || !isStrings(extractedArtifacts)) return null;
  // R3502000: a run kept before its time played was kept has no `elapsedMs` and starts counting when it loads;
  // one that holds a value nothing wrote is dropped like any other.
  if (elapsedMs !== undefined && !isCount(elapsedMs)) return null;
  return { file, startedAt, daily, catalog, extractedCards, extractedArtifacts, elapsedMs: elapsedMs ?? DUNGEON_ELAPSED_START_MS };
}

function parseDaily(raw: unknown): DailyCount | null {
  if (!isRecord(raw) || typeof raw.date !== "string" || !isCount(raw.started)) return null;
  return { date: raw.date, started: raw.started };
}

function parseProgress(raw: unknown): { progress: DungeonProgress; daily: DailyCount | null } {
  if (!isRecord(raw) || raw.v !== DUNGEON_STORAGE_VERSION) return { progress: emptyProgress(), daily: null };
  const unlocks = parseUnlocks(raw.unlocks);
  if (unlocks === null || !isCount(raw.embers)) return { progress: emptyProgress(), daily: null };
  return { progress: { unlocks, embers: raw.embers }, daily: parseDaily(raw.daily) };
}

function parseStats(raw: unknown): RunStats | null {
  if (!isRecord(raw)) return null;
  return Object.values(raw).every((value) => typeof value === "number" && Number.isFinite(value)) ? (raw as RunStats) : null;
}

function parseRecord(raw: unknown): RunRecord | null {
  if (!isRecord(raw)) return null;
  const { seed, daily, mode, curses, cursePoints, rowReached, won, score, embers, timeMs, endedAt, stats } = raw;
  const parsedStats = parseStats(stats);
  if (typeof seed !== "string" || typeof daily !== "boolean" || (mode !== "sandbox" && mode !== "campaign")) return null;
  if (!isStrings(curses) || !isCount(rowReached) || typeof won !== "boolean" || !isCount(embers) || !isCount(timeMs)) return null;
  if (typeof cursePoints !== "number" || !Number.isFinite(cursePoints) || typeof score !== "number" || !Number.isFinite(score)) return null;
  if (typeof endedAt !== "string" || parsedStats === null) return null;
  return { seed, daily, mode, curses, cursePoints, rowReached, won, score, embers, timeMs, endedAt, stats: parsedStats };
}

function parseHistory(raw: unknown): RunRecord[] {
  if (!isRecord(raw) || raw.v !== DUNGEON_STORAGE_VERSION || !Array.isArray(raw.runs)) return [];
  return raw.runs.flatMap((run) => {
    const record = parseRecord(run);
    return record === null ? [] : [record];
  });
}

function readJson(key: string): unknown {
  const raw = getStored(key);
  if (raw === null) return null;
  try {
    return JSON.parse(raw) as unknown;
  } catch {
    return null;
  }
}

function writeJson(key: string, value: unknown): void {
  try {
    setStored(key, JSON.stringify(value));
  } catch {
    // Keeping the run is a convenience: a value that cannot be written as JSON must not stop the run being played
    // (`setStored` already shrugs off a full or blocked storage).
  }
}

// The content a run is dealt from: the compiled-in catalog's, built once

let compiled: RunContent | null = null;

/** The compiled-in content, built once; the stats page names a past run's curses from it. */
export function contentOf(): RunContent {
  compiled ??= runContent();
  return compiled;
}

/**
 * R3496000: the content a kept run folds on: today's when it started on no kept version, else its
 * version's, or null when this build no longer carries it. A binding that throws lets the throw
 * out: `foldKept` drops it silently, since a transient failure must never mint a history record.
 */
function contentForVersion(version: string | null): RunContent | null {
  if (version === null || version === catalogVersion()) return contentOf();
  return runContentAt(version);
}

/**
 * R3496000: B19 the kept run folded back on the content of the catalog version it started on.
 * `"gone"` when no content folds it: its version is no longer carried, or its log is refused. Null
 * when it cannot be read at all: a transient failure must never mint a history record.
 */
function foldKept(kept: KeptRun): Live | "gone" | null {
  try {
    const content = contentForVersion(kept.catalog);
    if (content === null) return "gone";
    const folded = runFold(kept.file, content);
    return "error" in folded ? "gone" : { ...kept, run: folded.state, content, timeMs: null, resumedAt: sessionStart() };
  } catch {
    return null;
  }
}

/** R3496000: a run that never settled has no rows, no score and no stats. */
const EMPTY_RUN_STATS: RunStats = {
  nodesVisited: 0,
  fightsWon: 0,
  elitesWon: 0,
  ghostsWon: 0,
  miniBossesWon: 0,
  bossesWon: 0,
  damageDealt: 0,
  healingDone: 0,
  cardsDrawn: 0,
  turns: 0,
  damageTaken: 0,
  healed: 0,
  coinsEarned: 0,
  coinsSpent: 0,
  ticketsEarned: 0,
  ticketsSpent: 0,
  cardsKept: 0,
  cardsBought: 0,
  cardsRemoved: 0,
  cardsMadeRadiant: 0,
  cardsBuffed: 0,
  packsSkipped: 0,
  artifactsTaken: 0,
  artifactsLeveled: 0,
  blessingsTaken: 0,
  eventsSeen: 0,
  chestsOpened: 0,
};

/**
 * R3496000: what a run that cannot be folded at all settles: lost with no row, no score and no
 * Embers, banking what it had extracted of what the current content still knows.
 */
function settleUnfoldable(base: Core, kept: KeptRun, now: number): Core {
  const content = contentOf();
  const pool = new Set(content.pool.map((card) => card.id));
  const known = new Set(content.passives.map((passive) => passive.id));
  const { unlocks } = base.progress;
  const record: RunRecord = {
    seed: kept.file.setup.seed,
    daily: kept.daily,
    mode: "sandbox",
    curses: kept.file.setup.curses,
    cursePoints: cursePointsOf(kept.file.setup.curses, content),
    rowReached: 0,
    won: false,
    score: 0,
    embers: 0,
    // R3502000: the time played the run was kept with, not the calendar's gap since `startedAt`.
    timeMs: playedMs(kept.elapsedMs, null, now),
    endedAt: new Date(now).toISOString(),
    stats: EMPTY_RUN_STATS,
  };
  return {
    ...base,
    progress: {
      unlocks: {
        ...unlocks,
        cards: union(
          unlocks.cards,
          kept.extractedCards.filter((id) => pool.has(id)),
        ),
        artifacts: union(
          unlocks.artifacts,
          kept.extractedArtifacts.filter((id) => known.has(id)),
        ),
      },
      embers: base.progress.embers,
    },
    history: [...base.history, record],
    live: null,
  };
}

function loadCore(): Core {
  const { progress, daily } = parseProgress(readJson(DUNGEON_PROGRESS_KEY));
  const history = parseHistory(readJson(DUNGEON_HISTORY_KEY));
  const stored = readJson(DUNGEON_RUN_KEY);
  let live: Live | null = null;
  if (stored !== null) {
    const kept = parseKept(stored);
    if (kept === null) {
      // A run that cannot be read is dropped alone: the progress and the history stay.
      removeStored(DUNGEON_RUN_KEY);
    } else {
      const folded = foldKept(kept);
      if (folded === null) {
        // A run no binding can read is dropped alone, as before: never settled.
        removeStored(DUNGEON_RUN_KEY);
      } else if (folded === "gone") {
        // R3496000: a run no content folds ends lost but keeps what it had extracted.
        const settled = settleUnfoldable({ progress, daily, history, live: null, notice: null }, kept, Date.now());
        persistProgress(settled);
        persistHistory(settled);
        removeStored(DUNGEON_RUN_KEY);
        // The key is gone, so the notice shows once.
        return { ...settled, notice: t("The patch ended your last run as lost. What you had extracted is kept.") };
      } else {
        live = folded;
      }
    }
  }
  return { progress, daily, history, live, notice: null };
}

function keptOf(live: Live): KeptRun {
  return {
    file: live.file,
    startedAt: live.startedAt,
    daily: live.daily,
    catalog: live.catalog,
    extractedCards: [...live.run.extractedCards],
    extractedArtifacts: [...live.run.extractedArtifacts],
  elapsedMs: live.elapsedMs,
  };
}

function persistProgress(core: Core): void {
  writeJson(DUNGEON_PROGRESS_KEY, {
    v: DUNGEON_STORAGE_VERSION,
    unlocks: core.progress.unlocks,
    embers: core.progress.embers,
    daily: core.daily,
  });
}

function persistHistory(core: Core): void {
  writeJson(DUNGEON_HISTORY_KEY, { v: DUNGEON_STORAGE_VERSION, runs: core.history });
}

function persistRun(live: Live): void {
  writeJson(DUNGEON_RUN_KEY, { v: DUNGEON_STORAGE_VERSION, ...keptOf(live) });
}

// The store

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

// The run's time played (R3502000): the clock runs while a Dungeon page is open and the browser page is shown.

/** Whether a run's clock runs now: a Dungeon page is open and the browser page is not hidden. */
function onScreen(): boolean {
  return listeners.size > 0 && (typeof document === "undefined" || document.visibilityState !== "hidden");
}

/** The `resumedAt` of a run that was just loaded or begun: now when it is on screen, null while it waits. */
function sessionStart(): number | null {
  return onScreen() ? Date.now() : null;
}

/** `live` with its running session banked into `elapsedMs`, the next session beginning at `now`; a paused run is unchanged. */
function banked(live: Live, now: number): Live {
  return {
    ...live,
    elapsedMs: playedMs(live.elapsedMs, live.resumedAt, now),
    resumedAt: live.resumedAt === null ? null : now,
  };
}

/** Pause the run's clock, keeping what it banked (the page hid, closed or left), or set it running again. */
function setSession(running: boolean): void {
  const live = core?.live ?? null;
  // A settled run was removed from the device and is never kept again (B20).
  if (core === null || live === null || runFinished(live.run)) return;
  if (running === (live.resumedAt !== null)) return;
  const now = Date.now();
  let next: Live;
  if (running) {
    next = { ...live, resumedAt: now };
  } else {
    next = { ...banked(live, now), resumedAt: null };
    persistRun(next);
  }
  commit({ ...core, live: next });
}

function onStorage(event: StorageEvent): void {
  const keys = [DUNGEON_PROGRESS_KEY, DUNGEON_RUN_KEY, DUNGEON_HISTORY_KEY];
  if (typeof event.key === "string" && !keys.includes(event.key)) return;
  // Another tab changed what is kept: read it all again, but for a finished run this tab still shows.
  const kept = core?.live ?? null;
  const next = loadCore();
  commit(next.live === null && kept !== null && runFinished(kept.run) ? { ...next, live: kept } : next);
}

/** R3502000: the browser page hid or showed again: the run's clock stops or runs on. */
function onVisibility(): void {
  setSession(onScreen());
}

/** R3502000: the page is closing or going to the back-forward cache: bank the session now, as nothing may follow. */
function onPageHide(): void {
  setSession(false);
}

function listenToPage(): void {
  window.addEventListener("storage", onStorage);
  document.addEventListener("visibilitychange", onVisibility);
  window.addEventListener("pagehide", onPageHide);
  // A page restored from the back-forward cache shows again with no `visibilitychange`.
  window.addEventListener("pageshow", onVisibility);
}

function stopListeningToPage(): void {
  window.removeEventListener("storage", onStorage);
  document.removeEventListener("visibilitychange", onVisibility);
  window.removeEventListener("pagehide", onPageHide);
  window.removeEventListener("pageshow", onVisibility);
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (!pageListening && typeof window !== "undefined") {
    listenToPage();
    pageListening = true;
  }
  // The first render loads a run with no one listening, so its clock starts here.
  setSession(onScreen());
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && pageListening) {
      stopListeningToPage();
      pageListening = false;
    }
    // Leaving the Dungeon page stops the clock: the run waits until a page shows it again.
    setSession(onScreen());
  };
}

/** Tests: forget everything read, so the next read finds the storage as the test left it. */
export function __resetDungeonForTests(): void {
  core = null;
  compiled = null;
  listeners.clear();
  if (pageListening) stopListeningToPage();
  pageListening = false;
}

/** B20: every finished run on this device, oldest first. */
export function readRunHistory(): RunRecord[] {
  return [...(core?.history ?? parseHistory(readJson(DUNGEON_HISTORY_KEY)))];
}

/** B15: the number the day's next daily run takes: the day's first is 1, each started one moves it on. */
export function dailyRunNumber(date: string): number {
  const { daily } = current();
  return daily !== null && daily.date === date ? DUNGEON_FIRST_RUN + daily.started : DUNGEON_FIRST_RUN;
}

function union(held: readonly string[], more: readonly string[]): string[] {
  return [...new Set([...held, ...more])];
}

/** B20: the curses' points, summed from the content's own: a finished run's record's and the start screen's. */
export function cursePointsOf(curses: readonly string[], content: RunContent): number {
  return curses.reduce((sum, id) => sum + (content.curses.find((curse) => curse.id === id)?.points ?? 0), 0);
}

/** B20: what a finished run adds: its record, its extractions and its Embers, once. */
function settle(from: Core, live: Live, now: number): Core {
  const { run, content } = live;
  const score = runScore(run, content);
  const embers = runEmbers(score);
  const record: RunRecord = {
    seed: run.seed,
    daily: live.daily,
    mode: "sandbox",
    curses: run.curses,
    cursePoints: cursePointsOf(run.curses, content),
    rowReached: run.row === null ? 0 : run.row + DUNGEON_ROW_OFFSET,
    won: run.phase.phase === "won",
    score,
    embers,
    timeMs: playedMs(live.elapsedMs, live.resumedAt, now),
    endedAt: new Date(now).toISOString(),
    stats: run.stats,
  };
  const { unlocks } = from.progress;
  return {
    ...from,
    progress: {
      unlocks: {
        ...unlocks,
        cards: union(unlocks.cards, run.extractedCards),
        artifacts: union(unlocks.artifacts, run.extractedArtifacts),
      },
      embers: from.progress.embers + embers,
    },
    history: [...from.history, record],
    live: { ...live, timeMs: record.timeMs },
  };
}

function startRun(setup: RunSetup): string | null {
  const from = current();
  if (from.live !== null && !runFinished(from.live.run)) return t("Finish or abandon the run in progress first.");
  try {
    const content = contentOf();
    const { unlocks } = from.progress;
    const refusal = runCheckSetup(setup, unlocks, content);
    if (refusal !== null) return refusal;
    const created = runCreate(setup, unlocks, content);
    if ("error" in created) return created.error;
    const date = todayDate();
    // B15: only the day's own seed moves the day's number on, so a random or typed run never skips one.
    const daily = setup.seed === runDailySeed(date, dailyRunNumber(date));
    const live: Live = {
      file: { setup, unlocks, log: [] },
      startedAt: Date.now(),
      daily,
      catalog: catalogVersion(),
      extractedCards: [],
      extractedArtifacts: [],
      elapsedMs: DUNGEON_ELAPSED_START_MS,
      run: created.state,
      content,
      timeMs: null,
      resumedAt: sessionStart(),
    };
    const next: Core = {
      ...from,
      // The patch notice told of an earlier run; a new run ends it.
      notice: null,
      daily: daily
        ? { date, started: (from.daily?.date === date ? from.daily.started : 0) + DUNGEON_RUN_STEP }
        : from.daily,
      live,
    };
    persistRun(live);
    if (daily) persistProgress(next);
    // R3476000: a new run's first fight finds no kept fight.
    clearStoredRunFight();
    commit(next);
    return null;
  } catch (cause) {
    return messageOf(cause);
  }
}

/**
 * The run after `action`, kept on the device and settled when it finished (B20), or the run's own
 * refusal. The state in memory is the caller's to commit.
 */
function step(from: Core, live: Live, action: RunAction): Core | string {
  let reduced: ReturnType<typeof runReduce>;
  try {
    reduced = runReduce(live.run, live.content, action);
  } catch (cause) {
    return messageOf(cause);
  }
  if ("error" in reduced) return reduced.error;
  const next: Live = { ...live, run: reduced.state, file: { ...live.file, log: [...live.file.log, action] } };
  if (!runFinished(next.run)) {
    // R3502000: every save banks the session so far.
    const kept = banked(next, Date.now());
    persistRun(kept);
    return { ...from, live: kept };
  }
  try {
    const settled = settle(from, next, Date.now());
    persistProgress(settled);
    persistHistory(settled);
    removeStored(DUNGEON_RUN_KEY);
    return settled;
  } catch (cause) {
    return messageOf(cause);
  }
}

function actOn(action: RunAction): string | null {
  const from = current();
  if (from.live === null) return t("No run is in progress.");
  // A finished run has paid already: nothing it is sent may pay it again.
  if (runFinished(from.live.run)) return t("The run is over.");
  const next = step(from, from.live, action);
  if (typeof next === "string") return next;
  commit(next);
  return null;
}

function abandonRun(): void {
  const from = current();
  const { live } = from;
  if (live === null) return;
  // An unfinished run is given up first, which settles its loss; a finished one has settled already.
  const settled = runFinished(live.run) ? from : step(from, live, { type: "abandon" });
  if (typeof settled !== "string") commit({ ...settled, live: null, notice: null });
  // R3476000: an abandoned run keeps no fight.
  clearStoredRunFight();
}

function buyNextLevel(): string | null {
  const from = current();
  const { unlocks, embers } = from.progress;
  const cost = runLevelCost(unlocks.level);
  if (cost === null) return t("Your starting deck is at its top level.");
  if (embers < cost) return t("You need {cost} Embers to raise your starting deck.", { cost: String(cost) });
  const next: Core = {
    ...from,
    notice: null,
    progress: { unlocks: { ...unlocks, level: unlocks.level + DUNGEON_LEVEL_STEP }, embers: embers - cost },
  };
  persistProgress(next);
  commit(next);
  return null;
}

function snapshot(): Core {
  return current();
}

/**
 * The Dungeon run's state and verbs. Each verb answers the run's own refusal, word for word, or null.
 */
export function useDungeon(): Dungeon {
  const { progress, live, notice } = useSyncExternalStore(subscribe, snapshot, snapshot);
  const content = live === null ? contentOf() : live.content;
  return useMemo(
    () => ({
      content,
      progress,
      run: live === null ? null : live.run,
      elapsedMs: live === null ? null : live.elapsedMs,
      resumedAt: live === null ? null : live.resumedAt,
      timeMs: live === null ? null : live.timeMs,
      notice,
      start: startRun,
      act: actOn,
      abandon: abandonRun,
      buyLevel: buyNextLevel,
    }),
    [content, progress, live, notice],
  );
}
