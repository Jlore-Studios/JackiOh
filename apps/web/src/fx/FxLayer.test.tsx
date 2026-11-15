// Polish 1 (docs/polish/1-animations.md, S9): the effects layer, B36 to B38.
// The layer renders as `Game` renders it, a direct child of `.game`, next to a real animation
// runner whose `schedule` the test fires by hand; every browser API comes in through `seams`.

import type { CardDef, CardDefs, GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  ANIMATIONS,
  BURST_BUDGET_MS,
  animTestid,
  SET_PIECE_HOLD_MS,
  STARSHIP_SET_PIECE,
  createAnimationQueue,
  type AnimationEntry,
  type AnimationQueue,
  type RunnerSignal,
} from "../game/animations.ts";
import { CatalogContext, lookupFromDefs, type CardLookup } from "../game/catalog.ts";
import Game from "../game/Game.tsx";
import { resultRevealMs } from "../game/Result.tsx";
import { baseView, dormantStarship, emptySide, fullBoardView, pendingFor, waitingPending, withEvents } from "../test/fixtures.ts";
import { setReducedMotion } from "../test/setup.ts";
import { FX_CALLOUT_MS, FX_MOBILE_WIDTH, FX_NEXT_REFRESH_MODIFIER_ID, FX_STACK_LAND, FX_STARSHIP_STILL_MS, FX_TEXT } from "./constants.ts";
import FxLayer, {
  FX_BANNER_MARK,
  FX_HOLD_MARK,
  FX_LAYER_MARK,
  catalogFacts,
  defaultFrames,
  defaultVisibility,
  type FxSeams,
} from "./FxLayer.tsx";
import { HINDERED_ATTR, applyManaMarks, manaMarks } from "./manaMarks.ts";
import { resetFxSettingsForTests, setFxSettings, useFxSettings } from "./settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { resetShedForTests, shedPointer, subscribeShed } from "./shed.ts";
import { starshipClaim } from "./starship.ts";
import type { FxSurface } from "./surface.ts";
import type { FxAnchor, FxBox, FxFrameSource, FxShakeSink, FxVisibility } from "./types.ts";
import { ROOT_VARS_ATTRIBUTE, readRootVar } from "../game/rootVars.ts";

beforeEach(() => {
  localStorage.clear();
  resetFxSettingsForTests();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setReducedMotion(false);
  resetFxSettingsForTests();
  localStorage.clear();
});

// Seams

/** A `schedule` spy: every call is logged, and the callbacks run only when the test says so. */
function fakeSchedule() {
  const pending: { ms: number; run: () => void }[] = [];
  const log: number[] = [];
  return {
    schedule: (fn: () => void, ms: number): void => {
      pending.push({ ms, run: fn });
      log.push(ms);
    },
    log,
    pending: (): number => pending.length,
    tick(): void {
      const next = pending.shift();
      if (next === undefined) throw new Error("nothing scheduled");
      next.run();
    },
    flush(): void {
      for (let guard = 0; pending.length > 0; guard += 1) {
        if (guard > 10_000) throw new Error("schedule loop did not terminate");
        pending.shift()?.run();
      }
    },
  };
}

