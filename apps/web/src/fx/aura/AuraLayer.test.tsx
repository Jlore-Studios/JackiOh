// R2787000: the aura layer over the shown view. Every browser API comes in through `seams`: a fixed box
// for every aura, a frame source the test runs by hand and a visibility it toggles. R3337003: an
// Anomaly's backdrop on the page.

import type { BackrowView, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { anomalyArtUri } from "../../cards/art/svg.ts";
import { baseView, emptySide, faceDownBackrow, faceUpBackrow, resetIds } from "../../test/fixtures.ts";
import { setReducedMotion } from "../../test/setup.ts";
import { __resetSettingsForTests, writeSettings } from "../../settings/store.ts";
import { parseFxCardTable } from "../cards/tables.ts";
import { AURA_FADE_IN_MS, AURA_FADE_OUT_MS, AURA_PARTICLE_SHARE, FX_PARTICLE_CAP_MOBILE } from "../constants.ts";
import { resetFxSettingsForTests, setFxSettings } from "../settings.ts";
import { resetShedForTests, setShedLevel, setShedPointer } from "../shed.ts";
import type { FxSurface } from "../surface.ts";
import type { FxBox, FxFrameSource, FxVisibility } from "../types.ts";
import AuraLayer, { AURA_CANVAS_TESTID, AURA_LAYER_TESTID, type AuraSeams } from "./AuraLayer.tsx";
import { auraTestid } from "./plan.ts";

beforeEach(() => {
  localStorage.clear();
  resetFxSettingsForTests();
  __resetSettingsForTests();
});

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  resetFxSettingsForTests();
  __resetSettingsForTests();
  resetShedForTests();
  localStorage.clear();
});

/** plan.test.ts's table: every look and every scope once. */
const TABLE = parseFxCardTable(
  "aura.json5",
  `{
    "core-014": { aura: { look: "storm", scope: "side" } },
    "core-046": { aura: { look: "void", scope: "board" } },
    "classic-042": { aura: { look: "toxic", scope: "zone" } },
    "classic-075": { aura: { look: "holy", palette: "Fire", scope: "lane" } },
    "meditative-040": { aura: { look: "bloom", scope: "board" } },
    "meditativeplus-009": { aura: { look: "arcane", scope: "side" } },
    "stinkyscape-155": { aura: { look: "drift", scope: "side" } },
    "core-018": { aura: { look: "frost", scope: "zone" } },
    "core-035": { aura: { look: "ember", scope: "side" } },
  }`,
);

