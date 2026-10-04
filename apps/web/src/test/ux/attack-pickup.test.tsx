// R654 through the board: picking up one of your own Units to attack plays its `attack` hook, as in
// Hearthstone. Driven through `<Game/>` as a player's pointer drives it (drag-layer.test.tsx says
// how: `document.elementsFromPoint` is stubbed, the press goes to the element, the moves and the
// release to `window`), with the audio singleton swapped for a fake whose `playPickup` is the spy.
//
// What the engine does with a pick-up (the hook's effect and line, the gap, the cut-off) is the
// engine's own test. This one is the wiring: which gestures reach `playPickup`, how often, with
// which defId, and that nothing else the board does changes (CLAUDE.md rule 7: the cue is never a
// rule and is never sent).

import type { ActionBody, ActivationView, PlayerView, Selection } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import { setAudioEngineForTests } from "../../audio/engine.ts";
import { resetAudioSettingsForTests } from "../../audio/settings.ts";
import type { AudioEngine } from "../../audio/types.ts";
import Game from "../../game/Game.tsx";
import { DRAG_THRESHOLD_PX } from "../../game/drag/model.ts";
import { __resetSettingsForTests, writeSettings } from "../../settings/index.ts";
import { baseView, emptySide, unit } from "../fixtures.ts";

// ---------------------------------------------------------------------------------------------
// The fixture.
// ---------------------------------------------------------------------------------------------

/** The Rock: its `attack` hook is an effect ("rumble") in card-audio.json5. */
const ROCK = "core-066";
/** Tempo Timmy, a second attacker with a hook of its own ("zip"). */
const TIMMY = "core-011";
/** 4-mana 7/7: it has an `attack` hook, but `legal` gives it no attack this turn. */
const SEVEN_SEVEN = "core-025";
/** Brother Ping: no attack, one aimed Activate ability, so a drag lifts the ability instead (R384). */
const PING_UNIT = "classicplus-076-1";
/** Lava Golem, the opponent's: a hook of its own ("heavyStomp"), and never yours to pick up. */
const GOLEM = "core-055";

const PING: ActivationView = { ability: "ping", label: "Deal {damage} damage.", usesLeft: 1, usable: true };

/**
 * u1 (The Rock) may attack e1 or the enemy hero; u3 (Tempo Timmy) only the hero. u2 may only switch
 * position, and act1 (Brother Ping) may only ping e1. The opponent's e1 is a Lava Golem.
 */
function pickupView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", { instanceId: "u1", defId: ROCK }),
        unit("p1", { instanceId: "u2", defId: SEVEN_SEVEN, canAct: false }),
        unit("p1", { instanceId: "u3", defId: TIMMY }),
        unit("p1", { instanceId: "act1", defId: PING_UNIT, params: { damage: 1 }, activations: [PING] }),
        null,
      ],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1", defId: GOLEM }), null, null, null, null],
    }),
  });
}

const at = (instanceId: string): Selection => ({ pick: "instance", instanceId });
const U1_E1: ActionBody = { type: "attack", attackerId: "u1", targetId: "e1" };
const U1_HERO: ActionBody = { type: "attack", attackerId: "u1", targetId: "hero-p2" };
const U3_HERO: ActionBody = { type: "attack", attackerId: "u3", targetId: "hero-p2" };
const PING_E1: ActionBody = { type: "activate", instanceId: "act1", ability: "ping", targets: [at("e1")] };

const LEGAL: readonly ActionBody[] = [
  U1_E1,
  U1_HERO,
  U3_HERO,
  PING_E1,
  { type: "switchPosition", instanceId: "u2" },
  { type: "endTurn" },
  { type: "offerDraw" },
  { type: "concede" },
];

function renderGame() {
  const onAction = vi.fn<(body: ActionBody) => void>();
  const utils = render(<Game view={pickupView()} legal={LEGAL} onAction={onAction} />);
  return { ...utils, onAction };
}

const el = (testid: string): HTMLElement => screen.getByTestId(testid);

// ---------------------------------------------------------------------------------------------
// The audio engine.
// ---------------------------------------------------------------------------------------------

type FakeEngine = { [K in keyof AudioEngine]: Mock<AudioEngine[K]> };

