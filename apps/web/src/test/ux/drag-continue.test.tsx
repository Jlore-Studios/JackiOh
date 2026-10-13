// R658: pointer-driven follow-up choices through `<Game/>`.
// jsdom's `elementsFromPoint` is stubbed; presses target elements and moves/releases target `window`.
// Expected bodies come from `legal` and are compared with click-click actions.

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import Game from "../../game/Game.tsx";
import { DRAG_THRESHOLD_PX } from "../../game/drag/model.ts";
import { __resetSettingsForTests, writeSettings } from "../../settings/index.ts";
import { baseView, card, emptySide, faceUpBackrow, pendingFor, unit } from "../fixtures.ts";

// The fixture.

/** c1's Cry targets e1 or the hero after lane 3; fs1 activates on the board without a target. */
function boardView(over: Partial<PlayerView> = {}): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: "c1", defId: "core-068", cost: 2 })],
      units: [unit("p1", { instanceId: "u1" }), null, null, null, null],
      backrow: [
        faceUpBackrow("p1", {
          instanceId: "fs1",
          defId: "classic-020",
          activations: [{ ability: "punish", label: "Discard a card", usesLeft: 1, usable: true }],
        }),
        null,
        null,
        null,
        null,
      ],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1" }), null, null, null, null],
    }),
    ...over,
  });
}

const C1_E1: ActionBody = {
  type: "play",
  instanceId: "c1",
  zone: { row: "units", lane: 3 },
  targets: [{ pick: "instance", instanceId: "e1" }],
};
const C1_HERO: ActionBody = {
  type: "play",
  instanceId: "c1",
  zone: { row: "units", lane: 3 },
  targets: [{ pick: "hero", player: "p2" }],
};
const FS1: ActionBody = { type: "activate", instanceId: "fs1", ability: "punish" };
const LEGAL: readonly ActionBody[] = [C1_E1, C1_HERO, FS1, { type: "endTurn" }, { type: "concede" }];

const el = (testid: string): HTMLElement => screen.getByTestId(testid);

// The pointer.

const POINTER = 1;
const START = { x: 200, y: 400 };
let under: Element[] = [];

function over(...stack: Element[]): void {
  under = stack;
}

function press(target: Element): void {
  fireEvent.pointerDown(target, { pointerId: POINTER, button: 0, clientX: START.x, clientY: START.y });
}

function move(x: number, y: number): void {
  fireEvent.pointerMove(window, { pointerId: POINTER, clientX: x, clientY: y });
}

function release(x = START.x, y = START.y): void {
  fireEvent.pointerUp(window, { pointerId: POINTER, button: 0, clientX: x, clientY: y });
}

function lift(source: Element): void {
  over(source);
  press(source);
  move(START.x, START.y - 20);
}

function dragOnto(source: Element, target: Element): void {
  lift(source);
  over(target);
  move(400, 120);
  release(400, 120);
}

beforeEach(() => {
  under = [];
  document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
});

afterEach(() => {
  cleanup();
  delete (document as Partial<Document>).elementsFromPoint;
  document.documentElement.removeAttribute("data-dragging");
  localStorage.clear();
  __resetSettingsForTests();
});

// A play's second choice

describe("R658 after a drop places a card, its Cry target is dragged to from the zone", () => {
  it("R658 the zone a card was dropped in lifts the play again: the arrow starts there and its targets glow", () => {
    const onAction = vi.fn();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);

    dragOnto(el("hand-card-c1"), el("zone-you-units-3"));
    expect(onAction).not.toHaveBeenCalled();
    expect(el("zone-you-units-3")).toHaveAttribute("data-selected", "true");

    lift(el("zone-you-units-3"));
    expect(el("drag-layer")).toHaveAttribute("data-kind", "play");
    expect(el("drag-arrow")).toHaveAttribute("data-from", "zone-you-units-3");
    expect(el("card-e1")).toHaveAttribute("data-glow", "ready");
    expect(el("hero-opponent")).toHaveAttribute("data-glow", "ready");
    expect(el("zone-you-units-3")).toHaveAttribute("data-selected", "true");
  });

  it("R658 the second drop sends the play click-click sends, and the card lands in its zone", () => {
    const clicks = vi.fn();
    const first = render(<Game view={boardView()} legal={LEGAL} onAction={clicks} />);
    fireEvent.click(el("hand-card-c1"));
    fireEvent.click(el("zone-you-units-3"));
    fireEvent.click(el("hero-opponent"));
    expect(clicks).toHaveBeenCalledWith(C1_HERO);
    first.unmount();

    const onAction = vi.fn();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);
    dragOnto(el("hand-card-c1"), el("zone-you-units-3"));
    dragOnto(el("zone-you-units-3"), el("hero-opponent"));

    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(clicks.mock.calls[0]?.[0]);
    expect(el("drag-landing-card")).toHaveAttribute("data-instance-id", "c1");
  });

  it("R658 a second drag released on nothing keeps the card in its zone; Escape then backs out of the play", () => {
    const onAction = vi.fn();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);
    dragOnto(el("hand-card-c1"), el("zone-you-units-3"));

    dragOnto(el("zone-you-units-3"), el("board"));
    expect(onAction).not.toHaveBeenCalled();
    expect(screen.queryByTestId("drag-layer")).toBeNull();
    expect(el("zone-you-units-3")).toHaveAttribute("data-selected", "true");
    expect(el("card-e1")).toHaveAttribute("data-glow", "ready");

    fireEvent.keyDown(window, { key: "Escape" });
    expect(el("zone-you-units-3")).not.toHaveAttribute("data-selected");
  });

  it("R658 click-click still finishes a play a drop started", () => {
    const onAction = vi.fn();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);
    dragOnto(el("hand-card-c1"), el("zone-you-units-3"));
    // A press disarms the drop's click swallow (B39).
    over(el("card-e1"));
    press(el("card-e1"));
    release();
    fireEvent.click(el("card-e1"));
    expect(onAction).toHaveBeenCalledWith(C1_E1);
  });

  it("R658 with drag to play off a pick is not lifted", () => {
    writeSettings({ dragToPlay: false });
    const onAction = vi.fn();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);
    fireEvent.click(el("hand-card-c1"));
    fireEvent.click(el("zone-you-units-3"));
    lift(el("zone-you-units-3"));
    expect(screen.queryByTestId("drag-layer")).toBeNull();
  });
});