/** A 2D context with every method and property; FxLayer.test.tsx's. */
function recordingContext(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const state: Record<string, unknown> = {
    canvas,
    globalAlpha: 1,
    globalCompositeOperation: "source-over",
    fillStyle: "#000000",
  };
  return new Proxy(state, {
    get(target, key) {
      if (typeof key !== "string") return undefined;
      if (key in target) return target[key];
      return (): undefined => undefined;
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

function fakeSeams(width = 1280) {
  let t = 50_000;
  let nextHandle = 1;
  let hidden = false;
  const pending = new Map<number, (timestampMs: number) => void>();
  const requested: number[] = [];
  const cancelled: number[] = [];
  const listeners = new Set<() => void>();
  const frames: FxFrameSource = {
    request(callback) {
      const handle = nextHandle;
      nextHandle += 1;
      pending.set(handle, callback);
      requested.push(handle);
      return handle;
    },
    cancel(handle) {
      cancelled.push(handle);
      pending.delete(handle);
    },
  };
  const visibility: FxVisibility = {
    hidden: () => hidden,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  const seams: Partial<AuraSeams> = {
    now: () => t,
    frames,
    visibility,
    surface: stubSurface,
    measure: () => BOX,
    viewportWidth: () => width,
    seed: 11,
    table: TABLE,
  };
  const runFrames = (): void => {
    const due = [...pending.values()];
    pending.clear();
    for (const callback of due) callback(t);
  };
  return {
    seams,
    requested,
    cancelled,
    pending: (): number => pending.size,
    setHidden(next: boolean): void {
      hidden = next;
      act(() => {
        for (const listener of listeners) listener();
      });
    },
    pump(ms: number, step = 10): void {
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
type Slot = BackrowView | null;

function field(owner: PlayerId, defId: string, over: Partial<Extract<BackrowView, { faceDown: false }>> = {}): Extract<BackrowView, { faceDown: false }> {
  return faceUpBackrow(owner, { defId, type: defId === "core-018" ? "Field Trap" : "Field Spell", ...over }) as Extract<BackrowView, { faceDown: false }>;
}

function board(you: readonly Slot[], opponent: readonly Slot[], viewer: PlayerId = "p1", over: Partial<PlayerView> = {}): PlayerView {
  const other: PlayerId = viewer === "p1" ? "p2" : "p1";
  const pad = (slots: readonly Slot[]): Slot[] => [...slots, null, null, null, null, null].slice(0, 5);
  return baseView({
    viewer,
    active: viewer,
    you: emptySide(viewer, { backrow: pad(you) }),
    opponent: emptySide(other, { hand: { count: 4 }, backrow: pad(opponent) }),
    ...over,
  });
}

const fieldKey = (slot: { instanceId: string }): string => `field-${slot.instanceId}`;

function mount(view: PlayerView, fx: Fx, latest?: PlayerView) {
  const tree = (shown: PlayerView, newest?: PlayerView) => (
    <div className="game">
      <AuraLayer view={shown} latest={newest} seams={fx.seams} />
    </div>
  );
  const utils = render(tree(view, latest));
  const game = utils.container.firstElementChild as HTMLElement;
  return {
    show: (next: PlayerView, newest?: PlayerView): void => {
      utils.rerender(tree(next, newest));
    },
    layer: (): HTMLElement | null => game.querySelector<HTMLElement>(`[data-testid="${AURA_LAYER_TESTID}"]`),
    canvas: (): HTMLElement | null => game.querySelector<HTMLElement>(`[data-testid="${AURA_CANVAS_TESTID}"]`),
    aura: (key: string): HTMLElement | null => game.querySelector<HTMLElement>(`[data-testid="${auraTestid(key)}"]`),
    auras: (): HTMLElement[] => [...game.querySelectorAll<HTMLElement>(".aura")],
  };
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 });
const modifierAdded = (player: PlayerId, modifierId: string): GameEvent => ({ type: "modifierChanged", player, modifierId, added: true });

describe("R2787000 the aura layer", () => {
  it("R2787000 a Field Spell's arrival begins its aura and it stands after AURA_FADE_IN_MS; its leaving ends it and it is gone after AURA_FADE_OUT_MS", () => {
    resetIds();
    const fx = fakeSeams();
    const empty = board([], []);
    const m = mount(empty, fx);
    expect(m.layer()).toBeNull();
    const suppressive = field("p1", "core-046");
    m.show(board([suppressive], []));
    const aura = m.aura(fieldKey(suppressive));
    expect(aura).toHaveAttribute("data-aura-phase", "begin");
    expect(aura).toHaveAttribute("data-aura-look", "void");
    expect(aura).toHaveAttribute("data-aura-scope", "board");
    expect(m.canvas()).not.toBeNull();
    expect(fx.pending()).toBe(1);
    fx.pump(AURA_FADE_IN_MS - 20);
    expect(m.aura(fieldKey(suppressive))).toHaveAttribute("data-aura-phase", "begin");
    fx.pump(20);
    expect(m.aura(fieldKey(suppressive))).toHaveAttribute("data-aura-phase", "standing");
    m.show(empty);
    expect(m.aura(fieldKey(suppressive))).toHaveAttribute("data-aura-phase", "end");
    fx.pump(AURA_FADE_OUT_MS);
    expect(m.aura(fieldKey(suppressive))).toBeNull();
    expect(m.layer()).toBeNull();
    expect(fx.pending()).toBe(0);
  });

  it("R2787000 auras the first view already shows stand without a begin, two at once, in their looks and scopes, and no definition reaches the page", () => {
    resetIds();
    const fx = fakeSeams();
    const suppressive = field("p1", "core-046");
    const weapons = field("p2", "core-014");
    const m = mount(board([suppressive], [null, null, weapons]), fx);
    expect(m.auras()).toHaveLength(2);
    expect(m.aura(fieldKey(suppressive))).toHaveAttribute("data-aura-phase", "standing");
    expect(m.aura(fieldKey(weapons))).toHaveAttribute("data-aura-phase", "standing");
    expect(m.aura(fieldKey(weapons))).toHaveAttribute("data-aura-look", "storm");
    expect(m.aura(fieldKey(weapons))).toHaveAttribute("data-aura-scope", "side");
    expect(m.aura(fieldKey(suppressive))?.style.getPropertyValue("--aura-body")).not.toBe("");
    expect(m.aura(fieldKey(suppressive))?.style.left).toBe(`${String(BOX.x)}px`);
    expect(m.layer()).toHaveAttribute("data-aura", "on");
    expect(m.layer()).toHaveAttribute("aria-hidden", "true");
    expect(document.body.innerHTML).not.toMatch(/core-\d{3}/);
  });

  it("R2787000 a face-down card wears no aura, for the viewer p1 and for p2", () => {
    resetIds();
    const fx = fakeSeams();
    const m = mount(board([field("p1", "core-018", { unrevealed: true })], [faceDownBackrow, faceDownBackrow]), fx);
    expect(m.layer()).toBeNull();
    m.show(board([field("p2", "core-018", { unrevealed: true })], [faceDownBackrow, faceDownBackrow], "p2"));
    expect(m.layer()).toBeNull();
    const revealed = field("p2", "core-018");
    m.show(board([revealed], [faceDownBackrow], "p2"));
    expect(m.aura(fieldKey(revealed))).toHaveAttribute("data-aura-look", "frost");
  });

  it("R2787000 auras stack by the ruling: a zone and its lane bury a side and a board aura over the same zone", () => {
    resetIds();
    const fx = fakeSeams();
    const toxins = field("p1", "classic-042");
    const argusland = field("p2", "classic-075");
    const weapons = field("p1", "core-014");
    const suppressive = field("p1", "core-046");
    const m = mount(board([toxins, null, weapons, null, suppressive], [argusland]), fx);
    expect(m.auras().map((aura) => aura.dataset.testid).sort()).toEqual([auraTestid(fieldKey(toxins)), auraTestid(fieldKey(argusland))].sort());
    // The nearest is drawn last, on top.
    expect(m.auras().at(-1)).toBe(m.aura(fieldKey(toxins)));
  });

  it("R2787000 the frame loop runs while an aura shows, stops while the tab is hidden and asks again when it comes back", () => {
    resetIds();
    const fx = fakeSeams();
    mount(board([field("p1", "core-046")], []), fx);
    expect(fx.pending()).toBe(1);
    fx.pump(100);
    expect(fx.pending()).toBe(1);
    const asked = fx.requested.length;
    fx.setHidden(true);
    expect(fx.pending()).toBe(0);
    expect(fx.cancelled.length).toBeGreaterThan(0);
    fx.pump(100);
    expect(fx.requested.length).toBe(asked);
    fx.setHidden(false);
    expect(fx.pending()).toBe(1);
    expect(fx.requested.length).toBe(asked + 1);
  });

  it("R2787000 without a 2D context the loop runs only until every aura has settled", () => {
    resetIds();
    const fx = fakeSeams();
    fx.seams.surface = () => null;
    const m = mount(board([], []), fx);
    const suppressive = field("p1", "core-046");
    m.show(board([suppressive], []));
    expect(fx.pending()).toBe(1);
    fx.pump(AURA_FADE_IN_MS + 20);
    expect(m.aura(fieldKey(suppressive))).toHaveAttribute("data-aura-phase", "standing");
    expect(fx.pending()).toBe(0);
  });

  it("R2787000 under reduced motion an aura is a still tint: no canvas, no frame and no begin", () => {
    resetIds();
    setReducedMotion(true);
    const fx = fakeSeams();
    const suppressive = field("p1", "core-046");
    const m = mount(board([suppressive], []), fx);
    expect(m.layer()).toHaveAttribute("data-aura", "still");
    expect(m.layer()).toHaveAttribute("data-aura-motes", "0");
    expect(m.canvas()).toBeNull();
    const weapons = field("p1", "core-014");
    m.show(board([suppressive, weapons], []));
    expect(m.aura(fieldKey(weapons))).toHaveAttribute("data-aura-phase", "standing");
    m.show(board([weapons], []));
    expect(m.aura(fieldKey(suppressive))).toBeNull();
    expect(fx.requested).toEqual([]);
  });

  it("R2787000 the settings panel's Reduce motion stills it too", () => {
    resetIds();
    writeSettings({ reduceMotion: true });
    const fx = fakeSeams();
    const m = mount(board([field("p1", "core-046")], []), fx);
    expect(m.layer()).toHaveAttribute("data-aura", "still");
    expect(m.canvas()).toBeNull();
    expect(fx.requested).toEqual([]);
  });

  it("R2787000 with the effects intensity off there is no aura at all, and turning them on begins none", () => {
    resetIds();
    setFxSettings({ intensity: "off" });
    const fx = fakeSeams();
    const suppressive = field("p1", "core-046");
    const m = mount(board([suppressive], []), fx);
    expect(m.layer()).toBeNull();
    expect(fx.requested).toEqual([]);
    act(() => {
      setFxSettings({ intensity: "normal" });
    });
    expect(m.aura(fieldKey(suppressive))).toHaveAttribute("data-aura-phase", "standing");
  });

  it("R2787000 at the phone's particle cap eight auras together keep within AURA_PARTICLE_SHARE of it", () => {
    resetIds();
    const fx = fakeSeams(390);
    const toxins = (owner: PlayerId) => field(owner, "classic-042");
    const m = mount(board([toxins("p1"), toxins("p1"), toxins("p1"), toxins("p1"), toxins("p1")], [toxins("p2"), toxins("p2"), toxins("p2")]), fx);
    expect(m.auras()).toHaveLength(8);
    const motes = Number(m.layer()?.dataset.auraMotes);
    expect(motes).toBeGreaterThan(0);
    expect(motes).toBeLessThanOrEqual(Math.floor(AURA_PARTICLE_SHARE * FX_PARTICLE_CAP_MOBILE));
  });

  it("R2787000 a modifier a readable play added wears that card's aura once the shown view lists it, and a viewer change forgets its source", () => {
    resetIds();
    const fx = fakeSeams();
    const before = board([], []);
    const events = [played("p1", "c9", "core-035"), modifierAdded("p1", "m5"), resolved("p1", "c9", "core-035")];
    const after = board([], [], "p1", { events });
    after.you.modifiers = [{ id: "m5", label: "Your Spells cost (1) less" }];
    const m = mount(before, fx);
    // The newest view has the modifier; the board still shows the one before it.
    m.show(before, after);
    expect(m.layer()).toBeNull();
    m.show(after, after);
    expect(m.aura("mod-p1-m5")).toHaveAttribute("data-aura-phase", "begin");
    expect(m.aura("mod-p1-m5")).toHaveAttribute("data-aura-look", "ember");
    expect(m.aura("mod-p1-m5")).toHaveAttribute("data-aura-scope", "side");
    // The other seat takes over: its window never saw the play.
    const handedOver = board([], [], "p2");
    handedOver.opponent.modifiers = [{ id: "m5", label: "Your Spells cost (1) less" }];
    m.show(handedOver, handedOver);
    expect(m.aura("mod-p1-m5")).toHaveAttribute("data-aura-phase", "end");
    fx.pump(AURA_FADE_OUT_MS);
    expect(m.layer()).toBeNull();
  });
});

describe("R3337003 an Anomaly's backdrop", () => {
  const wormhole: GameEvent = { type: "boardReplaced", player: "p1", anomaly: "WH" };

  it("R3337003 an Anomaly's backdrop stands over its caster's side with its code and picture until the side is replaced again, and none under reduced motion", () => {
    resetIds();
    const fx = fakeSeams();
    const before = board([], []);
    const laid = board([], [], "p1", { events: [wormhole] });
    const m = mount(before, fx);
    // The newest view has the lay; the board still shows the one before it.
    m.show(before, laid);
    expect(m.layer()).toBeNull();
    m.show(laid, laid);
    const backdrop = m.aura("anomaly-p1");
    expect(backdrop).toHaveAttribute("data-anomaly", "WH");
    expect(backdrop).toHaveAttribute("data-aura-look", "void");
    expect(backdrop).toHaveAttribute("data-aura-scope", "side");
    expect(backdrop).toHaveAttribute("data-aura-phase", "begin");
    // Its picture is the card art's own, the board's (B27: the layer itself draws no image).
    const picture = backdrop?.querySelector(".aura-picture");
    expect(picture).toHaveAttribute("data-art-anomaly", "WH");
    expect((picture as HTMLElement | null)?.style.backgroundImage).toContain(anomalyArtUri("WH", false));
    // A Field Spell's aura carries neither.
    const weapons = field("p2", "core-014");
    const both = board([], [weapons], "p1", { events: [wormhole] });
    m.show(both, both);
    expect(m.aura(fieldKey(weapons))).not.toHaveAttribute("data-anomaly");
    expect(m.aura(fieldKey(weapons))?.querySelector(".aura-picture")).toBeNull();
    // Replaced again with no Anomaly, it ends.
    const cleared = board([], [], "p1", { events: [wormhole, { type: "boardReplaced", player: "p1" }] });
    m.show(cleared, cleared);
    expect(m.aura("anomaly-p1")).toHaveAttribute("data-aura-phase", "end");
    fx.pump(AURA_FADE_OUT_MS);
    expect(m.layer()).toBeNull();
  });

  it("R3337003 reduced motion, the media query's or the panel's, lays no backdrop", () => {
    resetIds();
    setReducedMotion(true);
    const laid = board([], [], "p1", { events: [wormhole] });
    const still = mount(laid, fakeSeams());
    expect(still.aura("anomaly-p1")).toBeNull();
    expect(still.layer()).toBeNull();
    cleanup();
    setReducedMotion(false);
    writeSettings({ reduceMotion: true });
    const panel = mount(laid, fakeSeams());
    expect(panel.aura("anomaly-p1")).toBeNull();
    act(() => {
      writeSettings({ reduceMotion: false });
    });
    expect(panel.aura("anomaly-p1")).toHaveAttribute("data-anomaly", "WH");
  });
});

describe("R3351000 the living game's shared shed order", () => {
  it("R3351000 at shed stage 2 the auras far from the pointer halve their motes and the one under it keeps them", () => {
    resetIds();
    resetShedForTests();
    // Two auras in different boxes: the zone aura near, the side aura far.
    const near: FxBox = { x: 0, y: 0, width: 100, height: 100 };
    const far: FxBox = { x: 500, y: 500, width: 100, height: 100 };
    const fx = fakeSeams();
    fx.seams.measure = (testids) => (testids.length > 2 ? far : near);
    const view = board([field("p1", "core-014")], [field("p2", "classic-042")]);
    const m = mount(view, fx);
    expect(m.auras()).toHaveLength(2);
    // Each aura draws its whole budget of 24 motes (AURA_MOTES_PER_AURA at normal intensity).
    expect(m.layer()).toHaveAttribute("data-aura-motes", "48");
    // Shed to stage 2 with the pointer over the zone aura: the side aura halves.
    act(() => {
      setShedLevel(2);
      setShedPointer({ x: 50, y: 50 });
    });
    expect(m.layer()).toHaveAttribute("data-aura-motes", "36");
    // With no pointer known every aura counts as far.
    act(() => {
      setShedPointer(null);
    });
    expect(m.layer()).toHaveAttribute("data-aura-motes", "24");
    // Recovery draws whole again.
    act(() => {
      setShedLevel(0);
    });
    expect(m.layer()).toHaveAttribute("data-aura-motes", "48");
  });

  it("R3351000 pointer moves below stage 2 re-render and re-measure nothing", () => {
    resetIds();
    resetShedForTests();
    // Two auras in different boxes: the zone aura near, the side aura far.
    const near: FxBox = { x: 0, y: 0, width: 100, height: 100 };
    const far: FxBox = { x: 500, y: 500, width: 100, height: 100 };
    const fx = fakeSeams();
    let measures = 0;
    fx.seams.measure = (testids) => {
      measures += 1;
      return testids.length > 2 ? far : near;
    };
    const view = board([field("p1", "core-014")], [field("p2", "classic-042")]);
    const m = mount(view, fx);
    expect(m.auras()).toHaveLength(2);
    expect(m.layer()).toHaveAttribute("data-aura-motes", "48");
    const measured = measures;
    // Pointer moves at stage 0 change no snapshot: no re-render, no new measure, no mote change.
    act(() => {
      setShedPointer({ x: 10, y: 10 });
    });
    act(() => {
      setShedPointer({ x: 600, y: 600 });
    });
    act(() => {
      setShedPointer({ x: 50, y: 50 });
    });
    expect(measures).toBe(measured);
    expect(m.layer()).toHaveAttribute("data-aura-motes", "48");
  });
});
