// R660: the opponent's aim on the client — what this seat sends while it aims (by click-select and
// by drag), that it names only public handles, and the opponent's arrow drawn and cleared.
//
// jsdom has no layout and no `document.elementsFromPoint`; each test that hovers stubs it to return
// the element the pointer is "over", as the drag layer's own tests do.

import type { ActionBody, Aim, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { IDLE, type Interaction } from "../actions.ts";
import Game from "../Game.tsx";
import { parseServerFrame } from "../net.ts";
import { __resetSettingsForTests } from "../../settings/index.ts";
import { baseView, card, emptySide, faceDownBackrow, heroPower, unit } from "../../test/fixtures.ts";
import { aimFor, aimSource, aimTargets } from "./aim.ts";

/** h1 aims at e1 or the enemy hero; z1 is placed in a zone; u1 may attack e1 or the hero. */
function aimView(over: Partial<PlayerView> = {}): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: "z1" }), card({ instanceId: "h1" })],
      units: [unit("p1", { instanceId: "u1" }), null, null, null, null],
      hero: { health: 30, armor: 0, powers: [heroPower], power: heroPower },
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [null, unit("p2", { instanceId: "e1" }), null, null, null],
      backrow: [faceDownBackrow, null, null, null, null],
    }),
    ...over,
  });
}

const H1_E1: ActionBody = { type: "play", instanceId: "h1", targets: [{ pick: "instance", instanceId: "e1" }] };
const H1_HERO: ActionBody = { type: "play", instanceId: "h1", targets: [{ pick: "hero", player: "p2" }] };
const Z1_LANE3: ActionBody = { type: "play", instanceId: "z1", zone: { row: "units", lane: 3 } };
const U1_E1: ActionBody = { type: "attack", attackerId: "u1", targetId: "e1" };
const U1_HERO: ActionBody = { type: "attack", attackerId: "u1", targetId: "hero-p2" };
const LEGAL: readonly ActionBody[] = [H1_E1, H1_HERO, Z1_LANE3, U1_E1, U1_HERO, { type: "endTurn" }, { type: "concede" }];

const playing = (instanceId: string): Interaction => ({
  stage: "playing",
  instanceId,
  candidates: LEGAL.filter((body) => body.type === "play" && body.instanceId === instanceId),
  picked: {},
});

let under: Element[] = [];

beforeEach(() => {
  under = [];
  document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
});

afterEach(() => {
  cleanup();
  delete (document as Partial<Document>).elementsFromPoint;
  localStorage.clear();
  __resetSettingsForTests();
});

describe("R660 the aim this seat sends", () => {
  it("R660 a hand card's aim starts at its position in the hand and names targets by zone or hero, never an instance id", () => {
    const view = aimView();
    const interaction = playing("h1");
    const targets = aimTargets(view, LEGAL, interaction);
    expect([...targets].sort()).toEqual(["card-e1", "hero-opponent"]);
    expect(aimSource(view, interaction)).toEqual({ at: "hand", player: "p1", index: 1 });

    const aim = aimFor(view, interaction, targets, { on: "unit", instanceId: "e1", side: "opponent", lane: 2 });
    expect(aim).toEqual({
      source: { at: "hand", player: "p1", index: 1 },
      target: { at: "zone", player: "p2", row: "units", lane: 2 },
    });
    expect(JSON.stringify(aim)).not.toMatch(/h1|e1|core-/);
  });

  it("R660 a placement is never shown: a card going into a zone aims at nothing", () => {
    const view = aimView();
    const interaction = playing("z1");
    expect(aimTargets(view, LEGAL, interaction).size).toBe(0);
    expect(aimFor(view, interaction, aimTargets(view, LEGAL, interaction), null)).toBeNull();
    expect(aimFor(view, IDLE, new Set(), null)).toBeNull();
  });

  it("R660 an attack starts at the attacker's zone, a Heroic Power at its hero", () => {
    const view = aimView();
    expect(aimSource(view, { stage: "attacking", attackerId: "u1", candidates: [U1_E1, U1_HERO] })).toEqual({
      at: "zone",
      player: "p1",
      row: "units",
      lane: 1,
    });
    expect(
      aimSource(view, { stage: "activating", instanceId: heroPower.instanceId, candidates: [], picked: {} }),
    ).toEqual({ at: "hero", player: "p1" });
  });

  it("R660 click-selecting an attacker sends its aim, hovering a target adds it, and cancelling sends null", () => {
    const emit = vi.fn();
    render(<Game view={aimView()} legal={LEGAL} onAction={vi.fn()} aim={{ emit, opponent: null }} />);
    expect(emit).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("card-u1"));
    expect(emit).toHaveBeenLastCalledWith({ source: { at: "zone", player: "p1", row: "units", lane: 1 }, target: null });

    under = [screen.getByTestId("hero-opponent")];
    fireEvent.pointerMove(window, { clientX: 10, clientY: 10 });
    expect(emit).toHaveBeenLastCalledWith({
      source: { at: "zone", player: "p1", row: "units", lane: 1 },
      target: { at: "hero", player: "p2" },
    });

    // Moving over the same target again sends nothing new.
    const sent = emit.mock.calls.length;
    fireEvent.pointerMove(window, { clientX: 12, clientY: 12 });
    expect(emit).toHaveBeenCalledTimes(sent);

    fireEvent.keyDown(window, { key: "Escape" });
    expect(emit).toHaveBeenLastCalledWith(null);
  });

  it("R660 a board that unmounts mid-aim clears the arrow", () => {
    const emit = vi.fn();
    const { unmount } = render(<Game view={aimView()} legal={LEGAL} onAction={vi.fn()} aim={{ emit, opponent: null }} />);
    fireEvent.click(screen.getByTestId("hand-card-h1"));
    expect(emit).toHaveBeenLastCalledWith({ source: { at: "hand", player: "p1", index: 1 }, target: null });
    unmount();
    expect(emit).toHaveBeenLastCalledWith(null);
  });
});

