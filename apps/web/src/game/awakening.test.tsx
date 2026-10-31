// #2435: an `awakened` entry breaks the Dormant seal (animations.css) and `data-awakening`
// holds it gone until the board catches up, through a Starship's launch (awakening.ts).

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { GameEvent, UnitView } from "@jackioh/shared";

import { ANIMATIONS } from "./animations.ts";
import { testid, type AnimatingMap, type AnimationFrames } from "./contract.ts";
import Game from "./Game.tsx";
import { awakeningsFrom } from "./awakening.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { renderBoard } from "../test/board.tsx";
import { baseView, emptySide, unit, withEvents } from "../test/fixtures.ts";
import { setReducedMotion } from "../test/setup.ts";
import { must } from "../test/stubServer.ts";
import { STARSHIP_GLYPH } from "../cards/cardState.ts";

function readSheet(fromWeb: string): string {
  for (const candidate of [fromWeb, `apps/web/${fromWeb}`]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, " ");
  }
  throw new Error(`${fromWeb} not found from ${process.cwd()}`);
}

const css = readSheet("src/game/animations.css");

function animationNameFor(selector: string): string | undefined {
  const escaped = selector.replace(/[[\]".=()]/g, "\\$&");
  return new RegExp(`${escaped}\\s*\\{[^}]*?animation-name:\\s*([\\w-]+);`).exec(css)?.[1];
}

function opacityAt(name: string, at: "0%" | "100%"): string {
  const block = new RegExp(`@keyframes ${name}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(css)?.[1];
  if (block === undefined) throw new Error(`@keyframes ${name} not found`);
  const found = new RegExp(`${at}\\s*\\{[^}]*?opacity:\\s*([^;]+);`).exec(block)?.[1];
  if (found === undefined) throw new Error(`${name} has no opacity at ${at}`);
  return found.trim();
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

const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];

const AWAKENED: GameEvent = { type: "awakened", player: "p1", instanceId: "u1", defId: "core-004" };
const LAUNCHED: GameEvent = {
  type: "starshipLaunched",
  player: "p1",
  instanceId: "u1",
  defId: "core-004",
  name: "Ember of Departure",
  pieces: ["core-008"],
};

function boardWith(u1Over: Partial<UnitView> = {}, events: GameEvent[] = START) {
  return withEvents(
    baseView({
      you: emptySide("p1", {
        units: [unit("p1", { instanceId: "u1", ...u1Over }), unit("p1", { instanceId: "u2" }), null, null, null],
      }),
      opponent: emptySide("p2", { hand: { count: 0 } }),
    }),
    events,
  );
}

const PIECES = [{ defId: "core-008" }];
const DORMANT_STARSHIP: Partial<UnitView> = { slumber: {}, starship: { pieces: PIECES }, canAct: false };
const LAUNCHED_STARSHIP: Partial<UnitView> = { starship: { pieces: PIECES, name: "Ember of Departure", launched: 1 } };

describe("#2435 awakeningsFrom", () => {
  it("#2435 awakeningsFrom marks the card an awakened entry played on, and keeps it after the entry", () => {
    const view = boardWith();
    const card = testid.card("u1");
    const awakenedFrames: AnimatingMap = new Map([[card, "awakened"]]);
    const awakenedEntry: AnimationFrames = { frames: awakenedFrames, events: [AWAKENED] };
    expect(awakeningsFrom(view, awakenedFrames, [awakenedEntry]).has(card)).toBe(true);

    const buffed: GameEvent = { type: "buffed", instanceId: "u2", attack: 1, health: 1 };
    const laterFrames: AnimatingMap = new Map([[testid.card("u2"), "buffed"]]);
    const laterEntry: AnimationFrames = { frames: laterFrames, events: [buffed] };
    const kept = awakeningsFrom(view, laterFrames, [awakenedEntry, laterEntry]);
    expect([...kept]).toEqual([card]);

    expect(awakeningsFrom(view, laterFrames, [laterEntry]).size).toBe(0);
    expect(awakeningsFrom(view, undefined).size).toBe(0);
  });

  it("#2435 without a burst awakeningsFrom reads the shown view's events, on the card animating awakened alone", () => {
    const view = boardWith();
    const card = testid.card("u1");
    const animating: AnimatingMap = new Map([[card, "awakened"]]);
    expect(awakeningsFrom(withEvents(view, [AWAKENED]), animating).has(card)).toBe(true);
    const otherCard: AnimatingMap = new Map([[testid.card("u2"), "awakened"]]);
    expect(awakeningsFrom(withEvents(view, [AWAKENED]), otherCard).size).toBe(0);
    const otherType: AnimatingMap = new Map([[card, "buffed"]]);
    expect(awakeningsFrom(withEvents(view, [AWAKENED]), otherType).size).toBe(0);
  });
});

describe("#2435 the board marks an awakened card", () => {
  it("#2435 the board writes data-awakening on the awakened card and on no other", () => {
    const card = testid.card("u1");
    const animating: AnimatingMap = new Map([[card, "awakened"]]);
    renderBoard(boardWith({}, [AWAKENED]), { animating });
    expect(screen.getByTestId(card)).toHaveAttribute("data-animating", "awakened");
    expect(screen.getByTestId(card)).toHaveAttribute("data-awakening", "true");
    expect(screen.getByTestId(testid.card("u2"))).not.toHaveAttribute("data-awakening");
  });

  it("#2435 an awakening plays the release on the sealed card, holds it through the Starship's launch, then the board catches up", () => {
    const { rerender } = render(<Game view={boardWith(DORMANT_STARSHIP, START)} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={boardWith(LAUNCHED_STARSHIP, [...START, AWAKENED, LAUNCHED])} legal={[]} onAction={vi.fn()} />);
    const card = screen.getByTestId("card-u1");
    expect(card).toHaveAttribute("data-animating", "awakened");
    expect(card).toHaveAttribute("data-awakening", "true");
    expect(must(card.querySelector(".cf-dormant"), "the seal mid-release")).not.toBeNull();

    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.awakened.durationMs);
    });
    expect(screen.getByTestId("card-u1")).toHaveAttribute("data-animating", "starshipLaunched");
    expect(screen.getByTestId("card-u1")).toHaveAttribute("data-awakening", "true");
    expect(screen.getByTestId("card-u1").querySelector('.cf-state[data-state="starship"]')?.textContent).toBe(
      `${STARSHIP_GLYPH}1`,
    );

    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.starshipLaunched.durationMs);
    });
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-animating");
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-awakening");
    expect(screen.getByTestId("card-u1").querySelector(".cf-dormant")).toBeNull();
    expect(screen.getByTestId("card-u1").querySelector(".cf[data-dormant]")).toBeNull();
  });

  it("#2435 under prefers-reduced-motion an awakening drains at once and the seal is gone", () => {
    setReducedMotion(true);
    const { rerender } = render(<Game view={boardWith(DORMANT_STARSHIP, START)} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={boardWith(LAUNCHED_STARSHIP, [...START, AWAKENED, LAUNCHED])} legal={[]} onAction={vi.fn()} />);
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-animating");
    expect(screen.getByTestId("card-u1")).not.toHaveAttribute("data-awakening");
    expect(screen.getByTestId("card-u1").querySelector(".cf-dormant")).toBeNull();
  });
});

describe("#2435 the release keyframes", () => {
  it("#2435 animations.css breaks the ribbons and lifts the veil over the awakened row's duration", () => {
    expect(animationNameFor('[data-animating="awakened"] .cf-dormant-veil')).toBe("jk-dormant-lift");
    expect(animationNameFor('[data-animating="awakened"] .cf-dormant-bands')).toBe("jk-dormant-break");
    const shared = /\[data-animating="awakened"\][\s\S]*?\.cf-dormant-bands\s*\{[^}]*?calc\((\d+)ms/.exec(css)?.[1];
    expect(Number(shared)).toBe(ANIMATIONS.awakened.durationMs);
    expect(opacityAt("jk-dormant-break", "0%")).toBe("1");
    expect(opacityAt("jk-dormant-break", "100%")).toBe("0");
    expect(opacityAt("jk-dormant-lift", "0%")).toBe("1");
    expect(opacityAt("jk-dormant-lift", "100%")).toBe("0");
  });

  it("#2435 a card the burst awakened holds its seal gone after its entry", () => {
    expect(/\[data-awakening\]:not\(\[data-animating="awakened"\]\)\s*\.cf-dormant\s*\{[^}]*?opacity:\s*0;/.test(css)).toBe(true);
  });
});
