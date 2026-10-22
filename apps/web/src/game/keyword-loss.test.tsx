// #1598: a `keywordGranted` with `lost` set reads as a loss, not a gain: the card carries
// `data-lost` beside `data-animating`, animations.css plays `jk-icon-pop-out` on it, the `keyword`
// recipe plans arrows down and a void burst, and the cue plays `debuff`.

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { GameEvent } from "@jackioh/shared";

import { ANIMATIONS } from "./animations.ts";
import Board from "./Board.tsx";
import { testid, type AnimatingMap, type AnimationFrames } from "./contract.ts";
import Game from "./Game.tsx";
import { keywordLossesFrom } from "./keywordLoss.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, emptySide, fullBoardView, unit, withEvents } from "../test/fixtures.ts";
import { setReducedMotion } from "../test/setup.ts";

function readSheet(fromWeb: string): string {
  for (const candidate of [fromWeb, `apps/web/${fromWeb}`]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8");
  }
  throw new Error(`${fromWeb} not found from ${process.cwd()}`);
}

const css = readSheet("src/game/animations.css");

/** The transform at one keyframe stop of `@keyframes <name>`. */
function stop(name: string, at: "0%" | "100%"): string {
  const block = new RegExp(`@keyframes ${name}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(css)?.[1];
  if (block === undefined) throw new Error(`@keyframes ${name} not found`);
  const found = new RegExp(`${at}\\s*\\{[^}]*?transform:\\s*([^;]+);`).exec(block)?.[1];
  if (found === undefined) throw new Error(`${name} has no transform at ${at}`);
  return found.trim();
}

function animationNameFor(selector: string): string | undefined {
  const escaped = selector.replace(/[[\]"=.]/g, "\\$&");
  return new RegExp(`${escaped}\\s*\\{[^}]*?animation-name:\\s*([\\w-]+);`).exec(css)?.[1];
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setReducedMotion(false);
  window.localStorage.clear();
  __resetSettingsForTests();
});

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`the fixture is missing ${what}`);
  return value;
}

const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];

function boardWith(events: GameEvent[]) {
  return withEvents(
    baseView({
      you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 0 } }),
    }),
    events,
  );
}

const LOST: GameEvent = { type: "keywordGranted", instanceId: "u1", keyword: { kind: "Taunt" }, lost: true };
const GAINED: GameEvent = { type: "keywordGranted", instanceId: "u1", keyword: { kind: "Taunt" } };

describe("#1598 keywordLossesFrom", () => {
  it("#1598 keywordLossesFrom marks a card whose keywordGranted in flight is a loss, not a grant", () => {
    const view = fullBoardView();
    const target = must(view.you.units[0], "your first unit");
    const cardId = testid.card(target.instanceId);
    const animating: AnimatingMap = new Map([[cardId, "keywordGranted"]]);
    const loss: GameEvent = { type: "keywordGranted", instanceId: target.instanceId, keyword: { kind: "Taunt" }, lost: true };
    const grant: GameEvent = { type: "keywordGranted", instanceId: target.instanceId, keyword: { kind: "Taunt" } };
    expect(keywordLossesFrom(withEvents(view, [loss]), animating).has(cardId)).toBe(true);
    expect(keywordLossesFrom(withEvents(view, [grant]), animating).size).toBe(0);
  });

  it("#1598 keywordLossesFrom reads the newest entry of the burst on the card", () => {
    const view = fullBoardView();
    const target = must(view.you.units[0], "your first unit");
    const cardId = testid.card(target.instanceId);
    const animating: AnimatingMap = new Map([[cardId, "keywordGranted"]]);
    const loss: GameEvent = { type: "keywordGranted", instanceId: target.instanceId, keyword: { kind: "Taunt" }, lost: true };
    const grant: GameEvent = { type: "keywordGranted", instanceId: target.instanceId, keyword: { kind: "Taunt" } };
    const frames: AnimatingMap = new Map([[cardId, "keywordGranted"]]);
    const lossFirst: readonly AnimationFrames[] = [
      { frames, events: [loss] },
      { frames, events: [grant] },
    ];
    expect(keywordLossesFrom(view, animating, lossFirst).size).toBe(0);
    const grantFirst: readonly AnimationFrames[] = [
      { frames, events: [grant] },
      { frames, events: [loss] },
    ];
    expect(keywordLossesFrom(view, animating, grantFirst).has(cardId)).toBe(true);
  });

  it("#1598 keywordLossesFrom marks nothing the runner is not animating as keywordGranted", () => {
    const view = fullBoardView();
    const target = must(view.you.units[0], "your first unit");
    const cardId = testid.card(target.instanceId);
    const loss: GameEvent = { type: "keywordGranted", instanceId: target.instanceId, keyword: { kind: "Taunt" }, lost: true };
    const buffed: AnimatingMap = new Map([[cardId, "buffed"]]);
    expect(keywordLossesFrom(withEvents(view, [loss]), buffed).size).toBe(0);
    expect(keywordLossesFrom(withEvents(view, [loss]), undefined).size).toBe(0);
  });
});

describe("#1598 the board marks a card losing its keyword", () => {
  it("#1598 the board writes data-lost beside data-animating on the card losing its keyword", () => {
    const view = fullBoardView();
    const target = must(view.you.units[0], "your first unit");
    const bystander = must(view.you.units[1], "your second unit");
    const cardId = testid.card(target.instanceId);
    const animating: AnimatingMap = new Map([[cardId, "keywordGranted"]]);
    const loss: GameEvent = { type: "keywordGranted", instanceId: target.instanceId, keyword: { kind: "Taunt" }, lost: true };
    render(<Board view={withEvents(view, [loss])} animating={animating} />);
    expect(screen.getByTestId(cardId)).toHaveAttribute("data-animating", "keywordGranted");
    expect(screen.getByTestId(cardId)).toHaveAttribute("data-lost", "true");
    expect(screen.getByTestId(testid.card(bystander.instanceId))).not.toHaveAttribute("data-lost");
    cleanup();
    const grant: GameEvent = { type: "keywordGranted", instanceId: target.instanceId, keyword: { kind: "Taunt" } };
    render(<Board view={withEvents(view, [grant])} animating={animating} />);
    expect(screen.getByTestId(cardId)).toHaveAttribute("data-animating", "keywordGranted");
    expect(screen.getByTestId(cardId)).not.toHaveAttribute("data-lost");
  });

  it("#1598 a lost keyword plays the pop-out for its duration, then the board catches up", () => {
    const { rerender } = render(<Game view={boardWith(START)} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={boardWith([...START, LOST])} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("card-u1")).toHaveAttribute("data-animating", "keywordGranted");
    expect(screen.getByTestId("card-u1")).toHaveAttribute("data-lost", "true");
    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.keywordGranted.durationMs);
    });
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-animating");
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-lost");
  });

  it("#1598 a grant still plays the pop-in", () => {
    const { rerender } = render(<Game view={boardWith(START)} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={boardWith([...START, GAINED])} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("card-u1")).toHaveAttribute("data-animating", "keywordGranted");
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-lost");
  });

  it("#1598 under prefers-reduced-motion a lost keyword drains at once", () => {
    setReducedMotion(true);
    const { rerender } = render(<Game view={boardWith(START)} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={boardWith([...START, LOST])} legal={[]} onAction={vi.fn()} />);
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-animating");
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-lost");
  });
});

describe("#1598 the pop-out keyframes", () => {
  it('#1598 a card marked data-lost plays jk-icon-pop-out; a grant still plays jk-icon-pop', () => {
    expect(animationNameFor('[data-animating="keywordGranted"][data-lost]')).toBe("jk-icon-pop-out");
    expect(animationNameFor('[data-animating="keywordGranted"]')).toBe("jk-icon-pop");
  });

  it("#1598 jk-icon-pop-out is jk-icon-pop played backwards", () => {
    expect(stop("jk-icon-pop-out", "0%")).toBe(stop("jk-icon-pop", "100%"));
    expect(stop("jk-icon-pop", "100%")).toBe("scale(1)");
    expect(stop("jk-icon-pop-out", "100%")).toBe(stop("jk-icon-pop", "0%"));
    expect(stop("jk-icon-pop", "0%")).toBe("scale(0.4)");
  });
});
