// R3821000: your own unit says why it cannot make its own switch of position now
// (`UnitView.switchReason`). A tap on it that `legal` offers no attack and that the view gives no
// `attackReason` shows that reason and sends nothing, so a spent unit says it has already acted; its
// greyed switch names it, and so do its inspect notes. The words are the engine's and the client
// works none of it out (CLAUDE.md rule 7).

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PlayerView, UnitView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { StateNotes, hasStateNotes } from "../cards/inspect/StateNotes.tsx";
import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import { IDLE, onClickTarget } from "./actions.ts";
import Board from "./Board.tsx";
import { lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import { liveFace } from "./faces.ts";
import Game from "./Game.tsx";

afterEach(cleanup);

const lookup = lookupFromDefs(CATALOG);
const ACTED = "that unit has already acted this turn";
const NO_DEFENSE = "that unit cannot be in Defense Position";
const ASLEEP = "that unit is summoning sick";
const FROZEN = "that Unit is Frozen";

/**
 * u1 has attacked (spent); u2 may attack but cannot be in Defense Position; u3 is summoning sick and
 * cannot be in Defense Position; u4 is Frozen; u5 is free to switch. The opponent has e1.
 */
function reasonView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", { instanceId: "u1", canAct: false, switchReason: ACTED }),
        unit("p1", { instanceId: "u2", switchReason: NO_DEFENSE }),
        unit("p1", { instanceId: "u3", sick: true, attackReason: ASLEEP, switchReason: NO_DEFENSE }),
        unit("p1", { instanceId: "u4", canAct: false, frozen: true, attackReason: FROZEN, switchReason: FROZEN }),
        unit("p1", { instanceId: "u5" }),
      ],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1" }), null, null, null, null],
    }),
  });
}

const LEGAL: readonly ActionBody[] = [
  { type: "attack", attackerId: "u2", targetId: "e1" },
  { type: "attack", attackerId: "u2", targetId: "hero-p2" },
  { type: "switchPosition", instanceId: "u5" },
  { type: "endTurn" },
  { type: "concede" },
];

const el = (id: string): HTMLElement => screen.getByTestId(id);

function unitOf(view: PlayerView, id: string): UnitView {
  const found = view.you.units.find((candidate) => candidate?.instanceId === id);
  if (found === null || found === undefined) throw new Error(`the fixture has no unit ${id}`);
  return found;
}

describe("R3821000 your own unit says why it cannot switch, and a spent unit says so when tapped", () => {
  it("R3821000 clicking your own unit with no attack listed and no attackReason returns its switchReason and sends nothing", () => {
    const view = reasonView();
    const click = (instanceId: string, lane: number) => onClickTarget(view, LEGAL, IDLE, { on: "unit", instanceId, side: "you", lane });

    expect(click("u1", 1)).toEqual({ interaction: IDLE, notice: ACTED });
    // R849000's attack reason comes first where the view gives both.
    expect(click("u3", 3)).toEqual({ interaction: IDLE, notice: ASLEEP });
    expect(click("u4", 4)).toEqual({ interaction: IDLE, notice: FROZEN });
    // A unit with no reason and no attack stays silent.
    expect(click("u5", 5)).toEqual({ interaction: IDLE });

    // `legal` decides: a unit with an attack on offer is picked up, whatever its switch reason.
    const attacker = click("u2", 2);
    expect(attacker.interaction.stage).toBe("attacking");
    expect(attacker.notice).toBeUndefined();
  });

  it("R3821000 the board shows a spent unit's reason as a notice after the click and sends nothing", () => {
    const onAction = vi.fn<(body: ActionBody) => void>();
    render(<Game view={reasonView()} legal={LEGAL} onAction={onAction} />);
    expect(screen.queryByTestId("attack-reason")).toBeNull();

    fireEvent.click(el(testid.card("u1")));
    expect(el("attack-reason")).toHaveTextContent(ACTED);

    fireEvent.click(el(testid.card("u4")));
    expect(el("attack-reason")).toHaveTextContent(FROZEN);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R3821000 a spent unit's click still reaches its zone while a play onto that zone is built", () => {
    const onAction = vi.fn<(body: ActionBody) => void>();
    const view = baseView({
      you: emptySide("p1", {
        hand: [card({ instanceId: "h1" })],
        units: [unit("p1", { instanceId: "u1", canAct: false, switchReason: ACTED }), null, null, null, null],
      }),
    });
    const stack: ActionBody = { type: "play", instanceId: "h1", zone: { row: "units", lane: 1 } };
    const plays: ActionBody[] = [stack, { type: "play", instanceId: "h1", zone: { row: "units", lane: 2 } }];
    render(<Game view={view} legal={plays} onAction={onAction} />);

    fireEvent.click(el(testid.handCard("h1")));
    expect(el(testid.zone("you", "units", 1))).toHaveAttribute("data-legal", "true");
    expect(el(testid.card("u1"))).toHaveAttribute("data-legal", "false");

    fireEvent.click(el(testid.card("u1")));
    expect(onAction.mock.calls).toEqual([[stack]]);
    expect(screen.queryByTestId("attack-reason")).toBeNull();
  });

  it("R3821000 a greyed switch names the reason, and one without a reason says Switch position alone", () => {
    const { rerender } = render(<Board view={reasonView()} />);

    const spent = el(testid.switchPosition("u1"));
    expect(spent).toBeDisabled();
    expect(spent).toHaveAttribute("aria-label", `Switch position: ${ACTED}`);
    expect(spent).toHaveAttribute("title", `Switch position: ${ACTED}`);
    for (const id of ["u5", "e1"]) {
      expect(el(testid.switchPosition(id)), id).toHaveAttribute("aria-label", "Switch position");
      expect(el(testid.switchPosition(id)), id).toHaveAttribute("title", "Switch position");
    }

    // A switch `legal` lists is named alone, whatever the view says.
    const legal = new Set([testid.switchPosition("u1")]);
    rerender(<Board view={reasonView()} highlight={{ legal, selected: new Set() }} />);
    expect(el(testid.switchPosition("u1"))).toHaveAttribute("aria-label", "Switch position");
  });

  it("R3821000 the inspect notes spell out the switch reason once, and a unit without one has no such note", () => {
    const view = reasonView();
    const faceOf = (unitView: UnitView) =>
      liveFace(lookup(unitView.defId, unitView.radiant) ?? { name: unitView.defId, type: "Unit", text: "", tags: [] }, unitView);

    const spent = faceOf(unitOf(view, "u1"));
    expect(spent.switchReason).toBe(ACTED);
    expect(hasStateNotes(spent)).toBe(true);
    const { container, rerender } = render(<StateNotes face={spent} />);
    expect(container.querySelector('[data-state="switchReason"]')).toHaveTextContent(ACTED);

    // A Frozen unit's reasons are one: it is said once, as the attack reason.
    const frozen = faceOf(unitOf(view, "u4"));
    expect(frozen.switchReason).toBeUndefined();
    rerender(<StateNotes face={frozen} />);
    expect(container.querySelector('[data-state="switchReason"]')).toBeNull();
    expect(container.querySelector('[data-state="attackReason"]')).toHaveTextContent(FROZEN);

    expect(faceOf(unitOf(view, "u5")).switchReason).toBeUndefined();
  });
});
