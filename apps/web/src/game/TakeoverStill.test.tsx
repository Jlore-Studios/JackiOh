// R2722001: a Mythic's entrance on the board. With motion, the entry in flight holds for the takeover
// (`data-takeover`, and since R2785000 its set piece's `data-set-piece` and `--set-piece-hold`); under
// Reduce motion there is no hold, no shake and no flash, and a still card stands over the board for
// TAKEOVER_STILL_MS instead. R3337003: a Mythic play that lays an Anomaly (Anomaly Found!) yields both to
// the Anomaly's rift and its still title card. R3389000: the still wears its card's own takeover look.
// R3405000: and each Meditative Mythic's still draws its own art. R3399002: a Mythic whose look is drawn as a
// picture of its own shows that drawing still behind the card. R3395001: Stinkyscape's five Mythics' stills
// show their art.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG } from "@jackioh/cards";
import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANOMALY_SET_PIECE, TAKEOVER_HOLD_MS, TAKEOVER_SET_PIECE } from "./animations.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import { TAKEOVER_STILL_MS } from "./TakeoverStill.tsx";
import { takeoverArtOf, takeoverLookOf } from "../fx/takeover.ts";
import { TAKEOVER_ART_DRAWINGS, TAKEOVER_ART_PARTS } from "../fx/takeoverArt.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, withEvents } from "../test/fixtures.ts";

