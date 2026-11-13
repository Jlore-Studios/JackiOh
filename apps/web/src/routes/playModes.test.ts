// R3729000: the Play hub's catalog: its groups, and each card's words, time and what it needs (#3729).

import { describe, expect, it } from "vitest";

import { dailyConstants } from "../wasm/index.ts";
import { paths } from "../net/navigate.ts";
import { TUTORIAL_LESSONS } from "../tutorial/lessons.ts";
import { GAME_MINUTES, PLAY_GROUPS, minutesText, playCards, type PlayCard } from "./playModes.ts";

function byId(cards: readonly PlayCard[], id: PlayCard["id"]): PlayCard {
  const card = cards.find((candidate) => candidate.id === id);
  if (card === undefined) throw new Error(`no ${id} card`);
  return card;
}

describe("R3729000 the Play hub's catalog", () => {
  const signedIn = playCards({ signedIn: true, dailyWaiting: 0 });

  it("R3729000 the groups come in the order online, daily, friends, solo, and every card sits in one", () => {
    expect(PLAY_GROUPS).toEqual(["online", "daily", "friends", "solo"]);
    const order = signedIn.map((card) => PLAY_GROUPS.indexOf(card.group));
    expect(order).toEqual([...order].sort((a, b) => a - b));
    expect(signedIn.map((card) => card.group)).toEqual([
      "online",
      "online",
      "online",
      "daily",
      "friends",
      "friends",
      "solo",
      "solo",
      "solo",
    ]);
  });

  it("R3729000 every card has a name, a line, a time and what it needs, and a testid of its own", () => {
    for (const card of signedIn) {
      expect(card.name, card.id).not.toBe("");
      expect(card.line, card.id).not.toBe("");
      expect(card.time, card.id).not.toBe("");
      expect(card.needs, card.id).not.toBe("");
    }
    expect(new Set(signedIn.map((card) => card.testid)).size).toBe(signedIn.length);
  });

  it("R3729000 a signed-out visitor gets the solo cards alone, and the engine's module is not asked", () => {
    const visitor = playCards({ signedIn: false, dailyWaiting: 5 });
    expect(visitor.map((card) => card.id)).toEqual(["practice", "dungeon", "tutorial"]);
    expect(visitor.every((card) => card.group === "solo")).toBe(true);
  });

  it("R3729000 the time of a card with a game length is GAME_MINUTES', as minutes", () => {
    for (const card of signedIn) {
      if (card.id === "daily") continue;
      expect(card.time, card.id).toBe(minutesText(GAME_MINUTES[card.id]));
    }
    for (const [min, max] of Object.values(GAME_MINUTES)) {
      expect(min).toBeGreaterThan(0);
      expect(max).toBeGreaterThanOrEqual(min);
    }
  });

  it("R3729000 the Daily card's time is the shortest and longest time per move, from the shared constants", () => {
    const days = dailyConstants().DAILY_TIME_PER_MOVE_DAYS;
    expect(days.length).toBeGreaterThan(0);
    const first = days[0];
    const last = days.at(-1);
    expect(byId(signedIn, "daily").time).toBe(`A move every ${String(first)} to ${String(last)} days`);
  });

  it("R3729000 the Daily card says how many games wait on the player while some do", () => {
    expect(byId(signedIn, "daily").line).not.toMatch(/Your move/);
    expect(byId(playCards({ signedIn: true, dailyWaiting: 3 }), "daily").line).toBe("Your move in 3");
  });

  it("R3729000 each card does what its group says: an Online card queues its mode, the rest link or open the lobby", () => {
    expect(signedIn.filter((card) => card.group === "online").map((card) => card.action)).toEqual([
      { kind: "queue", mode: "bo1" },
      { kind: "queue", mode: "bo3" },
      { kind: "queue", mode: "random" },
    ]);
    expect(byId(signedIn, "lobby").action).toEqual({ kind: "lobby" });
    expect(byId(signedIn, "daily").action).toEqual({ kind: "link", href: paths.daily });
    expect(byId(signedIn, "challenge").action).toEqual({ kind: "link", href: paths.together });
    expect(byId(signedIn, "practice").action).toEqual({ kind: "link", href: paths.practice });
    expect(byId(signedIn, "dungeon").action).toEqual({ kind: "link", href: paths.dungeon });
    const first = TUTORIAL_LESSONS[0];
    expect(byId(signedIn, "tutorial").action).toEqual({ kind: "link", href: `${paths.practice}?lesson=${first?.id ?? ""}` });
  });
});