/** A 2D context with every method and property; see director.test.ts for the same stub. */
function recordingContext(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const state: Record<string, unknown> = {
    canvas,
    globalAlpha: 1,
    globalCompositeOperation: "source-over",
    fillStyle: "#000000",
    strokeStyle: "#000000",
    lineWidth: 1,
    lineCap: "butt",
    lineJoin: "miter",
    filter: "none",
    font: "10px sans-serif",
    shadowBlur: 0,
    shadowColor: "rgba(0, 0, 0, 0)",
    imageSmoothingEnabled: true,
  };
  const gradient = { addColorStop: (): void => undefined };
  return new Proxy(state, {
    get(target, key) {
      if (typeof key !== "string") return undefined;
      if (key in target) return target[key];
      return (): unknown => {
        if (key.startsWith("create") && key.endsWith("Gradient")) return gradient;
        if (key === "createPattern") return null;
        if (key === "measureText") return { width: 0 };
        if (key === "getTransform") return { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
        if (key === "getLineDash") return [];
        if (key === "getImageData" || key === "createImageData") {
          return { data: new Uint8ClampedArray(4), width: 1, height: 1 };
        }
        return undefined;
      };
    },
    set(target, key, value) {
      if (typeof key === "string") target[key] = value;
      return true;
    },
  }) as unknown as CanvasRenderingContext2D;
}

function stubSurface(canvas: HTMLCanvasElement): FxSurface {
  return {
    canvas,
    ctx: recordingContext(canvas),
    dpr: () => 1,
    width: () => 1280,
    height: () => 720,
    resize: () => undefined,
    clear: () => undefined,
    dispose: () => undefined,
  };
}

const BOX: FxBox = { x: 300, y: 200, width: 90, height: 126 };

/** R2801000, R3334001: p1's Dormant Starship of two pieces in lane 3, and its launch. */
const shipView = (): PlayerView =>
  baseView({ you: emptySide("p1", { units: [null, null, dormantStarship("p1", ["stinkyscape-140", "stinkyscape-050"], { instanceId: "ship" }), null, null] }) });
const SHIP_LAUNCH: GameEvent = {
  type: "starshipLaunched",
  player: "p1",
  instanceId: "ship",
  defId: "stinkyscape-t-dormant-starship",
  name: "Ember of Departure",
  pieces: ["stinkyscape-140", "stinkyscape-050"],
};

function fakeSeams() {
  let t = 50_000;
  let nextHandle = 1;
  const pending = new Map<number, (timestampMs: number) => void>();
  const frames: FxFrameSource = {
    request(callback) {
      const handle = nextHandle;
      nextHandle += 1;
      pending.set(handle, callback);
      return handle;
    },
    cancel(handle) {
      pending.delete(handle);
    },
  };
  const visibility: FxVisibility = { hidden: () => false, subscribe: () => () => undefined };
  const measured: FxAnchor[] = [];
  const shakes: ("apply" | "clear")[] = [];
  const shakeSink: FxShakeSink = {
    apply: () => {
      shakes.push("apply");
    },
    clear: () => {
      shakes.push("clear");
    },
  };
  const seams: FxSeams = {
    now: () => t,
    frames,
    visibility,
    measure: (anchor) => {
      measured.push(anchor);
      return BOX;
    },
    surface: (canvas) => stubSurface(canvas),
    shakeSink,
    seed: 11,
    catalog: () => undefined,
    viewportWidth: () => 1280,
    element: () => null,
  };
  function runFrames(): void {
    const due = [...pending.values()];
    pending.clear();
    for (const callback of due) callback(t);
  }
  return {
    seams,
    measured,
    shakes,
    testids: (): string[] => measured.flatMap((anchor) => (anchor.kind === "testid" ? [anchor.testid] : [])),
    pump(ms = 100, step = 10): void {
      act(() => {
        const end = t + ms;
        runFrames();
        while (t + step <= end) {
          t += step;
          runFrames();
        }
      });
    },
  };
}

type Fx = ReturnType<typeof fakeSeams>;

function mountLayer(queue: AnimationQueue, view: PlayerView, fx: Fx) {
  const tree = (shown: PlayerView) => (
    <div className="game">
      <FxLayer queue={queue} view={shown} seams={fx.seams} />
    </div>
  );
  const utils = render(tree(view));
  const game = utils.container.firstElementChild as HTMLElement;
  return {
    game,
    layer: (): HTMLElement | null => game.querySelector<HTMLElement>('[data-testid="fx-layer"]'),
    fxDom: (): HTMLElement | null => game.querySelector<HTMLElement>('[data-testid="fx-dom"]'),
    canvas: (): HTMLElement | null => game.querySelector<HTMLElement>('[data-testid="fx-canvas"]'),
    show: (next: PlayerView): void => {
      utils.rerender(tree(next));
    },
    count: (kind: string): number => game.querySelectorAll(`[data-fx="${kind}"]`).length,
    squeeze: (): string => readRootVar(game, "--anim-squeeze").trim(),
  };
}

function queueWith(schedule: ReturnType<typeof fakeSchedule>, onSettled?: () => void): AnimationQueue {
  return createAnimationQueue({ schedule: schedule.schedule, reducedMotion: false, onSettled });
}

/** The same board with hot-seat's other seat looking at it: p2 is the viewer and the active player. */
function p2View(over: Partial<PlayerView> = {}): PlayerView {
  return baseView({
    viewer: "p2",
    active: "p2",
    you: emptySide("p2"),
    opponent: emptySide("p1", { hand: { count: 4 } }),
    ...over,
  });
}

/* fullBoardView's ids: your units u1..u5, the enemy's u6..u10, your hand c11..c14, your face-up
 * trap b20 in backrow lane 5. u2 carries every keyword; u10 is a 10/10. */
const HIT: GameEvent = { type: "damage", sourceId: "u1", targetId: "u10", amount: 4, combat: true };

describe("#3633 the layer's marks on its game", () => {
  it("#3633 the layer marks its game while its effects are on, a banner is up and a stand-in is held", async () => {
    const queue = queueWith(fakeSchedule());
    const m = mountLayer(queue, fullBoardView(), fakeSeams());
    expect(m.game).toHaveAttribute(FX_LAYER_MARK, "on");
    expect(m.game).not.toHaveAttribute(FX_BANNER_MARK);
    expect(m.game).not.toHaveAttribute(FX_HOLD_MARK);
    // The director mounts both into the layer's DOM root; the marks follow what it holds.
    const banner = document.createElement("div");
    banner.setAttribute("data-fx", "banner");
    const hold = document.createElement("div");
    hold.className = "fx-hold";
    await act(async () => {
      m.fxDom()?.append(banner, hold);
      await Promise.resolve();
    });
    expect(m.game).toHaveAttribute(FX_BANNER_MARK);
    expect(m.game).toHaveAttribute(FX_HOLD_MARK);
    await act(async () => {
      banner.remove();
      await Promise.resolve();
    });
    expect(m.game).not.toHaveAttribute(FX_BANNER_MARK);
    expect(m.game).toHaveAttribute(FX_HOLD_MARK);
    cleanup();
    expect(m.game).not.toHaveAttribute(FX_LAYER_MARK);
    expect(m.game).not.toHaveAttribute(FX_HOLD_MARK);
  });
});

describe("B36 — FxLayer follows the runner without pacing it", () => {
  it("R200 each start writes --anim-squeeze on the layer's parent and idle removes it", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    expect(m.squeeze()).toBe("");

    act(() => queue.enqueue([HIT], view));
    const entry = queue.inFlight();
    expect(entry).not.toBeNull();
    if (entry === null) return;
    expect(m.squeeze()).toBe((entry.durationMs / ANIMATIONS[entry.type].durationMs).toFixed(3));
    expect(m.squeeze()).toBe("1.000");
    // #3633: written onto the elements that read it, never onto the parent's own style.
    expect(m.game.style.getPropertyValue("--anim-squeeze")).toBe("");

    act(() => sched.flush());
    expect(queue.idle()).toBe(true);
    expect(m.squeeze()).toBe("");
  });

  it("B36 the squeeze follows the burst budget entry by entry", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    const traps = Array.from(
      { length: 10 },
      (_, i): GameEvent => ({
        type: "trapFired",
        instanceId: `t${i + 1}`,
        defId: "core-084",
        controller: "p2",
        row: "backrow",
        lane: (i % 5) + 1,
      }),
    );

    act(() => queue.enqueue(traps, view));
    for (let i = 0; i < traps.length; i += 1) {
      const entry = queue.inFlight();
      expect(entry, `entry ${i + 1} is in flight`).not.toBeNull();
      if (entry === null) return;
      const expected = (entry.durationMs / ANIMATIONS[entry.type].durationMs).toFixed(3);
      expect(Number(expected)).toBeLessThan(1);
      expect(m.squeeze()).toBe(expected);
      fx.pump(50);
      act(() => sched.tick());
    }
    expect(m.squeeze()).toBe("");
  });

  it("B36 the squeeze also carries the speed setting", () => {
    setFxSettings({ speed: 2 });
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);

    act(() => queue.enqueue([HIT], view));
    expect(m.squeeze()).toBe("0.500");
  });

  it("B36 the squeeze is written on start even while effects are off", () => {
    setFxSettings({ intensity: "off" });
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);

    act(() => queue.enqueue([HIT], view));
    expect(m.squeeze()).toBe("1.000");
    act(() => sched.flush());
    expect(m.squeeze()).toBe("");
  });

  it("B36 drain() removes the squeeze and every effect on screen", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);

    act(() => queue.enqueue([HIT, HIT], view));
    fx.pump(50);
    expect(m.fxDom()?.childElementCount ?? 0).toBeGreaterThan(0);

    act(() => queue.drain());
    expect(m.squeeze()).toBe("");
    expect(m.fxDom()?.childElementCount).toBe(0);
    fx.pump(2_000);
    expect(m.fxDom()?.childElementCount).toBe(0);
  });

  it("R2785000 a skip cuts the set piece's effects at once and the next entry plays", () => {
    const sched = fakeSchedule();
    // The fixture set piece: the action's first hit (one per action, so the second is a plain entry).
    const queue = createAnimationQueue({
      schedule: sched.schedule,
      reducedMotion: false,
      setPieceOf: (entry) => (entry.type === "damage" ? { key: "fixture", holdMs: SET_PIECE_HOLD_MS } : null),
    });
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);

    act(() => queue.enqueue([HIT, HIT], view));
    expect(queue.inFlight()?.setPiece?.key).toBe("fixture");
    // The hold is a wait before the motion, not a slower motion.
    expect(m.squeeze()).toBe("1.000");
    fx.pump(50);
    expect(m.fxDom()?.childElementCount ?? 0).toBeGreaterThan(0);

    let skipped = false;
    act(() => {
      skipped = queue.skip();
    });
    expect(skipped).toBe(true);
    // Nothing of the set piece is left on screen, and the next hit is in flight with its own squeeze.
    expect(m.fxDom()?.childElementCount).toBe(0);
    expect(fx.shakes.at(-1)).toBe("clear");
    expect(queue.inFlight()?.setPiece).toBeUndefined();
    expect(queue.inFlight()?.type).toBe("damage");
    expect(m.squeeze()).toBe(((queue.inFlight()?.durationMs ?? 0) / ANIMATIONS.damage.durationMs).toFixed(3));
    fx.pump(50);
    expect(m.fxDom()?.childElementCount ?? 0).toBeGreaterThan(0);
    // The skipped entry's timer finds nothing to end; the hit's own ends the burst.
    act(() => sched.tick());
    expect(queue.inFlight()?.type).toBe("damage");
    act(() => sched.tick());
    expect(queue.idle()).toBe(true);
    expect(m.squeeze()).toBe("");
  });

  it("B36 reset() removes the squeeze and every effect on screen", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);

    act(() => queue.enqueue([HIT, HIT], view));
    fx.pump(50);
    expect(m.fxDom()?.childElementCount ?? 0).toBeGreaterThan(0);

    act(() => queue.reset());
    expect(m.squeeze()).toBe("");
    expect(m.fxDom()?.childElementCount).toBe(0);
    fx.pump(2_000);
    expect(m.fxDom()?.childElementCount).toBe(0);
  });

  it("B36 an entry is planned against the view it was enqueued with, not the view the layer is given", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    // The layer shows an empty board, where u10 does not exist.
    const m = mountLayer(queue, baseView(), fx);

    act(() => queue.enqueue([HIT], fullBoardView()));
    fx.pump(50);

    expect(fx.testids()).toContain("card-u10");
    const splat = m.game.querySelector('[data-fx="splat"]');
    expect(splat).not.toBeNull();
    expect(splat?.getAttribute("data-amount")).toBe("-4");
  });

  it("B36 the layer remembers a caster across entries: a spell's damage flies from its caster's hero", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    mountLayer(queue, view, fx);

    act(() => queue.enqueue([{ type: "cardPlayed", player: "p2", instanceId: "s1", defId: "core-050", costPaid: 2 }], view));
    fx.pump(50);
    act(() => sched.flush());
    expect(fx.testids()).not.toContain("hero-opponent");

    act(() => queue.enqueue([{ type: "damage", sourceId: "s1", targetId: "u1", amount: 3, combat: false }], view));
    fx.pump(100);
    expect(fx.testids()).toContain("hero-opponent");
    expect(fx.testids()).toContain("card-u1");
  });

  it("B36 without the caster's cardPlayed, the same spell damage has no source to fly from", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    mountLayer(queue, view, fx);

    act(() => queue.enqueue([{ type: "damage", sourceId: "s1", targetId: "u1", amount: 3, combat: false }], view));
    fx.pump(300);
    expect(fx.testids()).toContain("card-u1");
    expect(fx.testids()).not.toContain("hero-opponent");
  });

  it("R3407002 a hook's marker plays nothing but opens its entry: IN33DH3's Death draws its recipe on the shuffle that follows it", () => {
    // R2797000: the Death's `hookResolving` lasts 0 ms, so only the layer's memory sees it; without it the
    // shuffle is no hook's and draws its plain row alone. The dissolve leaves his graveyard pile (R3407000).
    const graveyard = animTestid.graveyard("you");
    const marker: GameEvent = { type: "hookResolving", sourceId: "gone", defId: "stinkyscape-147", controller: "p1", hook: "death" };
    const shuffle: GameEvent = { type: "shuffledIn", player: "p1", instanceId: "prime", defId: "stinkyscape-147-1", position: 0 };
    const view = fullBoardView();
    const drawn = (events: readonly GameEvent[]): string[] => {
      const sched = fakeSchedule();
      const queue = queueWith(sched);
      const fx = fakeSeams();
      const m = mountLayer(queue, view, fx);
      act(() => queue.enqueue(events, view));
      fx.pump(50);
      const ids = fx.testids();
      m.show(view);
      act(() => sched.flush());
      cleanup();
      return ids;
    };
    expect(drawn([marker, shuffle])).toContain(graveyard);
    expect(drawn([shuffle])).not.toContain(graveyard);
  });

  it("B36 drain() forgets what the layer remembered", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    mountLayer(queue, view, fx);

    act(() => queue.enqueue([{ type: "cardPlayed", player: "p2", instanceId: "s1", defId: "core-050", costPaid: 2 }], view));
    fx.pump(50);
    act(() => queue.drain());

    act(() => queue.enqueue([{ type: "damage", sourceId: "s1", targetId: "u1", amount: 3, combat: false }], view));
    fx.pump(300);
    expect(fx.testids()).toContain("card-u1");
    expect(fx.testids()).not.toContain("hero-opponent");
  });

  it("R200 mounting the layer changes no schedule call the runner makes, and settles it exactly as often", () => {
    const events: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "c12", defId: "core-019", costPaid: 3 },
      { type: "summoned", player: "p1", instanceId: "c12", defId: "core-019", row: "units", lane: 3 },
      { type: "damage", sourceId: "u2", targetId: "u10", amount: 7, combat: false },
      { type: "healed", targetId: "hero-p1", amount: 3 },
      { type: "trapFired", instanceId: "b20", defId: "core-084", controller: "p1", row: "backrow", lane: 5 },
      { type: "destroyed", instanceId: "u9", defId: "core-040", owner: "p2", controller: "p2", attack: 1, maxHealth: 1, killerId: "u2" },
      { type: "manaChanged", player: "p1", current: 4, max: 4 },
      { type: "turnStarted", player: "p2", turn: 4 },
      { type: "gameOver", winner: "p1", reason: "hero-death" },
    ];
    const layered = fakeSchedule();
    const bare = fakeSchedule();
    const settledLayered = vi.fn();
    const settledBare = vi.fn();
    const withLayer = queueWith(layered, settledLayered);
    const without = queueWith(bare, settledBare);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(withLayer, view, fx);

    act(() => withLayer.enqueue(events, view));
    without.enqueue(events, view);

    let mostEffects = 0;
    while (layered.pending() > 0) {
      fx.pump(120);
      mostEffects = Math.max(mostEffects, m.fxDom()?.childElementCount ?? 0);
      act(() => layered.tick());
    }
    bare.flush();

    expect(mostEffects).toBeGreaterThan(0);
    expect(layered.log.length).toBeGreaterThan(0);
    expect(layered.log).toEqual(bare.log);
    expect(settledLayered).toHaveBeenCalledTimes(1);
    expect(settledBare).toHaveBeenCalledTimes(1);
  });
});

