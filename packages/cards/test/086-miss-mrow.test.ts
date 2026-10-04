// #86 "Miss" Mrow (SPEC §8.4, BUILD M4-T4 row 86): "Cannot attack; Death takes control of the Unit
// that destroyed it (R42's killer, R361), placed per R15, and nothing when no Unit on the field
// destroyed it; radiant has Rush instead". Patch v0.1.1 replaced "Death: steal all enemy units" and
// the radiant face's Taunt.

import { keywordsOf } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";

const MROW = "core-086";

// Field fixtures, chosen because every one of them has a Cry and nothing else: a unit placed by the
// harness's `field` setup never fires its Cry, so these are inert boards.
const GARY = "core-004"; // 1/1
const FELINORS = "core-012"; // 3/4, kills a 1/1 Mrow and survives the 1 back
const POSTDOC = "core-061"; // 2/4
const RENO = "core-053"; // 4/6
const SORCERER = "core-068"; // 5/5; its Cry deals 4 damage to a target
const STRAAZA = "core-054"; // 8/8
const HIT_JOB = "core-016"; // Spell: destroy target Unit
const LUNAR_ECLIPSE = "core-035"; // Spell: deal 3 damage to a target

// §2.5: a turn with nothing but `endTurn` left auto-ends and cascades into the next turn's draw.
// One always-playable card in each hand keeps every scenario on the turn it started on.
const FILLER = "core-005";

const SEED = "mrow-86";

describe('#86 "Miss" Mrow — base', () => {
  it("cannot attack: the printed keyword is the catalog's, and combat refuses both targets", () => {
    const s = scenario({
      seed: SEED,
      p1: { hand: [FILLER], field: [MROW] },
      p2: { hand: [FILLER], field: [GARY] },
    });

    expect(keywordsOf(s.state, s.card(MROW))).toEqual([{ kind: "Can't attack" }]);
    expect(() => s.attack(MROW, "hero")).toThrow(/cannot attack/);
    expect(() => s.attack(MROW, GARY)).toThrow(/cannot attack/);
  });

  it("R361 Death takes control of the Unit that destroyed it, into the same lane when free (R15), and only that one", () => {
    const s = scenario({
      seed: SEED,
      active: "p2",
      p1: { hand: [FILLER], field: [{ def: MROW, lane: 3 }] },
      p2: { hand: [FILLER], field: [{ def: FELINORS, lane: 1 }, { def: GARY, lane: 2 }] },
    });

    s.attack(FELINORS, MROW);

    s.expectInZone(MROW, "graveyard").expectEvents("destroyed", "controlChanged");
    // R15: p1's lane 1 was free, so the killer kept its lane.
    expect(s.unit("p1", 1)?.defId).toBe(FELINORS);
    expect(s.unit("p2", 1)).toBeNull();
    // R12, R662: control, and the current owner with it; R78: it never left the field, so its
    // damage came along.
    expect(s.unit("p1", 1)?.owner).toBe("p1");
    expect(s.unit("p1", 1)?.controller).toBe("p1");
    expect(s.unit("p1", 1)?.damage).toBe(1);
    // The other enemy unit had no part in it and stays with p2.
    expect(s.unit("p2", 2)?.defId).toBe(GARY);
    expect(s.unit("p2", 2)?.controller).toBe("p2");
  });

  it("R361 falls back to the first free zone, which Mrow's own death has freed (R15)", () => {
    const s = scenario({
      seed: SEED,
      active: "p2",
      p1: {
        hand: [FILLER],
        field: [
          { def: MROW, lane: 1 },
          { def: RENO, lane: 2 },
          { def: STRAAZA, lane: 3 },
          { def: SORCERER, lane: 4 },
          { def: GARY, lane: 5 },
        ],
      },
      p2: { hand: [FILLER], field: [{ def: GARY, lane: 1 }, { def: FELINORS, lane: 2 }, { def: POSTDOC, lane: 3 }] },
    });

    s.attack(FELINORS, MROW);

    expect(s.unit("p1", 1)?.defId).toBe(FELINORS);
    expect(s.unit("p1", 1)?.owner).toBe("p1");
    expect(s.unit("p2", 2)).toBeNull();
    expect(s.unit("p2", 1)?.controller).toBe("p2");
    expect(s.unit("p2", 3)?.controller).toBe("p2");
  });

  it("R361 a Unit whose Cry dealt the lethal damage destroyed it, so Mrow takes that Unit", () => {
    const s = scenario({
      seed: SEED,
      active: "p2",
      p1: { hand: [FILLER], field: [{ def: MROW, lane: 1 }] },
      p2: { hand: [SORCERER, FILLER] },
    });

    s.play(SORCERER, { zone: 2, targets: [{ pick: "instance", instanceId: s.card(MROW).id }] });

    s.expectInZone(MROW, "graveyard");
    expect(s.unit("p1", 2)?.defId).toBe(SORCERER);
    expect(s.unit("p1", 2)?.owner).toBe("p1");
  });

  it("R361 takes nothing when the killer died in the same combat (R78)", () => {
    const s = scenario({
      seed: SEED,
      active: "p2",
      p1: { hand: [FILLER], field: [{ def: MROW, lane: 1 }] },
      p2: { hand: [FILLER], field: [{ def: GARY, lane: 1 }, { def: FELINORS, lane: 2 }] },
    });

    s.attack(GARY, MROW);

    s.expectInZone(MROW, "graveyard");
    s.expectInZone(GARY, "graveyard");
    expect(s.events.some((event) => event.type === "controlChanged")).toBe(false);
    expect(s.unit("p2", 2)?.controller).toBe("p2");
  });

  it("R361 takes nothing when a Spell destroyed it or a Spell's damage killed it (R42)", () => {
    for (const spell of [HIT_JOB, LUNAR_ECLIPSE]) {
      const s = scenario({
        seed: SEED,
        active: "p2",
        p1: { hand: [FILLER], field: [{ def: MROW, lane: 1 }] },
        p2: { hand: [spell, FILLER], field: [{ def: FELINORS, lane: 1 }] },
      });

      s.play(spell, { targets: [{ pick: "instance", instanceId: s.card(MROW).id }] });

      s.expectInZone(MROW, "graveyard");
      expect(s.events.some((event) => event.type === "controlChanged")).toBe(false);
      expect(s.unit("p2", 1)?.controller).toBe("p2");
    }
  });

  it("R361 with no free zone the killer stays with its controller (R15)", () => {
    // Mrow sits on top of a pile in lane 5, so her death leaves that lane with the Gary beneath her
    // and p1's row stays full (§3.2).
    const s = scenario({
      seed: SEED,
      active: "p2",
      p1: {
        hand: [FILLER],
        field: [
          { def: RENO, lane: 1 },
          { def: RENO, lane: 2 },
          { def: STRAAZA, lane: 3 },
          { def: SORCERER, lane: 4 },
          { def: GARY, lane: 5 },
          { def: MROW, lane: 5, stack: true },
        ],
      },
      p2: { hand: [FILLER], field: [{ def: FELINORS, lane: 1 }] },
    });

    s.attack(FELINORS, MROW);

    s.expectInZone(MROW, "graveyard");
    expect(s.unit("p1", 5)?.defId).toBe(GARY);
    expect(s.unit("p2", 1)?.defId).toBe(FELINORS);
    expect(s.unit("p2", 1)?.controller).toBe("p2");
  });
});