describe("R660 the opponent's arrow", () => {
  const theirs: Aim = {
    source: { at: "hand", player: "p2", index: 2 },
    target: { at: "zone", player: "p1", row: "units", lane: 1 },
  };

  it("R660 draws the opponent's aim from its card back to its target, and clears it when the aim ends", () => {
    const { rerender } = render(<Game view={aimView()} legal={LEGAL} onAction={vi.fn()} aim={{ emit: vi.fn(), opponent: theirs }} />);
    expect(screen.getByTestId("opponent-aim")).toHaveAttribute("data-targeting", "true");
    expect(screen.getByTestId("opponent-aim-arrow")).toBeInTheDocument();

    // Over nothing yet: the source is marked, no arrow.
    rerender(<Game view={aimView()} legal={LEGAL} onAction={vi.fn()} aim={{ emit: vi.fn(), opponent: { ...theirs, target: null } }} />);
    expect(screen.getByTestId("opponent-aim")).toHaveAttribute("data-targeting", "false");
    expect(screen.queryByTestId("opponent-aim-arrow")).toBeNull();

    rerender(<Game view={aimView()} legal={LEGAL} onAction={vi.fn()} aim={{ emit: vi.fn(), opponent: null }} />);
    expect(screen.queryByTestId("opponent-aim")).toBeNull();
  });

  it("R660 draws nothing for a hand position the board does not show, and nothing once the game is over", () => {
    const past: Aim = { source: { at: "hand", player: "p2", index: 3 }, target: null };
    const { rerender } = render(<Game view={aimView()} legal={LEGAL} onAction={vi.fn()} aim={{ emit: vi.fn(), opponent: past }} />);
    expect(screen.queryByTestId("opponent-aim")).toBeNull();

    const over = aimView({ result: { winner: "p1", reason: "concede" } as PlayerView["result"] });
    act(() => {
      rerender(<Game view={over} legal={[]} onAction={vi.fn()} aim={{ emit: vi.fn(), opponent: theirs }} />);
    });
    expect(screen.queryByTestId("opponent-aim")).toBeNull();
  });

  it("R660 a relayed aim frame is parsed whole or not at all", () => {
    expect(parseServerFrame(JSON.stringify({ type: "aim", from: "p2", aim: theirs }))).toEqual({ type: "aim", from: "p2", aim: theirs });
    expect(parseServerFrame(JSON.stringify({ type: "aim", from: "p2", aim: null }))).toEqual({ type: "aim", from: "p2", aim: null });
    expect(parseServerFrame(JSON.stringify({ type: "aim", from: "p3", aim: null }))).toBeNull();
    expect(parseServerFrame(JSON.stringify({ type: "aim", from: "p2", aim: { source: { at: "card", instanceId: "x" }, target: null } }))).toBeNull();
  });
});
