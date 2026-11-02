// R2722001: a Mythic's entrance on the board. With motion, the entry in flight holds for the takeover
// (`data-takeover`, `--takeover-hold`); under Reduce motion there is no hold, no shake and no flash,
// and a still card stands over the board for TAKEOVER_STILL_MS instead.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TAKEOVER_HOLD_MS } from "./animations.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import { TAKEOVER_STILL_MS } from "./TakeoverStill.tsx";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, withEvents } from "../test/fixtures.ts";

const lookup = lookupFromDefs(CATALOG);
/** #97 Zephrys, a Mythic Spell; #52 Silly Silas, a Legendary Unit. */
const ZEPHRYS = "core-097";
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
    expect(game.style.getPropertyValue("--takeover-hold")).toBe(`${String(TAKEOVER_HOLD_MS)}ms`);
    expect(screen.queryByTestId("takeover-still")).toBeNull();
  });

  it("R2722001 under reduced motion the viewer's Mythic play shows a still card and no data-takeover", () => {
    writeSettings({ reduceMotion: true });
    playOut(ZEPHRYS);
    const game = screen.getByTestId("game");
    expect(game).not.toHaveAttribute("data-takeover");
    expect(game).not.toHaveAttribute("data-animating");
    const still = screen.getByTestId("takeover-still");
    expect(still).toHaveAttribute("data-takeover-def", ZEPHRYS);
    expect(still).toHaveAttribute("aria-hidden", "true");
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
