// #32 Prem Panther — SPEC §8.2 row 32, BUILD M4-T4 must-pass row 32, patch v0.2.0 (R426):
// "Rush / After this attacks and survives, draw 2 for each Unit that attack destroyed"; radiant
// "Rush, Cleave / the same".
//
// R426 rewrites the old reading (R42's "whenever this destroys a unit", on either side of a combat):
// it draws only after an attack it made — declared or forced (R53) — and only when it survives that
// combat, 2 for each Unit that attack destroyed (the Unit it attacked, plus the Cleave kills of the
// Radiant face). It never draws for a Unit it kills defending, and never when it dies in the combat.
// R42 still says who killed a Unit, per Unit.
//
// The sparring partners: #15 Me and Mr Token is a 1/1 with no keywords (its Cry does not fire from a
// `field` setup), #13 Jlockeed Shredder-10 is an 8/10 with no keywords, so it kills a 5/4 Panther on
// the swing back and survives, #11 Tempo Timmy is a 3/3 Rush, First Strike that attacks a Panther
// and dies to its strike back, and #9 Moths to the Flame (1/14) makes every enemy Unit attack it at
// its controller's start of turn.

import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";
import type { CardInstance } from "@jackioh/engine";

function must(card: CardInstance | null, what: string): CardInstance {
  if (card === null) throw new Error(`the scenario has no ${what}`);
  return card;
}

