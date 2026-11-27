// #5136: a pile builds its list's faces only when the list is drawn. A long game's graveyards and exile
// piles hold dozens of cards, every face is a call into the face model, and the board renders many
// times a move (each animation step): building every pile's list on each render made long games slower
// with every card that died.

import { act, cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { closeInspect } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/index.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { renderBoard } from "../test/board.tsx";
import { card, fullBoardView } from "../test/fixtures.ts";
import { listedFace } from "./faces.ts";

vi.mock("./faces.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./faces.ts")>();
  return { ...actual, listedFace: vi.fn(actual.listedFace) };
});

const spy = vi.mocked(listedFace);

/** How many faces were built for the cards of `ids`. */
function facesBuiltFor(ids: ReadonlySet<string>): number {
  return spy.mock.calls.filter(([, , shown]) => ids.has(shown.instanceId)).length;
}

const PILE = 30;

beforeEach(() => {
  vi.useFakeTimers();
  __resetSettingsForTests();
  spy.mockClear();
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  __resetSettingsForTests();
});

describe("#5136 a pile's list is built only when it is drawn", () => {
  it("a board with long graveyards and an exile pile builds no face for their cards until one is opened", () => {
    const view = fullBoardView();
    const graveyard = Array.from({ length: PILE }, () => card({ defId: "core-003" }));
    const theirs = Array.from({ length: PILE }, () => card({ defId: "core-005" }));
    const exile = Array.from({ length: PILE }, () => card({ defId: "core-009" }));
    const ids = new Set([...graveyard, ...theirs, ...exile].map((shown) => shown.instanceId));
    renderBoard({ ...view, you: { ...view.you, graveyard, exile }, opponent: { ...view.opponent, graveyard: theirs } });

    // Still browsable, and named with the whole count, without a face built.
    const pile = screen.getByTestId("graveyard-you");
    expect(pile).toHaveAttribute("data-browsable", "true");
    expect(pile).toHaveAttribute("aria-label", `Your graveyard: ${String(PILE)} cards. Show them`);
    expect(screen.getByTestId("exile-you")).toHaveAttribute("data-browsable", "true");
    expect(screen.getByTestId("graveyard-opponent")).toHaveAttribute("data-browsable", "true");
    expect(facesBuiltFor(ids)).toBe(0);

    // Opening one pile builds its own list, and no other pile's.
    fireEvent.pointerEnter(pile, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    const yours = new Set(graveyard.map((shown) => shown.instanceId));
    expect(facesBuiltFor(yours)).toBeGreaterThanOrEqual(PILE);
    expect(facesBuiltFor(new Set([...theirs, ...exile].map((shown) => shown.instanceId)))).toBe(0);
  });

  it("a pile whose cards have no face (the sentinel) is no button, as it was when its list was built to tell", () => {
    const view = fullBoardView();
    renderBoard({ ...view, you: { ...view.you, exile: [card({ defId: "hidden" }), card({ defId: "" })] } });
    expect(screen.getByTestId("exile-you")).not.toHaveAttribute("data-browsable");
  });
});
