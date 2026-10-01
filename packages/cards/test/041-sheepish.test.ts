// #41 Sheepish (SPEC §8.2, §5.1, §10.3, §10.5 step 7; R17, R23, R33, R61, R70, R120, R174, R427).
//
// The must-pass row (BUILD M4-T4 #41), as patch v0.2.0 rewrites it (R427, R17's Sheepish half):
// "Opponent's Unit becomes a Sheep after its Cry resolves; trap consumed; Immutable target → consumed
// with no effect; radiant adds 0-cost Lava Golem."
//
// "After its Cry" is proved with an observable Cry rather than with event order alone: #53 Reno's Cry
// is "heal your hero up to 30 health", so a p1 hero left at 10 that is at 30 after the play is a Cry
// that ran — and the Unit standing in its zone is a Sheep all the same.

import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";
import { base as sheepishBase, radiant as sheepishRadiant } from "../src/scripts/041-sheepish";

/**
 * p1 is active with `card` to play; p2 holds Sheepish face-down in backrow lane 1.
 *
 * #16 Hit Job rides along in p1's hand purely so the turn has something meaningful left after the
 * play and does not auto-end into p2's turn (R82; see the harness header).
 */
function trapSet(card: string | { def: string; radiant: boolean }, radiantTrap = false) {
  return scenario({
    seed: "sheepish",
    p1: { hand: [card, "core-016"], health: 10, mana: 10 },
    p2: { backrow: [{ def: "core-041", radiant: radiantTrap, lane: 1 }], health: 30 },
  });
}

/** A Radiant #19 Midrange Menace: "Taunt, Immutable" (§8.1 row 19), the Immutable Unit to play. */
const IMMUTABLE = { def: "core-019", radiant: true };

function firedTrap(events: readonly { type: string }[]): boolean {
  return events.some((event) => event.type === "trapFired");
}

function transformed(events: readonly { type: string }[]): boolean {
  return events.some((event) => event.type === "transformed");
}