/** A stand-in for the singleton: every method a spy, every answer the quiet one. */
function fakeEngine(): FakeEngine {
  return {
    playSfx: vi.fn<AudioEngine["playSfx"]>(() => true),
    playVoice: vi.fn<AudioEngine["playVoice"]>(() => true),
    playEffect: vi.fn<AudioEngine["playEffect"]>(() => true),
    playPickup: vi.fn<AudioEngine["playPickup"]>(() => true),
    state: vi.fn<AudioEngine["state"]>(() => "running"),
    unlock: vi.fn<AudioEngine["unlock"]>(),
    preloadVoices: vi.fn<AudioEngine["preloadVoices"]>(),
    setBusy: vi.fn<AudioEngine["setBusy"]>(),
    log: vi.fn<AudioEngine["log"]>(() => []),
    clearLog: vi.fn<AudioEngine["clearLog"]>(),
    contextsCreated: vi.fn<AudioEngine["contextsCreated"]>(() => 1),
    speaking: vi.fn<AudioEngine["speaking"]>(() => false),
    subscribeSpeaking: vi.fn<AudioEngine["subscribeSpeaking"]>(() => () => undefined),
    musicOutput: vi.fn<AudioEngine["musicOutput"]>(() => null),
    subscribeState: vi.fn<AudioEngine["subscribeState"]>(() => () => undefined),
    dispose: vi.fn<AudioEngine["dispose"]>(),
  };
}

let engine: FakeEngine;

/** The defIds `playPickup` was called with, in order. */
const pickups = (): string[] => engine.playPickup.mock.calls.map((call) => call[0]);

// ---------------------------------------------------------------------------------------------
// The pointer.
// ---------------------------------------------------------------------------------------------

const POINTER = 1;
const START = { x: 200, y: 400 };

/** What `document.elementsFromPoint` answers, topmost first. */
let under: Element[] = [];

function over(...stack: Element[]): void {
  under = stack;
}

function press(target: Element, button = 0): void {
  fireEvent.pointerDown(target, { pointerId: POINTER, button, clientX: START.x, clientY: START.y });
}

function move(x: number, y: number, button = 0): void {
  fireEvent.pointerMove(window, { pointerId: POINTER, button, clientX: x, clientY: y });
}

function release(x = START.x, y = START.y): void {
  fireEvent.pointerUp(window, { pointerId: POINTER, button: 0, clientX: x, clientY: y });
}

/** Press the source and move it straight up exactly the threshold: the drag is now in flight. */
function lift(source: Element): void {
  over(source);
  press(source);
  move(START.x, START.y - DRAG_THRESHOLD_PX);
}

beforeEach(() => {
  under = [];
  document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
  localStorage.clear();
  resetAudioSettingsForTests();
  engine = fakeEngine();
  setAudioEngineForTests(engine);
});

afterEach(() => {
  cleanup();
  setAudioEngineForTests(null);
  delete (document as Partial<Document>).elementsFromPoint;
  document.documentElement.removeAttribute("data-dragging");
  localStorage.clear();
  __resetSettingsForTests();
  resetAudioSettingsForTests();
});

// ---------------------------------------------------------------------------------------------
// A drag.
// ---------------------------------------------------------------------------------------------

