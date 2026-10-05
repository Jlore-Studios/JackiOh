// Reveal (R665): showing a backrow Trap or Field Trap to both players while it stays armed —
// Classic #88 Siphon Squad's "Start of Turn: Reveal" and Classic #65 Ace in the Hole's Radiant
// "Revealed regardless of the coin flip". Fixtures: `fixtures/field.ts`.

import { describe, expect, it } from "vitest";
import { reveal } from "../src/effects/reveal";
import { makeContext } from "../src/resolve";
import { isSpent } from "../src/traps";
import { viewFor } from "../src/viewFor";
import { plain } from "./fixtures/combat";
import { doom, listener, playing } from "./fixtures/field";
import { put, sinkFor, slot } from "./fixtures/harness";

describe("R665 Reveal", () => {
  it("R665 a revealed Trap reads face-up to the opponent but is not spent, so it still fires", () => {
    const state = playing("reveal-trap");
    const trap = put(state, doom.id, slot("p1", "backrow", 1));
    const sink = sinkFor(state);
    reveal().apply(makeContext(sink, trap, {}));

    expect(trap.revealed).toBe(true);
    // The opponent reads the face now, not a back.
    expect(viewFor(state, "p2").opponent.backrow[0]).toMatchObject({ faceDown: false, defId: doom.id });
    // …but it has not fired: a Trap that is merely revealed still answers.
    expect(isSpent(state, trap)).toBe(false);
  });

  it("R665 a revealed Field Trap stays live: the opponent reads it and it keeps firing", () => {
    const state = playing("reveal-field-trap");
    const trap = put(state, listener.id, slot("p1", "backrow", 2));
    const sink = sinkFor(state);
    reveal().apply(makeContext(sink, trap, {}));

    expect(trap.revealed).toBe(true);
    expect(viewFor(state, "p2").opponent.backrow[1]).toMatchObject({ faceDown: false, defId: listener.id });
    expect(isSpent(state, trap)).toBe(false);
  });

  it("R665 reveal touches only a backrow card: a unit is unchanged", () => {
    const state = playing("reveal-unit");
    const unit = put(state, plain.id, slot("p1", "units", 1));
    const sink = sinkFor(state);
    reveal().apply(makeContext(sink, unit, {}));

    expect(unit.revealed).toBe(undefined);
  });
});
