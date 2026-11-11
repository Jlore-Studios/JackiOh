// #3727: the Daily screen's order and words, worked out from the server's list and a time, with the
// numbers read from the WebAssembly module as the screen reads them (CLAUDE.md rule 9).

import { afterEach, describe, expect, it } from "vitest";

import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { DailyGameSummary, DailyInvite, DailyList } from "../net/daily.ts";
import { dailyConstants } from "../wasm/index.ts";
import {
  dailySections,
  dailyWaitingOf,
  isUrgent,
  nextYourMove,
  perMoveRowText,
  perMoveText,
  timeLeftText,
  yourMoveTitle,
} from "./order.ts";

const NOW = 1_800_000_000_000;

function game(matchId: string, over: Partial<DailyGameSummary> = {}): DailyGameSummary {
  return {
    matchId,
    mode: "bo1",
    perMoveDays: 3,
    seat: "p1",
    opponent: { profileId: `o-${matchId}`, username: `Opp${matchId}#1` },
    portraits: { me: "vanilla", them: "gary" },
    turn: 4,
    life: { me: 30, them: 22 },
    hasMove: false,
    theirMove: true,
    deadline: NOW + 1000,
    over: false,
    result: null,
    endedAt: null,
    createdAt: NOW - 1000,
    ...over,
  };
}

function invite(id: string, over: Partial<DailyInvite> = {}): DailyInvite {
  return {
    id,
    kind: "challenge",
    from: { profileId: "a", username: "Ann#2" },
    to: { profileId: "b", username: "Bob#3" },
    mode: "random",
    perMoveDays: 1,
    expiresAt: NOW + 1000,
    createdAt: NOW - 1000,
    ...over,
  };
}

function list(over: Partial<DailyList> = {}): DailyList {
  return { games: [], challenges: { incoming: [], outgoing: [] }, seeks: [], waiting: 0, ...over };
}

afterEach(() => {
  resetLocaleForTests();
});

describe("#3727 the Daily screen's sections", () => {
  it("#3727 Your move comes first, least time left first, then Their move, the challenges, the seeks and the finished, newest first", () => {
    const answer = list({
      games: [
        game("late", { hasMove: true, theirMove: false, deadline: NOW + 9000 }),
        game("theirs-late", { deadline: NOW + 8000 }),
        game("soon", { hasMove: true, theirMove: false, deadline: NOW + 2000 }),
        game("old-end", { over: true, deadline: null, endedAt: NOW - 9000 }),
        game("theirs-soon", { deadline: NOW + 3000 }),
        game("new-end", { over: true, deadline: null, endedAt: NOW - 100 }),
      ],
      challenges: {
        incoming: [invite("in-2", { createdAt: NOW - 10 }), invite("in-1", { createdAt: NOW - 20 })],
        outgoing: [invite("out-1")],
      },
      seeks: [invite("seek-1", { kind: "seek", to: null })],
      waiting: 2,
    });
    const sections = dailySections(answer, NOW);
    expect(sections.yourMove.map((entry) => entry.matchId)).toEqual(["soon", "late"]);
    expect(sections.theirMove.map((entry) => entry.matchId)).toEqual(["theirs-soon", "theirs-late"]);
    expect(sections.incoming.map((entry) => entry.id)).toEqual(["in-1", "in-2"]);
    expect(sections.outgoing.map((entry) => entry.id)).toEqual(["out-1"]);
    expect(sections.seeks.map((entry) => entry.id)).toEqual(["seek-1"]);
    expect(sections.finished.map((entry) => entry.matchId)).toEqual(["new-end", "old-end"]);
  });

  it("#3727 an invite past its time is left out, and a missing list reads as an empty one", () => {
    const answer = list({
      challenges: { incoming: [invite("gone", { expiresAt: NOW }), invite("kept")], outgoing: [] },
      seeks: [invite("seek-gone", { kind: "seek", to: null, expiresAt: NOW - 1 })],
    });
    const sections = dailySections(answer, NOW);
    expect(sections.incoming.map((entry) => entry.id)).toEqual(["kept"]);
    expect(sections.seeks).toEqual([]);
    const odd = dailySections({ waiting: 0 } as unknown as DailyList, NOW);
    expect(odd).toEqual({ yourMove: [], theirMove: [], incoming: [], outgoing: [], seeks: [], finished: [] });
  });

  it("#3727 Next game is the first Your move row but the board's own", () => {
    const answer = list({
      games: [
        game("here", { hasMove: true, theirMove: false, deadline: NOW + 10 }),
        game("next", { hasMove: true, theirMove: false, deadline: NOW + 20 }),
        game("theirs", { deadline: NOW + 5 }),
      ],
    });
    expect(nextYourMove(answer, NOW)?.matchId).toBe("here");
    expect(nextYourMove(answer, NOW, "here")?.matchId).toBe("next");
    expect(nextYourMove(list(), NOW)).toBeNull();
  });
});