// B37: on and off, and nothing the layer renders can be clicked, read or waited on
describe("B37 — when the layer draws, and what it never renders", () => {
  it("B37 with effects enabled it renders data-fx=\"on\", aria-hidden, and holds fx-canvas and fx-dom", () => {
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
    const layer = m.layer();

    expect(layer).not.toBeNull();
    expect(layer?.classList.contains("fx-layer")).toBe(true);
    expect(layer?.getAttribute("data-fx")).toBe("on");
    expect(layer?.getAttribute("aria-hidden")).toBe("true");

    const canvas = m.canvas();
    expect(canvas?.tagName).toBe("CANVAS");
    expect(canvas?.classList.contains("fx-canvas")).toBe(true);
    expect(canvas?.parentElement).toBe(layer);

    const fxDom = m.fxDom();
    expect(fxDom?.classList.contains("fx-dom")).toBe(true);
    expect(fxDom?.parentElement).toBe(layer);
  });

  it("B37 prefers-reduced-motion switches the layer off: no canvas, no cue, and an empty still DOM root (R2789000)", () => {
    setReducedMotion(true);
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);

    expect(m.layer()?.getAttribute("data-fx")).toBe("off");
    expect(m.layer()?.getAttribute("aria-hidden")).toBe("true");
    expect(m.canvas()).toBeNull();
    // R2789000: the one thing it can draw, a keyword's still callout, needs the DOM root.
    expect(m.fxDom()?.getAttribute("data-still")).toBe("true");
    expect(m.fxDom()?.childElementCount).toBe(0);

    act(() => queue.enqueue([HIT], view));
    fx.pump(500);
    expect(fx.measured).toHaveLength(0);
    expect(fx.shakes).not.toContain("apply");
    expect(m.layer()?.childElementCount).toBe(1);
    expect(m.fxDom()?.childElementCount).toBe(0);
  });

  it("B37 the viewer's reduce-motion setting switches the layer off", () => {
    setFxSettings({ motion: "reduce" });
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);

    expect(m.layer()?.getAttribute("data-fx")).toBe("off");
    expect(m.canvas()).toBeNull();
    expect(m.fxDom()?.getAttribute("data-still")).toBe("true");
    expect(m.fxDom()?.childElementCount).toBe(0);
  });

  it("R2789000 under reduced motion a keyword's callout draws still, and nothing at intensity off", () => {
    setReducedMotion(true);
    const cleave: GameEvent = { type: "damage", sourceId: "u1", targetId: "u7", amount: 1, combat: true, via: "Cleave" };
    const view = fullBoardView();
    const fx = fakeSeams();
    const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: true });
    const m = mountLayer(queue, view, fx);

    act(() => queue.enqueue([cleave], view));
    fx.pump(50);
    expect(m.count("callout")).toBe(1);
    const callout = m.game.querySelector<HTMLElement>('[data-fx="callout"]');
    expect(callout?.getAttribute("data-callout")).toBe("Cleave");
    expect(callout?.getAttribute("data-still")).toBe("true");
    // Only the callout: no splat, no shake, no canvas, and it is gone after FX_CALLOUT_MS.
    expect(m.fxDom()?.childElementCount).toBe(1);
    expect(m.canvas()).toBeNull();
    expect(fx.shakes).not.toContain("apply");
    fx.pump(FX_CALLOUT_MS + 100);
    expect(m.count("callout")).toBe(0);

    act(() => {
      setFxSettings({ intensity: "off" });
    });
    expect(m.fxDom()).toBeNull();
    act(() => queue.enqueue([cleave], view));
    fx.pump(100);
    expect(m.count("callout")).toBe(0);
    expect(m.layer()?.childElementCount).toBe(0);
  });

  it("R2801000 R3334001 reduced motion mounts the still title card, holds nothing", () => {
    setReducedMotion(true);
    const view = shipView();
    const fx = fakeSeams();
    const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: true, setPieceOf: starshipClaim });
    const m = mountLayer(queue, view, fx);

    act(() => queue.enqueue([SHIP_LAUNCH], view));
    fx.pump(50);
    // The queue held nothing: it drained at once, and the title card stands on its own.
    expect(queue.idle()).toBe(true);
    expect(m.count("cinematic")).toBe(1);
    const still = m.game.querySelector<HTMLElement>('[data-fx="cinematic"]');
    expect(still?.getAttribute("data-still")).toBe("true");
    expect(still?.querySelector(".fx-cinematic-name")?.getAttribute("data-text")).toBe("Ember of Departure");
    expect(still?.querySelector(".fx-cinematic-art")?.getAttribute("src")).toMatch(/^data:image\/svg\+xml/);
    expect(still?.querySelectorAll(".fx-cinematic-spark")).toHaveLength(0);
    expect(m.count("launch")).toBe(0);
    expect(m.canvas()).toBeNull();
    expect(fx.shakes).not.toContain("apply");
    fx.pump(FX_STARSHIP_STILL_MS + 100);
    expect(m.count("cinematic")).toBe(0);
  });

  it("R3334001 Big moments short and off mount it", () => {
    for (const bigMoments of ["short", "off"] as const) {
      setFxSettings({ bigMoments });
      const view = shipView();
      const fx = fakeSeams();
      const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: false, setPieceOf: starshipClaim });
      const m = mountLayer(queue, view, fx);
      act(() => queue.enqueue([SHIP_LAUNCH], view));
      fx.pump(50);
      // "short" still holds the launch, briefly; "off" holds nothing. Either way, the title card stands still.
      expect(queue.inFlight()?.setPiece !== undefined, bigMoments).toBe(bigMoments === "short");
      const card = m.game.querySelector<HTMLElement>('[data-fx="cinematic"]');
      expect(card?.getAttribute("data-still"), bigMoments).toBe("true");
      expect(card?.querySelectorAll(".fx-cinematic-spark"), bigMoments).toHaveLength(0);
      cleanup();
    }
    // At "full" the held launch is the moving cinematic.
    setFxSettings({ bigMoments: "full" });
    const view = shipView();
    const fx = fakeSeams();
    const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: false, setPieceOf: starshipClaim });
    const m = mountLayer(queue, view, fx);
    act(() => queue.enqueue([SHIP_LAUNCH], view));
    fx.pump(50);
    const cinematic = m.game.querySelector<HTMLElement>('[data-fx="cinematic"]');
    expect(cinematic?.hasAttribute("data-still")).toBe(false);
    expect(cinematic?.querySelectorAll(".fx-cinematic-stop").length).toBeGreaterThan(0);
  });

  it("R3334001 a skip removes the cinematic", () => {
    const view = shipView();
    const fx = fakeSeams();
    const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: false, setPieceOf: starshipClaim });
    const m = mountLayer(queue, view, fx);
    act(() => queue.enqueue([SHIP_LAUNCH], view));
    expect(queue.inFlight()?.setPiece?.key).toBe(STARSHIP_SET_PIECE);
    fx.pump(50);
    expect(m.count("cinematic")).toBe(1);
    let skipped = false;
    act(() => {
      skipped = queue.skip();
    });
    expect(skipped).toBe(true);
    expect(m.count("cinematic")).toBe(0);
    expect(m.fxDom()?.childElementCount).toBe(0);
  });

  it("B37 intensity \"off\" switches the layer off and plans no cue, while the runner still animates", () => {
    setFxSettings({ intensity: "off" });
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);

    expect(m.layer()?.getAttribute("data-fx")).toBe("off");
    expect(m.canvas()).toBeNull();

    act(() => queue.enqueue([HIT], view));
    expect(sched.log).toEqual([ANIMATIONS.damage.durationMs]);
    fx.pump(500);
    expect(fx.measured).toHaveLength(0);
    expect(fx.shakes).not.toContain("apply");
    expect(m.layer()?.childElementCount).toBe(0);
  });

  it("B37 the layer follows the settings live: off, then back on", () => {
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
    expect(m.layer()?.getAttribute("data-fx")).toBe("on");

    act(() => {
      setFxSettings({ intensity: "off" });
    });
    expect(m.layer()?.getAttribute("data-fx")).toBe("off");
    expect(m.canvas()).toBeNull();

    act(() => {
      setFxSettings({ intensity: "high" });
    });
    expect(m.layer()?.getAttribute("data-fx")).toBe("on");
    expect(m.canvas()).not.toBeNull();
    expect(m.fxDom()).not.toBeNull();
  });

  it("B37 nothing inside the layer carries a data-animating of its own, a role, focusable content or a text node", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    const events: GameEvent[] = [
      HIT,
      { type: "healed", targetId: "hero-p1", amount: 3 },
      { type: "buffed", instanceId: "u2", attack: 1, health: 1 },
      { type: "radiantSet", instanceId: "u3", defId: "core-017", zone: { z: "field", player: "p1", row: "units", lane: 3 } },
      { type: "turnStarted", player: "p1", turn: 4 },
    ];

    act(() => queue.enqueue(events, view));
    const kinds = new Set<string>();
    const check = (): void => {
      const layer = m.layer();
      expect(layer).not.toBeNull();
      if (layer === null) return;
      for (const el of layer.querySelectorAll("[data-fx]")) kinds.add(el.getAttribute("data-fx") ?? "");
      expect(layer.hasAttribute("data-animating")).toBe(false);
      expect(layer.querySelectorAll("[data-animating]")).toHaveLength(0);
      expect(layer.hasAttribute("role")).toBe(false);
      expect(layer.querySelectorAll("[role]")).toHaveLength(0);
      expect(
        layer.querySelectorAll("a, button, input, select, textarea, [tabindex], [contenteditable]"),
      ).toHaveLength(0);
      const walker = document.createTreeWalker(layer, NodeFilter.SHOW_TEXT);
      expect(walker.nextNode()).toBeNull();
    };
    while (sched.pending() > 0) {
      fx.pump(120);
      check();
      act(() => sched.tick());
    }
    fx.pump(120);
    check();

    for (const kind of ["splat", "rays", "arrows", "sheen", "banner"]) expect(kinds).toContain(kind);
  });

  it("B37 Game mounts the layer as a direct child of .game and logs no console.error through a burst and a game over", async () => {
    const errors = vi.spyOn(console, "error");
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    try {
      const view = fullBoardView();
      const events: GameEvent[] = [
        { type: "damage", sourceId: "c11", targetId: "u10", amount: 8, combat: false },
        { type: "summoned", player: "p1", instanceId: "c90", defId: "core-019", row: "units", lane: 3 },
        { type: "turnStarted", player: "p1", turn: 4 },
      ];
      const onAction = (): void => undefined;
      const { container, rerender, unmount } = render(<Game view={view} legal={[]} onAction={onAction} />);

      const game = container.querySelector('[data-testid="game"]');
      const layer = container.querySelector('[data-testid="fx-layer"]');
      expect(game).not.toBeNull();
      expect(layer).not.toBeNull();
      expect(layer?.parentElement).toBe(game);
      expect(layer?.getAttribute("data-fx")).toBe("on");

      const busy = withEvents(view, events);
      rerender(<Game view={busy} legal={[]} onAction={onAction} />);
      expect(container.querySelector("[data-animating]")).not.toBeNull();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(BURST_BUDGET_MS + 1);
      });
      expect(container.querySelector("[data-animating]")).toBeNull();
      expect(container.querySelector('[data-testid="fx-layer"] [data-animating]')).toBeNull();

      rerender(<Game view={{ ...busy, result: { winner: "p1", reason: "hero-death" } }} legal={[]} onAction={onAction} />);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(resultRevealMs() + 1);
      });
      unmount();

      expect(errors.mock.calls).toEqual([]);
    } finally {
      errors.mockRestore();
    }
  });
});

