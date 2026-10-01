// R89: the `destroyed` event says what the unit was as it died, its Radiant face included — the one
// fact a unit token that has ceased to exist can no longer tell (C+ #12.8 Frostspatula remembers each
// Unit it killed by definition and face, R409). Absent on a base face, so nothing else moves.

import { describe, expect, it } from "vitest";
import type { GameEvent } from "@jackioh/shared";
import { dealDamage } from "../src/damage";
import { stateCheck } from "../src/stateCheck";
import { plain } from "./fixtures/combat";
import { playing } from "./fixtures/field";
import { put, sinkFor, slot } from "./fixtures/harness";

function killed(radiant: boolean): Extract<GameEvent, { type: "destroyed" }> | undefined {
  const state = playing(`destroyed-face-${radiant}`);
  const unit = put(state, plain.id, slot("p2", "units", 1), { radiant });
  const sink = sinkFor(state);
  dealDamage(sink, { source: null, target: { kind: "unit", instance: unit }, amount: 99 });
  stateCheck(sink);
  return sink.events.find((event): event is Extract<GameEvent, { type: "destroyed" }> => event.type === "destroyed");
}

describe("R89 a destroyed unit's face", () => {
  it("R89 a Radiant unit's destroyed event says so", () => {
    expect(killed(true)).toMatchObject({ type: "destroyed", radiant: true });
  });

  it("R89 a base-face unit's event carries no flag", () => {
    const event = killed(false);
    expect(event?.type).toBe("destroyed");
    expect(event !== undefined && "radiant" in event).toBe(false);
  });
});
