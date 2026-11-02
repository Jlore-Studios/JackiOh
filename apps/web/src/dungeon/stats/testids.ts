// The dungeon run's device leaderboards (DR9, DR14 part G, #2708): the `data-testid`
// vocabulary of the `/dungeon/stats` screen. Every id is prefixed `dungeon-`, as each part of
// the dungeon run keeps its own in its directory.

import type { RunStats } from "@jackioh/run";

/** Which device leaderboard a board or an entry belongs to. */
export type DungeonLeaderboard = "deepest" | "curses" | "score" | "fastest";

export const dungeonStatsTestid = {
  /** The screen's root. */
  root: "dungeon-stats",
  /** The Sandbox/Campaign picker. */
  modeFilter: "dungeon-stats-mode",
  /** The daily-runs-only checkbox. */
  dailyFilter: "dungeon-stats-daily-only",
  /** The device holds no finished runs; role-less, the shared empty state carries the action. */
  noHistory: "dungeon-stats-no-history",
  /** The device holds runs, but none match the mode and daily filters; role="status". */
  noMatches: "dungeon-stats-no-matches",
  /** One of the four leaderboards. */
  board: (board: DungeonLeaderboard): string => `dungeon-stats-board-${board}`,
  /** The fastest board, when matching runs exist but none was won; role="status". */
  noWins: "dungeon-stats-no-wins",
  /** One board's button row, keyed by the record's original history index, so duplicate seeds stay distinct. */
  entry: (board: DungeonLeaderboard, index: number): string => `dungeon-stats-entry-${board}-${index}`,
  /** The run-details dialog. */
  dialog: "dungeon-stats-dialog",
  /** The dialog's Close button. */
  dialogClose: "dungeon-stats-dialog-close",
  /** The dialog's identifying metadata (seed, mode, date, outcome, rows, points, score, embers). */
  detailMeta: "dungeon-stats-detail-meta",
  /** The dialog's formatted elapsed time. */
  detailTime: "dungeon-stats-detail-time",
  /** The dialog's curse list, or its no-curses line. */
  detailCurses: "dungeon-stats-detail-curses",
  /** One `RunStats` field's value in the dialog's stats table. */
  stat: (field: keyof RunStats): string => `dungeon-stats-stat-${field}`,
} as const;