describe("R654 a drag that lifts your attacker plays its pick-up, once per lift", () => {
  it("R654 a press on your attacker plays nothing until it travels the threshold, then plays once with the Unit's defId and sends nothing", () => {
    const { onAction } = renderGame();
    const source = el("card-u1");

    over(source);
    press(source);
    move(START.x, START.y - (DRAG_THRESHOLD_PX - 1));
    expect(engine.playPickup).not.toHaveBeenCalled();

    move(START.x, START.y - DRAG_THRESHOLD_PX);
    expect(document.documentElement).toHaveAttribute("data-dragging", "attack");
    expect(engine.playPickup).toHaveBeenCalledTimes(1);
    expect(engine.playPickup).toHaveBeenCalledWith(ROCK);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R654 the rest of the drag plays nothing more: moving over its targets and off them again", () => {
    renderGame();
    lift(el("card-u1"));

    over(el("hero-opponent"));
    move(600, 60);
    over(el("card-e1"));
    move(500, 120);
    over(el("board"));
    move(420, 300);
    move(410, 310);

    expect(pickups()).toEqual([ROCK]);
  });

  it("R654 dropping it on a target sends the attack and plays nothing more, nor does the click after the release", () => {
    const { onAction } = renderGame();
    const source = el("card-u1");
    lift(source);

    over(el("hero-opponent"));
    move(600, 60);
    release(600, 60);
    fireEvent.click(source);

    expect(onAction.mock.calls).toEqual([[U1_HERO]]);
    expect(pickups()).toEqual([ROCK]);
  });

  it("R654 dropping it on the empty board sends nothing and plays nothing more", () => {
    const { onAction } = renderGame();
    const source = el("card-u1");
    lift(source);

    over(el("board"));
    move(420, 300);
    release(420, 300);
    fireEvent.click(source);

    expect(onAction).not.toHaveBeenCalled();
    expect(document.documentElement).not.toHaveAttribute("data-dragging");
    expect(pickups()).toEqual([ROCK]);
  });

  const CANCELS: [string, () => void][] = [
    ["Escape", () => fireEvent.keyDown(window, { key: "Escape" })],
    ["the right button", () => move(420, 300, 2)],
    ["contextmenu", () => fireEvent.contextMenu(el("board"))],
    ["pointercancel", () => fireEvent.pointerCancel(window, { pointerId: POINTER })],
  ];

  it.each(CANCELS)("R654 %s cancels the drag and plays nothing more", (_name, cancel) => {
    const { onAction } = renderGame();
    const source = el("card-u1");
    lift(source);
    expect(pickups()).toEqual([ROCK]);

    cancel();
    release();
    fireEvent.click(source);

    expect(document.documentElement).not.toHaveAttribute("data-dragging");
    expect(onAction).not.toHaveBeenCalled();
    expect(pickups()).toEqual([ROCK]);
  });

  it("R654 each pick-up plays: lifting the same attacker again plays it again, and another attacker plays its own", () => {
    const { onAction } = renderGame();

    lift(el("card-u1"));
    over(el("board"));
    release(420, 300);
    expect(pickups()).toEqual([ROCK]);

    lift(el("card-u1"));
    expect(pickups()).toEqual([ROCK, ROCK]);
    over(el("card-e1"));
    move(500, 120);
    release(500, 120);

    lift(el("card-u3"));
    expect(pickups()).toEqual([ROCK, ROCK, TIMMY]);
    fireEvent.keyDown(window, { key: "Escape" });

    expect(onAction.mock.calls).toEqual([[U1_E1]]);
    expect(pickups()).toEqual([ROCK, ROCK, TIMMY]);
  });
});

// ---------------------------------------------------------------------------------------------
// What never plays.
// ---------------------------------------------------------------------------------------------

describe("R654 only a Unit `legal` lets attack is picked up", () => {
  it("R654 your Unit with no legal attack plays nothing, dragged or clicked, though its card has an attack hook", () => {
    const { onAction } = renderGame();
    const source = el("card-u2");

    lift(source);
    expect(document.documentElement).not.toHaveAttribute("data-dragging");
    move(START.x, START.y - 60);
    release(START.x, START.y - 60);
    fireEvent.click(source);

    expect(source).not.toHaveAttribute("data-selected");
    expect(engine.playPickup).not.toHaveBeenCalled();
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R654 your Unit whose drag lifts its Activate ability, not an attack, plays nothing, dragged or clicked", () => {
    renderGame();
    const source = el("card-act1");

    lift(source);
    expect(document.documentElement).toHaveAttribute("data-dragging", "activate");
    fireEvent.keyDown(window, { key: "Escape" });
    release();

    over(source);
    press(source);
    release();
    fireEvent.click(source);

    expect(engine.playPickup).not.toHaveBeenCalled();
  });

  it("R654 the opponent's Unit plays nothing, dragged or clicked", () => {
    const { onAction } = renderGame();
    const source = el("card-e1");

    lift(source);
    expect(document.documentElement).not.toHaveAttribute("data-dragging");
    release(START.x, START.y - DRAG_THRESHOLD_PX);
    fireEvent.click(source);

    expect(engine.playPickup).not.toHaveBeenCalled();
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R654 a press that never travels the threshold and is no click on an attacker plays nothing", () => {
    const { onAction } = renderGame();
    const source = el("card-u1");

    over(source);
    press(source);
    move(START.x, START.y - (DRAG_THRESHOLD_PX - 1));
    release(START.x, START.y - (DRAG_THRESHOLD_PX - 1));

    over(el("board"));
    press(el("board"));
    move(START.x, START.y - 60);
    release(START.x, START.y - 60);
    fireEvent.click(el("board"));

    expect(engine.playPickup).not.toHaveBeenCalled();
    expect(onAction).not.toHaveBeenCalled();
    expect(source).not.toHaveAttribute("data-selected");
  });

  it("R654 the same short press followed by its click on the attacker is a pick-up by click, and plays once", () => {
    renderGame();
    const source = el("card-u1");

    over(source);
    press(source);
    move(START.x, START.y - (DRAG_THRESHOLD_PX - 1));
    release(START.x, START.y - (DRAG_THRESHOLD_PX - 1));
    fireEvent.click(source);

    expect(source).toHaveAttribute("data-selected", "true");
    expect(pickups()).toEqual([ROCK]);
  });
});

