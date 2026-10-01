// C+ #2 Groom Shroom — SPEC §8.7 row 2, BUILD M9 Classic+ row C+ 2: "Same window as C+ #1; fills
// every empty, unlocked, unreserved unit zone of yours with a random non-token Felinor Unit of any set
// (R64, R380; Felinor-tagged Units, never a Felinor Spell, R405; repeats allowed, R60), summoned with no
// Cry (R1) and granted Taunt; the declared attack still hits your hero and is never moved to a new
// Taunt, which arrived after §4.2 step 3 (R405); a full board summons nothing and draws no random
// number (R129); trap consumed; hidden until it fires (R33); radiant the Felinors are Radiant".
//
// Groom Shroom sits face-down in p1's backrow lane 1; p2 is active and attacks with a 3/3 Tempo Timmy.
// The two engine setups below (`lockZone`, `reserveZone`) put a Lock and a Reborn reservation on a
// zone, which no Core card can do to a unit zone.

import { defOf, lockZone, reserveZone } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/002-groom-shroom";

const GROOM = "classicplus-002";
const TIMMY = "core-011"; // (1) 3/3 Rush, First Strike.
const VANILLA = "core-008"; // (1) 4/4.
const FILLER = "core-010";
const STOCKPILE = "core-005";
/** Every non-token Felinor-tagged Unit of the three sets (R405, R380). */
const FELINOR_UNITS = ["core-012", "core-043", "core-086", "classic-047", "classicplus-030", "classicplus-046"];

function setup(p1: SideSetup = {}, radiantFace = false, seed?: string): Scenario {
  return scenario({
    ...(seed === undefined ? {} : { seed }),
    active: "p2",
    p1: { hand: [FILLER], library: [STOCKPILE], backrow: [{ def: GROOM, radiant: radiantFace, faceUp: false, lane: 1 }], ...p1 },
    p2: { hand: [FILLER], library: [STOCKPILE], field: [TIMMY] },
  });
}

function summonedIds(s: Scenario): string[] {
  return s.events.flatMap((event) => (event.type === "summoned" ? [event.instanceId] : []));
}