describe("#32 Prem Panther — base", () => {
  it("§6.1 Rush: a Panther played this turn may attack a unit but not the hero", () => {
    const s = scenario({
      seed: "panther-rush",
      p1: { hand: ["32"], field: ["15"], library: ["15", "15", "15"] },
      p2: { field: ["15", "15"] },
    });
    s.play("32");

    expect(() => s.attack("32", "hero")).toThrow(/Rush cannot hit the hero/);
    s.attack("32", must(s.unit("p2", 1), "p2 lane 1"));
    s.expectInZone(must(s.unit("p1", 2), "the Panther"), "field");
  });

  it("R426 draws 2 after it attacks and survives, for the Unit that attack destroyed", () => {
    const s = scenario({
      seed: "panther-kill",
      p1: { field: ["32", "15"], library: ["15", "15", "15"] },
      p2: { field: ["15"] },
    });
    const prey = must(s.unit("p2", 1), "p2 lane 1");
    const panther = must(s.unit("p1", 1), "the Panther");
    expect(s.hand("p1")).toHaveLength(0);

    s.attack(panther, prey);

    s.expectInZone(prey, "graveyard");
    s.expectInZone(panther, "field");
    expect(s.hand("p1")).toHaveLength(2);
    expect(s.pile("p1", "library")).toHaveLength(1);
    s.expectEvents("attackDeclared", "damage", "destroyed", "drawn", "drawn");
    // The death names the Panther as a killer that was attacking (R42, R426).
    const death = s.events.find((event) => event.type === "destroyed" && event.instanceId === prey.id);
    expect(death).toMatchObject({ killerId: panther.id, killerAttacking: true });
  });

  it("R426 draws nothing when it dies in that combat, though it destroyed the Unit it attacked", () => {
    const s = scenario({
      seed: "panther-trade",
      p1: { field: ["32", "15"], library: ["15", "15", "15"] },
      p2: { field: ["32"] },
    });
    const panther = must(s.unit("p1", 1), "p1's Panther");
    const other = must(s.unit("p2", 1), "p2's Panther");

    // 5 into a 5/4 and 5 back into a 5/4: both die.
    s.attack(panther, other);

    s.expectInZone(panther, "graveyard");
    s.expectInZone(other, "graveyard");
    expect(s.hand("p1")).toHaveLength(0);
    expect(s.pile("p1", "library")).toHaveLength(3);
  });

  it("R426 draws nothing when it dies without killing", () => {
    const s = scenario({
      seed: "panther-trade",
      p1: { field: ["32"], library: ["15", "15", "15"] },
      p2: { field: ["13"] },
    });
    const wall = must(s.unit("p2", 1), "p2 lane 1");
    const panther = must(s.unit("p1", 1), "p1 lane 1");

    s.attack(panther, wall);

    // 5 into an 8/10 is not lethal; 8 back into a 5/4 is.
    s.expectInZone(panther, "graveyard");
    s.expectInZone(wall, "field");
    expect(s.hand("p1")).toHaveLength(0);
    expect(s.pile("p1", "library")).toHaveLength(3);
  });

  it("R426 never draws while defending: a Unit it kills striking back is the attacker's attack", () => {
    const s = scenario({
      seed: "panther-defends",
      active: "p2",
      turn: 10,
      p1: { field: ["32"], library: ["15", "15", "15"] },
      p2: { field: ["11"], hand: ["15"], library: ["15", "15"] },
    });
    const panther = must(s.unit("p1", 1), "p1's Panther");
    const timmy = must(s.unit("p2", 1), "p2's Tempo Timmy");

    // First Strike 3 leaves the 5/4 Panther at 1; its 5 back kills the 3/3.
    s.attack(timmy, panther);

    s.expectInZone(timmy, "graveyard");
    s.expectInZone(panther, "field");
    const death = s.events.find((event) => event.type === "destroyed" && event.instanceId === timmy.id);
    // R42: the Panther killed it — but not attacking (R426).
    expect(death).toMatchObject({ killerId: panther.id });
    expect(death).not.toHaveProperty("killerAttacking");
    expect(s.hand("p1")).toHaveLength(0);
    expect(s.pile("p1", "library")).toHaveLength(3);
  });

  it("R426, R53 a forced attack is an attack: forced into #9 Moths to the Flame, it kills it, survives and draws 2", () => {
    // p2's start of turn: Moths makes every enemy Unit attack it. Its 14 health, 10 of it already
    // gone, falls to the Panther's 5; its 1 back leaves the Panther standing.
    const s = scenario({
      seed: "panther-forced",
      p1: { field: ["32"], hand: ["15"], library: ["15", "15", "15"] },
      p2: { field: [{ def: "9", damage: 10 }], hand: ["15"], library: ["15", "15", "15"] },
    });
    const panther = must(s.unit("p1", 1), "p1's Panther");
    const moths = must(s.unit("p2", 1), "p2's Moths");

    s.endTurn(); // p2's turn begins: the forced attack, on p2's turn.

    expect(s.state.active).toBe("p2");
    s.expectInZone(moths, "graveyard");
    s.expectInZone(panther, "field");
    expect(s.events.some((event) => event.type === "attackDeclared" && event.forced && event.attackerId === panther.id)).toBe(true);
    // p1 drew 2 on p2's turn, for the Unit its forced attack destroyed.
    expect(s.hand("p1")).toHaveLength(3);
    expect(s.pile("p1", "library")).toHaveLength(1);
  });

  it("R426 attacking the hero destroys no Unit and draws nothing", () => {
    const s = scenario({
      seed: "panther-face",
      p1: { field: ["32"], hand: ["5"], library: ["15", "15", "15"] },
      p2: { hand: ["15"], library: ["15"] },
    });
    s.attack("32", "hero");
    s.expectHealth("p2", 25);
    expect(s.lastEvents.some((event) => event.type === "drawn")).toBe(false);
    expect(s.hand("p1")).toHaveLength(1);
  });

  it("R42 R426 is per Unit, and a base Panther has no Cleave, so only the defender dies", () => {
    const s = scenario({
      seed: "panther-no-cleave",
      p1: { field: ["32"], library: ["15", "15", "15", "15", "15", "15", "15"] },
      p2: { field: ["15", "15", "15"] },
    });
    const middle = must(s.unit("p2", 2), "p2 lane 2");

    s.attack("32", middle);

    s.expectInZone(middle, "graveyard");
    s.expectInZone(must(s.unit("p2", 1), "p2 lane 1"), "field");
    s.expectInZone(must(s.unit("p2", 3), "p2 lane 3"), "field");
    expect(s.hand("p1")).toHaveLength(2);
  });

  it("R426 an attack by another Unit draws nothing", () => {
    const s = scenario({
      seed: "panther-someone-else",
      p1: { field: ["32", "13"], library: ["15", "15", "15"] },
      p2: { field: ["15"] },
    });
    const prey = must(s.unit("p2", 1), "p2 lane 1");

    // The Shredder swings, not the Panther.
    s.attack(must(s.unit("p1", 2), "p1 lane 2"), prey);

    s.expectInZone(prey, "graveyard");
    expect(s.hand("p1")).toHaveLength(0);
  });
});

