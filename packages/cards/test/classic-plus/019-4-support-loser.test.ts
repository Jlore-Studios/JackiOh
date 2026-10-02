// C+ #19.4 Support Loser — SPEC §8.7 row 19.4, BUILD M9 Classic+ row C+ 19.4: "Divine Shield, 0 attack,
// so it never declares an attack and strikes back with no damage instance (R63); at its controller's
// end of turn heals your hero 3 (past 30 allowed) and each of your Units 3 up to max health, itself
// included; an opposing Blood Moon turns each of those heals into Pierce damage (R413); the heal reads
// through `param()`; radiant 0/10 Divine Shield, Reborn, heal 6".

import { legalActions, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/019-4-support-loser";

const SUPPORT = "classicplus-019-4";
const BODY = "core-019"; // 9/9 Taunt; end of turn: heal this to full — kept off the board here
const VANILLA = "core-008"; // 4/4
const MOON = "classicplus-022";
const HIT_JOB = "core-016";
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER];

function support(p1: SideSetup = {}, radiantFace = false, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [FILLER], library: DECK, ...p1, field: [{ def: SUPPORT, lane: 3, radiant: radiantFace }, ...(p1.field ?? [])] },
    p2: { hand: [FILLER], library: DECK, ...p2 },
  });
}

function loser(s: Scenario): ReturnType<Scenario["card"]> {
  const unit = s.unit("p1", 3);
  if (unit === null || unit.defId !== SUPPORT) throw new Error("no Support Loser in lane 3");
  return unit;
}

describe("C+ #19.4 Support Loser", () => {
  it("is a (2) 0/5 Divine Shield Unit token, printed Legendary; one script serves both faces", () => {
    expect(def.base.attack).toBe(0);
    expect(def.base.keywords).toEqual([{ kind: "Divine Shield" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Divine Shield" }, { kind: "Reborn" }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("§4.1 with 0 attack it never declares an attack", () => {
      const s = support();
      expect(legalActions(s.state, "p1").some((action) => action.type === "attack" && action.attackerId === loser(s).id)).toBe(false);
    });

    it("R63 attacked, it strikes back with no damage instance", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [{ def: SUPPORT, lane: 3 }], library: DECK },
        p2: { hand: [FILLER], field: [{ def: VANILLA, lane: 3 }], library: DECK },
      });
      const attacker = s.unit("p2", 3);
      if (attacker === null) throw new Error("setup");
      s.attack(attacker, loser(s));
      expect(s.events.filter((event) => event.type === "damage" && event.sourceId === loser(s).id)).toEqual([]);
      // Its Divine Shield took the hit.
      s.expectStats(loser(s), { health: 5 });
      expect(s.stats(loser(s)).keywords.some((keyword) => keyword.kind === "Divine Shield")).toBe(false);
    });

    it("at its controller's end of turn heals the hero 3, past 30", () => {
      const s = support();
      s.endTurn();
      s.expectHealth("p1", 33);
    });

    it("heals each of your Units 3, up to max health, itself included, and no enemy", () => {
      const s = support(
        { field: [{ def: VANILLA, lane: 1, damage: 2 }, { def: BODY, lane: 5, damage: 5 }] },
        false,
        { field: [{ def: VANILLA, lane: 1, damage: 3 }] },
      );
      loser(s).damage = 4;
      s.endTurn();
      s.expectStats(s.unit("p1", 1) ?? "", { health: 4 });
      s.expectStats(s.unit("p1", 5) ?? "", { health: 9 });
      s.expectStats(loser(s), { health: 4 });
      s.expectStats(s.unit("p2", 1) ?? "", { health: 1 });
    });

    it("nothing at the opponent's end of turn", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [{ def: SUPPORT, lane: 3 }], library: DECK, health: 20 },
        p2: { hand: [FILLER], library: DECK },
      });
      s.endTurn();
      // p1's own start of turn follows; no heal came at p2's end.
      s.expectHealth("p1", 20);
    });

    it("R413 an opposing Blood Moon turns each heal into Pierce damage", () => {
      const s = support(
        { field: [{ def: VANILLA, lane: 1, damage: 2 }], health: 20 },
        false,
        { backrow: [{ def: MOON, lane: 1, faceUp: false }] },
      );
      const vanilla = s.unit("p1", 1);
      if (vanilla === null) throw new Error("setup");
      s.endTurn();
      // The hero heal fires the trap and is converted, and so is each Unit's heal that turn: the damaged
      // 4/4 (2 health left) takes 3 and dies, and the Support Loser's own Divine Shield takes its 3.
      s.expectHealth("p1", 17);
      s.expectInZone(vanilla, "graveyard");
      s.expectStats(loser(s), { health: 5 });
      expect(s.stats(loser(s)).keywords.some((keyword) => keyword.kind === "Divine Shield")).toBe(false);
      expect(s.events.filter((event) => event.type === "healed")).toEqual([]);
    });

    it("R386 the heal reads through param: an Upgrade heals 4", () => {
      const s = support();
      stepParam(loser(s), "heal", 1);
      s.endTurn();
      s.expectHealth("p1", 34);
    });

    it("a Token, it ceases to exist when destroyed", () => {
      const s = support({ hand: [HIT_JOB, FILLER] });
      const unit = loser(s);
      // Divine Shield does not stop a destroy.
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: unit.id }] });
      s.expectInZone(unit, "gone");
    });
  });

  describe("radiant", () => {
    it("is 0/10 with Divine Shield and Reborn and heals 6", () => {
      const s = support({ field: [{ def: VANILLA, lane: 1, damage: 3 }] }, true);
      s.expectStats(loser(s), { attack: 0, health: 10 });
      s.endTurn();
      s.expectHealth("p1", 36);
      s.expectStats(s.unit("p1", 1) ?? "", { health: 4 });
    });

    it("R11 R175 Reborn brings the token back once, at 1 health and without Reborn", () => {
      const s = support({ hand: [HIT_JOB, FILLER] }, true);
      const unit = loser(s);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: unit.id }] });
      const back = s.unit("p1", 3);
      expect(back?.defId).toBe(SUPPORT);
      s.expectStats(back ?? "", { health: 1 });
      expect(s.stats(back ?? "").keywords.some((keyword) => keyword.kind === "Reborn")).toBe(false);
    });
  });
});
