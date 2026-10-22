// R1584000: the floor's stat showcase: the cycle and its holds, reduced motion, and the board's
// animation each stat's card plays.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingShowcasePickTestid, landingTestid } from "../auth/testids.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { animationRow } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { SHOWCASE_BEAT_GAP_MS, SHOWCASE_INTERVAL_MS, SHOWCASE_LEAD_MS } from "./config.ts";
import { NO_COUNTS, type CardCounters, type PlayerStats } from "./model.ts";
import type { PocketPick } from "./pocketPick.ts";
import { showcaseStats, type ShowcaseKey, type ShowcaseStat } from "./showcase.ts";
import { STAGE_INSTANCE_ID } from "./showcaseStage.ts";
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
    games: 12,
    wins: 7,
    losses: 5,
    draws: 0,
    cards: Object.fromEntries(Object.entries(cards).map(([id, counts]) => [id, { ...NO_COUNTS, ...counts }])),
  };
}

const PICK: PocketPick = { id: "core-005", playerRate: 0.8, baseRate: 0.2 };

/** Toppled, Coveted, Old Faithful and The Record, all on core-004. */
const FOUR: readonly ShowcaseStat[] = showcaseStats(tally({ "core-004": { defeated: 37, allured: 6, played: 9, seen: 12 } }), null);

const KEYS: readonly ShowcaseKey[] = ["toppled", "coveted", "faithful", "record"];

function show(stats: readonly ShowcaseStat[] = FOUR, reduced = false): void {
  render(<StatShowcase stats={stats} reduced={reduced} />);
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
    show();
    expect(screen.getByTestId(landingTestid.showcaseTitle).textContent).toBe("Most Toppled");
    expect(screen.getByTestId(landingTestid.showcaseLine).textContent).toBe("taken down 37×");
    expect(card().getAttribute("data-def-id")).toBe("core-004");
    expect(card().childElementCount).toBeGreaterThan(0);
  });

  it("R1584000 cycles to the next stat every SHOWCASE_INTERVAL_MS and wraps", () => {
    show();
    expect(shown()).toBe("toppled");
    for (const key of [...KEYS.slice(1), KEYS[0]!]) {
      advance(SHOWCASE_INTERVAL_MS - 1);
      expect(shown()).not.toBe(key);
      advance(1);
      expect(shown()).toBe(key);
    }
  });

  it("R1584000 previous and next step, and a pick shows its stat", () => {
    show();
    const pick = (key: ShowcaseKey): HTMLElement => screen.getByTestId(landingShowcasePickTestid(key));
    expect(pick("toppled").getAttribute("aria-pressed")).toBe("true");
    expect(pick("coveted").getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(screen.getByTestId(landingTestid.showcaseNext));
    expect(shown()).toBe("coveted");
    expect(pick("coveted").getAttribute("aria-pressed")).toBe("true");
    expect(pick("toppled").getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(screen.getByTestId(landingTestid.showcasePrev));
    fireEvent.click(screen.getByTestId(landingTestid.showcasePrev));
    expect(shown()).toBe("record");

    fireEvent.click(pick("faithful"));
    expect(shown()).toBe("faithful");
    expect(screen.getByTestId(landingTestid.showcaseTitle).textContent).toBe("Old Faithful");
    expect(pick("faithful").getAttribute("aria-label")).toBe("Old Faithful");
    expect(screen.getByTestId(landingTestid.showcaseNext).getAttribute("aria-label")).toBe("Next stat");
    expect(screen.getByTestId(landingTestid.showcasePrev).getAttribute("aria-label")).toBe("Previous stat");
  });

  it("R1584000 a step by hand gives the stat it lands on its whole time", () => {
    show();
    advance(SHOWCASE_INTERVAL_MS - 100);
    fireEvent.click(screen.getByTestId(landingTestid.showcaseNext));
    advance(SHOWCASE_INTERVAL_MS - 1);
    expect(shown()).toBe("coveted");
    advance(1);
    expect(shown()).toBe("faithful");
  });

  it("R1584000 holds still under a pointer, under focus and while the page is hidden", () => {
    show();
    const root = screen.getByTestId(landingTestid.showcase);

    fireEvent.mouseEnter(root);
    advance(3 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe("toppled");
    fireEvent.mouseLeave(root);
    advance(SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe("coveted");

    fireEvent.focus(screen.getByTestId(landingTestid.showcaseNext));
    advance(3 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe("coveted");
    fireEvent.blur(screen.getByTestId(landingTestid.showcaseNext));
    advance(SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe("faithful");

    pageIs("hidden");
    advance(3 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe("faithful");
    pageIs("visible");
    advance(SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe("record");
  });

  it("R1584000 a lone stat has no controls and never moves, and none draws nothing", () => {
    show(FOUR.slice(0, 1));
    expect(screen.queryByTestId(landingTestid.showcaseNext)).toBeNull();
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
  /** Nothing plays and nothing is drawn, the stat holds, and the controls still step it. */
  function expectStill(): void {
    expect(card().hasAttribute("data-entry")).toBe(false);
    expect(document.querySelector("[data-animating]")).toBeNull();
    expect(screen.queryByTestId(landingTestid.showcaseFx)).toBeNull();
    advance(2 * SHOWCASE_INTERVAL_MS);
    expect(shown()).toBe("toppled");
    expect(card().hasAttribute("data-entry")).toBe(false);
    expect(document.querySelector("[data-animating]")).toBeNull();
    expect(screen.getByTestId(landingTestid.showcaseLine).textContent).toBe("taken down 37×");
    fireEvent.click(screen.getByTestId(landingTestid.showcaseNext));
    expect(shown()).toBe("coveted");
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
});

describe("R1584000 the card's animation", () => {
  const ms = (type: string): number => animationRow(type)!.durationMs;

  it("R1584000 the card plays its stat's board animation after the lead", () => {
    // Most Toppled: it comes in, plays its Death animation and comes back.
    show();
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
    show();
    fireEvent.click(screen.getByTestId(landingTestid.showcaseNext));
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
    show(showcaseStats(tally({ "core-004": { played: 9 } }), PICK));
    fireEvent.click(screen.getByTestId(landingShowcasePickTestid("pocketPick")));
    expect(card().getAttribute("data-def-id")).toBe("core-005");
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-animating")).toBe("trapFired");
    expect(card().getAttribute("data-field-trap")).toBe("true");
    advance(ms("trapFired") + SHOWCASE_BEAT_GAP_MS);
    expect(card().hasAttribute("data-animating")).toBe(false);
  });

  it("R1584000 an opponent's card lunges down the screen", () => {
    show(showcaseStats(tally({ "core-004": { playedAgainst: 3 } }), null));
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-animating")).toBe("attackDeclared");
    expect(card().style.getPropertyValue("--lunge-dir")).toBe("1");
  });

  it("R1584000 with effects off the keyframes play without the effects layer", () => {
    setFxSettings({ intensity: "off" });
    show();
    expect(screen.queryByTestId(landingTestid.showcaseFx)).toBeNull();
    advance(SHOWCASE_LEAD_MS);
    expect(card().getAttribute("data-animating")).toBe("destroyed");
  });

  it("R1584000 a new stat's card comes in again", () => {
    show();
    advance(SHOWCASE_LEAD_MS + ms("destroyed") + SHOWCASE_BEAT_GAP_MS);
    fireEvent.click(screen.getByTestId(landingTestid.showcaseNext));
    expect(card().getAttribute("data-entry")).toBe("enter");
  });
});