describe("R200 — the reduce setting stops CSS-only motion exactly as the media query does", () => {
  it("R200 the reduce setting zeroes --anim-scale on the game root, and turning it back restores it (B51)", () => {
    // index.css zeroes --anim-scale on :root under prefers-reduced-motion; the setting must too, or
    // the result overlay's fade and the board's transitions keep moving.
    setFxSettings({ motion: "reduce" });
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
    expect(m.game.style.getPropertyValue("--anim-scale")).toBe("0");
    expect(m.layer()?.getAttribute("data-fx")).toBe("off");

    act(() => {
      setFxSettings({ motion: "system" });
    });
    expect(m.game.style.getPropertyValue("--anim-scale")).toBe("");
    expect(m.layer()?.getAttribute("data-fx")).toBe("on");
  });

  it("R200 intensity off (motion left to the system) draws nothing but leaves the table's motion alone", () => {
    setFxSettings({ intensity: "off" });
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fakeSeams());
    expect(m.layer()?.getAttribute("data-fx")).toBe("off");
    expect(m.game.style.getPropertyValue("--anim-scale")).toBe("");
  });
});

describe("B38 — the result sequence and the hot-seat hand-over banner", () => {
  it("B38 a result arriving during the mount plays the victory sequence for the winner", () => {
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queueWith(fakeSchedule()), view, fx);
    fx.pump(50);
    expect(m.count("result")).toBe(0);

    m.show({ ...view, result: { winner: "p1", reason: "hero-death" } });
    fx.pump(50);

    expect(m.count("result")).toBe(1);
    const result = m.game.querySelector('[data-fx="result"]');
    expect(result?.getAttribute("data-outcome")).toBe("victory");
    expect(result?.getAttribute("data-text")).toBe(FX_TEXT.victory);
  });

  it("B38 a result against the viewer plays the defeat sequence", () => {
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queueWith(fakeSchedule()), view, fx);

    m.show({ ...view, result: { winner: "p2", reason: "hero-death" } });
    fx.pump(50);

    const result = m.game.querySelector('[data-fx="result"]');
    expect(result?.getAttribute("data-outcome")).toBe("defeat");
    expect(result?.getAttribute("data-text")).toBe(FX_TEXT.defeat);
  });

  it("B38 a layer that mounts on a finished game plays nothing", () => {
    const fx = fakeSeams();
    const finished = fullBoardView({ result: { winner: "p1", reason: "concede" } });
    const m = mountLayer(queueWith(fakeSchedule()), finished, fx);
    fx.pump(500);

    expect(m.count("result")).toBe(0);
    expect(fx.measured).toHaveLength(0);
    expect(m.fxDom()?.childElementCount).toBe(0);
  });

  it("B38 a later view with the same result does not replay the sequence", () => {
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queueWith(fakeSchedule()), view, fx);
    m.show({ ...view, result: { winner: "p1", reason: "hero-death" } });
    fx.pump(50);
    expect(m.count("result")).toBe(1);

    m.show({ ...view, turn: view.turn + 1, result: { winner: "p1", reason: "hero-death" } });
    fx.pump(50);
    expect(m.count("result")).toBe(1);
  });

  it("B38 a viewer change after the first render plays the \"Your turn\" hand-over banner", () => {
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
    fx.pump(50);
    expect(m.count("banner")).toBe(0);

    m.show(p2View());
    fx.pump(50);

    expect(m.count("banner")).toBe(1);
    const banner = m.game.querySelector('[data-fx="banner"]');
    expect(banner?.getAttribute("data-tone")).toBe("you");
    expect(banner?.getAttribute("data-text")).toBe(FX_TEXT.yourTurn);
  });

  it("B38 a new view for the same viewer plays no banner", () => {
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queueWith(fakeSchedule()), view, fx);

    m.show({ ...view, turn: view.turn + 1 });
    fx.pump(500);
    expect(m.count("banner")).toBe(0);
  });

  it("B38 the first render plays no hand-over banner", () => {
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), p2View(), fx);
    fx.pump(500);
    expect(m.count("banner")).toBe(0);
    expect(fx.measured).toHaveLength(0);
  });

  it("B38 a hand-over to a seat that is not the active one plays no banner", () => {
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);

    m.show(p2View({ active: "p1" }));
    fx.pump(500);
    expect(m.count("banner")).toBe(0);
  });

  it("R200 the killing blow a finished game drains is replayed first, and the result lands on its beat", () => {
    // Game.tsx drains the runner the moment a finished view arrives; the lethal hit must still draw.
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    const lethal: GameEvent[] = [
      { type: "attackDeclared", attackerId: "u1", targetId: "hero-p2", forced: false },
      { type: "damage", sourceId: "u1", targetId: "hero-p2", amount: 5, combat: true },
    ];
    act(() => queue.enqueue(lethal, view));
    act(() => queue.drain());
    m.show({ ...view, result: { winner: "p1", reason: "hero-death" } });

    fx.pump(0);
    expect(m.count("splat")).toBe(1);
    expect(m.game.querySelector('[data-fx="splat"]')?.getAttribute("data-amount")).toBe("-5");
    expect(m.count("result")).toBe(0);

    fx.pump(ANIMATIONS.damage.durationMs);
    expect(m.count("result")).toBe(1);
  });

  it("R200 a game that ends with no blow (a concession) plays its result at once", () => {
    const fx = fakeSeams();
    const queue = queueWith(fakeSchedule());
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    act(() => queue.drain());
    m.show({ ...view, result: { winner: "p1", reason: "concede" } });
    fx.pump(0);
    expect(m.count("result")).toBe(1);
    expect(m.count("splat")).toBe(0);
  });

  it("B38 with effects off, neither a result nor a hand-over plays anything", () => {
    setFxSettings({ intensity: "off" });
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queueWith(fakeSchedule()), view, fx);

    m.show({ ...view, result: { winner: "p1", reason: "hero-death" } });
    fx.pump(200);
    m.show(p2View({ result: { winner: "p1", reason: "hero-death" } }));
    fx.pump(200);

    expect(m.count("result")).toBe(0);
    expect(m.count("banner")).toBe(0);
    expect(fx.measured).toHaveLength(0);
  });
});

// Every test above overrides `seams.catalog`; these leave it out, as production does.

function def(id: string, over: Partial<CardDef> = {}): CardDef {
  return {
    id,
    index: id.slice(5),
    name: `Card ${id}`,
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 3,
    base: { attack: 2, health: 2, keywords: [], text: "" },
    radiant: { attack: 3, health: 3, keywords: [], text: "" },
    ...over,
  };
}

const DEFS: CardDefs = {
  "core-019": def("core-019", { rarity: "Legendary" }),
  "core-020": def("core-020", { rarity: "Mythic" }),
  "core-021": def("core-021"),
  // A big base face and a small radiant one: only the base face's stats shake the board.
  "core-022": def("core-022", {
    base: { attack: 6, health: 6, keywords: [], text: "" },
    radiant: { attack: 1, health: 1, keywords: [], text: "" },
  }),
};

function summon(defId: string): GameEvent {
  return { type: "summoned", player: "p1", instanceId: "c90", defId, row: "units", lane: 3 };
}

function withoutCatalog(fx: Fx): Partial<FxSeams> {
  const seams: Partial<FxSeams> = { ...fx.seams };
  delete seams.catalog;
  return seams;
}

function mountWithCatalog(queue: AnimationQueue, view: PlayerView, seams: Partial<FxSeams>, lookup: CardLookup | null) {
  const utils = render(
    <CatalogContext.Provider value={lookup}>
      <div className="game">
        <FxLayer queue={queue} view={view} seams={seams} />
      </div>
    </CatalogContext.Provider>,
  );
  return {
    rays: (tone: string): number => utils.container.querySelectorAll(`[data-fx="rays"][data-tone="${tone}"]`).length,
  };
}

