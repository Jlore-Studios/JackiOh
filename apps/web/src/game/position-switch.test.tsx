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

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import Game from "./Game.tsx";
import { baseView, emptySide, unit } from "../test/fixtures.ts";

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