describe("#41 Sheepish — base", () => {
  it("R427 transforms the opponent's played Unit into a Sheep Token after its Cry resolves, so the Cry is kept", () => {
    const s = trapSet("core-053").play("core-053", { zone: 1 });

    // §10.5 step 5 runs the Cry, step 7 emits `cardResolved`, and only then does the trap fire.
    s.expectEvents("cardPlayed", "healed", "cardResolved", "trapFired", "transformed");
    expect(s.unit("p1", 1)?.defId).toBe("core-t-sheep");
    // #53 Reno's Cry healed p1's hero to 30 before the Unit became a Sheep.
    s.expectHealth("p1", 30);
  });

  it("R427 what the Cry made stays: #15's Rush Token is on the field beside the Sheep", () => {
    const s = trapSet("core-015").play("core-015", { zone: 1 });

    expect(s.unit("p1", 1)?.defId).toBe("core-t-sheep");
    expect(s.unit("p1", 2)?.defId).toBe("core-t-rush");
  });

  it("§7 the replacement is the 1/1 Sheep Token, in the played unit's own zone (§6.3 Transform)", () => {
    const s = trapSet("core-053").play("core-053", { zone: 1 });

    s.expectInZone("core-t-sheep", "field");
    s.expectStats("core-t-sheep", { attack: 1, health: 1, maxHealth: 1 });
  });

  it("§5.1 the Trap is consumed: it leaves the backrow for its owner's graveyard", () => {
    const s = trapSet("core-053").play("core-053", { zone: 1 });

    s.expectInZone("core-041", "graveyard");
    expect(s.backrow("p2", 1)).toBe(null);
  });

  it("R17 an Immutable target still fires the trap, which is consumed with no effect (R23)", () => {
    // A Radiant #19 Midrange Menace is Immutable, and R23 makes Immutable refuse Transform on the
    // card itself.
    const s = trapSet(IMMUTABLE).play("core-019", { zone: 1 });

    expect(firedTrap(s.events)).toBe(true);
    expect(transformed(s.events)).toBe(false);
    expect(s.unit("p1", 1)?.defId).toBe("core-019");
    s.expectInZone("core-041", "graveyard");
  });

  it("R61 a Spell leaves the trap armed and face-down: the condition is a `when`, not an empty `run`", () => {
    const s = scenario({
      seed: "sheepish",
      p1: { hand: ["core-044", "core-016"], mana: 10 },
      p2: { backrow: [{ def: "core-041", lane: 1 }], health: 30 },
    }).play("core-044", { targets: [{ pick: "hero", player: "p2" }] });

    // The spell resolved, so the play really happened and the trap really saw the event.
    s.expectHealth("p2", 26);
    expect(firedTrap(s.events)).toBe(false);
    s.expectInZone("core-041", "field");
  });

  it("§8 fires only on the OPPONENT's play: the controller's own Unit leaves it armed", () => {
    const s = scenario({
      seed: "sheepish",
      active: "p2",
      turn: 2,
      p2: {
        hand: ["core-053", "core-016"],
        backrow: [{ def: "core-041", lane: 1 }],
        health: 10,
        mana: 10,
      },
    }).play("core-053", { zone: 1 });

    expect(firedTrap(s.events)).toBe(false);
    s.expectInZone("core-041", "field");
    expect(s.unit("p2", 1)?.defId).toBe("core-053");
    // Reno's own Cry ran, because nothing interrupted it.
    s.expectHealth("p2", 30);
  });

  it("R427, R174 a Unit an earlier trap answering the same play has taken off the field is no play left to answer: Sheepish stays set", () => {
    // p2's backrow: Bear Honeypot in lane 1, Sheepish in lane 2. Both answer the same `cardResolved`
    // in lane order (R68): the Honeypot's two Rush Tokens attack the played 1/1 and kill it, so by the
    // time Sheepish is offered the play its Unit has been taken off the field after it resolved.
    const s = scenario({
      seed: "sheepish-honeypot",
      p1: { hand: ["core-086", "core-016"], mana: 10 },
      p2: {
        backrow: [
          { def: "core-060", lane: 1 },
          { def: "core-041", radiant: true, lane: 2 },
        ],
      },
    });
    const mrow = s.card("core-086");
    s.play(mrow, { zone: 1 });

    const fired = s.events.flatMap((event) => (event.type === "trapFired" ? [event.defId] : []));
    expect(fired).toEqual(["core-060"]);
    expect(transformed(s.events)).toBe(false);
    s.expectInZone(mrow, "graveyard");
    // Still face-down in its zone, armed for the next Unit, and no Lava Golem came of it.
    expect(s.backrow("p2", 2)?.defId).toBe("core-041");
    expect(s.backrow("p2", 2)?.faceUp).toBeUndefined();
    expect(s.hand("p2").some((card) => card.defId === "core-055")).toBe(false);
  });

  it("R427 a Unit that left the field in its own resolution still fires Sheepish, which is consumed and transforms nothing", () => {
    // A Radiant #52 Silly Silas played into lane 5 and rotating right crosses to the other side in
    // its own Cry, and its Radiant face bounces what crosses: it is in p1's hand as the play resolves.
    const s = scenario({
      seed: "sheepish-silas",
      p1: { hand: [{ def: "core-052", radiant: true }, "core-016"], mana: 10 },
      // Lane 3: the rotation moves the trap a step along p2's own row, and it stays p2's.
      p2: { backrow: [{ def: "core-041", radiant: true, lane: 3 }] },
    });
    const silas = s.card("core-052");
    s.play(silas, { zone: 5, modes: ["right"] });

    s.expectInZone(silas, "hand");
    expect(firedTrap(s.events)).toBe(true);
    expect(transformed(s.events)).toBe(false);
    s.expectInZone("core-041", "graveyard");
    // R120: the Radiant face's Lava Golem is its own clause, and still lands.
    expect(s.hand("p2").some((card) => card.defId === "core-055" && card.costOverride === 0)).toBe(true);
  });

  it("R427, R113 a Cry that asks: the Unit becomes a Sheep only once the answer has resolved the Cry, across a round trip", () => {
    // #7 Jewelosco Scarab's Cry is a Discover: the play pauses at step 5 with the trap unfired.
    const s = trapSet("core-007").play("core-007", { zone: 1 });
    expect(s.state.pending?.kind).toBe("discover");
    expect(firedTrap(s.events)).toBe(false);
    expect(s.unit("p1", 1)?.defId).toBe("core-007");

    // §9.3: the paused game is plain JSON.
    expect(JSON.parse(JSON.stringify(s.state))).toEqual(s.state);

    const offered = s.state.pending?.options[0]?.key;
    expect(offered).toBeDefined();
    s.answer(offered ?? "");

    s.expectEvents("promptAnswered", "addedToHand", "cardResolved", "trapFired", "transformed");
    expect(s.unit("p1", 1)?.defId).toBe("core-t-sheep");
    s.expectInZone("core-041", "graveyard");
  });

  it("R81 R427 the trap declares no play-time choice and watches `cardResolved`", () => {
    expect(sheepishBase.targets).toBeUndefined();
    expect(sheepishBase.modes).toBeUndefined();
    expect(sheepishBase.triggers?.map((trigger) => trigger.on)).toEqual([["cardResolved"]]);
  });
});

