// C #41 State of the Game — SPEC §8.6 row 41, BUILD M9 Classic row C 41: "3/3 Indestructible: damage
// and destroy effects don't remove it (a destroy knocks it into Attack Position, R46), while exile,
// bounce and a Tribute do; never has Taunt, even in Defense Position (R347); radiant 6/6
// Indestructible, Lifesteal (the adopted Radiant face): its damage heals your hero".
//
// The Engine cell is "Keywords only", so both scripts are empty and these fixtures prove that the
// keywords printed on the catalog faces do the work through §4.1, §4.2, §4.4 and §4.5. The removals
// need a source, each a Core card with its own tests: Hit Job (core-016, "Destroy target Unit"),
// Collateral Damage (core-034, exile a target permanent), Flood (core-017, bounce all Units) and
// Carnivorous Cube (core-022, whose Cry Tributes one of your other Units, R428).

import { describe, expect, it } from "vitest";
import type { PlayerId } from "@jackioh/shared";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/041-state-of-the-game";

const STATE = "classic-041";
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const COLLATERAL = "core-034"; // (4) Spell: Exile target permanent and a random card from their deck.
const FLOOD = "core-017"; // (4) Spell: Bounce all Units.
const CUBE = "core-022"; // (3) Unit: Cry: Tribute one of your other Units and remember it.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const VANILLA = "core-008"; // (1) Unit 4/4, no text.
const FILLER = "core-005"; // (1) Spell, a card to keep a hand from auto-ending the turn (§2.5).
const FLAME = "classic-016"; // (1) Spell, Book: Deal 4 damage.

function keywordKinds(s: Scenario, player: PlayerId, lane: number): string[] {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return s.stats(unit).keywords.map((keyword) => keyword.kind);
}

