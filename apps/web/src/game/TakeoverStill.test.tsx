// R2722001: a Mythic's entrance on the board. With motion, the entry in flight holds for the takeover
// (`data-takeover`, and since R2785000 its set piece's `data-set-piece` and `--set-piece-hold`); under
// Reduce motion there is no hold, no shake and no flash, and a still card stands over the board for
// TAKEOVER_STILL_MS instead. R3337003: a Mythic play that lays an Anomaly (Anomaly Found!) yields both to
// the Anomaly's rift and its still title card. R3402000: a Mythic whose takeover has art of its own shows it
// at rest there.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANOMALY_SET_PIECE, TAKEOVER_HOLD_MS, TAKEOVER_SET_PIECE } from "./animations.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import TakeoverStill, { TAKEOVER_STILL_MS } from "./TakeoverStill.tsx";
import { TAKEOVER_TINTS, takeoverLookOf } from "../fx/takeover.ts";
import { TAKEOVER_ART, TAKEOVER_ART_ASPECT, TAKEOVER_ART_VIEWBOX } from "../fx/takeoverArt.ts";
import { TAKEOVER_ART_PARTS } from "../fx/takeoverArt.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, withEvents } from "../test/fixtures.ts";

const lookup = lookupFromDefs(CATALOG);
/** #97 Zephrys, a Mythic Spell; #52 Silly Silas, a Legendary Unit. */
const ZEPHRYS = "core-097";
/** R3402000: #98 Heroic Power, a Mythic whose takeover has its own art; Meditative #8 Reach the Summit, a Mythic on the shared look. */
const HEROIC_POWER = "core-098";
const PRISMATIC_MYTHIC = "meditative-008";
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

  it("R3400000 under reduced motion each Mythic with art shows it still", () => {
    writeSettings({ reduceMotion: true });
    for (const [defId, look] of [["meditativeplus-001", "summit"], ["meditativeplus-026", "canvas"], ["traininggrounds-040", "napkin"]] as const) {
      playOut(defId);
      const still = screen.getByTestId("takeover-still");
      expect(still).toHaveAttribute("data-takeover-def", defId);
      const art = still.querySelector(".takeover-art");
      expect(art, defId).not.toBeNull();
      expect(art).toHaveAttribute("data-look", look);
      expect(art).toHaveAttribute("data-still", "true");
      expect([...(art?.children ?? [])].map((part) => part.className), defId).toEqual(TAKEOVER_ART_PARTS[look]?.map((part) => `takeover-art-${part}`));
      // Shapes only: no text, and nothing in it animates.
      expect(art?.textContent, defId).toBe("");
      expect(still.querySelector("[data-animating]")).toBeNull();
      cleanup();
    }
    // A Mythic with no art of its own shows the still card alone.
    playOut(ZEPHRYS);
    expect(screen.getByTestId("takeover-still").querySelector(".takeover-art")).toBeNull();
  });

  it("R2722001 a Legendary takes neither the hold nor the still", () => {
    writeSettings({ reduceMotion: true });
    playOut("core-083");
    expect(screen.queryByTestId("takeover-still")).toBeNull();
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-takeover");
  });

  it("R3402000 reduced motion shows the Mythic's takeover art still", () => {
    const stand = (defId: string): HTMLElement => {
      render(
        <CatalogContext.Provider value={lookup}>
          <TakeoverStill defId={defId} instanceId={`i-${defId}`} view={baseView()} onDone={vi.fn()} />
        </CatalogContext.Provider>,
      );
      return screen.getByTestId("takeover-still");
    };
    // #98 Heroic Power: its rotunda stands first in the still, behind the rays, the card and the ring.
    const still = stand(HEROIC_POWER);
    const art = still.firstElementChild;
    expect(art?.matches('svg.takeover-still-art[data-look="rotunda"]')).toBe(true);
    expect(art?.getAttribute("aria-hidden")).toBe("true");
    expect(art?.getAttribute("viewBox")).toBe(TAKEOVER_ART_VIEWBOX);
    expect(art?.getAttribute("preserveAspectRatio")).toBe(TAKEOVER_ART_ASPECT);
    const shapes = TAKEOVER_ART.rotunda ?? [];
    expect(shapes.length).toBeGreaterThan(0);
    expect([...(art?.children ?? [])].map((shape) => shape.getAttribute("class"))).toEqual(shapes.map((shape) => `takeover-still-art-${shape.part}`));
    expect(art?.querySelectorAll(".takeover-still-art-door")).toHaveLength(13);
    // Painted from its look's tint; it names no card (R202) and holds nothing for the queue (R200).
    expect((art as SVGElement | null)?.style.getPropertyValue("--takeover-glow")).toBe(TAKEOVER_TINTS.rotunda.glow);
    expect(art?.querySelector("text")).toBeNull();
    expect(art?.outerHTML).not.toMatch(/core-|Heroic Power/);
    expect(still.querySelector("[data-animating]")).toBeNull();
    expect(still.querySelector(".takeover-still-card")).not.toBeNull();
    cleanup();

    // Each Core Mythic's own scene stands still under its card.
    for (const [defId, look] of [["core-096", "pawn"], ["core-097", "genie"], ["core-099", "forge"]] as const) {
      const own = stand(defId).querySelector(".takeover-still-art");
      expect(own?.getAttribute("data-look"), defId).toBe(look);
      expect(own?.children.length, defId).toBe(TAKEOVER_ART[look]?.length);
      cleanup();
    }

    // A Mythic on the shared prismatic look keeps the still as it was.
    expect(takeoverLookOf(PRISMATIC_MYTHIC)).toBe("prismatic");
    const plain = stand(PRISMATIC_MYTHIC);
    expect(plain.querySelector(".takeover-still-art")).toBeNull();
    expect(plain.firstElementChild?.classList.contains("takeover-still-rays")).toBe(true);
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
