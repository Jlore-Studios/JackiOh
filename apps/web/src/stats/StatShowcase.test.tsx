// R1584000: the floor's stat showcase: the cycle and its holds, reduced motion, and the board's
// animation each stat's card plays. R1743000: the order is a shuffled bag drawn from the page's random
// source, and the showcase itself, not an arrow or a dot, is the control. R2649000: Brittle's and
// Temporary's aura is on the card before the beat it warns of, and a beat played twice starts again.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid } from "../auth/testids.ts";
import { marksTestid } from "../cards/CardMarks.tsx";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { animationRow } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { shuffled, type RandomSource } from "../routes/landingFan.ts";
import { seeded } from "../test/random.ts";
import { SHOWCASE_BEAT_GAP_MS, SHOWCASE_INTERVAL_MS, SHOWCASE_LEAD_MS } from "./config.ts";
import { EMPTY_STATS, NO_COUNTS, type CardCounters, type PlayerStats } from "./model.ts";
import type { PocketPick } from "./pocketPick.ts";
import { showcaseStats, type ShowcaseKey, type ShowcaseStat } from "./showcase.ts";
import { STAGE_INSTANCE_ID, firstBeatAt, stageScript } from "./showcaseStage.ts";
import { StatShowcase } from "./StatShowcase.tsx";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  // The effects settings persist to storage, which the next read would find again.
  window.localStorage.clear();
  resetFxSettingsForTests();
  Reflect.deleteProperty(document, "visibilityState");
});

function tally(cards: Record<string, Partial<CardCounters>>): PlayerStats {
  return {
    ...EMPTY_STATS,
    games: 12,
    wins: 7,
    losses: 5,
    draws: 0,
    cards: Object.fromEntries(Object.entries(cards).map(([id, counts]) => [id, { ...NO_COUNTS, ...counts }])),
  };
}

const PICK: PocketPick = { id: "core-005", playerRate: 0.8, baseRate: 0.2 };

const KEYS: readonly ShowcaseKey[] = ["toppled", "coveted", "faithful", "record"];

/** Toppled, Coveted, Old Faithful and The Record, all on core-004 (the tally's other stats are left off). */
const FOUR: readonly ShowcaseStat[] = showcaseStats(tally({ "core-004": { defeated: 37, allured: 6, played: 9, playedInWin: 9, seen: 12 } }), null, seeded(1)).filter(
  (stat) => KEYS.includes(stat.key),
);

function only(key: ShowcaseKey): ShowcaseStat[] {
  return FOUR.filter((stat) => stat.key === key);
}

function show(stats: readonly ShowcaseStat[] = FOUR, reduced = false, random: RandomSource = seeded(7)): void {
  render(<StatShowcase stats={stats} reduced={reduced} random={random} />);
}

function root(): HTMLElement {
  return screen.getByTestId(landingTestid.showcase);
}