describe("C+ #2 Groom Shroom", () => {
  it("is a (3) Trap tagged Felinor; each face declares one trap trigger on `attackDeclared`", () => {
    expect(def.type).toBe("Trap");
    expect(def.tags).toEqual(["Felinor"]);
    expect(base.triggers?.map((t) => t.on)).toEqual([["attackDeclared"]]);
    expect(radiant.triggers?.map((t) => t.on)).toEqual([["attackDeclared"]]);
  });

  describe("base", () => {
    it("R64 R405 fills every empty unit zone of yours with a non-token Felinor Unit, each granted Taunt, no Cry", () => {
      const s = setup({ field: [{ def: VANILLA, lane: 2 }] });
      const groom = s.card(GROOM);

      s.attack(s.unit("p2", 1)!, "hero");

      s.expectEvents("attackDeclared", "trapFired", "summoned", "keywordGranted");
      const ids = summonedIds(s);
      expect(ids).toHaveLength(4);
      expect([1, 3, 4, 5].map((lane) => s.unit("p1", lane)?.id)).toEqual(ids);
      for (const id of ids) {
        const card = s.card(id);
        const cardDef = defOf(s.state, card.defId);
        expect(FELINOR_UNITS).toContain(card.defId);
        expect(cardDef.type).toBe("Unit");
        expect(cardDef.token).toBe(false);
        expect(card.controller).toBe("p1");
        expect(card.radiant).toBe(false);
        expect(card.grantedKeywords.map((k) => k.kind)).toContain("Taunt");
        expect(s.stats(card).keywords.map((k) => k.kind)).toContain("Taunt");
      }
      // R1: summoned, not played — no Cry (a Duplicating Felinors' copy, a Big Felinor's sweep).
      expect(s.events.filter((event) => event.type === "cardPlayed")).toHaveLength(0);
      expect(s.unit("p1", 2)?.defId).toBe(VANILLA);
      s.expectInZone(groom, "graveyard");
    });

    it("R405 the declared attack still hits your hero: the new Taunts arrived after §4.2 step 3", () => {
      const s = setup();

      s.attack(s.unit("p2", 1)!, "hero");

      s.expectHealth("p1", 27);
      expect(s.events.filter((event) => event.type === "redirected")).toHaveLength(0);
    });

    it("R60 R380 draws from every set with repeats allowed, and never a Felinor Spell or token", () => {
      const seen = new Set<string>();
      let repeated = false;
      for (let seed = 1; seed <= 12; seed += 1) {
        const s = setup({}, false, `groom-${seed}`);
        s.attack(s.unit("p2", 1)!, "hero");
        const defs = summonedIds(s).map((id) => s.card(id).defId);
        expect(defs).toHaveLength(5);
        // R1: no Cry ran — Big Felinor's would destroy the attacker, Felinor Fuser's would open a
        // Discover, Felinor Flagbearer's would give armor.
        expect(s.events.filter((event) => event.type === "destroyed")).toHaveLength(0);
        expect(s.state.pending).toBeNull();
        expect(s.state.players.p1.hero.armor).toBe(0);
        if (new Set(defs).size < defs.length) repeated = true;
        for (const id of defs) seen.add(id);
      }
      expect(repeated).toBe(true);
      expect([...seen].every((id) => FELINOR_UNITS.includes(id))).toBe(true);
      expect([...seen].some((id) => !id.startsWith("core-"))).toBe(true);
      // The no-Cry checks above bit: the Felinors whose Cries would show were among those summoned.
      expect(["core-043", "classicplus-030", "classicplus-046"].every((id) => seen.has(id))).toBe(true);
      expect(seen.has("core-062")).toBe(false); // Friend of Felinors, a Felinor Spell
      expect(seen.has("core-t-felinor")).toBe(false);
    });

    it("R64 skips a Locked zone and a zone reserved for a Reborn return", () => {
      const s = setup();
      lockZone(s.state, { player: "p1", row: "units", lane: 2 });
      reserveZone(s.state, { player: "p1", row: "units", lane: 4 });

      s.attack(s.unit("p2", 1)!, "hero");

      expect(summonedIds(s)).toHaveLength(3);
      expect(s.unit("p1", 2)).toBeNull();
      expect(s.unit("p1", 4)).toBeNull();
    });

    it("R129 a full board summons nothing and draws no random number; the trap is still consumed", () => {
      // Five Mr. Vanilla, no Taunt among them, so the attack on the hero is legal.
      const s = setup({ field: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] });
      const before = s.state.rngCursor;

      s.attack(s.unit("p2", 1)!, "hero");

      s.expectEvents("trapFired");
      expect(summonedIds(s)).toHaveLength(0);
      expect(s.state.rngCursor).toBe(before);
      s.expectInZone(GROOM, "graveyard");
      s.expectHealth("p1", 27);
    });

    it("never fires on an attack on a unit", () => {
      const s = setup({ field: [VANILLA] });

      s.attack(s.unit("p2", 1)!, s.unit("p1", 1)!);

      expect(s.events.filter((event) => event.type === "trapFired")).toHaveLength(0);
      expect(s.card(GROOM).zone).toMatchObject({ z: "field", row: "backrow", lane: 1 });
    });

    it("R33 R97 the opponent reads only a face-down card until it fires", () => {
      const s = setup();
      expect(JSON.stringify(s.view("p1").you.backrow)).toContain(GROOM);
      expect(JSON.stringify(s.view("p2"))).not.toContain(GROOM);

      s.attack(s.unit("p2", 1)!, "hero");

      expect(s.view("p2").opponent.graveyard.map((card) => card.defId)).toContain(GROOM);
    });
  });

  describe("radiant", () => {
    it("the Felinors are summoned on their Radiant faces, each granted Taunt", () => {
      const s = setup({}, true);

      s.attack(s.unit("p2", 1)!, "hero");

      const ids = summonedIds(s);
      expect(ids).toHaveLength(5);
      for (const id of ids) {
        expect(s.card(id).radiant).toBe(true);
        expect(s.stats(id).keywords.map((k) => k.kind)).toContain("Taunt");
      }
      s.expectHealth("p1", 27);
    });
  });
});