describe("C #41 State of the Game", () => {
  it("is keywords only: both faces are printed on the catalog, so neither script adds anything", () => {
    expect(def.id).toBe(STATE);
    expect(def.base.keywords).toEqual([{ kind: "Indestructible" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Indestructible" }, { kind: "Lifesteal" }]);
    expect(base).toEqual({});
    expect(radiant).toEqual({});
  });

  describe("base", () => {
    it("is a 3/3 with Indestructible on the field", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [STATE] }, p2: { hand: [FILLER] } });

      s.expectStats(STATE, { attack: 3, health: 3, maxHealth: 3 });
      expect(keywordKinds(s, "p1", 1)).toEqual(["Indestructible"]);
    });

    it("§4.4 step 4: combat damage never removes it — it attacks a 9/9 and takes nothing back", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [STATE] },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      const menace = s.unit("p2", 1);
      if (menace === null) throw new Error("the 9/9 should be on the board");

      s.attack(STATE, menace);

      s.expectInZone(STATE, "field");
      s.expectStats(STATE, { health: 3, maxHealth: 3 });
      s.expectStats(menace, { health: 6 });
      // R63: the strike back is reduced to nothing, so no damage event names it.
      const state = s.card(STATE);
      expect(s.events.filter((event) => event.type === "damage" && event.targetId === state.id)).toHaveLength(0);
    });

    it("§4.4 step 4: a damage effect never removes it — Book of Flame's 4 deals it nothing", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [STATE] },
        p2: { hand: [FLAME, FILLER] },
        active: "p2",
      });
      const state = s.card(STATE);

      s.play(FLAME, { targets: [{ pick: "instance", instanceId: state.id }] });

      s.expectInZone(state, "field");
      s.expectStats(state, { health: 3, maxHealth: 3 });
      expect(s.events.filter((event) => event.type === "damage" && event.targetId === state.id)).toHaveLength(0);
    });

    it("R46 a destroy effect leaves it on the field and knocks it into Attack Position", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: STATE, position: "DEF" }] },
        p2: { hand: [HIT_JOB, FILLER] },
        active: "p2",
      });
      const state = s.card(STATE);

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: state.id }] });

      s.expectInZone(state, "field");
      expect(s.stats(state).position).toBe("ATK");
      expect(s.pile("p1", "graveyard")).toHaveLength(0);
      s.expectEvents("positionSwitched");
    });

    it("R347 it never has Taunt, even in Defense Position, where it keeps only the Armor +1", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: STATE, position: "DEF" }, { def: VANILLA, lane: 2 }] },
        p2: { hand: [FILLER], field: [VANILLA] },
        active: "p2",
      });

      expect(keywordKinds(s, "p1", 1)).toEqual(["Indestructible", "Armor"]);
      expect(s.stats(STATE).armor).toBe(1);

      // §4.2 step 3 finds no Taunt on p1's side, so the attacker may pass it by for the Vanilla.
      const attacker = s.unit("p2", 1);
      const other = s.unit("p1", 2);
      if (attacker === null || other === null) throw new Error("both Vanillas should be on the board");
      s.attack(attacker, other);
      s.expectEvents("attackDeclared");
    });

    it("§6.1 an exile removes it: no destroy is involved, so Indestructible does not help", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [STATE] },
        p2: { hand: [COLLATERAL, FILLER], library: [FILLER, FILLER] },
        active: "p2",
      });
      const state = s.card(STATE);

      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: state.id }] });

      s.expectInZone(state, "exile");
      expect(s.unit("p1", 1)).toBeNull();
    });

    it("§6.1 a bounce removes it to its owner's hand", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [STATE] },
        p2: { hand: [FLOOD, FILLER] },
        active: "p2",
      });
      const state = s.card(STATE);

      s.play(FLOOD);

      s.expectInZone(state, "hand");
      expect(s.hand("p1").map((card) => card.id)).toContain(state.id);
    });

    it("§6.3 a Tribute removes it: Sacrifice bypasses Indestructible and counts as a death", () => {
      const s = scenario({ p1: { hand: [CUBE, FILLER], field: [STATE] }, p2: { hand: [FILLER] } });
      const state = s.card(STATE);

      s.play(CUBE, { targets: [{ pick: "instance", instanceId: state.id }] });

      s.expectInZone(state, "graveyard");
      s.expectEvents("destroyed");
    });
  });

  describe("radiant", () => {
    it("R275 the Radiant face is a 6/6 with Indestructible and Lifesteal", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [{ def: STATE, radiant: true }] }, p2: { hand: [FILLER] } });

      s.expectStats(STATE, { attack: 6, health: 6, maxHealth: 6 });
      expect(keywordKinds(s, "p1", 1)).toEqual(["Indestructible", "Lifesteal"]);
    });

    it("§4.4 step 8: its damage to the enemy hero heals your hero the amount dealt", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: STATE, radiant: true }], health: 20 },
        p2: { hand: [FILLER] },
      });

      s.attack(STATE, "hero");

      s.expectHealth("p2", 24);
      s.expectHealth("p1", 26);
    });

    it("§4.4 step 8: its damage to a Unit heals you too, and the strike back still deals it nothing", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: STATE, radiant: true }], health: 10 },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      const menace = s.unit("p2", 1);
      if (menace === null) throw new Error("the 9/9 should be on the board");

      s.attack(STATE, menace);

      s.expectStats(menace, { health: 3 });
      s.expectStats(STATE, { health: 6, maxHealth: 6 });
      s.expectHealth("p1", 16);
    });

    it("R347 still no Taunt in Defense Position", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: STATE, radiant: true, position: "DEF" }] },
        p2: { hand: [FILLER] },
      });

      expect(keywordKinds(s, "p1", 1)).toEqual(["Indestructible", "Lifesteal", "Armor"]);
    });

    it("R46 a destroy effect leaves the Radiant face on the field too", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: STATE, radiant: true }] },
        p2: { hand: [HIT_JOB, FILLER] },
        active: "p2",
      });
      const state = s.card(STATE);

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: state.id }] });

      s.expectInZone(state, "field");
      s.expectStats(state, { health: 6 });
    });
  });
});