function lineShown(): string {
  return FOUR.find((stat) => stat.key === shown())!.line();
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

function shown(): string | null {
  return screen.getByTestId(landingTestid.showcase).getAttribute("data-stat");
}

function card(): HTMLElement {
  return screen.getByTestId(testid.card(STAGE_INSTANCE_ID));
}

function pageIs(state: "hidden" | "visible"): void {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
}

describe("R1584000 the cycle", () => {
  it("R1584000 shows a stat's title, card and number", () => {
    expect(FOUR.map((stat) => stat.key)).toEqual(KEYS);
    show(only("toppled"));
    expect(screen.getByTestId(landingTestid.showcaseTitle).textContent).toBe("Most Toppled");
    expect(screen.getByTestId(landingTestid.showcaseLine).textContent).toBe("taken down 37×");
    expect(card().getAttribute("data-def-id")).toBe("core-004");
    expect(card().childElementCount).toBeGreaterThan(0);
  });

  it("R1743000 a seeded source sets the order: each round is every stat once, the seed's shuffle first, none twice in a row, the same on a second render", () => {
    const rounds = 5;
    const run = (): string[] => {
      show(FOUR, false, seeded(7));
      const order: string[] = [shown()!];
      for (let i = 1; i < KEYS.length * rounds; i += 1) {
        advance(SHOWCASE_INTERVAL_MS - 1);
        expect(shown()).toBe(order[i - 1]);
        advance(1);
        order.push(shown()!);
      }
      cleanup();
      return order;
    };
    const order = run();
    for (let round = 0; round < rounds; round += 1) {
      expect(order.slice(round * KEYS.length, (round + 1) * KEYS.length).sort()).toEqual([...KEYS].sort());
    }
    expect(order.slice(0, KEYS.length)).toEqual(shuffled(KEYS, seeded(7)));
    order.forEach((key, i) => {
      if (i > 0) expect(key).not.toBe(order[i - 1]);
    });
    expect(run()).toEqual(order);
    show(FOUR, false, seeded(8));
    expect(shown()).toBe(shuffled(KEYS, seeded(8))[0]);
  });

  it("R1743000 a click, Enter or Space moves on to another stat; there is no button, prev, next or pick control", () => {
    show();
    expect(root().getAttribute("role")).toBe("button");
    expect(root().getAttribute("tabindex")).toBe("0");
    expect(root().querySelector("button")).toBeNull();
    expect(screen.getAllByRole("button")).toEqual([root()]);
    expect(document.querySelector('[data-testid*="prev"], [data-testid*="next"], [data-testid*="pick"], .landing-showcase-controls')).toBeNull();
    expect(screen.queryByLabelText("Next stat")).toBeNull();
    expect(screen.queryByLabelText("Previous stat")).toBeNull();
    expect(screen.queryByLabelText("Pick a stat")).toBeNull();

    const order = [shown()];
    fireEvent.click(root());
    order.push(shown());
    // Enter and Space are taken (the page does not scroll); another key is left alone.
    expect(fireEvent.keyDown(root(), { key: "Enter" })).toBe(false);
    order.push(shown());
    expect(fireEvent.keyDown(root(), { key: " " })).toBe(false);
    order.push(shown());
    // Four stops make the first round: every stat once.
    expect([...order].sort()).toEqual([...KEYS].sort());
    const now = shown();
    expect(fireEvent.keyDown(root(), { key: "a" })).toBe(true);
    expect(fireEvent.keyDown(root(), { key: "Tab" })).toBe(true);
    expect(shown()).toBe(now);
    fireEvent.click(root());
    expect(shown()).not.toBe(now);
  });

  it("R1743000 a step by hand gives the stat it lands on its whole time", () => {
    show();
    advance(SHOWCASE_INTERVAL_MS - 100);
    const before = shown();
    fireEvent.click(root());
    const landed = shown();
    expect(landed).not.toBe(before);
    advance(SHOWCASE_INTERVAL_MS - 1);
    expect(shown()).toBe(landed);
    advance(1);
    expect(shown()).not.toBe(landed);
  });

  it("R1584000 holds still under a pointer, under focus and while the page is hidden", () => {
    show();
    const first = shown();

    fireEvent.mouseEnter(root());
    advance(3 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe(first);
    fireEvent.mouseLeave(root());
    advance(SHOWCASE_INTERVAL_MS);
    const second = shown();
    expect(second).not.toBe(first);

    fireEvent.focus(root());
    advance(3 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe(second);
    fireEvent.blur(root());
    advance(SHOWCASE_INTERVAL_MS);
    const third = shown();
    expect(third).not.toBe(second);

    pageIs("hidden");
    advance(3 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe(third);
    pageIs("visible");
    advance(SHOWCASE_INTERVAL_MS);
    expect(shown()).not.toBe(third);
  });

  it("R1584000 a lone stat has no control and never moves, and none draws nothing", () => {
    show(only("toppled"));
    expect(root().hasAttribute("role")).toBe(false);
    expect(root().hasAttribute("tabindex")).toBe(false);
    expect(screen.queryAllByRole("button")).toEqual([]);
    fireEvent.click(root());
    fireEvent.keyDown(root(), { key: "Enter" });
    advance(2 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe("toppled");
    cleanup();
    show([]);
    expect(screen.queryByTestId(landingTestid.showcase)).toBeNull();
  });

  it("R1584000 clears its timers when it goes", () => {
    show();
    advance(SHOWCASE_LEAD_MS);
    cleanup();
    expect(vi.getTimerCount()).toBe(0);
  });
});

describe("R1584000 reduced motion", () => {
  /** Nothing plays and nothing is drawn, the stat holds, and a click or Enter still steps it. */
  function expectStill(): void {
    const first = shown();
    expect(card().hasAttribute("data-entry")).toBe(false);
    expect(document.querySelector("[data-animating]")).toBeNull();
    expect(screen.queryByTestId(landingTestid.showcaseFx)).toBeNull();
    advance(2 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe(first);
    expect(card().hasAttribute("data-entry")).toBe(false);
    expect(document.querySelector("[data-animating]")).toBeNull();
    expect(screen.getByTestId(landingTestid.showcaseLine).textContent).toBe(lineShown());
    fireEvent.click(root());
    const second = shown();
    expect(second).not.toBe(first);
    fireEvent.keyDown(root(), { key: "Enter" });
    expect(shown()).not.toBe(second);
    expect(card().hasAttribute("data-entry")).toBe(false);
  }

  it("R1584000 reduced motion shows the still card and number, plays nothing and draws no effects", () => {
    show(FOUR, true);
    expectStill();
  });

  it("R1584000 the effects setting's reduced motion holds it still too", () => {
    setFxSettings({ motion: "reduce" });
    show();
    expectStill();
  });

  it("R1743000 under reduced motion a click still moves it, and it never moves on by itself", () => {
    show(FOUR, true);
    const first = shown();
    advance(10 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe(first);
    fireEvent.click(root());
    expect(shown()).not.toBe(first);
    const second = shown();
    advance(10 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe(second);
  });
});

describe("R1584000 the card's animation", () => {
  const ms = (type: string): number => animationRow(type)!.durationMs;

  it("R1584000 the card plays its stat's board animation after the lead", () => {
    // Most Toppled: it comes in, plays its Death animation and comes back.
    show(only("toppled"));
    expect(card().getAttribute("data-entry")).toBe("enter");
    expect(card().hasAttribute("data-animating")).toBe(false);
    expect(screen.getByTestId(landingTestid.showcaseFx).getAttribute("data-fx")).toBe("on");
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-animating")).toBe("destroyed");
    expect(card().hasAttribute("data-entry")).toBe(false);
    advance(ms("destroyed") + SHOWCASE_BEAT_GAP_MS);
    expect(card().hasAttribute("data-animating")).toBe(false);
    expect(card().getAttribute("data-entry")).toBe("return");
  });

  it("R1584000 Most Coveted marks the card with Allure, then takes it to the other side", () => {
    show(only("coveted"));
    expect(shown()).toBe("coveted");
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-animating")).toBe("marked");
    expect(card().querySelector('[data-marks="allure"]')).not.toBeNull();
    advance(ms("marked") + SHOWCASE_BEAT_GAP_MS);
    expect(screen.getByTestId(testid.zone("opponent", "units", 0)).getAttribute("data-animating")).toBe("controlChanged");
    expect(card().querySelector("[data-marks]")).toBeNull();
    advance(ms("controlChanged") + SHOWCASE_BEAT_GAP_MS);
    expect(document.querySelector("[data-animating]")).toBeNull();
  });

  it("R1584000 the Pocket Pick plays the trap reveal and the card stays face-up", () => {
    show(showcaseStats(tally({ "core-004": { played: 9 } }), PICK, seeded(1)).filter((stat) => stat.key === "pocketPick"));
    expect(card().getAttribute("data-def-id")).toBe("core-005");
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-animating")).toBe("trapFired");
    expect(card().getAttribute("data-field-trap")).toBe("true");
    advance(ms("trapFired") + SHOWCASE_BEAT_GAP_MS);
    expect(card().hasAttribute("data-animating")).toBe(false);
  });

  it("R1584000 an opponent's card lunges down the screen", () => {
    show(showcaseStats(tally({ "core-004": { playedAgainst: 3 } }), null, seeded(1)).filter((stat) => stat.key === "nemesis"));
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-animating")).toBe("attackDeclared");
    expect(card().style.getPropertyValue("--lunge-dir")).toBe("1");
  });

  it("R2648000 a Spell's lunge stat plays the board's cardPlayed on the stage card", () => {
    show(showcaseStats(tally({ "core-005": { playedAgainst: 3 } }), null, seeded(1)).filter((stat) => stat.key === "nemesis"));
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-def-id")).toBe("core-005");
    expect(card().getAttribute("data-animating")).toBe("cardPlayed");
  });

  it("R1584000 with effects off the keyframes play without the effects layer", () => {
    setFxSettings({ intensity: "off" });
    show(only("toppled"));
    expect(screen.queryByTestId(landingTestid.showcaseFx)).toBeNull();
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-animating")).toBe("destroyed");
  });

  it("R1584000 a new stat's card comes in again", () => {
    show();
    advance(SHOWCASE_LEAD_MS + ms("destroyed") + SHOWCASE_BEAT_GAP_MS);
    const before = shown();
    fireEvent.click(root());
    expect(shown()).not.toBe(before);
    expect(card().getAttribute("data-entry")).toBe("enter");
  });
});

describe("R2649000 the aura before the beat", () => {
  const ms = (type: string): number => animationRow(type)!.durationMs;
  /** The one stat `key` of a tally with `counts` on core-004. */
  const statFor = (key: ShowcaseKey, counts: Partial<CardCounters>): ShowcaseStat[] =>
    showcaseStats(tally({ "core-004": counts }), null, seeded(1)).filter((stat) => stat.key === key);
  /** The color of the aura the stage's card wears, or null for none. */
  const aura = (): string | null => card().querySelector(`[data-testid="${marksTestid(STAGE_INSTANCE_ID)}"]`)?.getAttribute("data-mark-color") ?? null;

  it("R2649000 the aura is on the card until the first beat and gone when it plays", () => {
    const cases = [
      ["sandCastle", { crumbled: 3 }, "gray", "crumbled"],
      ["justInTime", { tempPlayed: 4, tempExpired: 1 }, "fuchsia", "cardPlayed"],
      ["useItOrLoseIt", { tempExpired: 2 }, "fuchsia", "exiled"],
    ] as const;
    for (const [key, counts, color, type] of cases) {
      const stats = statFor(key, counts);
      expect(stats).toHaveLength(1);
      const stat = stats[0]!;
      const first = firstBeatAt(stageScript(stat.animation, stat.cardId, stat.side));
      expect(first).toBe(SHOWCASE_LEAD_MS + SHOWCASE_BEAT_GAP_MS);
      show(stats);
      // The aura alone, the board's: no badge and nothing marked.
      expect([key, aura()]).toEqual([key, color]);
      expect(card().querySelector("[data-marks]")).toBeNull();
      expect(card().hasAttribute("data-animating")).toBe(false);
      advance(SHOWCASE_LEAD_MS);
      expect([key, aura(), card().hasAttribute("data-animating")]).toEqual([key, color, false]);
      advance(first - SHOWCASE_LEAD_MS - 1);
      expect([key, aura()]).toEqual([key, color]);
      // The beat plays, and the warning it followed is gone.
      advance(1);
      expect([key, card().getAttribute("data-animating"), aura()]).toEqual([key, type, null]);
      advance(ms(type) + SHOWCASE_BEAT_GAP_MS);
      expect([key, card().hasAttribute("data-animating"), aura()]).toEqual([key, false, null]);
      cleanup();
    }
  });

  it("R2649000 a stat with no aura plays its first beat after the lead, as before", () => {
    show(statFor("martyr", { sacrificed: 2 }));
    expect(aura()).toBeNull();
    advance(SHOWCASE_LEAD_MS);
    // Doom's first beat is the red destroy mark, the board's own badge.
    expect(card().getAttribute("data-animating")).toBe("marked");
    expect(card().querySelector('[data-marks="destroy"]')).not.toBeNull();
    expect(aura()).toBe("red");
    advance(ms("marked") + SHOWCASE_BEAT_GAP_MS);
    expect(card().getAttribute("data-animating")).toBe("destroyed");
  });

  it("R2649000 under reduced motion the still card wears the aura", () => {
    show(statFor("sandCastle", { crumbled: 3 }), true);
    expect(aura()).toBe("gray");
    advance(2 * SHOWCASE_INTERVAL_MS);
    expect(aura()).toBe("gray");
    expect(document.querySelector("[data-animating]")).toBeNull();
    expect(card().hasAttribute("data-entry")).toBe(false);
    cleanup();
    setFxSettings({ motion: "reduce" });
    show(statFor("justInTime", { tempPlayed: 4, tempExpired: 1 }));
    expect(aura()).toBe("fuchsia");
    advance(2 * SHOWCASE_INTERVAL_MS);
    expect(aura()).toBe("fuchsia");
    expect(document.querySelector("[data-animating]")).toBeNull();
  });

  it("R2649000 a repeated beat is a new element, so its keyframe starts again", () => {
    const zone = (): HTMLElement => screen.getByTestId(testid.zone("you", "units", 0));
    show(statFor("massProduced", { created: 4 }));
    advance(SHOWCASE_LEAD_MS);
    const first = zone();
    expect(first.getAttribute("data-animating")).toBe("summoned");
    advance(ms("summoned") + SHOWCASE_BEAT_GAP_MS);
    const second = zone();
    expect(second.getAttribute("data-animating")).toBe("summoned");
    expect(second).not.toBe(first);
    expect(first.isConnected).toBe(false);
    advance(ms("summoned") + SHOWCASE_BEAT_GAP_MS);
    expect(zone().getAttribute("data-animating")).toBe("summoned");
    expect(zone()).not.toBe(second);
    advance(ms("summoned") + SHOWCASE_BEAT_GAP_MS);
    expect(document.querySelector("[data-animating]")).toBeNull();
  });
});