describe("S9 — the default catalog and the surface seam", () => {
  it("S11 lookupFromDefs carries each def's catalog rarity on both faces", () => {
    const lookup = lookupFromDefs(DEFS);

    expect(lookup("core-019", false)?.rarity).toBe("Legendary");
    expect(lookup("core-019", true)?.rarity).toBe("Legendary");
    expect(lookup("core-020", false)?.rarity).toBe("Mythic");
    expect(lookup("core-021", false)?.rarity).toBe("Common");
    expect(lookup("core-999", false)).toBeUndefined();
  });

  it("S9 with no catalog seam, a Legendary summon reads its rarity from CatalogContext and plays legendary rays", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountWithCatalog(queue, view, withoutCatalog(fx), lookupFromDefs(DEFS));

    act(() => queue.enqueue([summon("core-019")], view));
    fx.pump(50);

    expect(m.rays("legendary")).toBe(1);
    expect(m.rays("mythic")).toBe(0);
  });

  it("S9 a Mythic reads as mythic, and a Common plays no rays at all", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountWithCatalog(queue, view, withoutCatalog(fx), lookupFromDefs(DEFS));

    act(() => queue.enqueue([summon("core-020")], view));
    fx.pump(50);
    expect(m.rays("mythic")).toBe(1);
    act(() => sched.flush());
    fx.pump(2_000);

    act(() => queue.enqueue([summon("core-021")], view));
    fx.pump(50);
    expect(m.rays("legendary") + m.rays("mythic")).toBe(0);
  });

  it("S9 the default catalog looks up the base face and maps its printed stats, so a big base unit shakes the board", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const lookup = vi.fn<CardLookup>(lookupFromDefs(DEFS));
    mountWithCatalog(queue, view, withoutCatalog(fx), lookup);

    act(() => queue.enqueue([summon("core-022")], view));
    fx.pump(400);

    expect(lookup).toHaveBeenCalledWith("core-022", false);
    expect(lookup).not.toHaveBeenCalledWith("core-022", true);
    expect(fx.shakes).toContain("apply");
  });

  it("S9 a hidden defId never reaches the catalog lookup and gets no entrance", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const lookup = vi.fn<CardLookup>(lookupFromDefs(DEFS));
    const m = mountWithCatalog(queue, view, withoutCatalog(fx), lookup);

    act(() => queue.enqueue([summon("hidden")], view));
    fx.pump(400);

    expect(lookup.mock.calls.map(([defId]) => defId)).not.toContain("hidden");
    expect(m.rays("legendary") + m.rays("mythic")).toBe(0);
    expect(fx.shakes).not.toContain("apply");
  });

  it("S9 with no catalog at all, a Brute summon plans the dust burst plus its heavy landing (and its stand-in, B46)", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountWithCatalog(queue, view, withoutCatalog(fx), null);

    act(() => queue.enqueue([summon("core-019")], view));
    fx.pump(400);

    expect(m.rays("legendary")).toBe(0);
    // core-019 is Brute in the bundled catalog, so its entrance shakes the board (#1445).
    expect(fx.shakes).toContain("apply");
    // Every measure is the landing zone: at its foot for the dust (the base burst plus the Brute's
    // two), and bare for the stand-in that follows it. R2791000: lane 3 stands occupied (u3, the
    // file's id scheme above), so the Stack beat measures first: its burst over the occupant, its
    // callout and count over it bare.
    const zone = { kind: "testid", testid: "zone-you-units-3" };
    const occupant = { kind: "testid", testid: "card-u3" };
    expect(fx.measured.filter((anchor) => anchor.kind === "testid" && anchor.at !== undefined)).toEqual([
      { ...occupant, at: { ...FX_STACK_LAND } },
      { ...zone, at: { x: 0.5, y: 1 } },
      { ...zone, at: { x: 0.5, y: 1 } },
      { ...zone, at: { x: 0.5, y: 1 } },
    ]);
    expect(fx.measured.filter((anchor) => !(anchor.kind === "testid" && anchor.at !== undefined)).length).toBeGreaterThan(0);
    for (const anchor of fx.measured) {
      expect(anchor.kind === "testid" ? [zone.testid, occupant.testid].includes(anchor.testid) : true).toBe(true);
    }
  });

  it("S9 a catalog seam overrides CatalogContext", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountWithCatalog(queue, view, { ...fx.seams, catalog: () => ({ rarity: "Mythic" }) }, lookupFromDefs(DEFS));

    act(() => queue.enqueue([summon("core-019")], view));
    fx.pump(50);

    expect(m.rays("mythic")).toBe(1);
    expect(m.rays("legendary")).toBe(0);
  });

  it("S9 the surface seam is handed the layer's own fx-canvas", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const surface = vi.fn((canvas: HTMLCanvasElement) => stubSurface(canvas));
    const view = fullBoardView();
    const m = mountLayer(queue, view, { ...fx, seams: { ...fx.seams, surface } });

    act(() => queue.enqueue([HIT], view));
    fx.pump(100);

    expect(surface).toHaveBeenCalled();
    for (const [canvas] of surface.mock.calls) expect(canvas).toBe(m.canvas());
  });

  it("S9 with effects off the surface seam is never asked for a surface", () => {
    setFxSettings({ intensity: "off" });
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const surface = vi.fn((canvas: HTMLCanvasElement) => stubSurface(canvas));
    const view = fullBoardView();
    mountLayer(queue, view, { ...fx, seams: { ...fx.seams, surface } });

    act(() => queue.enqueue([HIT], view));
    fx.pump(100);

    expect(surface).not.toHaveBeenCalled();
  });
});

describe("R3351000 — the shed pointer is followed once an animation frame (#3610)", () => {
  /** `count` pointermoves on the window inside one `act`, the last at (`x`, `y`) and the ones before it elsewhere. */
  function sweep(count: number, x: number, y: number): void {
    act(() => {
      for (let step = 1; step < count; step += 1) window.dispatchEvent(new MouseEvent("pointermove", { clientX: step, clientY: step * 2 }));
      window.dispatchEvent(new MouseEvent("pointermove", { clientX: x, clientY: y }));
    });
  }

  it("R3351000 ten moves inside one frame set the pointer once, to the last move's place, when the frame runs", () => {
    resetShedForTests();
    const fx = fakeSeams();
    mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
    const heard = vi.fn();
    const stop = subscribeShed(heard);
    sweep(10, 640, 360);
    // The moves are kept, not handled: the shed pointer waits for the frame.
    expect(shedPointer()).toBeNull();
    expect(heard).not.toHaveBeenCalled();
    fx.pump(0);
    expect(shedPointer()).toEqual({ x: 640, y: 360 });
    expect(heard).toHaveBeenCalledTimes(1);
    // The next frame follows the next moves, again once.
    sweep(10, 20, 30);
    fx.pump(0);
    expect(shedPointer()).toEqual({ x: 20, y: 30 });
    expect(heard).toHaveBeenCalledTimes(2);
    stop();
  });

  it("R3351000 unmounting drops the move waiting for its frame, and leaves no pointer", () => {
    resetShedForTests();
    const fx = fakeSeams();
    mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
    sweep(1, 640, 360);
    cleanup();
    fx.pump(0);
    expect(shedPointer()).toBeNull();
    // Nothing listens after it.
    sweep(1, 10, 10);
    fx.pump(0);
    expect(shedPointer()).toBeNull();
  });

  it("R200 R3351000 where there is no requestAnimationFrame the pointer is never set, and no timer is asked for", () => {
    resetShedForTests();
    const frame = window.requestAnimationFrame;
    const timer = vi.spyOn(window, "setTimeout");
    try {
      Object.defineProperty(window, "requestAnimationFrame", { value: undefined, configurable: true, writable: true });
      // No frame seam: the layer's own default source, which never fires here.
      render(
        <div className="game">
          <FxLayer queue={queueWith(fakeSchedule())} view={fullBoardView()} seams={{ ...fakeSeams().seams, frames: undefined }} />
        </div>,
      );
      timer.mockClear();
      sweep(3, 640, 360);
      expect(shedPointer()).toBeNull();
      expect(timer).not.toHaveBeenCalled();
    } finally {
      Object.defineProperty(window, "requestAnimationFrame", { value: frame, configurable: true, writable: true });
      timer.mockRestore();
    }
  });
});

// The mutation audit (#3490): what the tests above left unheld, by the layer's parts.

describe("S9 — the layer's default seams and catalog facts", () => {
  it("R200 defaultFrames hands requestAnimationFrame's handles to its cancel, and where there is none it never fires: every handle 0", () => {
    const cancel = vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => undefined);
    const frame = window.requestAnimationFrame;
    try {
      defaultFrames().cancel(7);
      expect(cancel).toHaveBeenCalledWith(7);
      Object.defineProperty(window, "requestAnimationFrame", { value: undefined, configurable: true, writable: true });
      const never = defaultFrames();
      expect(never.request(() => undefined)).toBe(0);
    } finally {
      Object.defineProperty(window, "requestAnimationFrame", { value: frame, configurable: true, writable: true });
      cancel.mockRestore();
    }
  });

  it("R200 defaultVisibility reads document.hidden and hears visibilitychange until it unsubscribes", () => {
    const visibility = defaultVisibility();
    expect(visibility.hidden()).toBe(false);
    Object.defineProperty(document, "hidden", { configurable: true, get: () => true });
    try {
      expect(visibility.hidden()).toBe(true);
    } finally {
      delete (document as { hidden?: boolean }).hidden;
    }
    const heard = vi.fn();
    const stop = visibility.subscribe(heard);
    document.dispatchEvent(new Event("visibilitychange"));
    expect(heard).toHaveBeenCalledTimes(1);
    stop();
    document.dispatchEvent(new Event("visibilitychange"));
    expect(heard).toHaveBeenCalledTimes(1);
  });

  it("R202 S9 catalogFacts asks nothing for a hidden card or with no catalog, and maps a card's base face", () => {
    const answers = vi.fn<CardLookup>(() => lookupFromDefs(DEFS)("core-019", false));
    expect(catalogFacts(answers, "hidden")).toBeUndefined();
    expect(answers).not.toHaveBeenCalled();
    expect(catalogFacts(null, "core-019")).toBeUndefined();
    expect(catalogFacts(lookupFromDefs(DEFS), "core-022")).toEqual({ rarity: "Common", attack: 6, health: 6, type: "Unit", tags: [] });
  });

  it("R3334001 a phone-wide viewport throws fewer cinematic sparks; with no viewport seam the layer reads window.innerWidth", () => {
    const sparks = (viewportWidth: FxSeams["viewportWidth"] | undefined): number => {
      const view = shipView();
      const fx = fakeSeams();
      const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: false, setPieceOf: starshipClaim });
      const { container } = render(
        <div className="game">
          <FxLayer queue={queue} view={view} seams={{ ...fx.seams, viewportWidth }} />
        </div>,
      );
      act(() => queue.enqueue([SHIP_LAUNCH], view));
      fx.pump(50);
      const count = container.querySelectorAll('[data-fx="cinematic"] .fx-cinematic-spark').length;
      cleanup();
      return count;
    };
    const wide = sparks(() => FX_MOBILE_WIDTH);
    expect(wide).toBeGreaterThan(0);
    expect(sparks(() => FX_MOBILE_WIDTH - 1)).toBeLessThan(wide);
    expect(window.innerWidth).toBeGreaterThanOrEqual(FX_MOBILE_WIDTH);
    expect(sparks(undefined)).toBe(wide);
  });

  it("R201 R200 a freshly loaded layer writes --anim-squeeze and --anim-scale under their own names", async () => {
    // Module-level names: loaded afresh inside the test, so a mutation of them is live when they are built.
    vi.resetModules();
    const { default: Fresh } = await import("./FxLayer.tsx");
    const { readRootVar: read } = await import("../game/rootVars.ts");
    const fresh = await import("./settings.ts");
    const view = fullBoardView();
    const queue = queueWith(fakeSchedule());
    const { container } = render(
      <div className="game">
        <Fresh queue={queue} view={view} seams={fakeSeams().seams} />
      </div>,
    );
    const game = container.firstElementChild as HTMLElement;
    act(() => queue.enqueue([HIT], view));
    expect(read(game, "--anim-squeeze")).toBe("1.000");
    act(() => {
      fresh.setFxSettings({ motion: "reduce" });
    });
    expect(game.style.getPropertyValue("--anim-scale")).toBe("0");
    fresh.resetFxSettingsForTests();
  });

  it("R200 the panel's Reduce motion turns the layer still and zeroes --anim-scale, live", () => {
    try {
      const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fakeSeams());
      expect(m.layer()?.getAttribute("data-fx")).toBe("on");
      act(() => {
        writeSettings({ reduceMotion: true });
      });
      expect(m.layer()?.getAttribute("data-fx")).toBe("off");
      expect(m.fxDom()?.getAttribute("data-still")).toBe("true");
      expect(m.game.style.getPropertyValue("--anim-scale")).toBe("0");
      act(() => {
        writeSettings({ reduceMotion: false });
      });
      expect(m.layer()?.getAttribute("data-fx")).toBe("on");
      expect(m.game.style.getPropertyValue("--anim-scale")).toBe("");
    } finally {
      __resetSettingsForTests();
    }
  });
});

