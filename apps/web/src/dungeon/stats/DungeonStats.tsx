// The dungeon run's device leaderboards (DR9, DR14 part G, #2708): the `/dungeon/stats`
// screen ranks every finished run the device holds (part D's `readRunHistory`) on four boards —
// the deepest row, the most curse points, the top score and the fastest win — behind a
// Sandbox/Campaign and daily-only filter, and each entry opens that run's stats in a dialog.
//
// CLAUDE.md rule 7: this reads completed `RunRecord`s only, never a live run, the engine or
// hidden game information.

import { useEffect, useRef, useState, type ReactElement } from "react";

import type { RunStats } from "@jackioh/run";

import "../../auth/tavern.css";
import { EmptyState } from "../../empty/EmptyState.tsx";
import { Dialog } from "../../game/Dialog.tsx";
import { MS_PER_SECOND } from "../../game/clockConstants.ts";
import { formatDateTime, formatNumber, t, useLocale } from "../../i18n/i18n.ts";
import { paths } from "../../net/navigate.ts";
import { BackLink } from "../../routes/nav.tsx";
import { RUN_STAT_LABELS } from "../map/RunOverScreen.tsx";
import { readRunHistory, type DungeonMode, type RunRecord } from "../store.ts";
import { dungeonStatsTestid, type DungeonLeaderboard } from "./testids.ts";
import "./dungeon-stats.css";

/** Unit conversions for a run's elapsed time, shown whole-second as `M:SS`, or `H:MM:SS` past the hour. */
const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;
/** The seconds, and the minutes past the hour, are always written with this many digits ("1:02:03"). */
const CLOCK_DIGITS = 2;

/** The mode filter's options, in the order the picker offers them. */
const MODES: readonly DungeonMode[] = ["sandbox", "campaign"];

/** The mode words, kept as data so `t()` reads them at render and a locale change stays live. */
const MODE_WORDS = { sandbox: "Sandbox", campaign: "Campaign" } as const;

/** The details table's rows, in the order the run-over screen lists them (`RUN_STAT_LABELS`). */
const STAT_FIELDS = Object.keys(RUN_STAT_LABELS) as (keyof RunStats)[];

/** The four boards, in the order the screen shows them. */
const BOARDS: readonly { key: DungeonLeaderboard; title: string }[] = [
  { key: "deepest", title: "Deepest row" },
  { key: "curses", title: "Most curse points" },
  { key: "score", title: "Top score" },
  { key: "fastest", title: "Fastest win" },
];

/** A matching record with its original history index, for stable ties and distinct duplicate seeds. */
type RankedRun = { record: RunRecord; index: number };

/** The records the filters keep, in device-history order; the store's array is never sorted in place. */
function matchingRuns(history: readonly RunRecord[], mode: DungeonMode, dailyOnly: boolean): RankedRun[] {
  return history
    .map((record, index) => ({ record, index }))
    .filter(({ record }) => record.mode === mode && (!dailyOnly || record.daily));
}

/** A board's numeric value: the ranked metric, exact `timeMs` for the fastest board. */
function metricOf(board: DungeonLeaderboard, record: RunRecord): number {
  switch (board) {
    case "deepest":
      return record.rowReached;
    case "curses":
      return record.cursePoints;
    case "score":
      return record.score;
    case "fastest":
      return record.timeMs;
  }
}

/**
 * B32: every matching run is ranked — `rowReached`, `cursePoints` and `score` descending, and
 * only won runs by `timeMs` ascending — with the original history index breaking numeric ties so
 * equal records never jump.
 */
function boardRuns(board: DungeonLeaderboard, matching: readonly RankedRun[]): RankedRun[] {
  const pool = board === "fastest" ? matching.filter(({ record }) => record.won) : [...matching];
  const direction = board === "fastest" ? 1 : -1;
  return pool.sort((a, b) => direction * (metricOf(board, a.record) - metricOf(board, b.record)) || a.index - b.index);
}

/** A run's elapsed time, whole seconds: `M:SS` below the hour, `H:MM:SS` past it. */
function formatElapsed(timeMs: number): string {
  const totalSeconds = Math.floor(timeMs / MS_PER_SECOND);
  const seconds = totalSeconds % SECONDS_PER_MINUTE;
  const totalMinutes = Math.floor(totalSeconds / SECONDS_PER_MINUTE);
  const minutes = totalMinutes % MINUTES_PER_HOUR;
  const hours = Math.floor(totalMinutes / MINUTES_PER_HOUR);
  const rest = `${hours > 0 ? String(minutes).padStart(CLOCK_DIGITS, "0") : String(minutes)}:${String(seconds).padStart(CLOCK_DIGITS, "0")}`;
  return hours > 0 ? `${String(hours)}:${rest}` : rest;
}