describe('#86 "Miss" Mrow — radiant', () => {
  it("patch v0.1.1: the radiant face prints Rush alone, so it may attack a Unit the turn it lands", () => {
    const s = scenario({
      seed: SEED,
      p1: { hand: [{ def: MROW, radiant: true }, FILLER] },
      p2: { hand: [FILLER], field: [GARY] },
    });

    s.play(MROW, { zone: 1 });
    expect(keywordsOf(s.state, s.card(MROW))).toEqual([{ kind: "Rush" }]);
    s.expectStats(MROW, { attack: 2, maxHealth: 2 });
    // Rush: units only on the turn it lands (§6.1).
    expect(() => s.attack(MROW, "hero")).toThrow();
    s.attack(MROW, GARY);
    s.expectInZone(GARY, "graveyard");
  });

  it("the radiant face has no Taunt: an enemy attack may go past it to the hero", () => {
    const s = scenario({
      seed: SEED,
      active: "p2",
      p1: { hand: [FILLER], field: [{ def: MROW, radiant: true, lane: 1 }] },
      p2: { hand: [FILLER], field: [{ def: FELINORS, lane: 1 }] },
    });

    s.attack(FELINORS, "hero");
    s.expectHealth("p1", 27);
  });

  it("R361 radiant: the same Death takes the Unit whose strike back killed it", () => {
    const s = scenario({
      seed: SEED,
      p1: { hand: [FILLER], field: [{ def: MROW, radiant: true, lane: 1 }] },
      p2: { hand: [FILLER], field: [{ def: SORCERER, lane: 1 }, { def: GARY, lane: 2 }] },
    });

    // The radiant 2/2 attacks a 5/5: it deals 2, takes 5 and dies, and the 5/5 destroyed it.
    s.attack(MROW, SORCERER);

    s.expectInZone(MROW, "graveyard");
    expect(s.unit("p1", 1)?.defId).toBe(SORCERER);
    expect(s.unit("p1", 1)?.damage).toBe(2);
    expect(s.unit("p1", 1)?.owner).toBe("p1");
    expect(s.unit("p2", 2)?.defId).toBe(GARY);
  });
});
