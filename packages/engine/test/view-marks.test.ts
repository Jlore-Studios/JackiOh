// Two marks v0.1.1 adds to a backrow card's view (SPEC §10.8), each read by the client and decided
// by the engine alone (CLAUDE.md rule 7):
//
//   R371  `unrevealed: true` on the controller's own view of a Trap or Field Trap that is still
//         face-down, so the board can say "your opponent can't see this card" without working out
//         R33 itself. It is the same answer `backrowIsPublic` gives the other seat, so the mark and
//         the back cannot disagree: present exactly when the other player sees a back.
//   R372  `counters.gradeLetter`, the letter #93 Combo-Index's grade counter stands for (E..S), and
//         a preview value's `display`, the word a value prints as, carried through `previewOf`
//         untouched. The Core card's own values are proved in packages/cards
//         (test/093-combo-index.test.ts).
//
// Test-only definitions, registered over the fixture catalog as viewFor.test.ts and
// preview.test.ts register theirs, and put back afterwards.

import type { BackrowView, CardDef, PreviewValue } from "@jackioh/shared";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { steal } from "../src/effects";
import { isFaceDown } from "../src/preview";
import { makeContext } from "../src/resolve";
import type { CardScripts } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import type { GameState } from "../src/state";
import { GRADES } from "../src/subsystems/comboIndex";
import { viewFor } from "../src/viewFor";
import { newGame, put, sinkFor, slot } from "./fixtures/harness";

let nextIndex = 3700;
function def(name: string, type: CardDef["type"]): CardDef {
  nextIndex += 1;
  return {
    id: `vm-${name}`,
    index: String(nextIndex),
    name,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 2,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
  };
}

const trap = def("trap", "Trap");
const fieldTrap = def("field-trap", "Field Trap");
const fieldSpell = def("field-spell", "Field Spell");
/** A Field Spell whose preview names its value by a word, as #93's grade does. */
const worded = def("worded", "Field Spell");

const WORDED: PreviewValue[] = [
  { label: "Grade", value: 4, display: "B" },
  { label: "N", value: 4 },
  // An empty `display` is no word at all: the number prints.
  { label: "M", value: 2, display: "" },
];

const SCRIPTS: Record<string, CardScripts> = {
  [worded.id]: { base: { preview: () => WORDED }, radiant: { preview: () => WORDED } },
};

let savedCatalog: ReturnType<typeof registeredCatalog> = {};
let savedScripts: ReturnType<typeof registeredScripts> = {};

beforeAll(() => {
  savedCatalog = registeredCatalog();
  savedScripts = registeredScripts();
});

afterAll(() => {
  registerCatalog(savedCatalog);
  registerScripts(savedScripts);
});

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({
    ...registeredCatalog(),
    ...Object.fromEntries([trap, fieldTrap, fieldSpell, worded].map((d) => [d.id, d])),
  });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  return state;
}

/** The public half of `BackrowView`, failing on a back or an empty zone. */
function publicCard(entry: BackrowView | undefined): Extract<BackrowView, { faceDown: false }> {
  if (entry === null || entry === undefined || entry.faceDown) throw new Error("expected a card the viewer reads");
  return entry;
}

describe("R371 the controller's view marks a face-down trap as unrevealed", () => {
  it("R371 a face-down Trap and Field Trap read in full by their controller carry unrevealed: true", () => {
    const state = game("r371-own");
    put(state, trap.id, slot("p1", "backrow", 1));
    put(state, fieldTrap.id, slot("p1", "backrow", 2));

    const mine = viewFor(state, "p1").you.backrow;
    expect(publicCard(mine[0])).toMatchObject({ faceDown: false, defId: trap.id, unrevealed: true });
    expect(publicCard(mine[1])).toMatchObject({ faceDown: false, defId: fieldTrap.id, unrevealed: true });

    // Exactly where the other seat sees a back: the mark and R33's marker are one answer.
    const theirs = viewFor(state, "p2").opponent.backrow;
    expect(theirs[0]?.faceDown).toBe(true);
    expect(theirs[1]?.faceDown).toBe(true);
    expect(JSON.stringify(theirs)).not.toContain("unrevealed");
  });

  it("R371 a Field Spell and a Field Trap that has fired are public, so neither carries the mark", () => {
    const state = game("r371-public");
    put(state, fieldSpell.id, slot("p1", "backrow", 1));
    const fired = put(state, fieldTrap.id, slot("p1", "backrow", 2));
    fired.faceUp = true;

    for (const viewer of ["p1", "p2"] as const) {
      const view = viewFor(state, viewer);
      const row = viewer === "p1" ? view.you.backrow : view.opponent.backrow;
      for (const entry of [row[0], row[1]]) expect("unrevealed" in publicCard(entry)).toBe(false);
    }
    expect(isFaceDown(state, fired)).toBe(false);
  });

  it("R371 the mark follows control (R33): a stolen face-down trap is unrevealed for its thief", () => {
    const state = game("r371-steal");
    const hidden = put(state, trap.id, slot("p2", "backrow", 1));
    expect(publicCard(viewFor(state, "p2").you.backrow[0]).unrevealed).toBe(true);

    const sink = sinkFor(state);
    steal({ instanceId: hidden.id }).apply(makeContext(sink, null, { controller: "p1" }));
    state.rngCursor = sink.rng.cursor;

    expect(hidden.controller).toBe("p1");
    expect(publicCard(viewFor(state, "p1").you.backrow[0])).toMatchObject({ defId: trap.id, unrevealed: true });
    expect(viewFor(state, "p2").opponent.backrow[0]?.faceDown).toBe(true);
  });
});

describe("R372 a grade's letter and a worded value travel in the view", () => {
  it("R372 counters.gradeLetter names each grade 1..6 as E, D, C, B, A, S, on both seats", () => {
    const state = game("r372-letters");
    const card = put(state, fieldSpell.id, slot("p1", "backrow", 1));
    GRADES.forEach((letter, at) => {
      card.counters.grade = at + 1;
      expect(publicCard(viewFor(state, "p1").you.backrow[0]).counters).toEqual({ grade: at + 1, gradeLetter: letter });
      expect(publicCard(viewFor(state, "p2").opponent.backrow[0]).counters).toEqual({ grade: at + 1, gradeLetter: letter });
    });
    expect(GRADES).toEqual(["E", "D", "C", "B", "A", "S"]);
  });

  it("R372 a card with no grade counter carries neither number nor letter", () => {
    const state = game("r372-none");
    put(state, fieldSpell.id, slot("p1", "backrow", 1));
    expect(publicCard(viewFor(state, "p1").you.backrow[0]).counters).toEqual({});
  });

  it("R372 previewOf carries a value's display word, and drops an empty one", () => {
    const state = game("r372-display");
    put(state, worded.id, slot("p1", "backrow", 1));
    for (const viewer of ["p1", "p2"] as const) {
      const view = viewFor(state, viewer);
      const entry = publicCard((viewer === "p1" ? view.you : view.opponent).backrow[0]);
      expect(entry.preview).toEqual([
        { label: "Grade", value: 4, display: "B" },
        { label: "N", value: 4 },
        { label: "M", value: 2 },
      ]);
    }
  });
});
