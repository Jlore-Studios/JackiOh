// R2722001: a Mythic's entrance on the board. With motion, the entry in flight holds for the takeover
// (`data-takeover`, and since R2785000 its set piece's `data-set-piece` and `--set-piece-hold`); under
// Reduce motion there is no hold, no shake and no flash, and a still card stands over the board for
// TAKEOVER_STILL_MS instead. R3399002: a Mythic whose look has a picture of its own shows it still behind the card.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TAKEOVER_HOLD_MS, TAKEOVER_SET_PIECE } from "./animations.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import { TAKEOVER_STILL_MS } from "./TakeoverStill.tsx";
import { TAKEOVER_ART } from "../fx/takeoverArt.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, withEvents } from "../test/fixtures.ts";

const lookup = lookupFromDefs(CATALOG);
/** #97 Zephrys, a Mythic Spell; #52 Silly Silas, a Legendary Unit. */
const ZEPHRYS = "core-097";
/** Classic+'s Zephrys Zealotism, a Mythic Spell whose takeover wears the "zealot" look (R3399002). */
const ZEALOTISM = "classicplus-027";
const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];
const cast = (defId: string): GameEvent => ({ type: "cardPlayed", player: "p1", instanceId: `i-${defId}`, defId, costPaid: 3 });

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
});

function playOut(defId: string): void {
  const before = withEvents(baseView(), START);
  const { rerender } = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={before} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  rerender(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(baseView(), [...START, cast(defId)])} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
}

describe("R2722001 a Mythic's takeover on the board", () => {
  it("R2722001 with motion the viewer's Mythic play holds for the takeover, and shows no still card", () => {
    playOut(ZEPHRYS);
    const game = screen.getByTestId("game");
    expect(game).toHaveAttribute("data-takeover", "mythic");
    expect(game).toHaveAttribute("data-set-piece", TAKEOVER_SET_PIECE);
    expect(game.style.getPropertyValue("--set-piece-hold")).toBe(`${String(TAKEOVER_HOLD_MS)}ms`);
    expect(screen.queryByTestId("takeover-still")).toBeNull();
  });

  it("R2722001 R2803000 under reduced motion the viewer's Mythic play shows a still card and no data-takeover", () => {
    writeSettings({ reduceMotion: true });
    playOut(ZEPHRYS);
    const game = screen.getByTestId("game");
    expect(game).not.toHaveAttribute("data-takeover");
    expect(game).not.toHaveAttribute("data-animating");
    const still = screen.getByTestId("takeover-still");
    expect(still).toHaveAttribute("data-takeover-def", ZEPHRYS);
    expect(still).toHaveAttribute("aria-hidden", "true");
    expect(still.querySelector("[data-animating]")).toBeNull();
    // R2803000: the still echoes the takeover at rest: a motionless rays disc and the landing ring.
    expect(still.querySelector(".takeover-still-rays")).not.toBeNull();
    expect(still.querySelector(".takeover-still-ring")).not.toBeNull();
    // R3399002: #97 Zephrys's "genie" look has no picture of its own, so its still shows none.
    expect(still.querySelector(".takeover-still-art")).toBeNull();
    act(() => {
      vi.advanceTimersByTime(TAKEOVER_STILL_MS);
    });
    expect(screen.queryByTestId("takeover-still")).toBeNull();
  });

  it("R3399002 under Reduce motion Zephrys Zealotism's still shows its art and animates nothing", () => {
    writeSettings({ reduceMotion: true });
    playOut(ZEALOTISM);
    const game = screen.getByTestId("game");
    expect(game).not.toHaveAttribute("data-takeover");
    expect(game).not.toHaveAttribute("data-animating");
    const still = screen.getByTestId("takeover-still");
    const art = still.querySelector("svg.takeover-still-art");
    const zealot = TAKEOVER_ART.zealot;
    if (art === null || zealot === undefined) throw new Error("no picture");
    // First, so the card, the rays and the ring lie over it; it carries its look, never the card (R202).
    expect(still.firstElementChild).toBe(art);
    expect(art).toHaveAttribute("data-takeover-art", "zealot");
    expect(art).toHaveAttribute("viewBox", zealot.viewBox);
    expect(art).toHaveAttribute("preserveAspectRatio", "xMidYMid slice");
    expect(art).toHaveAttribute("aria-hidden", "true");
    expect(art.outerHTML).not.toContain(ZEALOTISM);
    const paths = [...art.querySelectorAll("path")];
    expect(paths.map((path) => [path.getAttribute("d"), path.getAttribute("fill")])).toEqual(zealot.paths.map((path) => [path.d, path.fill]));
    // Nothing in it moves: no effects-layer class, no SMIL, no inline style, no data-animating.
    expect(art.querySelector("[class], [style], animate, animateTransform, [data-animating]")).toBeNull();
    expect(still.querySelector("[data-animating]")).toBeNull();
    act(() => {
      vi.advanceTimersByTime(TAKEOVER_STILL_MS);
    });
    expect(screen.queryByTestId("takeover-still")).toBeNull();
  });

  it("R2722001 a Legendary takes neither the hold nor the still", () => {
    writeSettings({ reduceMotion: true });
    playOut("core-083");
    expect(screen.queryByTestId("takeover-still")).toBeNull();
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-takeover");
  });
});