describe("S9 — the director's lifetime, its pause and its marks", () => {
  it("S9 the director lives exactly as long as the layer draws: one visibility subscription while on or still, none while off", () => {
    let live = 0;
    const visibility: FxVisibility = {
      hidden: () => false,
      subscribe: () => {
        live += 1;
        return () => {
          live -= 1;
        };
      },
    };
    setFxSettings({ intensity: "off" });
    const fx = fakeSeams();
    mountLayer(queueWith(fakeSchedule()), fullBoardView(), { ...fx, seams: { ...fx.seams, visibility } });
    expect(live).toBe(0);
    act(() => {
      setFxSettings({ intensity: "normal" });
    });
    expect(live).toBe(1);
    // R2789000: still, a director of its own.
    act(() => {
      setFxSettings({ motion: "reduce" });
    });
    expect(live).toBe(1);
    act(() => {
      setFxSettings({ intensity: "off" });
    });
    expect(live).toBe(0);
    act(() => {
      setFxSettings({ intensity: "normal", motion: "system" });
    });
    expect(live).toBe(1);
    cleanup();
    expect(live).toBe(0);
  });

  it("R2789000 a still layer has no canvas and asks the surface seam for nothing", () => {
    setReducedMotion(true);
    const fx = fakeSeams();
    const surface = vi.fn((canvas: HTMLCanvasElement) => stubSurface(canvas));
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), { ...fx, seams: { ...fx.seams, surface } });
    expect(m.fxDom()?.getAttribute("data-still")).toBe("true");
    expect(surface).not.toHaveBeenCalled();
  });

  it("Game's hit-stop freezes the effects on screen and marks the layer data-paused; letting go plays them out", () => {
    const fx = fakeSeams();
    const queue = queueWith(fakeSchedule());
    const view = fullBoardView();
    const tree = (paused: boolean) => (
      <div className="game">
        <FxLayer queue={queue} view={view} paused={paused} seams={fx.seams} />
      </div>
    );
    const utils = render(tree(false));
    const layer = (): Element | null => utils.container.querySelector('[data-testid="fx-layer"]');
    const splats = (): number => utils.container.querySelectorAll('[data-fx="splat"]').length;
    expect(layer()).not.toHaveAttribute("data-paused");
    act(() => queue.enqueue([HIT], view));
    fx.pump(50);
    expect(splats()).toBe(1);
    utils.rerender(tree(true));
    expect(layer()?.getAttribute("data-paused")).toBe("true");
    fx.pump(5_000, 50);
    expect(splats()).toBe(1);
    utils.rerender(tree(false));
    expect(layer()).not.toHaveAttribute("data-paused");
    fx.pump(5_000, 50);
    expect(splats()).toBe(0);
  });

  it("S9 with no director (the effects off) a hit-stop, a prompt for the viewer, a skip and a drain throw nothing", () => {
    const sched = fakeSchedule();
    const queue = createAnimationQueue({
      schedule: sched.schedule,
      reducedMotion: false,
      setPieceOf: (entry) => (entry.type === "damage" ? { key: "fixture", holdMs: SET_PIECE_HOLD_MS } : null),
    });
    const fx = fakeSeams();
    const view = fullBoardView();
    const tree = (shown: PlayerView, paused: boolean) => (
      <div className="game">
        <FxLayer queue={queue} view={shown} paused={paused} seams={fx.seams} />
      </div>
    );
    const utils = render(tree(view, false));
    // A set piece in flight when the effects go off (R2785000): its skip finds no director to cut.
    act(() => queue.enqueue([HIT, HIT], view));
    expect(queue.inFlight()?.setPiece?.key).toBe("fixture");
    act(() => {
      setFxSettings({ intensity: "off" });
    });
    let skipped = false;
    act(() => {
      skipped = queue.skip();
    });
    expect(skipped).toBe(true);
    act(() => queue.drain());
    utils.rerender(tree(fullBoardView({ pending: pendingFor("target", []) }), true));
    const layer = utils.container.querySelector('[data-testid="fx-layer"]');
    expect(layer?.getAttribute("data-fx")).toBe("off");
    expect(layer?.getAttribute("data-paused")).toBe("true");
  });

  it("#3633 the marks follow the settings: no layer mark while the effects are off, and the new effects root is read once they are back", async () => {
    setFxSettings({ intensity: "off" });
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fakeSeams());
    expect(m.game).not.toHaveAttribute(FX_LAYER_MARK);
    act(() => {
      setFxSettings({ intensity: "normal" });
    });
    expect(m.game).toHaveAttribute(FX_LAYER_MARK, "on");
    const banner = document.createElement("div");
    banner.setAttribute("data-fx", "banner");
    await act(async () => {
      m.fxDom()?.append(banner);
      await Promise.resolve();
    });
    expect(m.game).toHaveAttribute(FX_BANNER_MARK);
    act(() => {
      setFxSettings({ intensity: "off" });
    });
    expect(m.game).not.toHaveAttribute(FX_LAYER_MARK);
    expect(m.game).not.toHaveAttribute(FX_BANNER_MARK);
  });

  it("#3633 a hold deep in the root marks the game, the marks are read again as the layer turns still, and unmounting lets go of every mark", async () => {
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fakeSeams());
    expect(m.game).toHaveAttribute(ROOT_VARS_ATTRIBUTE);
    const fxDom = m.fxDom();
    if (fxDom === null) throw new Error("no effects root");
    const wrapper = document.createElement("div");
    await act(async () => {
      fxDom.append(wrapper);
      await Promise.resolve();
    });
    const hold = document.createElement("div");
    hold.className = "fx-hold";
    const banner = document.createElement("div");
    banner.setAttribute("data-fx", "banner");
    await act(async () => {
      wrapper.append(hold, banner);
      await Promise.resolve();
    });
    expect(m.game).toHaveAttribute(FX_HOLD_MARK);
    expect(m.game).toHaveAttribute(FX_BANNER_MARK);
    // R2789000: the layer turns still; its root stays, with what it holds, and is read again.
    act(() => {
      setFxSettings({ motion: "reduce" });
    });
    expect(m.fxDom()).toBe(fxDom);
    expect(m.game).toHaveAttribute(FX_HOLD_MARK);
    expect(m.game).toHaveAttribute(FX_BANNER_MARK);
    cleanup();
    expect(m.game).not.toHaveAttribute(FX_BANNER_MARK);
    expect(m.game).not.toHaveAttribute(FX_HOLD_MARK);
    // It lets go of the game's variables (game/rootVars.ts) and watches the old root no more.
    expect(m.game).not.toHaveAttribute(ROOT_VARS_ATTRIBUTE);
    await act(async () => {
      fxDom.append(document.createElement("div"));
      await Promise.resolve();
    });
    expect(m.game).not.toHaveAttribute(FX_BANNER_MARK);
  });

  it("#3633 a layer with no parent element marks nothing, writes no variable and throws nothing", () => {
    for (const motion of ["system", "reduce"] as const) {
      setFxSettings({ motion });
      const fragment = document.createDocumentFragment();
      const queue = queueWith(fakeSchedule());
      const fx = fakeSeams();
      const view = fullBoardView();
      render(<FxLayer queue={queue} view={view} seams={fx.seams} />, { container: fragment as unknown as HTMLElement });
      const layer = fragment.querySelector('[data-testid="fx-layer"]');
      expect(layer?.parentElement, motion).toBeNull();
      expect(layer?.getAttribute("data-fx"), motion).toBe(motion === "reduce" ? "off" : "on");
      act(() => queue.enqueue([HIT], view));
      fx.pump(50);
      expect(fragment.querySelector('[data-testid="fx-layer"]'), motion).toBe(layer);
      cleanup();
    }
  });
});

/** A queue whose signals the test can also send by hand, to the listener the layer subscribed. */
function tapped(queue: AnimationQueue): { queue: AnimationQueue; send: (signal: RunnerSignal) => void } {
  let heard: ((signal: RunnerSignal) => void) | null = null;
  return {
    queue: {
      ...queue,
      subscribeSignals: (listener) => {
        heard = listener;
        return queue.subscribeSignals(listener);
      },
    },
    send: (signal) => {
      act(() => {
        heard?.(signal);
      });
    },
  };
}

const SPELL_CAST: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "s1", defId: "core-050", costPaid: 2 };
const SPELL_HIT: GameEvent = { type: "damage", sourceId: "s1", targetId: "u1", amount: 3, combat: false };
/** R3407002: IN33DH3's Death marker, and the shuffle its entry draws its recipe on (out of his graveyard pile). */
const DEATH_MARKER: GameEvent = { type: "hookResolving", sourceId: "gone", defId: "stinkyscape-147", controller: "p1", hook: "death" };
const DEATH_SHUFFLE: GameEvent = { type: "shuffledIn", player: "p1", instanceId: "prime", defId: "stinkyscape-147-1", position: 0 };
const CLEAVE: GameEvent = { type: "damage", sourceId: "u1", targetId: "u7", amount: 1, combat: true, via: "Cleave" };

