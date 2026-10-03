// #37, Global Cosmetic: "fix laggy switching-to-defense-position animation". The positionSwitched
// entry (animations.ts) plays over the board as it was BEFORE the switch (Game.tsx plans entries
// against the view still on screen), so the card's own `data-position` says which way it is turning.
//
// The old keyframes always ran upright -> 90deg at full size. Entering DEF they ended at full size and
// snapped to the DEF scale when the new view swapped in; leaving DEF they started upright, turned the wrong way
// and snapped upright. The sheet is read as text (vitest stubs CSS imports, as fx/css.test.ts does), and
// the one fact it must agree with Card.tsx on, the resting DEF transform, is read off a rendered card.

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ANIMATIONS } from "./animations.ts";
import Game from "./Game.tsx";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, emptySide, unit, withEvents } from "../test/fixtures.ts";

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

afterEach(cleanup);

function renderUnit(position: "ATK" | "DEF"): HTMLElement {
  const view = baseView({
    you: emptySide("p1", { units: [unit("p1", { instanceId: "u1", position }), null, null, null, null] }),
    opponent: emptySide("p2", { hand: { count: 0 } }),
  });
  render(<Game view={view} legal={[]} onAction={() => {}} />);
  return screen.getByTestId("card-u1");
}

describe("#37 positionSwitched turns the card between its two resting transforms", () => {
  it("a unit at rest in ATK has no transform, and in DEF is turned and scaled", () => {
    expect(renderUnit("ATK").style.transform).toBe("");
    cleanup();
    expect(renderUnit("DEF").style.transform).toBe("rotate(90deg) scale(0.72)");
  });

  it("jk-rotate-def (ATK to DEF) starts upright and ends on the DEF transform the new view rests on", () => {
    expect(stop("jk-rotate-def", "0%")).toBe("rotate(0deg) scale(1)");
    expect(stop("jk-rotate-def", "100%")).toBe(renderUnit("DEF").style.transform);
  });

  it("jk-rotate-atk (DEF to ATK) starts on the DEF transform and ends upright", () => {
    expect(stop("jk-rotate-atk", "0%")).toBe(renderUnit("DEF").style.transform);
    expect(stop("jk-rotate-atk", "100%")).toBe("rotate(0deg) scale(1)");
  });

  it("the card still showing DEF turns back to ATK; any other turns into DEF", () => {
    expect(animationNameFor('[data-animating="positionSwitched"]')).toBe("jk-rotate-def");
    expect(animationNameFor('[data-animating="positionSwitched"][data-position="DEF"]')).toBe("jk-rotate-atk");
  });
});

// The board swaps to the newest view only once the whole burst has played, and a switch is routinely
// followed by more of it (the turn's end and the next turn's start). Drawn from the held-back view the
// card snapped back to its old pose the moment its 250 ms turn ended, then jumped again when the burst
// drained; `withSwitchPoses` (Game.tsx) holds the pose it turned to instead.
describe("#37 a unit keeps the pose it switched to for the rest of the burst", () => {
  afterEach(() => {
    vi.useRealTimers();
    localStorage.clear();
    __resetSettingsForTests();
  });

  const advance = (ms: number): void => {
    act(() => {
      vi.advanceTimersByTime(ms);
    });
  };

  function burstAfter(from: "ATK" | "DEF", to: "ATK" | "DEF"): void {
    vi.useFakeTimers();
    const started: GameEvent = { type: "turnStarted", player: "p1", turn: 3 };
    const switched: GameEvent = { type: "positionSwitched", instanceId: "u1", position: to };
    const ended: GameEvent = { type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 };
    const theirs: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };
    const at = (position: "ATK" | "DEF", active: "p1" | "p2", turn: number) =>
      baseView({
        active,
        turn,
        you: emptySide("p1", { units: [unit("p1", { instanceId: "u1", position }), null, null, null, null] }),
        opponent: emptySide("p2", { hand: { count: 0 } }),
      });
    const before = withEvents(at(from, "p1", 3), [started]);
    const after = withEvents(at(to, "p2", 4), [started, switched, ended, theirs]);
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);
  }

  const card = (): HTMLElement => screen.getByTestId("card-u1");
  const playing = (): string | null => screen.queryByTestId("animation-queue")?.getAttribute("data-animating") ?? null;

  it("turns from ATK while its entry plays, then stays in DEF while the turn's end and the next turn's start play", () => {
    burstAfter("ATK", "DEF");
    expect(card()).toHaveAttribute("data-animating", "positionSwitched");
    expect(card()).toHaveAttribute("data-position", "ATK");

    advance(ANIMATIONS.positionSwitched.durationMs);
    expect(playing()).toBe("turnEnded");
    expect(card()).not.toHaveAttribute("data-animating");
    expect(card()).toHaveAttribute("data-position", "DEF");
    expect(card().style.transform).toBe("rotate(90deg) scale(0.72)");

    advance(ANIMATIONS.turnEnded.durationMs);
    expect(playing()).toBe("turnStarted");
    expect(card()).toHaveAttribute("data-position", "DEF");

    advance(ANIMATIONS.turnStarted.durationMs);
    expect(playing()).toBeNull();
    expect(card()).toHaveAttribute("data-position", "DEF");
    expect(card().style.transform).toBe("rotate(90deg) scale(0.72)");
  });

  it("turns from DEF back upright and stays upright for the rest of the burst", () => {
    burstAfter("DEF", "ATK");
    expect(card()).toHaveAttribute("data-animating", "positionSwitched");
    expect(card()).toHaveAttribute("data-position", "DEF");

    advance(ANIMATIONS.positionSwitched.durationMs);
    expect(playing()).toBe("turnEnded");
    expect(card()).toHaveAttribute("data-position", "ATK");
    expect(card().style.transform).toBe("");
  });
});
