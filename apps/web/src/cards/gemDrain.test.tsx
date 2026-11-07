// R3338002: Ceaseless Void's cost gem drains as its cost falls, in its owner's own hand only (gemDrain.ts,
// game/Card.tsx, CardFace.tsx, cards.css). It draws the card's own visible cost, so no other seat's view
// carries it (R202): not the board's, not an opponent's hand.

import { cleanup, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { testid } from "../game/contract.ts";
import { renderBoard } from "../test/board.tsx";
import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import { GEM_DRAIN_CARDS, gemDrainOf } from "./gemDrain.ts";

afterEach(cleanup);

const VOID_ID = "core-100";
const VOID_PRINTED = 100;
const VOID_NOW = 40;

function gemOf(root: HTMLElement): HTMLElement {
  const gem = root.querySelector<HTMLElement>(".cost-gem");
  if (gem === null) throw new Error("the face has no cost gem");
  return gem;
}

describe("R3338002: Ceaseless Void's gem drains as its cost falls", () => {
  it("R3338002 gemDrainOf is the share of the printed cost gone, 0 to 1, for the Void alone", () => {
    expect([...GEM_DRAIN_CARDS]).toEqual([VOID_ID]);
    expect(gemDrainOf(VOID_ID, VOID_PRINTED, VOID_PRINTED)).toBe(0);
    expect(gemDrainOf(VOID_ID, VOID_NOW, VOID_PRINTED)).toBeCloseTo(0.6);
    expect(gemDrainOf(VOID_ID, 0, VOID_PRINTED)).toBe(1);
    // Clamped: a cost raised past its printed one keeps the gem full; one below 0, empty.
    expect(gemDrainOf(VOID_ID, VOID_PRINTED + 20, VOID_PRINTED)).toBe(0);
    expect(gemDrainOf(VOID_ID, -5, VOID_PRINTED)).toBe(1);
    // Nothing to draw: no printed cost to drain from, a cost not known, or a card whose gem never drains.
    expect(gemDrainOf(VOID_ID, VOID_NOW, 0)).toBeNull();
    expect(gemDrainOf(VOID_ID, VOID_NOW, -1)).toBeNull();
    expect(gemDrainOf(VOID_ID, undefined, VOID_PRINTED)).toBeNull();
    expect(gemDrainOf(VOID_ID, VOID_NOW, undefined)).toBeNull();
    expect(gemDrainOf("core-004", 1, 3)).toBeNull();
  });

  it("R3338002 the Void's gem drains in its owner's hand only", () => {
    const mine = card({ defId: VOID_ID, cost: VOID_NOW });
    const other = card({ defId: "core-004", cost: 1 });
    const theirs = card({ defId: VOID_ID, cost: VOID_NOW });
    const onBoard = unit("p1", { defId: VOID_ID, cost: VOID_NOW, attack: 10, maxHealth: 10, health: 10 });
    const view = baseView({
      you: emptySide("p1", { hand: [mine, other], units: [onBoard, null, null, null, null] }),
      // An opponent's hand the view shows (a revealed hand): their Void is theirs to drain, never ours to draw.
      opponent: emptySide("p2", { hand: [theirs] }),
    });
    const board = renderBoard(view);

    const drained = gemOf(screen.getByTestId(testid.handCard(mine.instanceId)));
    expect(drained.getAttribute("data-gem-drain")).toBe("true");
    expect(Number(drained.style.getPropertyValue("--cost-drain"))).toBeCloseTo(1 - VOID_NOW / VOID_PRINTED);
    expect(drained.querySelector(".cf-cost-drain")?.getAttribute("aria-hidden")).toBe("true");
    // The number reads as before: the level draws no text.
    expect(drained.textContent).toBe(String(VOID_NOW));

    for (const whole of [gemOf(screen.getByTestId(testid.handCard(other.instanceId))), gemOf(screen.getByTestId(testid.card(onBoard.instanceId)))]) {
      expect(whole.hasAttribute("data-gem-drain")).toBe(false);
      expect(whole.style.getPropertyValue("--cost-drain")).toBe("");
      expect(whole.querySelector(".cf-cost-drain")).toBeNull();
    }
    const opponentHand = screen.getByTestId("hand-opponent");
    expect(opponentHand.querySelectorAll(".cost-gem").length).toBeGreaterThan(0);
    expect(opponentHand.querySelector(".cost-gem[data-gem-drain], .cf-cost-drain")).toBeNull();
    // One gem on the whole board drains: the one in its owner's own hand.
    expect(board.querySelectorAll(".cost-gem[data-gem-drain]")).toHaveLength(1);
  });
});
