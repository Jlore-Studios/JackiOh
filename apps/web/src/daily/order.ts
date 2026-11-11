// The Daily screen's list, in the order it is shown (#3727): the games waiting on you, least time left
// first; the games waiting on your opponent; the challenges to and from you; your seeks in the daily
// queue; and the games that ended, newest first. Pure: the server's answer and a time in, words and
// order out. Nothing here is a rule (CLAUDE.md rule 7): who holds the move and its deadline are the
// server's, and the numbers are `crates/shared`'s, read from the WebAssembly module (rule 9).

import { t } from "../i18n/i18n.ts";
import type { DailyGameSummary, DailyInvite, DailyList } from "../net/daily.ts";
import { dailyConstants } from "../wasm/index.ts";

export type DailySections = {
  /** The games waiting on you, least time left first. */
  yourMove: readonly DailyGameSummary[];
  /** The games waiting on your opponent, least time left first. */
  theirMove: readonly DailyGameSummary[];
  /** Challenges your friends sent you, oldest first. */
  incoming: readonly DailyInvite[];
  /** Challenges you sent, oldest first. */
  outgoing: readonly DailyInvite[];
  /** Your seeks in the daily queue, oldest first. */
  seeks: readonly DailyInvite[];
  /** The games that ended, newest first. */
  finished: readonly DailyGameSummary[];
};

const NONE: readonly never[] = Object.freeze([]);

function listOf<T>(value: unknown): readonly T[] {
  return Array.isArray(value) ? (value as T[]) : NONE;
}

/** A deadline's place in a list: the soonest first, a game with none after every one that has one. */
function byDeadline(a: DailyGameSummary, b: DailyGameSummary): number {
  const left = a.deadline ?? Number.POSITIVE_INFINITY;
  const right = b.deadline ?? Number.POSITIVE_INFINITY;
  return left === right ? a.createdAt - b.createdAt : left - right;
}

function byCreated(a: DailyInvite, b: DailyInvite): number {
  return a.createdAt - b.createdAt;
}

/** The list's sections at `now`: an invite past its time is gone, though the server has not yet said so. */
export function dailySections(list: DailyList, now: number): DailySections {
  const games = listOf<DailyGameSummary>(list.games);
  const open = (invite: DailyInvite): boolean => invite.expiresAt > now;
  const challenges = (list as { challenges?: { incoming?: unknown; outgoing?: unknown } }).challenges;
  return {
    yourMove: games.filter((game) => !game.over && game.hasMove).sort(byDeadline),
    theirMove: games.filter((game) => !game.over && !game.hasMove).sort(byDeadline),
    incoming: listOf<DailyInvite>(challenges?.incoming).filter(open).sort(byCreated),
    outgoing: listOf<DailyInvite>(challenges?.outgoing).filter(open).sort(byCreated),
    seeks: listOf<DailyInvite>(list.seeks).filter(open).sort(byCreated),
    finished: games
      .filter((game) => game.over)
      .sort((a, b) => (b.endedAt ?? b.createdAt) - (a.endedAt ?? a.createdAt)),
  };
}

/** The game the board's Next game opens: the first of Your move, or none. */
export function nextYourMove(list: DailyList, now: number, except: string | null = null): DailyGameSummary | null {
  return dailySections(list, now).yourMove.find((game) => game.matchId !== except) ?? null;
}

/** How long a move has left, in days and hours: "1d 4h left", "5h left", "Under 1h left", "Out of time". */
export function timeLeftText(ms: number): string {
  const { MS_PER_DAY, MS_PER_HOUR } = dailyConstants();
  if (ms <= 0) return t("Out of time");
  if (ms < MS_PER_HOUR) return t("Under 1h left");
  const days = Math.floor(ms / MS_PER_DAY);
  const hours = Math.floor((ms % MS_PER_DAY) / MS_PER_HOUR);
  return days === 0 ? t("{hours}h left", { hours }) : t("{days}d {hours}h left", { days, hours });
}

/** Whether a move's time left is short enough to show in the urgent color (`DAILY_URGENT_HOURS`). */
export function isUrgent(ms: number): boolean {
  const { DAILY_URGENT_HOURS, MS_PER_HOUR } = dailyConstants();
  return ms < DAILY_URGENT_HOURS * MS_PER_HOUR;
}

/** A time per move in words: "1 day", "3 days". */
export function perMoveText(days: number): string {
  return days === 1 ? t("1 day") : t("{count} days", { count: days });
}

/** A time per move as a row says it: "1 day per move". */
export function perMoveRowText(days: number): string {
  return days === 1 ? t("1 day per move") : t("{count} days per move", { count: days });
}

/** The games waiting on you, as the social read carries it (`dailyWaiting`): anything not a count reads as none. */
export function dailyWaitingOf(value: unknown): number {
  return typeof value === "number" && Number.isInteger(value) && value > 0 ? value : 0;
}

/** The tab's title while games wait on you: "Your move (3)", as the live board's says "Your turn". */
export function yourMoveTitle(count: number): string {
  return t("Your move ({count})", { count });
}