// A backrow card whose ability aims at nothing

describe("R658 a backrow card whose ability aims at nothing is dragged onto the board", () => {
  it("R658 it lifts as a ghost of the card and a drop on the board sends what its click would", () => {
    const onAction = vi.fn();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);

    lift(el("card-fs1"));
    expect(el("drag-layer")).toHaveAttribute("data-kind", "activate");
    expect(el("drag-ghost")).toHaveAttribute("data-instance-id", "fs1");
    expect(screen.queryByTestId("drag-arrow")).toBeNull();

    over(el("board"));
    move(400, 120);
    expect(el("drag-ghost")).toHaveAttribute("data-valid", "true");
    release(400, 120);
    expect(onAction).toHaveBeenCalledWith(FS1);
  });

  it("R658 released back over the hand it sends nothing", () => {
    const onAction = vi.fn();
    render(<Game view={boardView()} legal={LEGAL} onAction={onAction} />);
    dragOnto(el("card-fs1"), el("hand-you"));
    expect(onAction).not.toHaveBeenCalled();
  });
});

// A prompt's options

const DISCOVER = pendingFor("discover", [
  { key: "mode:core-043", label: "Flood", defId: "core-043" },
  { key: "mode:core-055", label: "Archivist", defId: "core-055" },
]);
const ANSWER: ActionBody = { type: "answer", choiceId: "ch1", selection: [{ pick: "mode", option: "core-055" }] };

function renderDiscover(onAction = vi.fn()) {
  render(<Game view={boardView({ pending: DISCOVER })} legal={[ANSWER, { type: "concede" }]} onAction={onAction} />);
  return onAction;
}

/** Give the picker a box so drops can land inside or outside. */
function panelBox(): void {
  const panel = el("prompt-modal");
  panel.getBoundingClientRect = () => ({ left: 100, top: 300, right: 500, bottom: 600, width: 400, height: 300, x: 100, y: 300, toJSON: () => ({}) });
}

describe("R658 a prompt's option is dragged out of the picker", () => {
  it("R658 a lifted option shows its ghost and fades in place, with the picker still shown", () => {
    renderDiscover();
    panelBox();
    const option = el("prompt-option-mode:core-055");
    press(option);
    move(START.x, START.y - DRAG_THRESHOLD_PX);

    expect(document.documentElement).toHaveAttribute("data-dragging", "option");
    expect(el("drag-option-ghost")).toHaveAttribute("data-option", "prompt-option-mode:core-055");
    expect(el("drag-option-ghost")).toHaveAttribute("data-valid", "false");
    expect(option).toHaveAttribute("data-drag-source", "true");

    move(300, 100);
    expect(el("drag-option-ghost")).toHaveAttribute("data-valid", "true");
  });

  it("R658 released outside the panel it answers as its click does, once", () => {
    const onAction = renderDiscover();
    panelBox();
    const option = el("prompt-option-mode:core-055");
    press(option);
    move(300, 100);
    release(300, 100);
    // Swallow the release click so it cannot pick twice.
    fireEvent.click(option);

    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(ANSWER);
    expect(screen.queryByTestId("drag-option-ghost")).toBeNull();
    expect(document.documentElement).not.toHaveAttribute("data-dragging");
  });

  it("R658 released back over the panel, or cancelled with Escape, it sends nothing", () => {
    const onAction = renderDiscover();
    panelBox();
    const option = el("prompt-option-mode:core-055");
    press(option);
    move(300, 100);
    move(300, 450);
    release(300, 450);
    expect(onAction).not.toHaveBeenCalled();

    press(option);
    move(300, 100);
    fireEvent.keyDown(window, { key: "Escape" });
    release(300, 100);
    expect(onAction).not.toHaveBeenCalled();
    expect(option).not.toHaveAttribute("data-drag-source");
  });

  it("R658 a short press is still a click, and with drag to play off nothing lifts", () => {
    const onAction = renderDiscover();
    const option = el("prompt-option-mode:core-055");
    press(option);
    move(START.x, START.y - (DRAG_THRESHOLD_PX - 1));
    release();
    fireEvent.click(option);
    expect(onAction).toHaveBeenCalledWith(ANSWER);
    cleanup();

    writeSettings({ dragToPlay: false });
    renderDiscover();
    press(el("prompt-option-mode:core-055"));
    move(300, 100);
    expect(screen.queryByTestId("drag-option-ghost")).toBeNull();
  });
});