describe("#41 Sheepish — radiant", () => {
  it("transforms the played Unit after its Cry AND adds a Lava Golem costing 0 to the trap controller's hand", () => {
    const s = trapSet("core-053", true).play("core-053", { zone: 1 });

    expect(s.unit("p1", 1)?.defId).toBe("core-t-sheep");
    s.expectHealth("p1", 30);

    const golem = s.hand("p2").find((card) => card.defId === "core-055");
    expect(golem).toBeDefined();
    // R65: a `costOverride` of 0 is what "It costs (0)" means, and it survives every zone.
    expect(golem?.costOverride).toBe(0);
  });

  it("R427 the radiant trap is consumed too, once, and the Sheep replaces the unit", () => {
    const s = trapSet("core-053", true).play("core-053", { zone: 1 });

    s.expectEvents("cardPlayed", "cardResolved", "trapFired", "transformed", "addedToHand");
    s.expectInZone("core-041", "graveyard");
    expect(s.hand("p2").filter((card) => card.defId === "core-055")).toHaveLength(1);
  });

  it("R120 an Also clause stands on its own: an Immutable target refuses the Transform and the rest of the text still resolves", () => {
    const s = trapSet(IMMUTABLE, true).play("core-019", { zone: 1 });

    expect(firedTrap(s.events)).toBe(true);
    expect(transformed(s.events)).toBe(false);
    expect(s.unit("p1", 1)?.defId).toBe("core-019");
    expect(s.hand("p2").some((card) => card.defId === "core-055" && card.costOverride === 0)).toBe(true);
    s.expectInZone("core-041", "graveyard");
  });

  it("R61 the radiant face arms on the same condition: a Spell does not fire it", () => {
    const s = scenario({
      seed: "sheepish",
      p1: { hand: ["core-044", "core-016"], mana: 10 },
      p2: { backrow: [{ def: "core-041", radiant: true, lane: 1 }], health: 30 },
    }).play("core-044", { targets: [{ pick: "hero", player: "p2" }] });

    expect(firedTrap(s.events)).toBe(false);
    expect(s.hand("p2").some((card) => card.defId === "core-055")).toBe(false);
    s.expectInZone("core-041", "field");
  });

  it("R74 R427 both faces watch the same event, so the radiant text changes what fires, not when", () => {
    expect(sheepishRadiant.triggers?.map((trigger) => trigger.on)).toEqual([["cardResolved"]]);
  });
});
