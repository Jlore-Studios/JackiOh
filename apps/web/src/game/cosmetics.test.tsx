// R1295000: on the board, only the viewer's own cards wear the device's card cosmetics — the hand,
// the units and face-up backrow cards it owns, a unit the opponent has taken included — and only on
// a board that wears them (`wearCosmetics`). The opponent's cards, and every card on a board that
// wears none, draw the defaults whatever the device picked.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { PlayerView } from "@jackioh/shared";

import { cosmeticsOf, resetCosmeticsForTests, writeCosmetics, type CosmeticKind } from "../cards/cosmetics.ts";
import { baseView, card, emptySide, faceUpBackrow, resetIds, unit } from "../test/fixtures.ts";
import Board from "./Board.tsx";

/** The registry's first entry of `kind` that is not its default. */
function item(kind: CosmeticKind): string {
  const found = cosmeticsOf(kind).find((cosmetic) => !cosmetic.default);
  if (found === undefined) throw new Error(`no ${kind} item`);
  return found.id;
}

const BORDER = item("border");

function view(): PlayerView {
  resetIds();
  return baseView({
    you: emptySide("p1", {
      hand: [card({ defId: "core-004" })],
      // Your own unit, and the opponent's unit you have taken: owned by p2, controlled by you.
      units: [unit("p1"), unit("p2", { controller: "p1" }), null, null, null],
      backrow: [faceUpBackrow("p1"), null, null, null, null],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      // Their own unit, and yours they have taken: owned by you still.
      units: [unit("p2"), unit("p1", { controller: "p2" }), null, null, null],
      backrow: [faceUpBackrow("p2"), null, null, null, null],
    }),
  });
}

/** The face inside each card the board draws with this owner. */
function facesOwnedBy(owner: string): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>(`.card[data-owner="${owner}"] > .cf`)];
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  writeCosmetics({ foil: item("foil"), finish: item("finish"), border: BORDER });
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
  localStorage.clear();
});

describe("R1295000 the board's cards and the device's cosmetics", () => {
  it("R1295000 the opponent's cards draw the defaults whatever the device picked; the viewer's own wear them", () => {
    render(<Board view={view()} wearCosmetics />);

    const hand = [...screen.getByTestId("hand-you").querySelectorAll<HTMLElement>(".cf")];
    expect(hand).toHaveLength(1);
    // Two units, one on each side of the board, and a face-up backrow card are the viewer's own.
    const yours = facesOwnedBy("p1");
    expect(yours).toHaveLength(3);
    for (const face of [...hand, ...yours]) expect(face.getAttribute("data-frame")).toBe(BORDER);

    const theirs = facesOwnedBy("p2");
    expect(theirs).toHaveLength(3);
    for (const face of theirs) {
      expect(face.getAttribute("data-frame")).toBeNull();
      expect(face.getAttribute("data-finish")).toBeNull();
      expect(face.getAttribute("data-cosmetic-foil")).toBeNull();
    }
  });

  it("R1295000 a board that wears none draws every card with the defaults", () => {
    render(<Board view={view()} />);
    expect(document.querySelectorAll(".cf").length).toBeGreaterThan(0);
    expect(document.querySelectorAll(".cf[data-frame], .cf[data-finish], .cf[data-cosmetic-foil]")).toHaveLength(0);
  });
});
