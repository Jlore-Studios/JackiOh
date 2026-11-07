// R3348001 (#3348): a playmat's corner props in the DOM: what a tap sounds and shows, escalation, the rare
// outcome, a hidden sequence, the settings, the drain's quiet, the opponent's still half, and where they fit.

import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { setAudioEngineForTests } from "../../audio/engine.ts";
import { fakeAudioEngine, type FakeAudioEngine } from "../../audio/test/fakeAudio.ts";
import { resetFxSettingsForTests, setFxSettings } from "../../fx/settings.ts";
import { HAPTIC_PATTERNS, resetTapHapticsForTests } from "../../haptics/haptics.ts";
import { resetHapticsSettingsForTests, writeHapticsSettings } from "../../haptics/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../../settings/store.ts";
import {
  CORNER_BIG_MS,
  CORNER_ESCALATE_TAPS,
  CORNER_RARE_CHANCE,
  CORNER_RARE_MS,
  CORNER_SEQUENCE_MS,
  CORNER_STILL_MS,
  CORNER_TAP_MS,
} from "./config.ts";
import CornerProps from "./CornerProps.tsx";
import { setArenaRandomForTests } from "./play.ts";
import { propsOf } from "./props.ts";

const TAVERN = "table-tavern";
const [CANDLE, MUG] = propsOf(TAVERN);
if (CANDLE === undefined || MUG === undefined) throw new Error("the tavern has two props");

let audio: FakeAudioEngine;

beforeEach(() => {
  vi.useFakeTimers();
  audio = fakeAudioEngine();
  setAudioEngineForTests(audio);
  setArenaRandomForTests(() => 0.5);
});

afterEach(() => {
  cleanup();
  setAudioEngineForTests(null);
  setArenaRandomForTests(null);
  vi.useRealTimers();
  vi.restoreAllMocks();
  localStorage.clear();
  resetFxSettingsForTests();
  __resetSettingsForTests();
  resetHapticsSettingsForTests();
  resetTapHapticsForTests();
  Reflect.deleteProperty(navigator, "vibrate");
});

const buttons = (): HTMLElement[] => [...document.querySelectorAll<HTMLElement>("button.corner-prop")];
const button = (id: string): HTMLElement => {
  const found = buttons().find((candidate) => candidate.dataset.prop === id);
  if (found === undefined) throw new Error(`no prop ${id}`);
  return found;
};
const advance = (ms: number): void => {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
};
const tap = (id: string, times = 1): void => {
  for (let count = 0; count < times; count += 1) fireEvent.click(button(id));
};
const soundsOf = (id: string) => audio.playSfx.mock.calls.filter(([sfx]) => sfx === id);