describe("#32 Prem Panther — radiant", () => {
  it("R426 radiant Cleave draws 2 per Unit that attack destroyed: three kills draw 6", () => {
    const s = scenario({
      seed: "panther-cleave",
      p1: {
        field: [{ def: "32", radiant: true }],
        library: ["15", "15", "15", "15", "15", "15", "15", "15"],
      },
      p2: { field: ["15", "15", "15"] },
    });
    const left = must(s.unit("p2", 1), "p2 lane 1");
    const middle = must(s.unit("p2", 2), "p2 lane 2");
    const right = must(s.unit("p2", 3), "p2 lane 3");

    s.attack("32", middle);

    // §4.4 step 10: Cleave hits the same-side neighbours as separate instances of 10.
    s.expectInZone(left, "graveyard");
    s.expectInZone(middle, "graveyard");
    s.expectInZone(right, "graveyard");
    expect(s.hand("p1")).toHaveLength(6);
    expect(s.pile("p1", "library")).toHaveLength(2);
  });

  it("R426 radiant: Cleave kills draw nothing when the Panther dies in that combat", () => {
    const s = scenario({
      seed: "panther-cleave-dies",
      p1: {
        field: [{ def: "32", radiant: true }],
        hand: ["5"],
        library: ["15", "15", "15", "15", "15", "15", "15", "15"],
      },
      p2: { field: ["15", "13", "15"], hand: ["15"] },
    });
    const panther = must(s.unit("p1", 1), "the radiant Panther");

    // 10 kills the 8/10 and cleaves both 1/1s; 8 back kills the 10/8.
    s.attack(panther, must(s.unit("p2", 2), "p2's Shredder"));

    s.expectInZone(panther, "graveyard");
    expect([1, 2, 3].map((lane) => s.unit("p2", lane))).toEqual([null, null, null]);
    expect(s.lastEvents.some((event) => event.type === "drawn")).toBe(false);
    expect(s.hand("p1")).toHaveLength(1);
  });

  it("§3.1 Cleave never crosses sides, so the Panther's own neighbours are not kills", () => {
    const s = scenario({
      seed: "panther-cleave-sides",
      p1: {
        field: [{ def: "32", radiant: true, lane: 2 }, { def: "15", lane: 1 }, { def: "15", lane: 3 }],
        library: ["15", "15", "15", "15", "15"],
      },
      p2: { field: [{ def: "15", lane: 2 }] },
    });
    const ally1 = must(s.unit("p1", 1), "p1 lane 1");
    const ally3 = must(s.unit("p1", 3), "p1 lane 3");

    s.attack(must(s.unit("p1", 2), "the Panther"), must(s.unit("p2", 2), "p2 lane 2"));

    s.expectInZone(ally1, "field");
    s.expectInZone(ally3, "field");
    expect(s.hand("p1")).toHaveLength(2);
  });

  it("radiant is 10/8 and its text is unchanged (\"same\"), so one kill still draws 2", () => {
    const s = scenario({
      seed: "panther-radiant-same",
      p1: { field: [{ def: "32", radiant: true }], library: ["15", "15", "15"] },
      p2: { field: ["43"] },
    });
    const panther = must(s.unit("p1", 1), "p1 lane 1");
    s.expectStats(panther, { attack: 10, maxHealth: 8 });

    // 10 into #43's 3/10 is lethal; 3 back into a 10/8 is not.
    s.attack(panther, must(s.unit("p2", 1), "p2 lane 1"));

    s.expectInZone(panther, "field");
    expect(s.hand("p1")).toHaveLength(2);
  });
});
