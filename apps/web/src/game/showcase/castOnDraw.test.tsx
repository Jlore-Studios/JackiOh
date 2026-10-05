// R502: a card cast the moment it was drawn is held up on both seats, under a "Cast on draw!" ribbon,
// longer than a play, bursting out of its drawer's Deck pile; with Game's runner it appears as the
// runner reaches its `cardPlayed`. R436: a Call to Chaos roll is said in words on both seats, and
// shown still where the effects layer draws nothing (CardShowcase.tsx, ChaosBanner.tsx).

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FX_BANNER_TAIL_MS } from "../../fx/constants.ts";
import { resetFxSettingsForTests, setFxSettings } from "../../fx/settings.ts";
import { PRACTICE_SHOWCASE_HOLD_MAX_MS } from "../../practice/config.ts";
import { __resetSettingsForTests, writeSettings } from "../../settings/store.ts";
import { baseView, withEvents } from "../../test/fixtures.ts";
import { ANIMATIONS, createAnimationQueue, type AnimationQueue } from "../animations.ts";
import { CatalogContext, lookupFromDefs } from "../catalog.ts";
import CardShowcase, { chaosHoldMs } from "./CardShowcase.tsx";
import {
  CAST_ON_DRAW_TEXT,
  SHOWCASE_CAST_HOLD_CAP_MS,
  SHOWCASE_CAST_HOLD_MS,
  SHOWCASE_GATE_MAX_MS,
  SHOWCASE_HOLD_MS,
  showcaseCastHoldMs,
  showcaseTestid as T,
} from "./constants.ts";

const lookup = lookupFromDefs(CATALOG);
const HINDER = "core-021";
const HINDER_NAME = CATALOG[HINDER]?.name ?? "";
const BLOOD = "core-027";
const TURN: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };

const drawn = (player: "p1" | "p2", instanceId: string, defId: string): GameEvent => ({ type: "drawn", player, instanceId, defId });
const cast = (player: "p1" | "p2", instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 0 });

function withCatalog(node: ReactElement): ReactElement {
  return <CatalogContext.Provider value={lookup}>{node}</CatalogContext.Provider>;
}

/** Mounts on a first view, then gives it `view` (with `events` after TURN). */
function mountThen(events: GameEvent[], options: { queue?: AnimationQueue; view?: PlayerView } = {}) {
  const base = options.view ?? baseView();
  const tree = (view: PlayerView) => withCatalog(<CardShowcase view={view} {...(options.queue === undefined ? {} : { queue: options.queue })} />);
  const utils = render(tree(withEvents(base, [TURN])));
  const next = withEvents(base, [TURN, ...events]);
  utils.rerender(tree(next));
  return { next, rerender: (view: PlayerView) => utils.rerender(tree(view)) };
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  resetFxSettingsForTests();
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  resetFxSettingsForTests();
  __resetSettingsForTests();
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
});