const lookup = lookupFromDefs(CATALOG);
/** The board's stylesheet, for the stills' looks. */
const boardCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "board.css"), "utf8");
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
    // R3399002: #97 Zephrys's "genie" look has no drawing of its own, so its still shows none.
    expect(still.querySelector(".takeover-art-drawing")).toBeNull();
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
    const layer = still.querySelector(".takeover-art");
    const art = layer?.querySelector("svg.takeover-art-drawing") ?? null;
    const zealot = TAKEOVER_ART_DRAWINGS.zealot;
    if (layer === null || art === null || zealot === undefined) throw new Error("no drawing");
    // In its look's art layer (R3400000), first, so the card, the rays and the ring lie over it; it carries its
    // look, never the card (R202).
    expect(still.firstElementChild).toBe(layer);
    expect(layer).toHaveAttribute("data-look", "zealot");
    expect(layer).toHaveAttribute("data-still", "true");
    expect(art).toHaveAttribute("viewBox", zealot.viewBox);
    expect(art).toHaveAttribute("preserveAspectRatio", "xMidYMid slice");
    expect(art).toHaveAttribute("aria-hidden", "true");
    expect(layer.outerHTML).not.toContain(ZEALOTISM);
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

  it("R3389000 the still wears its card's own look", () => {
    writeSettings({ reduceMotion: true });
    // In Too Deep, a Mythic Field Spell, lands face up in the backrow: its still wears its abyss.
    playOut("classic-090", [{ type: "summoned", player: "p1", instanceId: "i-classic-090", defId: "classic-090", row: "backrow", lane: 1 }]);
    const still = screen.getByTestId("takeover-still");
    expect(still).toHaveAttribute("data-takeover-def", "classic-090");
    expect(still).toHaveAttribute("data-takeover-look", "abyss");
    expect(still.querySelector("[data-animating]")).toBeNull();
    act(() => {
      vi.advanceTimersByTime(TAKEOVER_STILL_MS);
    });
    cleanup();
    // Zephrys wears its genie; the shared look is "prismatic".
    playOut(ZEPHRYS);
    expect(screen.getByTestId("takeover-still")).toHaveAttribute("data-takeover-look", "genie");
    // The abyss still is drawn at rest: its rules animate nothing.
    const rules = boardCss.split("}").filter((rule) => rule.includes('.takeover-still[data-takeover-look="abyss"]'));
    expect(rules.length).toBeGreaterThan(0);
    for (const rule of rules) expect(rule).not.toMatch(/animation|transition/);
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

  it("R3395001 under reduced motion each Stinkyscape Mythic shows its art still", () => {
    writeSettings({ reduceMotion: true });
    const looks = [
      ["stinkyscape-006", "vault", "units"],
      ["stinkyscape-016", "rod", "backrow"],
      ["stinkyscape-044", "wire", null],
      ["stinkyscape-074", "whiteDwarf", "units"],
      ["stinkyscape-086", "forge", "units"],
    ] as const;
    for (const [defId, look, row] of looks) {
      // A Unit or a Field Spell lands as it is played; a Spell is its cast alone.
      playOut(defId, row === null ? [] : [{ type: "summoned", player: "p1", instanceId: `i-${defId}`, defId, row, lane: 1 }]);
      const still = screen.getByTestId("takeover-still");
      expect(still).toHaveAttribute("data-takeover-def", defId);
      expect(still).toHaveAttribute("data-takeover-look", look);
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
  });
  it("R3405000 under reduced motion each Meditative Mythic shows its own art still", () => {
    writeSettings({ reduceMotion: true });
    const mythics = Object.values(CATALOG).filter((card) => card.id.startsWith("meditative-") && (card.printedRarity ?? card.rarity) === "Mythic");
    expect(mythics).toHaveLength(13);
    for (const card of mythics) {
      const look = takeoverLookOf(card.id);
      // A Unit or a Field Spell enters as it lands face up; a Spell at its play.
      const row = card.type === "Unit" ? "units" : card.type === "Field Spell" ? "backrow" : undefined;
      playOut(card.id, row === undefined ? [] : [{ type: "summoned", player: "p1", instanceId: `i-${card.id}`, defId: card.id, row, lane: 2 }]);
      const art = screen.getByTestId("takeover-still").querySelector(".takeover-art");
      expect(art, card.id).not.toBeNull();
      expect(art).toHaveAttribute("data-look", look);
      expect(art).toHaveAttribute("data-still", "true");
      expect([...(art?.children ?? [])].map((part) => part.className), card.id).toEqual(TAKEOVER_ART_PARTS[look]?.map((part) => `takeover-art-${part}`));
      expect(art?.textContent, card.id).toBe("");
      cleanup();
    }
  });

  it("R3407000 under reduced motion each Stinkyscape Mythic Spell shows its own picture still, behind the rays", () => {
  writeSettings({ reduceMotion: true });
  for (const defId of ["stinkyscape-148-1", "stinkyscape-t-red-narcor"]) {
    playOut(defId);
    const still = screen.getByTestId("takeover-still");
    expect(still).toHaveAttribute("data-takeover-def", defId);
    const picture = still.querySelector("img.takeover-still-art");
    expect(picture, defId).not.toBeNull();
    expect(picture).toHaveAttribute("src", takeoverArtOf(defId));
    expect(picture).toHaveAttribute("alt", "");
    expect(picture).toHaveAttribute("aria-hidden", "true");
    // Behind the rays disc, and nothing in the still animates.
    expect(still.firstElementChild, defId).toBe(picture);
    expect(still.querySelector("[data-animating]")).toBeNull();
    cleanup();
  }
  // A Mythic whose look draws no picture of its own shows none.
  playOut(ZEPHRYS);
  expect(screen.getByTestId("takeover-still").querySelector("img.takeover-still-art")).toBeNull();
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

  it("R3345000 under reduced motion an opponent's public Mythic play shows TakeoverStill with the card's own picture", () => {
    writeSettings({ reduceMotion: true });
    const oppCast: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "i-opp-zephrys", defId: ZEPHRYS, costPaid: 3 };
    const before = withEvents(baseView(), START);
    const { rerender } = render(
      <CatalogContext.Provider value={lookup}>
        <Game view={before} legal={[]} onAction={vi.fn()} />
      </CatalogContext.Provider>,
    );
    rerender(
      <CatalogContext.Provider value={lookup}>
        <Game view={withEvents(baseView(), [...START, oppCast])} legal={[]} onAction={vi.fn()} />
      </CatalogContext.Provider>,
    );
    const still = screen.getByTestId("takeover-still");
    expect(still).toHaveAttribute("data-takeover-def", ZEPHRYS);
    act(() => {
      vi.advanceTimersByTime(TAKEOVER_STILL_MS);
    });
    expect(screen.queryByTestId("takeover-still")).toBeNull();
  });
});
