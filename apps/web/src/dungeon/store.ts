// The Dungeon Run's store (DR14 part D, #2705): the player's progress, the run in progress and the
// finished runs, kept on this device (DR1: the Sandbox is offline and its stats stay on the device),
// as `tutorial/progress.ts` keeps its own: one snapshot, `useSyncExternalStore`, unavailable or
// malformed storage meaning nothing stored.
//
// The run's rules are `crates/run`'s, which this module reaches only through `wasm/index.ts`
// (CLAUDE.md rule 7): `runCreate` deals a run, `runReduce` applies an action or refuses it, `runFold`
// replays a kept run, `runScore` and `runEmbers` price a finished one. This store holds their results.
//
// A run is kept as its `RunFile`: its setup, the unlocks it started from and every action taken (B19).
// A reload folds it back; the custom cards its pool held when it started are kept beside it, so a later
// edit of the creator's cards never changes a run that is under way (B21). A run that reaches `won` or
// `dead` settles once (B20): a `RunRecord` joins the history, what it extracted joins the unlocks and its
// Embers join the balance, and the kept run is cleared. The finished state stays in memory, for the
// run-over screen (part F), until `abandon()` leaves it; a reload finds the start screen.

import { useCallback, useMemo, useSyncExternalStore } from "react";

import type { CustomCard } from "@jackioh/shared";
import type { RunAction, RunContent, RunFile, RunSetup, RunState, RunStats, RunUnlocks } from "@jackioh/run";

import { t } from "../i18n/i18n.ts";
import { useAccount } from "../net/gate.ts";
import { localStorageOrNull } from "../net/storage.ts";
import {
  runCheckSetup,
  runContent,
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
import { useSandboxCustomCards } from "./customCards.ts";
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
  /** `Date.now()` when the run started, for the timer. */
  startedAt: number | null;
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

/** The run on its way, as the device keeps it. */
type KeptRun = { file: RunFile; customCards: CustomCard[]; startedAt: number; daily: boolean };

/** The run the page plays: the kept one folded, with the content it folded on. */
type Live = KeptRun & { run: RunState; content: RunContent };

type Core = {
  progress: DungeonProgress;
  daily: DailyCount | null;
  history: RunRecord[];
  live: Live | null;
};

/** Nothing yet: the first level, no extracted card or relic, no Embers, and no account's collection to hold to. */
function emptyProgress(): DungeonProgress {
  return { unlocks: { level: DUNGEON_FIRST_LEVEL, cards: [], artifacts: [], owned: null }, embers: 0 };
}

/** A run is finished when it is won or dead, the two phases nothing leaves. */
export function runFinished(run: RunState): boolean {
  return run.phase.phase === "won" || run.phase.phase === "dead";
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
  const { customCards, startedAt, daily } = raw;
  if (file === null || !Array.isArray(customCards) || !customCards.every(isRecord)) return null;
  if (!isCount(startedAt) || typeof daily !== "boolean") return null;
  return { file, customCards: customCards as unknown as CustomCard[], startedAt, daily };
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
  try {
    const raw = localStorageOrNull()?.getItem(key) ?? null;
    return raw === null ? null : (JSON.parse(raw) as unknown);
  } catch {
    return null;
  }
}

function writeJson(key: string, value: unknown): void {
  try {
    localStorageOrNull()?.setItem(key, JSON.stringify(value));
  } catch {
    // Keeping the run is a convenience: a full or blocked storage must not stop the run being played.
  }
}

function removeKey(key: string): void {
  try {
    localStorageOrNull()?.removeItem(key);
  } catch {
    // As above.
  }
}

// The content a run is dealt from: built once per list of custom cards

const contents = new WeakMap<readonly CustomCard[], RunContent>();

/** B21: `runContent` of these custom cards, built once for the same list. */
function contentFor(customCards: CustomCard[]): RunContent {
  let content = contents.get(customCards);
  if (content === undefined) {
    content = runContent(customCards);
    contents.set(customCards, content);
  }
  return content;
}

/** B19: the kept run folded back on the content it started on, or null when it will not fold. */
function foldKept(kept: KeptRun): Live | null {
  try {
    const content = runContent(kept.customCards);
    const folded = runFold(kept.file, content);
    return "error" in folded ? null : { ...kept, run: folded.state, content };
  } catch {
    return null;
  }
}

function loadCore(): Core {
  const { progress, daily } = parseProgress(readJson(DUNGEON_PROGRESS_KEY));
  const history = parseHistory(readJson(DUNGEON_HISTORY_KEY));
  const stored = readJson(DUNGEON_RUN_KEY);
  let live: Live | null = null;
  if (stored !== null) {
    const kept = parseKept(stored);
    live = kept === null ? null : foldKept(kept);
    // A run that cannot be read or folded is dropped alone: the progress and the history stay.
    if (live === null) removeKey(DUNGEON_RUN_KEY);
  }
  return { progress, daily, history, live };
}

function keptOf(live: Live): KeptRun {
  return { file: live.file, customCards: live.customCards, startedAt: live.startedAt, daily: live.daily };
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
let storageListening = false;

function current(): Core {
  core ??= loadCore();
  return core;
}

function commit(next: Core): void {
  core = next;
  for (const listener of [...listeners]) listener();
}

function onStorage(event: StorageEvent): void {
  const keys = [DUNGEON_PROGRESS_KEY, DUNGEON_RUN_KEY, DUNGEON_HISTORY_KEY];
  if (typeof event.key === "string" && !keys.includes(event.key)) return;
  // Another tab changed what is kept: read it all again, but for a finished run this tab still shows.
  const kept = core?.live ?? null;
  const next = loadCore();
  commit(next.live === null && kept !== null && runFinished(kept.run) ? { ...next, live: kept } : next);
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (!storageListening && typeof window !== "undefined") {
    window.addEventListener("storage", onStorage);
    storageListening = true;
  }
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && storageListening) {
      window.removeEventListener("storage", onStorage);
      storageListening = false;
    }
  };
}

