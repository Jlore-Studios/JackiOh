// R502: the crystals #21 Hinder's rider will not fill, read off the view (manaMarks.ts) and marked on
// the board's own trays by FxLayer: from the shown view, and from the newest view as soon as the runner
// reaches the `modifierChanged` that lays the rider. The mark is information, so it stays under
// reduced motion and with the effects off.

import type { GameEvent, ModifierView, PlayerView } from "@jackioh/shared";
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { castOnDrawViews } from "../audio/test/realGame.ts";
import Board from "../game/Board.tsx";
import { createAnimationQueue, type AnimationQueue } from "../game/animations.ts";
import { baseView, emptySide, fullBoardView } from "../test/fixtures.ts";
import { setReducedMotion } from "../test/setup.ts";
import FxLayer, { type FxSeams } from "./FxLayer.tsx";
import { HINDERED_ATTR, applyManaMarks, crystalsShown, lostCrystals, manaMarks, nextRefreshRider } from "./manaMarks.ts";
import { resetFxSettingsForTests, setFxSettings } from "./settings.ts";

beforeEach(() => {
  localStorage.clear();
  resetFxSettingsForTests();
});

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  resetFxSettingsForTests();
  localStorage.clear();
});

const rider = (label: string): ModifierView => ({ id: "nextTurnMana", label });

/** p1 is the viewer; p1's next refresh is `lower` lower (the engine's own caption, U+2212). */
function hindered(lower: number, mana = { current: 1, max: 3 }): PlayerView {
  return baseView({ you: emptySide("p1", { mana, modifiers: [rider(`Next refresh −${String(lower)} mana`)] }) });
}

describe("R502 the rider as the view states it", () => {
  it("R502 reads the engine's caption: lower with its minus sign or a hyphen, higher with a plus; none without the badge", () => {
    expect(nextRefreshRider({ modifiers: [rider("Next refresh −1 mana")] })).toBe(-1);
    expect(nextRefreshRider({ modifiers: [rider("Next refresh -2 mana")] })).toBe(-2);
    expect(nextRefreshRider({ modifiers: [rider("Next refresh +4 mana")] })).toBe(4);
    expect(nextRefreshRider({ modifiers: [{ id: "m1", label: "Next refresh −1 mana" }] })).toBeNull();
    expect(nextRefreshRider({ modifiers: [rider("Next refresh")] })).toBeNull();
    expect(nextRefreshRider({ modifiers: [] })).toBeNull();
  });

  it("R502 the real engine's view of a Hinder cast on draw says 1 lower on both seats, which is what the client reads", () => {
    const deckA = [
      "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-008", "core-009", "core-010", "core-011",
      "core-012", "core-013", "core-014", "core-015", "core-016", "core-017", "core-018", "core-019", "core-020", "core-022",
    ];
    const deckB = [
      "core-021", "core-027", "core-030", "core-032", "core-033", "core-034", "core-035", "core-036", "core-037", "core-038",
      "core-040", "core-042", "core-043", "core-044", "core-045", "core-046", "core-047", "core-048", "core-051", "core-052",
    ];
    const seats = castOnDrawViews("r502", [deckA, deckB], "core-021", ({ p1 }) =>
      p1.you.modifiers.some((modifier) => modifier.id === "nextTurnMana"),
    );
    if (seats === null) throw new Error("no seed lowered p1's next refresh in its opening turns");
    expect(nextRefreshRider(seats.p1.you)).toBe(-1);
    expect(nextRefreshRider(seats.p2.opponent)).toBe(-1);
  });

  it("R502 the crystals lost are the last ones the tray draws, never more than it draws, and none for a rider that raises or is absent", () => {
    expect(crystalsShown({ current: 1, max: 3 })).toBe(3);
    expect(crystalsShown({ current: 6, max: 4 })).toBe(6);
    expect(lostCrystals(hindered(1), hindered(1), "p1")).toEqual({ from: 2, count: 1 });
    expect(lostCrystals(hindered(2), hindered(2), "p1")).toEqual({ from: 1, count: 2 });
    expect(lostCrystals(hindered(9), hindered(9), "p1")).toEqual({ from: 0, count: 3 });
    const raised = baseView({ you: emptySide("p1", { modifiers: [rider("Next refresh +2 mana")] }) });
    expect(lostCrystals(raised, raised, "p1").count).toBe(0);
    expect(lostCrystals(undefined, hindered(1), "p1").count).toBe(0);
    expect(lostCrystals(hindered(1), hindered(1), "p2").count).toBe(0);
    // The rider read off one view, the tray off another.
    expect(lostCrystals(hindered(1), baseView({ you: emptySide("p1", { mana: { current: 0, max: 5 } }) }), "p1")).toEqual({ from: 4, count: 1 });
  });

  it("R502 both trays of a view, the early sides read off the newer view", () => {
    const shown = baseView();
    const early = hindered(2, { current: 4, max: 4 });
    expect(manaMarks(shown)).toEqual([
      { side: "you", run: { from: 0, count: 0 } },
      { side: "opponent", run: { from: 0, count: 0 } },
    ]);
    expect(manaMarks(shown, { view: early, sides: new Set(["you"]) })[0]).toEqual({ side: "you", run: { from: 2, count: 2 } });
    expect(manaMarks(shown, { view: early, sides: new Set(["opponent"]) })[0]?.run.count).toBe(0);
  });
});