// ---------------------------------------------------------------------------------------------
// Click-click.
// ---------------------------------------------------------------------------------------------

describe("R654 click-click: choosing your attacker is a pick-up", () => {
  it("R654 clicking your attacker plays once, and clicking its target attacks, plays nothing more and sends exactly the attack", () => {
    const { onAction } = renderGame();

    fireEvent.click(el("card-u1"));
    expect(el("card-u1")).toHaveAttribute("data-selected", "true");
    expect(pickups()).toEqual([ROCK]);
    expect(onAction).not.toHaveBeenCalled();

    fireEvent.click(el("hero-opponent"));

    expect(onAction.mock.calls).toEqual([[U1_HERO]]);
    expect(pickups()).toEqual([ROCK]);
  });

  it("R654 clicking the chosen attacker again deselects it and plays nothing; choosing it once more plays again", () => {
    const { onAction } = renderGame();
    const source = el("card-u1");

    fireEvent.click(source);
    fireEvent.click(source);
    expect(source).not.toHaveAttribute("data-selected");
    expect(pickups()).toEqual([ROCK]);

    fireEvent.click(source);
    expect(source).toHaveAttribute("data-selected", "true");
    expect(pickups()).toEqual([ROCK, ROCK]);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R654 a click that leaves the chosen attacker chosen (your own Unit, not its target) plays nothing more", () => {
    const { onAction } = renderGame();

    fireEvent.click(el("card-u1"));
    fireEvent.click(el("card-u2"));
    fireEvent.click(el("card-u3"));

    expect(el("card-u1")).toHaveAttribute("data-selected", "true");
    expect(pickups()).toEqual([ROCK]);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R654 with drag to play off, a long move on your attacker plays nothing, and its click still plays once", () => {
    writeSettings({ dragToPlay: false });
    const { onAction } = renderGame();
    const source = el("card-u1");

    lift(source);
    move(START.x, START.y - 60);
    release(START.x, START.y - 60);
    expect(engine.playPickup).not.toHaveBeenCalled();

    fireEvent.click(source);
    fireEvent.click(el("card-e1"));

    expect(pickups()).toEqual([ROCK]);
    expect(onAction.mock.calls).toEqual([[U1_E1]]);
  });

  it("R654 a pick-up the engine refuses or fails on changes nothing the board does: the attacker is chosen and the attack sent as before", () => {
    engine.playPickup.mockImplementationOnce(() => false).mockImplementationOnce(() => {
      throw new Error("no audio");
    });
    const { onAction } = renderGame();

    fireEvent.click(el("card-u1"));
    fireEvent.click(el("hero-opponent"));
    lift(el("card-u1"));
    expect(document.documentElement).toHaveAttribute("data-dragging", "attack");
    over(el("card-e1"));
    move(500, 120);
    release(500, 120);

    expect(pickups()).toEqual([ROCK, ROCK]);
    expect(onAction.mock.calls).toEqual([[U1_HERO], [U1_E1]]);
  });
});