describe("R3348001 a playmat's corner props", () => {
  it("R3348001 the viewer's half draws the mat's two props as buttons, and each names itself", () => {
    render(<CornerProps mat={TAVERN} half="yours" />);
    expect(buttons().map((candidate) => candidate.dataset.prop)).toEqual([CANDLE.id, MUG.id]);
    expect(buttons().map((candidate) => candidate.dataset.corner)).toEqual(["left", "right"]);
    for (const candidate of buttons()) {
      expect(candidate.getAttribute("aria-label")).toBe("A decoration on your playmat");
      expect(candidate.getAttribute("type")).toBe("button");
      expect(candidate.querySelector("svg")).not.toBeNull();
    }
    expect(document.querySelector(".corner-props")?.getAttribute("data-mat")).toBe(TAVERN);
  });

  it("R3348001 a tap sounds the prop's cue and moves it, and the move clears after CORNER_TAP_MS", () => {
    render(<CornerProps mat={TAVERN} half="yours" />);
    tap(CANDLE.id);
    expect(audio.playSfx).toHaveBeenCalledTimes(1);
    expect(audio.playSfx).toHaveBeenCalledWith(
      CANDLE.sfx,
      { variation: CANDLE.pitch, sandVariant: CANDLE.variant, sandBuild: 2 },
      0,
    );
    expect(button(CANDLE.id).dataset.state).toBe("tap");
    expect(button(CANDLE.id).dataset.play).toBe("move");
    expect(button(MUG.id).dataset.state).toBeUndefined();
    advance(CORNER_TAP_MS - 1);
    expect(button(CANDLE.id).dataset.state).toBe("tap");
    advance(1);
    expect(button(CANDLE.id).dataset.state).toBeUndefined();
    expect(button(CANDLE.id).dataset.play).toBeUndefined();
  });

  it("R3348001 a repeat tap restarts the animation: the art is a fresh element", () => {
    render(<CornerProps mat={TAVERN} half="yours" />);
    tap(CANDLE.id);
    const first = button(CANDLE.id).querySelector(".corner-prop__art");
    tap(CANDLE.id);
    expect(button(CANDLE.id).querySelector(".corner-prop__art")).not.toBe(first);
  });

  it("R3348001 the Nth tap in a streak escalates, with its bigger sound, and clears after CORNER_BIG_MS", () => {
    // 0.5 draws the middle goal: one past CORNER_ESCALATE_TAPS min.
    const goal = CORNER_ESCALATE_TAPS.min + 1;
    render(<CornerProps mat={TAVERN} half="yours" />);
    tap(CANDLE.id, goal - 1);
    expect(button(CANDLE.id).dataset.state).toBe("tap");
    audio.playSfx.mockClear();

    tap(CANDLE.id);
    expect(button(CANDLE.id).dataset.state).toBe("escalate");
    expect(audio.playSfx).toHaveBeenCalledTimes(3);
    expect(audio.playSfx.mock.calls[0]?.[1]?.sandBuild).toBe(6);
    advance(CORNER_BIG_MS);
    expect(button(CANDLE.id).dataset.state).toBeUndefined();
  });

  it("R3348001 a draw under CORNER_RARE_CHANCE from the client's random is the rare outcome, with four chimes", () => {
    // The goal draw, then the rare draw.
    const draws = [0.5, CORNER_RARE_CHANCE / 2];
    setArenaRandomForTests(() => draws.shift() ?? 0.5);
    render(<CornerProps mat={TAVERN} half="yours" />);
    tap(MUG.id);
    expect(button(MUG.id).dataset.state).toBe("rare");
    expect(soundsOf("tapAstral")).toHaveLength(4);
    advance(CORNER_RARE_MS);
    expect(button(MUG.id).dataset.state).toBeUndefined();
  });

  it("R3348001 the hidden sequence lights both props and plays four cues, then clears after CORNER_SEQUENCE_MS", () => {
    render(<CornerProps mat={TAVERN} half="yours" />);
    tap(MUG.id);
    tap(CANDLE.id);
    audio.playSfx.mockClear();
    tap(MUG.id);
    expect(button(CANDLE.id).dataset.state).toBe("sequence");
    expect(button(MUG.id).dataset.state).toBe("sequence");
    expect(audio.playSfx).toHaveBeenCalledTimes(4);
    advance(CORNER_SEQUENCE_MS);
    expect(button(CANDLE.id).dataset.state).toBeUndefined();
    expect(button(MUG.id).dataset.state).toBeUndefined();
  });

  it("R3348001 a mat with no hidden sequence lights only the prop tapped", () => {
    render(<CornerProps mat="table-wood" half="yours" />);
    for (const id of ["wood-coin", "wood-dice", "wood-coin"]) tap(id);
    expect(button("wood-dice").dataset.state).toBe("tap");
    expect(button("wood-coin").dataset.state).toBe("tap");
  });

  it("R3348001 while the animation queue drains a prop answers quietly: one soft cue, still, and no buzz", () => {
    const vibrate = vi.fn((_pattern: number[]) => true);
    Object.defineProperty(navigator, "vibrate", { configurable: true, value: vibrate });
    render(<CornerProps mat={TAVERN} half="yours" draining />);
    fireEvent.pointerDown(button(CANDLE.id), { pointerType: "touch" });
    tap(CANDLE.id);
    expect(audio.playSfx).toHaveBeenCalledTimes(1);
    expect(audio.playSfx.mock.calls[0]?.[1]?.quiet).toBe(true);
    expect(button(CANDLE.id).dataset.play).toBe("still");
    expect(vibrate).not.toHaveBeenCalled();
    advance(CORNER_STILL_MS);
    expect(button(CANDLE.id).dataset.state).toBeUndefined();
  });

  it("R3348001 reduced motion keeps the sound and shows the change still", () => {
    writeSettings({ reduceMotion: true });
    render(<CornerProps mat={TAVERN} half="yours" />);
    tap(CANDLE.id);
    expect(audio.playSfx).toHaveBeenCalledTimes(1);
    expect(audio.playSfx.mock.calls[0]?.[1]?.quiet).toBeUndefined();
    expect(button(CANDLE.id).dataset.state).toBe("tap");
    expect(button(CANDLE.id).dataset.play).toBe("still");
    advance(CORNER_STILL_MS);
    expect(button(CANDLE.id).dataset.state).toBeUndefined();
  });

  it("R3348001 with effects off a prop is a sound and nothing more", () => {
    setFxSettings({ intensity: "off" });
    render(<CornerProps mat={TAVERN} half="yours" />);
    tap(CANDLE.id);
    expect(audio.playSfx).toHaveBeenCalledTimes(1);
    expect(button(CANDLE.id).dataset.state).toBeUndefined();
    expect(button(CANDLE.id).dataset.play).toBeUndefined();
  });

  it("R3348001 a touch press buzzes by its outcome, a mouse press or a key does not, and the Vibration switch silences it", () => {
    const vibrate = vi.fn((_pattern: number[]) => true);
    Object.defineProperty(navigator, "vibrate", { configurable: true, value: vibrate });
    let now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => (now += 1000));
    render(<CornerProps mat={TAVERN} half="yours" />);

    fireEvent.pointerDown(button(CANDLE.id), { pointerType: "mouse" });
    tap(CANDLE.id);
    tap(CANDLE.id); // a key press: a click with no pointer down
    expect(vibrate).not.toHaveBeenCalled();

    fireEvent.pointerDown(button(CANDLE.id), { pointerType: "touch" });
    tap(CANDLE.id);
    expect(vibrate).toHaveBeenLastCalledWith([...HAPTIC_PATTERNS.corner]);

    vibrate.mockClear();
    writeHapticsSettings({ vibration: false });
    fireEvent.pointerDown(button(CANDLE.id), { pointerType: "touch" });
    tap(CANDLE.id);
    expect(vibrate).not.toHaveBeenCalled();
  });

  it("R3348001 the opponent's half is the same props, aria-hidden, with no button, no state and no sound", () => {
    render(<CornerProps mat="table-astral" half="theirs" />);
    expect(buttons()).toEqual([]);
    const props = [...document.querySelectorAll<HTMLElement>(".corner-prop")];
    expect(props.map((candidate) => candidate.dataset.prop)).toEqual(["astral-comet", "astral-satellite"]);
    for (const candidate of props) {
      expect(candidate.getAttribute("aria-hidden")).toBe("true");
      fireEvent.pointerDown(candidate);
      fireEvent.click(candidate);
      expect(candidate.dataset.state).toBeUndefined();
      expect(candidate.dataset.play).toBeUndefined();
    }
    expect(audio.playSfx).not.toHaveBeenCalled();
    expect(document.querySelector(".corner-props")?.getAttribute("data-half")).toBe("theirs");
  });

  it("R3348001 a new playmat starts its streaks again", () => {
    const view = render(<CornerProps mat={TAVERN} half="yours" />);
    tap(CANDLE.id, 3);
    view.rerender(<CornerProps mat="table-wood" half="yours" />);
    expect(buttons().map((candidate) => candidate.dataset.prop)).toEqual(["wood-dice", "wood-coin"]);
    expect(buttons().every((candidate) => candidate.dataset.state === undefined)).toBe(true);
    // Five more taps on the old mat's count would have escalated; here they are plain.
    tap("wood-dice", 5);
    expect(button("wood-dice").dataset.state).toBe("tap");
  });
});