describe("#3727 the time left", () => {
  it("#3727 says days and hours, hours alone under a day, and less once under an hour", () => {
    const { MS_PER_DAY, MS_PER_HOUR } = dailyConstants();
    expect(timeLeftText(MS_PER_DAY + 4 * MS_PER_HOUR + 1)).toBe("1d 4h left");
    expect(timeLeftText(13 * MS_PER_DAY)).toBe("13d 0h left");
    expect(timeLeftText(5 * MS_PER_HOUR + MS_PER_HOUR - 1)).toBe("5h left");
    expect(timeLeftText(MS_PER_HOUR)).toBe("1h left");
    expect(timeLeftText(MS_PER_HOUR - 1)).toBe("Under 1h left");
    expect(timeLeftText(0)).toBe("Out of time");
    expect(timeLeftText(-MS_PER_HOUR)).toBe("Out of time");
  });

  it("#3727 turns urgent under DAILY_URGENT_HOURS and not at it", () => {
    const { DAILY_URGENT_HOURS, MS_PER_HOUR } = dailyConstants();
    expect(DAILY_URGENT_HOURS).toBeGreaterThan(0);
    expect(isUrgent(DAILY_URGENT_HOURS * MS_PER_HOUR)).toBe(false);
    expect(isUrgent(DAILY_URGENT_HOURS * MS_PER_HOUR - 1)).toBe(true);
    expect(isUrgent(0)).toBe(true);
  });

  it("#3727 every time per move the server offers has its words, in the reader's language", async () => {
    const { DAILY_TIME_PER_MOVE_DAYS } = dailyConstants();
    expect(DAILY_TIME_PER_MOVE_DAYS.length).toBeGreaterThan(0);
    expect(perMoveText(1)).toBe("1 day");
    expect(perMoveText(3)).toBe("3 days");
    expect(perMoveRowText(1)).toBe("1 day per move");
    expect(perMoveRowText(14)).toBe("14 days per move");
    await setLocale("zh-CN");
    for (const days of DAILY_TIME_PER_MOVE_DAYS) expect(perMoveText(days)).toContain(String(days));
    expect(timeLeftText(dailyConstants().MS_PER_DAY)).not.toContain("left");
  });
});

describe("#3727 the badge and the title", () => {
  it("#3727 the badge counts the server's dailyWaiting, and reads anything else as none", () => {
    expect(dailyWaitingOf(3)).toBe(3);
    expect(dailyWaitingOf(0)).toBe(0);
    expect(dailyWaitingOf(undefined)).toBe(0);
    expect(dailyWaitingOf(-1)).toBe(0);
    expect(dailyWaitingOf(1.5)).toBe(0);
    expect(dailyWaitingOf("2")).toBe(0);
  });

  it("#3727 the title says how many games wait on you", async () => {
    expect(yourMoveTitle(3)).toBe("Your move (3)");
    await setLocale("zh-CN");
    expect(yourMoveTitle(3)).toContain("3");
    expect(yourMoveTitle(3)).not.toContain("Your move");
  });
});
