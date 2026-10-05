// I2's Windfury shadow (R636): the oracle reads Windfury from the event stream, so a grant that
// lands mid-action counts before the declarations that follow it in the same action's events.
// Found by fuzz seed 359: R44's AI turn summoned a Conjure Rush Token, granted it Windfury among
// six keywords, and fought with it twice, all inside one outer action — no between-action state
// ever held the token, so the old oracle read attack number 2 as an extra attack.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";
import { createInvariantMonitor } from "./_invariants";

const VANILLA = "core-008";

describe("I2: a Windfury granted inside one action's events", () => {
  it("two declarations after a mid-action Windfury grant are one legal Windfury turn (R636, R44)", () => {
    const g = scenario({
      p1: { field: [{ def: VANILLA, lane: 1 }] },
      p2: { field: [{ def: VANILLA, lane: 1 }] },
    });
    const unit = g.unit("p1", 1);
    if (unit === null) throw new Error("setup: p1 should hold a unit in lane 1");
    const prey = g.unit("p2", 1);
    if (prey === null) throw new Error("setup: p2 should hold a unit in lane 1");
    // The end state holds Windfury, as if the grant below resolved mid-action.
    g.card(unit).grantedKeywords = [{ kind: "Windfury" }];

    const monitor = createInvariantMonitor(g.state);
    const events: GameEvent[] = [
      { type: "keywordGranted", instanceId: unit.id, keyword: { kind: "Windfury" } },
      { type: "attackDeclared", attackerId: unit.id, targetId: prey.id, forced: false },
      { type: "attackDeclared", attackerId: unit.id, targetId: prey.id, forced: false },
    ];
    expect(monitor.after(events, g.state)).toEqual([]);
  });

  it("two declarations with no Windfury anywhere are still an extra attack (R636)", () => {
    const g = scenario({
      p1: { field: [{ def: VANILLA, lane: 1 }] },
      p2: { field: [{ def: VANILLA, lane: 1 }] },
    });
    const unit = g.unit("p1", 1);
    if (unit === null) throw new Error("setup: p1 should hold a unit in lane 1");
    const prey = g.unit("p2", 1);
    if (prey === null) throw new Error("setup: p2 should hold a unit in lane 1");

    const monitor = createInvariantMonitor(g.state);
    const events: GameEvent[] = [
      { type: "attackDeclared", attackerId: unit.id, targetId: prey.id, forced: false },
      { type: "attackDeclared", attackerId: unit.id, targetId: prey.id, forced: false },
    ];
    const found = monitor.after(events, g.state);
    expect(found).toHaveLength(1);
    expect(found[0]).toMatch(/I2 extra attack/);
  });
});