describe("R502 the cast on draw is held up", () => {
  it("R502 the opponent's: its face under the ribbon, captioned and said as a cast on draw", () => {
    mountThen([drawn("p2", "c21", HINDER), cast("p2", "c21", HINDER)]);
    const showcase = screen.getByTestId(T.root);
    expect(showcase).toHaveAttribute("data-showcase", "cast");
    expect(showcase).toHaveAttribute("data-showcase-def", HINDER);
    expect(screen.getByTestId(T.ribbon)).toHaveTextContent(CAST_ON_DRAW_TEXT.ribbon);
    expect(screen.getByTestId(T.face)).toContainElement(screen.getByTestId(T.ribbon));
    expect(screen.getByTestId(T.caption)).toHaveTextContent(CAST_ON_DRAW_TEXT.opponent);
    expect(screen.getByTestId(T.live)).toHaveTextContent(`${CAST_ON_DRAW_TEXT.opponent} ${HINDER_NAME}: ${CAST_ON_DRAW_TEXT.said}`);
    expect(showcase.style.pointerEvents).toBe("none");
  });

  it("R502 the viewer's own too, captioned as theirs, though their other plays are never held up", () => {
    // The other play pays its cost: a cost-0 play nested in the cast's still-open play would be one
    // of its casts (issue #124), not an ordinary play.
    const other: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "c3", defId: "core-011", costPaid: 1 };
    mountThen([drawn("p1", "c27", BLOOD), cast("p1", "c27", BLOOD), other]);
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase-def", BLOOD);
    expect(screen.getByTestId(T.caption)).toHaveTextContent(CAST_ON_DRAW_TEXT.you);
    advance(showcaseCastHoldMs(1));
    expect(screen.queryByTestId(T.root)).toBeNull();
  });

  it("R502 R97 a hidden one is a back under the ribbon that names nothing", () => {
    mountThen([drawn("p2", "hidden", "hidden"), cast("p2", "hidden", "hidden")]);
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase", "cast");
    expect(screen.getByTestId(T.root)).not.toHaveAttribute("data-showcase-def");
    expect(screen.getByTestId(T.back)).toContainElement(screen.getByTestId(T.ribbon));
    expect(screen.getByTestId(T.caption)).toHaveTextContent(CAST_ON_DRAW_TEXT.hidden);
    expect(document.body.innerHTML).not.toMatch(/core-\d+/);
  });

  it("R502 holds longer than a play, scaled by the effects speed, and never past the practice hold's cap", () => {
    mountThen([drawn("p2", "c21", HINDER), cast("p2", "c21", HINDER)]);
    advance(SHOWCASE_HOLD_MS);
    expect(screen.getByTestId(T.root), "still up after a play's hold").toBeInTheDocument();
    advance(SHOWCASE_CAST_HOLD_MS - SHOWCASE_HOLD_MS);
    expect(screen.queryByTestId(T.root)).toBeNull();
    expect(showcaseCastHoldMs(1)).toBe(SHOWCASE_CAST_HOLD_MS);
    expect(showcaseCastHoldMs(2)).toBe(SHOWCASE_CAST_HOLD_MS / 2);
    expect(showcaseCastHoldMs(0.25)).toBe(SHOWCASE_CAST_HOLD_CAP_MS);
    expect(SHOWCASE_CAST_HOLD_CAP_MS).toBeLessThan(PRACTICE_SHOWCASE_HOLD_MAX_MS);
    setFxSettings({ speed: 2 });
    mountThen([drawn("p2", "c22", HINDER), cast("p2", "c22", HINDER)]);
    advance(SHOWCASE_CAST_HOLD_MS / 2);
    expect(screen.queryAllByTestId(T.root)).toHaveLength(0);
  });

  it("R502 reduced motion still holds it up, still, for the same hold", () => {
    writeSettings({ reduceMotion: true });
    mountThen([drawn("p2", "c21", HINDER), cast("p2", "c21", HINDER)]);
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-motion", "reduce");
    expect(screen.getByTestId(T.ribbon)).toBeInTheDocument();
    advance(SHOWCASE_CAST_HOLD_MS);
    expect(screen.queryByTestId(T.root)).toBeNull();
  });

  it("R502 bursts out of the drawer's Deck pile: the card starts on the pile's centre, measured before the first paint", () => {
    const pile = document.createElement("div");
    pile.setAttribute("data-testid", "library-opponent");
    document.body.appendChild(pile);
    const rect = (left: number, top: number, width: number, height: number): DOMRect =>
      ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) }) as DOMRect;
    pile.getBoundingClientRect = () => rect(900, 40, 60, 80);
    const spy = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      return this === pile ? rect(900, 40, 60, 80) : rect(20, 200, 200, 280);
    });
    mountThen([drawn("p2", "c21", HINDER), cast("p2", "c21", HINDER)]);
    const face = screen.getByTestId(T.face);
    expect(face.style.getPropertyValue("--showcase-from-x")).toBe(`${String(930 - 120)}.0px`);
    expect(face.style.getPropertyValue("--showcase-from-y")).toBe(`${String(80 - 340)}.0px`);
    expect(face.style.getPropertyValue("--showcase-from-s")).toBe((80 / 280).toFixed(3));
    spy.mockRestore();
    pile.remove();
  });
});

