// #25 4-mana 7/7 — SPEC §8.2, BUILD M4-T4 row 25: "Armor 7 zeroes a 7 hit; radiant 14/14 Armor 7,
// Reborn: it comes back once at 1 health, from combat or a Tribute, and an exile removes it for good"
// (patch v0.1.1: the Radiant face used to be Indestructible).
//
// The §8.2 Engine cell is "Keywords only", so both scripts are empty and these fixtures prove the
// keywords printed on the catalog faces do the work through §4.4 and §4.5. The removals need a
// source: the Tribute is #22 Carnivorous Cube, whose Cry tributes one of your other permanents
// (§6.3 Tribute), and the exile is #34 Collateral Damage, the Core card that exiles a target
// permanent — so those two fixtures depend on those cards' scripts as well as on these keywords.

import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "./_harness";
import { base, def, radiant } from "../src/scripts/025-4-mana-7-7";

const BIG = "core-025";
const CUBE = "core-022"; // Cry: Tribute one of your other permanents.
const EXILER = "core-034"; // Collateral Damage: exile target permanent, cost 3.
const FILLER = "core-005";

function unitViewOf(s: Scenario, player: "p1" | "p2", lane: number) {
  const side = player === "p1" ? s.view("p1").you : s.view("p1").opponent;
  return side.units[lane - 1];
}

describe("#25 4-mana 7/7", () => {
  it("the keywords are printed on the catalog faces, so neither script grants anything", () => {
    // §10.4: Armor sums across sources, so a script that re-granted Armor 7 would show 14.
    expect(def.base.keywords).toEqual([{ kind: "Armor", n: 7 }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Armor", n: 7 }, { kind: "Reborn" }]);
    expect(base).toEqual({});
    expect(radiant).toEqual({});
  });

  describe("base", () => {
    it("§4.4 step 2: Armor 7 zeroes a 7 hit", () => {
      const s = scenario({
        seed: "big-armor",
        p1: { hand: [FILLER], field: [BIG], library: [FILLER] },
        p2: { hand: [FILLER], field: [BIG], library: [FILLER] },
      });
      const mine = s.unit("p1", 1);
      const theirs = s.unit("p2", 1);
      if (mine === null || theirs === null) throw new Error("both 7/7s should be on the board");

      s.endTurn(); // p2 attacks into p1.
      s.attack(theirs, mine);

      // §4.3 step 2 is simultaneous, and 7 − 7 is 0 on each side, which R63 makes a non-event.
      s.expectStats(mine, { attack: 7, maxHealth: 7, health: 7 });
      s.expectStats(theirs, { attack: 7, maxHealth: 7, health: 7 });
      expect(unitViewOf(s, "p1", 1)?.armor).toBe(7);
      // R63: a hit reduced to 0 emits nothing, so neither unit ever took a damage instance.
      const hits = s.events.filter(
        (event) => event.type === "damage" && [mine.id, theirs.id].includes(event.targetId),
      );
      expect(hits).toHaveLength(0);
    });
  });

  describe("radiant", () => {
    it("R275 the Radiant face is a 14/14 with Armor 7 and Reborn", () => {
      const s = scenario({
        seed: "big-radiant-face",
        p1: { hand: [FILLER], field: [{ def: BIG, radiant: true }] },
        p2: { hand: [FILLER] },
      });

      s.expectStats(BIG, { attack: 14, health: 14, maxHealth: 14 });
      expect(unitViewOf(s, "p1", 1)?.keywords).toEqual([{ kind: "Armor", n: 7 }, { kind: "Reborn" }]);
      expect(unitViewOf(s, "p1", 1)?.armor).toBe(7);
    });

    it("§4.4 step 2: its Armor 7 zeroes a 7 hit, and its 14 back kills a base 7/7 through that one's Armor", () => {
      const s = scenario({
        seed: "big-radiant-armor",
        p1: { hand: [FILLER], field: [{ def: BIG, radiant: true }] },
        p2: { hand: [FILLER], field: [BIG] },
      });
      const mine = s.unit("p1", 1);
      const theirs = s.unit("p2", 1);
      if (mine === null || theirs === null) throw new Error("both 7/7s should be on the board");

      s.endTurn();
      s.attack(theirs, mine);

      s.expectStats(mine, { health: 14, maxHealth: 14 });
      s.expectInZone(theirs, "graveyard");
    });

    it("§4.5 step 4: Reborn brings it back once, at 1 health and without Reborn, when it dies in combat", () => {
      const s = scenario({
        seed: "big-radiant-reborn",
        active: "p2",
        p1: { hand: [FILLER], field: [{ def: BIG, radiant: true, damage: 10 }], library: [FILLER] },
        // A radiant 18/18 Midrange Menace hits for 18: 11 through the Armor, which kills the 4 left.
        p2: { hand: [FILLER], field: [{ def: "core-019", radiant: true }], library: [FILLER] },
      });
      const big = s.card(BIG);

      s.attack("core-019", big);

      s.expectInZone(big, "field");
      s.expectStats(big, { health: 1, maxHealth: 14 });
      expect(unitViewOf(s, "p1", 1)?.keywords).toEqual([{ kind: "Armor", n: 7 }]);
      s.expectEvents("destroyed", "summoned");
    });

    it("§6.3 a Tribute is a death too, so Reborn brings it back from that as well", () => {
      const s = scenario({
        seed: "big-sacrifice",
        p1: { hand: [CUBE, FILLER], field: [{ def: BIG, radiant: true }] },
        p2: { hand: [FILLER] },
      });
      const big = s.card(BIG);
      s.play(CUBE, { targets: [{ pick: "instance", instanceId: big.id }] });

      // §6.3 Sacrifice counts as a death, and §6.1 Reborn answers it (R64).
      s.expectEvents("destroyed", "summoned");
      s.expectInZone(big, "field");
      s.expectStats(big, { health: 1 });
    });

    it("§6.1, §6.3 an exile removes it for good: no death, so no Reborn", () => {
      const s = scenario({
        seed: "big-exile",
        p1: { hand: [EXILER, FILLER], field: ["core-008"] },
        p2: { hand: [FILLER], field: [{ def: BIG, radiant: true }] },
      });
      const big = s.card(BIG);
      s.play(EXILER, { targets: [{ pick: "instance", instanceId: big.id }] });

      // §6.3 Exile: from anywhere to the exile pile, with no Death trigger (R55 counts it).
      s.expectInZone(big, "exile");
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).not.toContain(BIG);
      expect(s.unit("p2", 1)).toBeNull();
    });
  });
});