type Shot = { events: GameEvent[] | null; thrown?: unknown };

/**
 * Enqueues `shot.events` as it renders after the layer in one pass: the layer has rendered a settings change
 * React has not committed. What the enqueue throws is kept in `shot.thrown` (React would retry the render).
 */
function Midway({ queue, view, shot }: { queue: AnimationQueue; view: PlayerView; shot: Shot }): null {
  useFxSettings();
  const events = shot.events;
  if (events !== null) {
    shot.events = null;
    try {
      queue.enqueue(events, view);
    } catch (error) {
      shot.thrown = error;
    }
  }
  return null;
}

describe("B36 — the runner's signals, signal by signal", () => {
  it("issue #769 a start whose type has no row takes the squeeze off, and a skip takes it off at once", () => {
    setFxSettings({ intensity: "off" });
    const { queue, send } = tapped(queueWith(fakeSchedule()));
    const view = fullBoardView();
    const m = mountLayer(queue, view, fakeSeams());
    act(() => queue.enqueue([HIT], view));
    const entry = queue.inFlight();
    if (entry === null) throw new Error("nothing in flight");
    expect(m.squeeze()).toBe("1.000");
    send({ kind: "start", entry: { ...entry, type: "fromANewerPatch" as unknown as AnimationEntry["type"] } });
    expect(m.squeeze()).toBe("");
    send({ kind: "start", entry });
    expect(m.squeeze()).toBe("1.000");
    send({ kind: "skip", entry });
    expect(m.squeeze()).toBe("");
  });

  it("B36 a new queue: the layer drops the old one's squeeze and follows the new one alone", () => {
    const fx = fakeSeams();
    const view = fullBoardView();
    const first = queueWith(fakeSchedule());
    const second = queueWith(fakeSchedule());
    const tree = (queue: AnimationQueue) => (
      <div className="game">
        <FxLayer queue={queue} view={view} seams={fx.seams} />
      </div>
    );
    const utils = render(tree(first));
    const game = utils.container.firstElementChild as HTMLElement;
    act(() => first.enqueue([HIT], view));
    expect(readRootVar(game, "--anim-squeeze")).toBe("1.000");
    utils.rerender(tree(second));
    expect(readRootVar(game, "--anim-squeeze")).toBe("");
    act(() => {
      first.reset();
      first.enqueue([HIT], view);
    });
    expect(readRootVar(game, "--anim-squeeze")).toBe("");
    act(() => second.enqueue([HIT], view));
    expect(readRootVar(game, "--anim-squeeze")).toBe("1.000");
  });

  it("B36 the layer remembers a caster across a re-render of the same view", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    act(() => queue.enqueue([SPELL_CAST], view));
    fx.pump(50);
    act(() => sched.flush());
    m.show(view);
    act(() => queue.enqueue([SPELL_HIT], view));
    fx.pump(100);
    expect(fx.testids()).toContain("hero-opponent");
  });

  it("B36 a newer view after a drain brings back nothing the drain forgot", () => {
    const queue = queueWith(fakeSchedule());
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    act(() => queue.enqueue([SPELL_CAST], view));
    fx.pump(50);
    act(() => queue.drain());
    m.show({ ...view, turn: view.turn + 1 });
    act(() => queue.enqueue([SPELL_HIT], view));
    fx.pump(300);
    expect(fx.testids()).toContain("card-u1");
    expect(fx.testids()).not.toContain("hero-opponent");
  });

  it("R2785000 a skip forgets nothing the layer remembered: the spell after the cut set piece flies from its caster", () => {
    const sched = fakeSchedule();
    const queue = createAnimationQueue({
      schedule: sched.schedule,
      reducedMotion: false,
      setPieceOf: (entry) => (entry.type === "damage" ? { key: "fixture", holdMs: SET_PIECE_HOLD_MS } : null),
    });
    const fx = fakeSeams();
    const view = fullBoardView();
    mountLayer(queue, view, fx);
    act(() => queue.enqueue([SPELL_CAST], view));
    fx.pump(50);
    act(() => sched.flush());
    act(() => queue.enqueue([HIT, SPELL_HIT], view));
    expect(queue.inFlight()?.setPiece?.key).toBe("fixture");
    fx.pump(50);
    fx.measured.length = 0;
    act(() => {
      queue.skip();
    });
    expect(queue.inFlight()?.events).toEqual([SPELL_HIT]);
    fx.pump(100);
    expect(fx.testids()).toContain("hero-opponent");
  });

  it("R318 a drain keeps the entries its burst already played: a lethal fatigue hit replays out of its empty library", () => {
    const sched = fakeSchedule();
    const queue = queueWith(sched);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    act(() =>
      queue.enqueue(
        [
          { type: "fatigue", player: "p1", count: 7, amount: 7 },
          { type: "damage", sourceId: null, targetId: "hero-p1", amount: 7, combat: false },
        ],
        view,
      ),
    );
    act(() => sched.tick());
    expect(queue.inFlight()?.type).toBe("damage");
    act(() => queue.drain());
    fx.measured.length = 0;
    m.show({ ...view, result: { winner: "p2", reason: "hero-death" } });
    fx.pump(0);
    expect(fx.testids()).toContain(animTestid.library("you"));
  });

  it("R200 a spell's killing blow the drain cut flies again from the caster the drained entries name", () => {
    const queue = queueWith(fakeSchedule());
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queue, view, fx);
    act(() =>
      queue.enqueue(
        [
          { type: "cardPlayed", player: "p2", instanceId: "s9", defId: "classicplus-015", costPaid: 3 },
          { type: "damage", sourceId: "s9", targetId: "hero-p1", amount: 6, combat: false },
        ],
        view,
      ),
    );
    act(() => queue.drain());
    fx.measured.length = 0;
    m.show({ ...view, result: { winner: "p2", reason: "hero-death" } });
    fx.pump(0);
    expect(m.count("result")).toBe(0);
    expect(fx.testids()).toContain("hero-opponent");
  });

  it("R200 a killing blow replays after an earlier burst played out, after a drain and after a reset; a reset forgets what a drain kept", () => {
    const lethal: GameEvent[] = [
      { type: "attackDeclared", attackerId: "u1", targetId: "hero-p2", forced: false },
      { type: "damage", sourceId: "u1", targetId: "hero-p2", amount: 5, combat: true },
    ];
    const view = fullBoardView();
    const won: PlayerView = { ...view, result: { winner: "p1", reason: "hero-death" } };
    /** The splats and the result a finished view draws after `before`. */
    const finish = (before: (queue: AnimationQueue, sched: ReturnType<typeof fakeSchedule>) => void): { splats: number; results: number } => {
      const sched = fakeSchedule();
      const queue = queueWith(sched);
      const fx = fakeSeams();
      const m = mountLayer(queue, view, fx);
      before(queue, sched);
      m.show(won);
      fx.pump(ANIMATIONS.damage.durationMs);
      const drawn = { splats: m.count("splat"), results: m.count("result") };
      cleanup();
      return drawn;
    };
    const blowAfter = (first: (queue: AnimationQueue, sched: ReturnType<typeof fakeSchedule>) => void) =>
      finish((queue, sched) => {
        first(queue, sched);
        act(() => queue.enqueue(lethal, view));
        act(() => queue.drain());
      });
    const replayed = { splats: 1, results: 1 };
    expect(blowAfter((queue, sched) => {
      act(() => queue.enqueue([HIT], view));
      act(() => sched.flush());
    }), "after an idle runner").toEqual(replayed);
    expect(blowAfter((queue) => {
      act(() => queue.enqueue([HIT], view));
      act(() => queue.drain());
    }), "after a drain").toEqual(replayed);
    expect(blowAfter((queue) => act(() => queue.reset())), "after a reset").toEqual(replayed);
    expect(finish((queue) => {
      act(() => queue.enqueue(lethal, view));
      act(() => queue.drain());
      act(() => queue.reset());
    }), "a reset after the drain").toEqual({ splats: 0, results: 1 });
  });

  it("R3407002 a hook's marker opens its entry on a still layer, but not on one that is off, nor in the render that turns it on or off", () => {
    const graveyard = animTestid.graveyard("you");
    const view = fullBoardView();
    /** Whether a shuffle after `before`, with the effects on, draws IN33DH3's Death recipe. */
    const opens = (before: (queue: AnimationQueue, shot: Shot) => void): boolean => {
      resetFxSettingsForTests();
      const sched = fakeSchedule();
      const queue = queueWith(sched);
      const fx = fakeSeams();
      const shot: Shot = { events: null };
      render(
        <div className="game">
          <FxLayer queue={queue} view={view} seams={fx.seams} />
          <Midway queue={queue} view={view} shot={shot} />
        </div>,
      );
      before(queue, shot);
      act(() => sched.flush());
      act(() => {
        setFxSettings({ intensity: "normal", motion: "system" });
      });
      fx.measured.length = 0;
      act(() => queue.enqueue([DEATH_SHUFFLE], view));
      fx.pump(50);
      const drawn = fx.testids().includes(graveyard);
      expect(shot.thrown).toBeUndefined();
      cleanup();
      return drawn;
    };
    const set = (patch: Parameters<typeof setFxSettings>[0]): void => {
      act(() => {
        setFxSettings(patch);
      });
    };
    expect(opens((queue) => act(() => queue.enqueue([DEATH_MARKER], view))), "on").toBe(true);
    expect(
      opens((queue) => {
        set({ motion: "reduce" });
        act(() => queue.enqueue([DEATH_MARKER], view));
      }),
      "still",
    ).toBe(true);
    expect(
      opens((queue) => {
        set({ intensity: "off" });
        act(() => queue.enqueue([DEATH_MARKER], view));
      }),
      "off",
    ).toBe(false);
    expect(
      opens((_queue, shot) => {
        set({ intensity: "off" });
        shot.events = [DEATH_MARKER];
        set({ intensity: "normal" });
      }),
      "turning on",
    ).toBe(false);
    expect(
      opens((_queue, shot) => {
        shot.events = [DEATH_MARKER];
        set({ intensity: "off" });
      }),
      "turning off",
    ).toBe(false);
  });

  it("S9 a start or a still entry in the render that turns the layer on plays nothing and throws nothing: its director is not built yet", () => {
    for (const reduced of [false, true]) {
      setReducedMotion(reduced);
      setFxSettings({ intensity: "off" });
      const fx = fakeSeams();
      const view = fullBoardView();
      const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: reduced });
      const shot: Shot = { events: null };
      const { container } = render(
        <div className="game">
          <FxLayer queue={queue} view={view} seams={fx.seams} />
          <Midway queue={queue} view={view} shot={shot} />
        </div>,
      );
      shot.events = [reduced ? CLEAVE : HIT];
      act(() => {
        setFxSettings({ intensity: "normal" });
      });
      expect(shot.events, "sent").toBeNull();
      expect(shot.thrown, `reduced ${String(reduced)}`).toBeUndefined();
      fx.pump(50);
      const layer = container.querySelector('[data-testid="fx-layer"]');
      expect(layer, `reduced ${String(reduced)}`).not.toBeNull();
      expect(container.querySelector('[data-testid="fx-dom"]')?.childElementCount, `reduced ${String(reduced)}`).toBe(0);
      cleanup();
      resetFxSettingsForTests();
    }
  });

  it("R2789000 a still layer remembers each still entry: a Windfury unit's second attack of the turn calls out Windfury, its first does not", () => {
    setReducedMotion(true);
    const view = fullBoardView();
    const fx = fakeSeams();
    const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: true });
    const m = mountLayer(queue, view, fx);
    const callouts = (): (string | null)[] => [...m.game.querySelectorAll('[data-fx="callout"]')].map((el) => el.getAttribute("data-callout"));
    act(() => queue.enqueue([{ type: "attackDeclared", attackerId: "u2", targetId: "u6", forced: false }], view));
    fx.pump(50);
    expect(callouts()).not.toContain("Windfury");
    fx.pump(FX_CALLOUT_MS + 100);
    act(() => queue.enqueue([{ type: "attackDeclared", attackerId: "u2", targetId: "u7", forced: false }], view));
    fx.pump(50);
    expect(callouts()).toContain("Windfury");
  });

  it("R2789000 a still entry the runner hands a moving layer draws no still callout", () => {
    const view = fullBoardView();
    const fx = fakeSeams();
    const queue = createAnimationQueue({ schedule: fakeSchedule().schedule, reducedMotion: true });
    const m = mountLayer(queue, view, fx);
    expect(m.layer()?.getAttribute("data-fx")).toBe("on");
    act(() => queue.enqueue([CLEAVE], view));
    fx.pump(50);
    expect(m.count("callout")).toBe(0);
  });

  it("R2789000 a still layer plays no result sequence and no hand-over banner", () => {
    setReducedMotion(true);
    const fx = fakeSeams();
    const view = fullBoardView();
    const m = mountLayer(queueWith(fakeSchedule()), view, fx);
    m.show(p2View());
    fx.pump(200);
    m.show(p2View({ result: { winner: "p2", reason: "hero-death" } }));
    fx.pump(200);
    expect(m.count("banner")).toBe(0);
    expect(m.count("result")).toBe(0);
  });
});

