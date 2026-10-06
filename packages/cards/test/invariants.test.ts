// I2's Windfury shadow (R636): the oracle reads Windfury from the event stream, so a grant that
// lands mid-action counts before the declarations that follow it in the same action's events.
// Found by fuzz seed 359: R44's AI turn summoned a Conjure Rush Token, granted it Windfury among
// six keywords, and fought with it twice, all inside one outer action — no between-action state
// ever held the token, so the old oracle read attack number 2 as an extra attack.
//
// I6 (hidden information, issue #348) is proved by feeding the oracle a view or an event that names a
// card the seat may not read, and by the exceptions §10.8 and its rulings make: a face-down trap's
// bare id as a target (R177) and a `stolen` event naming a card its viewer could read where it was
// taken (R466).

import { HIDDEN_ID, legalActions, viewFor } from "@jackioh/engine";
import type { ActionBody, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";
import { createInvariantMonitor, hiddenInformationViolations } from "./_invariants";

const VANILLA = "core-008";
const BIGOT = "core-002";
const HIT_JOB = "core-016";
const MY_PAWN = "core-096";

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

describe("I6: hidden information in what each seat is sent", () => {
  /** p1 sees a unit and p2 holds a hand card, a library card and a face-down trap. */
  function board() {
    const g = scenario({
      p1: { field: [{ def: VANILLA, lane: 1 }] },
      p2: { hand: [BIGOT], library: [HIT_JOB], backrow: [{ def: MY_PAWN, lane: 1 }] },
    });
    const unit = g.unit("p1", 1);
    if (unit === null) throw new Error("setup: p1 should hold a unit in lane 1");
    const secret = g.hand("p2")[0];
    if (secret === undefined) throw new Error("setup: p2 should hold a card in hand");
    const trap = g.backrow("p2", 1);
    if (trap === null) throw new Error("setup: p2 should hold a face-down trap in lane 1");
    return { g, unit, secret, trap };
  }

  function violations(g: ReturnType<typeof scenario>, seat: PlayerId, events: readonly GameEvent[] = []): string[] {
    const view = viewFor(g.state, seat);
    return hiddenInformationViolations(g.state, seat, { ...view, events: [...view.events, ...events] }, legalActions(g.state, seat));
  }

  it("I6 is clean on a board with a hidden hand, library and face-down trap, from both seats (§10.8)", () => {
    const { g } = board();
    expect(violations(g, "p1")).toEqual([]);
    expect(violations(g, "p2")).toEqual([]);
    expect(createInvariantMonitor(g.state).hidden(g.state)).toEqual([]);
  });

  it("I6 fires on an event naming a hidden hand card by id and definition (R97)", () => {
    const { g, secret } = board();
    const found = violations(g, "p1", [{ type: "drawn", player: "p2", instanceId: secret.id, defId: secret.defId }]);
    expect(found.every((message) => message.startsWith("I6"))).toBe(true);
    expect(found.some((message) => message.includes(`"${secret.id}"`))).toBe(true);
    expect(found.some((message) => message.includes(`"${secret.defId}"`))).toBe(true);
  });

  it("I6 fires on a hidden card's definition left beside the sentinel (R97)", () => {
    const { g, secret } = board();
    const found = violations(g, "p1", [{ type: "drawn", player: "p2", instanceId: HIDDEN_ID, defId: secret.defId }]);
    expect(found.some((message) => message.includes(`"${secret.defId}"`))).toBe(true);
    expect(found.some((message) => message.includes(`"${secret.id}"`))).toBe(false);
  });

  it("I6 lets a face-down trap through legalActions as a bare target and fires on a hidden hand card (R177)", () => {
    const { g, unit, secret, trap } = board();
    const view = viewFor(g.state, "p1");
    const offer = (targetId: string): ActionBody[] => [{ type: "attack", attackerId: unit.id, targetId }];
    expect(hiddenInformationViolations(g.state, "p1", view, offer(trap.id))).toEqual([]);
    const found = hiddenInformationViolations(g.state, "p1", view, offer(secret.id));
    expect(found.some((message) => message.includes("in legalActions"))).toBe(true);
  });

  it("I6 lets a stolen event name a card its viewer could read where it was taken (R466)", () => {
    const { g, secret } = board();
    g.state.applied = [
      {
        nonce: "i6-stolen",
        events: [
          { type: "stolen", instanceId: secret.id, defId: secret.defId, from: "p1", to: "p2", zone: "hand", readableFrom: ["p1"] },
        ],
      },
    ];
    expect(JSON.stringify(viewFor(g.state, "p1").events)).toContain(secret.id);
    expect(violations(g, "p1")).toEqual([]);
  });
});