/** A board entry's metric: the clock for the fastest board, a localized number elsewhere. */
function formatMetric(board: DungeonLeaderboard, record: RunRecord): string {
  return board === "fastest" ? formatElapsed(record.timeMs) : formatNumber(metricOf(board, record));
}

/** One run's entry on one board: its rank and metric, seed, mode, date and daily status. */
function BoardEntry({
  board,
  rank,
  entry,
  onOpen,
}: {
  board: DungeonLeaderboard;
  rank: number;
  entry: RankedRun;
  onOpen: (index: number, opener: HTMLElement) => void;
}): ReactElement {
  const { record, index } = entry;
  return (
    <li>
      <button
        type="button"
        className="dungeon-stats__entry"
        data-testid={dungeonStatsTestid.entry(board, index)}
        onClick={(event) => {
          onOpen(index, event.currentTarget);
        }}
      >
        <span className="dungeon-stats__rank">{formatNumber(rank)}</span>
        <span className="dungeon-stats__metric">{formatMetric(board, record)}</span>
        <span className="dungeon-stats__seed">{record.seed}</span>
        <span className="dungeon-stats__mode">{t(MODE_WORDS[record.mode])}</span>
        <span className="dungeon-stats__date">{formatDateTime(new Date(record.endedAt), { dateStyle: "medium" })}</span>
        <span className="dungeon-stats__daily">{record.daily ? t("Daily run") : t("Not a daily run")}</span>
      </button>
    </li>
  );
}

/** One board: its heading and every matching run in rank order, or the fastest board's no-wins line. */
function Board({
  board,
  title,
  runs,
  onOpen,
}: {
  board: DungeonLeaderboard;
  title: string;
  runs: readonly RankedRun[];
  onOpen: (index: number, opener: HTMLElement) => void;
}): ReactElement {
  // Only the fastest board can be empty here: every other board ranks each matching run, and the
  // screen renders no board when nothing matches.
  return (
    <section className="dungeon-stats__board" data-testid={dungeonStatsTestid.board(board)} aria-label={t(title)}>
      <h2 className="dungeon-stats__board-title">{t(title)}</h2>
      {runs.length === 0 ? (
        <p className="dungeon-stats__empty" data-testid={dungeonStatsTestid.noWins} role="status">
          {t("No winning runs match these filters.")}
        </p>
      ) : (
        <ol className="dungeon-stats__entries">
          {runs.map((entry, at) => (
            <BoardEntry key={entry.index} board={board} rank={at + 1} entry={entry} onOpen={onOpen} />
          ))}
        </ol>
      )}
    </section>
  );
}

/**
 * One run's complete stats, as a modal like the client's others: drawn through Dialog.tsx (R3221000),
 * whose ✕ takes the focus as on the statistics drill-down's, and whose keys are the app's modal contract.
 */
function RunDetails({ record, onClose }: { record: RunRecord; onClose: () => void }): ReactElement {
  return (
    <Dialog
      role="dialog"
      testid={dungeonStatsTestid.dialog}
      closeTestid={dungeonStatsTestid.dialogClose}
      className="dungeon-stats__dialog"
      title={t("Run {seed}", { seed: record.seed })}
      onClose={onClose}
      actions={[]}
    >
      <dl className="dungeon-stats__meta" data-testid={dungeonStatsTestid.detailMeta}>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Seed")}</dt>
          <dd>{record.seed}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Mode")}</dt>
          <dd>{t(MODE_WORDS[record.mode])}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Date")}</dt>
          <dd>{formatDateTime(new Date(record.endedAt), { dateStyle: "medium" })}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Outcome")}</dt>
          <dd>{record.won ? t("Won") : t("Lost")}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Daily")}</dt>
          <dd>{record.daily ? t("Daily run") : t("Not a daily run")}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Row reached")}</dt>
          <dd>{formatNumber(record.rowReached)}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Curse points")}</dt>
          <dd>{formatNumber(record.cursePoints)}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Score")}</dt>
          <dd>{formatNumber(record.score)}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Embers")}</dt>
          <dd>{formatNumber(record.embers)}</dd>
        </div>
        <div className="dungeon-stats__meta-row">
          <dt>{t("Time")}</dt>
          <dd data-testid={dungeonStatsTestid.detailTime}>{formatElapsed(record.timeMs)}</dd>
        </div>
      </dl>
      <h3 className="dungeon-stats__subhead">{t("Curses")}</h3>
      <div data-testid={dungeonStatsTestid.detailCurses}>
        {record.curses.length === 0 ? (
          <p className="dungeon-stats__empty">{t("No curses")}</p>
        ) : (
          <ul className="dungeon-stats__curses">
            {record.curses.map((curse: string) => (
              <li key={curse}>{t(curse)}</li>
            ))}
          </ul>
        )}
      </div>
      <h3 className="dungeon-stats__subhead">{t("Run stats")}</h3>
      <dl className="dungeon-stats__table">
        {STAT_FIELDS.map((field) => (
          <div className="dungeon-stats__stat" key={field}>
            <dt>{t(RUN_STAT_LABELS[field])}</dt>
            <dd data-testid={dungeonStatsTestid.stat(field)}>{formatNumber(record.stats[field])}</dd>
          </div>
        ))}
      </dl>
    </Dialog>
  );
}