describe("B38 — what takes the turn banner away", () => {
  it("B38 the first pointer down takes the banner away, in the capture phase, so a control that stops the event still does", () => {
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
    m.show(p2View());
    fx.pump(50);
    expect(m.count("banner")).toBe(1);
    const control = document.createElement("button");
    control.addEventListener("pointerdown", (event) => {
      event.stopPropagation();
    });
    document.body.append(control);
    try {
      act(() => {
        control.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
      });
      expect(m.count("banner")).toBe(0);
    } finally {
      control.remove();
    }
  });

  it("B38 unmounting removes the capture listener it added", () => {
    const add = vi.spyOn(document, "addEventListener");
    const remove = vi.spyOn(document, "removeEventListener");
    try {
      mountLayer(queueWith(fakeSchedule()), fullBoardView(), fakeSeams());
      const ours = add.mock.calls.find(([type, , options]) => type === "pointerdown" && options === true);
      expect(ours).toBeDefined();
      cleanup();
      expect(remove).toHaveBeenCalledWith("pointerdown", ours?.[1], true);
    } finally {
      add.mockRestore();
      remove.mockRestore();
    }
  });

  it("B38 a prompt opening for the viewer takes the banner away; one for the other seat, or one closing, leaves it", () => {
    const fx = fakeSeams();
    const m = mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
    m.show(p2View());
    fx.pump(50);
    expect(m.count("banner")).toBe(1);
    m.show(p2View({ pending: waitingPending }));
    fx.pump(50);
    expect(m.count("banner"), "the other seat's prompt").toBe(1);
    m.show(p2View({ pending: pendingFor("target", []) }));
    fx.pump(50);
    expect(m.count("banner"), "the viewer's prompt").toBe(0);
    // The seat changes back with the prompt still open: its banner plays, and the prompt closing leaves it.
    m.show(fullBoardView({ pending: pendingFor("target", []) }));
    fx.pump(50);
    expect(m.count("banner"), "the hand-over").toBe(1);
    m.show(fullBoardView());
    fx.pump(50);
    expect(m.count("banner"), "the prompt closed").toBe(1);
  });
});

describe("R502 — the layer's marks on the board's mana trays", () => {
  /** Both seats' trays, as Board.tsx draws them: `count` crystals each. */
  function trays(count = 4): () => void {
    const made = (["you", "opponent"] as const).map((side) => {
      const tray = document.createElement("div");
      tray.setAttribute("data-testid", `mana-${side}`);
      for (let k = 0; k < count; k += 1) {
        const crystal = document.createElement("span");
        crystal.className = "mana-crystal";
        tray.append(crystal);
      }
      document.body.append(tray);
      return tray;
    });
    return () => {
      for (const tray of made) tray.remove();
    };
  }
  const marked = (side: "you" | "opponent"): number => document.querySelectorAll(`[data-testid="mana-${side}"] .mana-crystal[${HINDERED_ATTR}]`).length;
  const rider = (lower: number) => ({ mana: { current: 1, max: 3 }, modifiers: [{ id: FX_NEXT_REFRESH_MODIFIER_ID, label: `Next refresh −${String(lower)} mana` }] });
  const plain = (): PlayerView => baseView({ you: emptySide("p1", { mana: { current: 1, max: 3 } }) });
  const LAYS: GameEvent = { type: "modifierChanged", player: "p1", modifierId: FX_NEXT_REFRESH_MODIFIER_ID, added: true };

  function mountRider(queue: AnimationQueue, shown: PlayerView, latest: PlayerView) {
    const tree = (s: PlayerView, l: PlayerView) => (
      <div className="game">
        <FxLayer queue={queue} view={s} latest={l} seams={fakeSeams().seams} />
      </div>
    );
    const utils = render(tree(shown, latest));
    return { show: (s: PlayerView, l: PlayerView) => utils.rerender(tree(s, l)) };
  }

  it("R502 only the rider's own modifierChanged marks the newest view's crystals early; any other start touches no tray", () => {
    const untray = trays();
    try {
      const sched = fakeSchedule();
      const queue = queueWith(sched);
      mountRider(queue, plain(), baseView({ you: emptySide("p1", rider(2)) }));
      act(() =>
        queue.enqueue(
          [
            { type: "manaChanged", player: "p1", current: 2, max: 3 },
            { type: "modifierChanged", player: "p1", modifierId: "m1", added: true },
            LAYS,
          ],
          plain(),
        ),
      );
      expect(queue.inFlight()?.events[0]?.type).toBe("manaChanged");
      expect(marked("you")).toBe(0);
      act(() => sched.tick());
      expect(queue.inFlight()?.events[0]?.type).toBe("modifierChanged");
      expect(marked("you"), "another modifier").toBe(0);
      act(() => sched.tick());
      expect(queue.inFlight()?.events[0]).toBe(LAYS);
      expect(marked("you")).toBe(2);
      cleanup();
      // A start that lays no rider writes nothing on the trays, whatever they show.
      const shown = baseView({ you: emptySide("p1", rider(1)) });
      const later = fakeSchedule();
      const again = queueWith(later);
      mountRider(again, shown, shown);
      expect(marked("you")).toBe(1);
      applyManaMarks(document, manaMarks(baseView()));
      act(() =>
        again.enqueue(
          [
            { type: "manaChanged", player: "p2", current: 2, max: 2 },
            { type: "modifierChanged", player: "p1", modifierId: "m1", added: true },
          ],
          shown,
        ),
      );
      expect(marked("you")).toBe(0);
      act(() => later.tick());
      expect(again.inFlight()?.events[0]?.type).toBe("modifierChanged");
      expect(marked("you"), "another modifier").toBe(0);
    } finally {
      untray();
    }
  });

  it("R502 a newer shown view forgets the riders the last burst reached", () => {
    const untray = trays();
    try {
      const sched = fakeSchedule();
      const queue = queueWith(sched);
      const m = mountRider(queue, plain(), baseView({ you: emptySide("p1", rider(2)) }));
      act(() => queue.enqueue([LAYS], plain()));
      expect(marked("you")).toBe(2);
      act(() => sched.flush());
      m.show(plain(), baseView({ you: emptySide("p1", rider(1)) }));
      expect(marked("you")).toBe(0);
    } finally {
      untray();
    }
  });

  it("R502 unmounting takes the marks off both trays", () => {
    const untray = trays();
    try {
      const both = baseView({ you: emptySide("p1", rider(1)), opponent: emptySide("p2", rider(2)) });
      mountRider(queueWith(fakeSchedule()), both, both);
      expect(marked("you")).toBe(1);
      expect(marked("opponent")).toBe(2);
      cleanup();
      expect(marked("you")).toBe(0);
      expect(marked("opponent")).toBe(0);
    } finally {
      untray();
    }
  });
});

describe("R3351000 — the layer's pointer listener", () => {
  it("R3351000 the pointer is followed by one passive listener, and unmounting leaves no pointer", () => {
    resetShedForTests();
    const add = vi.spyOn(window, "addEventListener");
    try {
      const fx = fakeSeams();
      mountLayer(queueWith(fakeSchedule()), fullBoardView(), fx);
      expect(add).toHaveBeenCalledWith("pointermove", expect.any(Function), { passive: true });
      act(() => {
        window.dispatchEvent(new MouseEvent("pointermove", { clientX: 640, clientY: 360 }));
      });
      fx.pump(0);
      expect(shedPointer()).toEqual({ x: 640, y: 360 });
      cleanup();
      expect(shedPointer()).toBeNull();
    } finally {
      add.mockRestore();
    }
  });
});