describe("R502 with Game's runner, it appears as the runner reaches its cardPlayed", () => {
  function manual(): { queue: AnimationQueue; tick: () => void } {
    const pending: (() => void)[] = [];
    const queue = createAnimationQueue({ schedule: (fn) => pending.push(fn), reducedMotion: false });
    return { queue, tick: () => act(() => pending.shift()?.()) };
  }

  it("R502 waits through the draw, shows on the cast's own start, and keeps an ordinary play's timing", () => {
    const { queue, tick } = manual();
    const events = [drawn("p2", "c21", HINDER), cast("p2", "c21", HINDER)];
    const { next } = mountThen(events, { queue });
    expect(screen.queryByTestId(T.root), "not before the runner has it").toBeNull();
    // Game feeds the runner the same objects the view holds.
    act(() => queue.enqueue(next.events.slice(1), next));
    expect(queue.inFlight()?.type).toBe("drawn");
    expect(screen.queryByTestId(T.root)).toBeNull();
    tick();
    expect(queue.inFlight()?.type).toBe("cardPlayed");
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase", "cast");
  });

  it("R502 the runner going idle at once (reduced motion drains it) lets it go straight away", () => {
    const queue = createAnimationQueue({ schedule: () => undefined, reducedMotion: true });
    const { next } = mountThen([drawn("p2", "c21", HINDER), cast("p2", "c21", HINDER)], { queue });
    expect(screen.queryByTestId(T.root)).toBeNull();
    act(() => queue.enqueue(next.events.slice(1), next));
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase", "cast");
  });

  it("R502 a runner that never reaches it lets it go after SHOWCASE_GATE_MAX_MS", () => {
    const { queue } = manual();
    mountThen([drawn("p2", "c21", HINDER), cast("p2", "c21", HINDER)], { queue });
    advance(SHOWCASE_GATE_MAX_MS - 1);
    expect(screen.queryByTestId(T.root)).toBeNull();
    advance(1);
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase", "cast");
  });

  it("R502 an ordinary play of the opponent is still up as its view arrives", () => {
    const { queue } = manual();
    mountThen([cast("p2", "c7", "core-032")], { queue });
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-showcase", "played");
  });
});

describe("R436 a Call to Chaos roll, in words on both seats", () => {
  const roll = (player: "p1" | "p2"): GameEvent => ({ type: "chaosRolled", player, instanceId: "c95", defId: "core-095", effects: ["Summon 3 random (3) Cost Units", "Heal your hero 30", "Summon a Chaos Golem"] });
  const SAID = "Call to Chaos rolled: Summon 3 random (3) Cost Units, Heal the caster's hero 30, Summon a Chaos Golem";

  it("R436 the live region says every effect in order, on the caster's seat and the other's alike", () => {
    mountThen([roll("p2")]);
    expect(screen.getByTestId(T.chaosLive)).toHaveTextContent(SAID);
    expect(screen.getByTestId(T.chaosLive)).toHaveAttribute("aria-live", "polite");
    cleanup();
    mountThen([roll("p1")]);
    expect(screen.getByTestId(T.chaosLive)).toHaveTextContent(SAID);
  });

  it("R436 with the effects layer drawing, no still banner: the layer's reveal is the one drawn", () => {
    mountThen([roll("p2")]);
    expect(screen.queryByTestId(T.chaos)).toBeNull();
  });

  it("R436 under reduced motion a plain still banner lists them, for as long as the reveal would stand, and goes", () => {
    writeSettings({ reduceMotion: true });
    mountThen([roll("p2")]);
    const banner = screen.getByTestId(T.chaos);
    expect(screen.getAllByTestId(T.chaosLine).map((line) => line.textContent)).toEqual([
      "Summon 3 random (3) Cost Units",
      "Heal the caster's hero 30",
      "Summon a Chaos Golem",
    ]);
    expect(banner.style.pointerEvents).toBe("none");
    expect(banner).toHaveAttribute("aria-hidden", "true");
    expect(chaosHoldMs(1)).toBe(ANIMATIONS.chaosRolled.durationMs + FX_BANNER_TAIL_MS);
    advance(chaosHoldMs(1) - 1);
    expect(screen.getByTestId(T.chaos)).toBeInTheDocument();
    advance(1);
    expect(screen.queryByTestId(T.chaos)).toBeNull();
  });

  it("R436 with the effects off the still banner is drawn too", () => {
    setFxSettings({ intensity: "off" });
    mountThen([roll("p1")]);
    expect(screen.getByTestId(T.chaos)).toBeInTheDocument();
  });
});