describe("R3348001 where a prop may show", () => {
  /** Boxes by class, as a layout would give them: the zone fills the top left; props sit where a test puts them. */
  function layout(at: { left: number; top: number } | null): void {
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.classList.contains("zone")) return new DOMRect(0, 0, 200, 200);
      if (this.classList.contains("corner-prop") && at !== null) return new DOMRect(at.left, at.top, 44, 44);
      return new DOMRect(0, 0, 0, 0);
    });
  }

  const board = (): ReturnType<typeof render> =>
    render(
      <div className="board">
        <div className="zone" />
        <CornerProps mat={TAVERN} half="yours" />
      </div>,
    );

  it("R3348001 a prop with no layout shows (a test's DOM), and one clear of every tile does too", () => {
    layout(null);
    board();
    expect(buttons().map((candidate) => candidate.dataset.fit)).toEqual(["true", "true"]);
    cleanup();

    layout({ left: 300, top: 300 });
    board();
    expect(buttons().map((candidate) => candidate.dataset.fit)).toEqual(["true", "true"]);
  });

  it("R3348001 a prop over a tile, or off the screen, is not shown and takes no tap", () => {
    layout({ left: 20, top: 20 });
    board();
    expect(buttons().map((candidate) => candidate.dataset.fit)).toEqual(["false", "false"]);
    cleanup();

    layout({ left: window.innerWidth, top: 300 });
    board();
    expect(buttons().map((candidate) => candidate.dataset.fit)).toEqual(["false", "false"]);
  });

  it("R3348001 the fit is measured again when the window resizes", () => {
    layout({ left: 300, top: 300 });
    board();
    expect(button(CANDLE.id).dataset.fit).toBe("true");
    layout({ left: 20, top: 20 });
    act(() => {
      window.dispatchEvent(new Event("resize"));
    });
    expect(button(CANDLE.id).dataset.fit).toBe("false");
  });
});

