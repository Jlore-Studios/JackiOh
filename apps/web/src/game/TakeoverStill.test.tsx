// R2722001: a Mythic's entrance on the board. With motion, the entry in flight holds for the takeover
// (`data-takeover`, and since R2785000 its set piece's `data-set-piece` and `--set-piece-hold`); under
// Reduce motion there is no hold, no shake and no flash, and a still card stands over the board for
// TAKEOVER_STILL_MS instead. R3337003: a Mythic play that lays an Anomaly (Anomaly Found!) yields both to
// the Anomaly's rift and its still title card.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANOMALY_SET_PIECE, TAKEOVER_HOLD_MS, TAKEOVER_SET_PIECE } from "./animations.ts";
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

function playOut(defId: string, after: GameEvent[] = []): void {
  const before = withEvents(baseView(), START);
  const { rerender } = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={before} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  rerender(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(baseView(), [...START, cast(defId), ...after])} legal={[]} onAction={vi.fn()} />
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

/** Anomaly Found!, a token Spell that prints Mythic, and the Wormhole it lays (R3337000). */
const ANOMALY_FOUND = "stinkyscape-t-anomaly-found";
const LAID: GameEvent[] = [
  { type: "boardReplaced", player: "p1", anomaly: "WH" },
  { type: "cardResolved", player: "p1", instanceId: `i-${ANOMALY_FOUND}`, defId: ANOMALY_FOUND, permanent: false, costPaid: 3 },
];

describe("R3337003 a Mythic play that lays an Anomaly yields its takeover to the rift", () => {
  it("R3337003 with motion the play holds no takeover, and the rift holds as the action's set piece", () => {
    playOut(ANOMALY_FOUND, LAID);
    const game = screen.getByTestId("game");
    const seen = new Set<string>();
    for (let step = 0; step < 80 && screen.queryByTestId("animation-queue") !== null; step += 1) {
      const key = game.getAttribute("data-set-piece");
      if (key !== null) seen.add(key);
      expect(game).not.toHaveAttribute("data-takeover");
      act(() => {
        vi.advanceTimersByTime(100);
      });
    }
    expect([...seen]).toEqual([ANOMALY_SET_PIECE]);
  });

  it("R3337003 under reduced motion it stands no still Mythic card", () => {
    writeSettings({ reduceMotion: true });
    playOut(ANOMALY_FOUND, LAID);
    expect(screen.queryByTestId("takeover-still")).toBeNull();
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-set-piece");
  });
});