describe("R502 the mark on the board's trays", () => {
  it("R502 marks exactly the lost crystals and the tray's count, and takes it off again", () => {
    render(<Board view={hindered(2)} />);
    applyManaMarks(document, manaMarks(hindered(2)));
    const tray = document.querySelector('[data-testid="mana-you"]');
    const crystals = Array.from(document.querySelectorAll('[data-testid="mana-you"] .mana-crystal'));
    expect(tray?.getAttribute(HINDERED_ATTR)).toBe("2");
    expect(crystals.map((crystal) => crystal.getAttribute(HINDERED_ATTR))).toEqual([null, "true", "true"]);
    expect(document.querySelector(`[data-testid="mana-opponent"][${HINDERED_ATTR}]`)).toBeNull();
    applyManaMarks(document, manaMarks(baseView()));
    expect(document.querySelectorAll(`[${HINDERED_ATTR}]`)).toHaveLength(0);
  });
});

/* ------------------------------------------------------------------------------------------- *
 * FxLayer, next to a real board
 * ------------------------------------------------------------------------------------------- */

const QUIET: Partial<FxSeams> = {
  now: () => 0,
  frames: { request: () => 0, cancel: () => undefined },
  visibility: { hidden: () => false, subscribe: () => () => undefined },
  measure: () => null,
  surface: () => null,
  shakeSink: { apply: () => undefined, clear: () => undefined },
  catalog: () => undefined,
  viewportWidth: () => 1280,
  element: () => null,
};

function mount(queue: AnimationQueue, shown: PlayerView, latest: PlayerView) {
  const tree = (s: PlayerView, l: PlayerView) => (
    <div className="game">
      <Board view={s} />
      <FxLayer queue={queue} view={s} latest={l} seams={QUIET} />
    </div>
  );
  const utils = render(tree(shown, latest));
  return {
    rerender: (s: PlayerView, l: PlayerView) => utils.rerender(tree(s, l)),
    marked: (side: "you" | "opponent"): number =>
      document.querySelectorAll(`[data-testid="mana-${side}"] .mana-crystal[${HINDERED_ATTR}]`).length,
  };
}

/** A runner whose timers run only when `tick` runs the next one. */
function manualQueue(): { queue: AnimationQueue; tick: () => void } {
  const pending: (() => void)[] = [];
  const queue = createAnimationQueue({ schedule: (fn) => pending.push(fn), reducedMotion: false });
  return {
    queue,
    tick: () => {
      act(() => pending.shift()?.());
    },
  };
}

const LAYS: GameEvent = { type: "modifierChanged", player: "p1", modifierId: "nextTurnMana", added: true };
const BEFORE: GameEvent = { type: "manaChanged", player: "p2", current: 2, max: 2 };

describe("R502 FxLayer marks the trays", () => {
  it("R502 from the shown view: a view listing the rider marks its crystals, and the next view without it clears them", () => {
    const { queue } = manualQueue();
    const m = mount(queue, hindered(1), hindered(1));
    expect(m.marked("you")).toBe(1);
    m.rerender(baseView(), baseView());
    expect(m.marked("you")).toBe(0);
  });

  it("R502 from the newest view as the runner reaches the rider's modifierChanged, before the burst has played out", () => {
    const { queue, tick } = manualQueue();
    const shown = baseView({ you: emptySide("p1", { mana: { current: 1, max: 3 } }) });
    const latest = hindered(2);
    const m = mount(queue, shown, latest);
    act(() => queue.enqueue([BEFORE, LAYS, BEFORE], shown));
    expect(queue.inFlight()?.events[0]).toBe(BEFORE);
    expect(m.marked("you"), "not before the runner reaches it").toBe(0);
    // The first entry ends; the modifierChanged starts, and the board still shows the old view.
    tick();
    expect(queue.inFlight()?.events[0]).toBe(LAYS);
    expect(m.marked("you")).toBe(2);
    tick();
    expect(queue.idle()).toBe(false);
    expect(m.marked("you"), "held for the rest of the burst").toBe(2);
    // The board then shows the newest view, which keeps it.
    m.rerender(latest, latest);
    expect(m.marked("you")).toBe(2);
  });

  it("R502 the mark is information: drawn under reduced motion and with the effects off, where no effect is", () => {
    setReducedMotion(true);
    const { queue } = manualQueue();
    const m = mount(queue, hindered(1), hindered(1));
    expect(document.querySelector('[data-testid="fx-layer"]')?.getAttribute("data-fx")).toBe("off");
    expect(m.marked("you")).toBe(1);
    cleanup();
    setReducedMotion(false);
    act(() => {
      setFxSettings({ intensity: "off" });
    });
    const again = mount(manualQueue().queue, hindered(2), hindered(2));
    expect(again.marked("you")).toBe(2);
  });

  it("R502 unmounting the layer takes its marks off the board", () => {
    const { queue } = manualQueue();
    const utils = render(
      <div className="game">
        <Board view={fullBoardView()} />
      </div>,
    );
    const layer = render(<FxLayer queue={queue} view={hindered(1)} latest={hindered(1)} seams={QUIET} />);
    applyManaMarks(document, manaMarks(hindered(3, { current: 2, max: 4 })));
    expect(document.querySelectorAll(`.mana-crystal[${HINDERED_ATTR}]`).length).toBeGreaterThan(0);
    layer.unmount();
    expect(document.querySelectorAll(`[${HINDERED_ATTR}]`)).toHaveLength(0);
    utils.unmount();
  });
});