/** B32: the device's dungeon leaderboards, at `/dungeon/stats`. */
export function DungeonStats(): ReactElement {
  useLocale();
  // The device history, read once and synchronously: no effect, promise or loading state.
  const [history] = useState(readRunHistory);
  const [mode, setMode] = useState<DungeonMode>("sandbox");
  const [dailyOnly, setDailyOnly] = useState(false);
  /** The open run's original history index; the button that opened it, for the focus back. */
  const [openIndex, setOpenIndex] = useState<number | null>(null);
  const opener = useRef<HTMLElement | null>(null);

  function openDetails(index: number, element: HTMLElement): void {
    opener.current = element;
    setOpenIndex(index);
  }
  function closeDetails(): void {
    setOpenIndex(null);
  }
  // The control that opened the dialog has the focus back once it closes, as the drill-down's does.
  // The opener is kept when pressed, not read off the focus, which a click does not move everywhere.
  useEffect(() => {
    if (openIndex !== null) return;
    const element = opener.current;
    opener.current = null;
    if (element !== null && element.isConnected) element.focus({ preventScroll: true });
  }, [openIndex]);

  const matching = matchingRuns(history, mode, dailyOnly);
  const open = openIndex === null ? null : (history[openIndex] ?? null);

  return (
    <div className="app-shell tavern one-screen" data-testid={dungeonStatsTestid.root}>
      <BackLink to={paths.dungeon} title={t("Dungeon Run stats")} />
      <main className="one-screen__body">
        <div className="dungeon-stats">
          {history.length === 0 ? (
            <EmptyState
              text={t("No finished runs on this device yet.")}
              testId={dungeonStatsTestid.noHistory}
              action={{ label: t("Start a dungeon run"), to: paths.dungeon }}
            />
          ) : (
            <>
              <div className="dungeon-stats__filters">
                <label className="dungeon-stats__filter">
                  {t("Mode")}
                  <select
                    className="dungeon-stats__select"
                    data-testid={dungeonStatsTestid.modeFilter}
                    value={mode}
                    onChange={(event) => {
                      setMode(event.target.value as DungeonMode);
                    }}
                  >
                    {MODES.map((option) => (
                      <option key={option} value={option}>
                        {t(MODE_WORDS[option])}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="dungeon-stats__check">
                  <input
                    type="checkbox"
                    data-testid={dungeonStatsTestid.dailyFilter}
                    checked={dailyOnly}
                    onChange={(event) => {
                      setDailyOnly(event.target.checked);
                    }}
                  />
                  {t("Daily runs only")}
                </label>
              </div>
              {matching.length === 0 ? (
                <p className="dungeon-stats__empty" data-testid={dungeonStatsTestid.noMatches} role="status">
                  {t("No runs match these filters.")}
                </p>
              ) : (
                <div className="dungeon-stats__boards">
                  {BOARDS.map(({ key, title }) => (
                    <Board key={key} board={key} title={title} runs={boardRuns(key, matching)} onOpen={openDetails} />
                  ))}
                </div>
              )}
            </>
          )}
        </div>
      </main>
      {open === null ? null : <RunDetails record={open} onClose={closeDetails} />}
    </div>
  );
}