/** Tests: forget everything read, so the next read finds the storage as the test left it. */
export function __resetDungeonForTests(): void {
  core = null;
  listeners.clear();
  if (storageListening) window.removeEventListener("storage", onStorage);
  storageListening = false;
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

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function union(held: readonly string[], more: readonly string[]): string[] {
  return [...new Set([...held, ...more])];
}

/** B20: the curses' points, summed from the content's own. */
function cursePointsOf(run: RunState, content: RunContent): number {
  return run.curses.reduce((sum, id) => sum + (content.curses.find((curse) => curse.id === id)?.points ?? 0), 0);
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
    cursePoints: cursePointsOf(run, content),
    rowReached: run.row === null ? 0 : run.row + DUNGEON_ROW_OFFSET,
    won: run.phase.phase === "won",
    score,
    embers,
    timeMs: Math.max(DUNGEON_ELAPSED_MIN_MS, now - live.startedAt),
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
    live,
  };
}

function startRun(setup: RunSetup, customCards: CustomCard[]): string | null {
  const from = current();
  if (from.live !== null && !runFinished(from.live.run)) return t("Finish or abandon the run in progress first.");
  try {
    const content = contentFor(customCards);
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
      customCards,
      startedAt: Date.now(),
      daily,
      run: created.state,
      content,
    };
    const next: Core = {
      ...from,
      daily: daily
        ? { date, started: (from.daily?.date === date ? from.daily.started : 0) + DUNGEON_RUN_STEP }
        : from.daily,
      live,
    };
    persistRun(live);
    if (daily) persistProgress(next);
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
    persistRun(next);
    return { ...from, live: next };
  }
  try {
    const settled = settle(from, next, Date.now());
    persistProgress(settled);
    persistHistory(settled);
    removeKey(DUNGEON_RUN_KEY);
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
  if (typeof settled !== "string") commit({ ...settled, live: null });
}

function buyNextLevel(): string | null {
  const from = current();
  const { unlocks, embers } = from.progress;
  const cost = runLevelCost(unlocks.level);
  if (cost === null) return t("Your starting deck is at its top level.");
  if (embers < cost) return t("You need {cost} Embers to raise your starting deck.", { cost: String(cost) });
  const next: Core = {
    ...from,
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
 * The Dungeon Run's state and verbs. In the Sandbox a run's pool takes the custom cards an active
 * account has saved (B21); a run under way keeps the cards it started with. Each verb answers the
 * run's own refusal, word for word, or null.
 */
export function useDungeon(): Dungeon {
  const account = useAccount();
  const customCards = useSandboxCustomCards(account);
  const { progress, live } = useSyncExternalStore(subscribe, snapshot, snapshot);
  const fresh = useMemo(() => contentFor(customCards), [customCards]);
  const start = useCallback((setup: RunSetup) => startRun(setup, customCards), [customCards]);
  const content = live === null ? fresh : live.content;
  return useMemo(
    () => ({
      content,
      progress,
      run: live === null ? null : live.run,
      startedAt: live === null ? null : live.startedAt,
      start,
      act: actOn,
      abandon: abandonRun,
      buyLevel: buyNextLevel,
    }),
    [content, progress, live, start],
  );
}
