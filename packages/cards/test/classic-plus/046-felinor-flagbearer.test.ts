// C+ #46 Felinor Flagbearer — SPEC §8.7 row 46, BUILD M9 Classic+ row C+ 46: "Rush, Cleave; Cry gives
// your hero +1 Armor for the rest of the game (`hero.armor`, stacking with every source, cutting each
// hit at §4.4 step 2, skipped by Pierce, kept after it leaves); Aura: your other Felinor Units have
// +1/+1 while it is on the field (not itself, not the opponent's); Death shuffles a Felinor
// Flagbearer Prime (C+ #46.1) into your deck (R80's cap), shown in your library list; Armor and aura
// read through `param()`; radiant +2 Armor and +2/+2 to every Felinor Unit you control, itself
// included". The verb is proved again in packages/engine/test/effects-perks.test.ts.

import { LIBRARY_CAP, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { def } from "../../src/scripts/classic-plus/046-felinor-flagbearer";

const FLAG = "classicplus-046";
const PRIME = "classicplus-046-1";
const DUPE = "core-012"; // Duplicating Felinors 3/4, a Felinor
const VANILLA = "core-008"; // Mr. Vanilla 4/4, no tag
const HIT_JOB = "core-016"; // (3) Spell: destroy target Unit
const TRUE_STRIKE = "core-044"; // (1) Spell: Pierce, deal 4 damage
const FILLER = "core-005";

function kinds(s: Scenario, ref: string): string[] {
  return s.stats(ref).keywords.map((keyword) => keyword.kind);
}

/** The Flagbearer in p1's hand, beside a Felinor and a non-Felinor of p1's and a Felinor of p2's. */
function rally(opts: { radiant?: boolean; armor?: number; library?: readonly string[] } = {}): Scenario {
  return scenario({
    p1: {
      hand: [{ def: FLAG, ...(opts.radiant === true ? { radiant: true } : {}) }, HIT_JOB, FILLER],
      field: [DUPE, VANILLA],
      library: opts.library ?? [FILLER],
      mana: 10,
      ...(opts.armor === undefined ? {} : { armor: opts.armor }),
    },
    p2: { hand: [TRUE_STRIKE, FILLER], field: [DUPE, VANILLA] },
  });
}

function destroyFlag(s: Scenario): void {
  s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(FLAG).id }] });
}

describe("C+ #46 Felinor Flagbearer", () => {
  it("is a (2) Felinor Unit, 4/4 Rush, Cleave (8/8 Radiant), naming its Prime", () => {
    expect(def.tags).toEqual(["Felinor"]);
    expect(def.refs).toContain(PRIME);
    const s = rally();
    s.play(FLAG);
    s.expectStats(FLAG, { attack: 4, health: 4 });
    expect(kinds(s, FLAG)).toEqual(["Rush", "Cleave"]);
  });

  describe("base", () => {
    it("§4.4 its Cry gives your hero +1 Armor, which the view carries", () => {
      const s = rally();
      s.play(FLAG);
      expect(s.state.players.p1.hero.armor).toBe(1);
      expect(s.view("p1").you.hero.armor).toBe(1);
      expect(s.view("p2").opponent.hero.armor).toBe(1);
      expect(s.state.players.p2.hero.armor).toBe(0);
    });

    it("R124 the Armor stacks with every source: hero Armor 3 becomes 4", () => {
      const s = rally({ armor: 3 });
      s.play(FLAG);
      expect(s.view("p1").you.hero.armor).toBe(4);
    });

    it("§4.4 step 2 it cuts each hit on your hero, and stays after the Flagbearer has left", () => {
      const s = rally();
      s.play(FLAG);
      destroyFlag(s);
      s.expectInZone(FLAG, "graveyard");
      s.endTurn();
      s.attack(s.unit("p2", 2) ?? "", "hero");
      s.expectHealth("p1", 27);
      expect(s.view("p1").you.hero.armor).toBe(1);
    });

    it("§4.4 R346 a Pierce hit skips it (True Strike deals its full 4)", () => {
      const s = rally();
      s.play(FLAG);
      s.endTurn();
      s.play(TRUE_STRIKE, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", 26);
    });

    it("§10.4 its aura gives your other Felinors +1/+1 — not itself, not a non-Felinor, not the opponent's", () => {
      const s = rally();
      s.play(FLAG);
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 4, health: 5 });
      s.expectStats(FLAG, { attack: 4, health: 4 });
      s.expectStats(s.unit("p1", 2) ?? "", { attack: 4, health: 4 });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 3, health: 4 });
    });

    it("§10.4 the aura lasts only while it is on the field", () => {
      const s = rally();
      s.play(FLAG);
      destroyFlag(s);
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 3, health: 4 });
    });

    it("§6.3 its Death shuffles a Felinor Flagbearer Prime into your deck, shown in your library list (R311)", () => {
      const s = rally();
      s.play(FLAG);
      destroyFlag(s);
      const prime = s.pile("p1", "library").filter((card) => card.defId === PRIME);
      expect(prime).toHaveLength(1);
      expect(prime[0]?.radiant).toBe(false);
      expect(s.view("p1").you.ownLibrary?.cards).toEqual(expect.arrayContaining([{ defId: PRIME, radiant: false, count: 1 }]));
      expect(JSON.stringify(s.view("p2"))).not.toContain(prime[0]?.id ?? "?");
    });

    it("R80 a full deck turns the Prime away", () => {
      const s = rally({ library: Array.from({ length: LIBRARY_CAP }, () => FILLER) });
      s.play(FLAG);
      destroyFlag(s);
      expect(s.pile("p1", "library").some((card) => card.defId === PRIME)).toBe(false);
      expect(s.pile("p1", "library")).toHaveLength(LIBRARY_CAP);
    });

    it("R386 an Upgrade makes the Armor 2 and the aura +2/+2", () => {
      const s = rally();
      stepParam(s.card(FLAG), "armor", 1);
      stepParam(s.card(FLAG), "aura", 1);
      s.play(FLAG);
      expect(s.view("p1").you.hero.armor).toBe(2);
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 5, health: 6 });
    });
  });

  describe("radiant", () => {
    it("§5.2 8/8 Rush, Cleave; its Cry gives your hero +2 Armor", () => {
      const s = rally({ radiant: true });
      s.play(FLAG);
      expect(s.view("p1").you.hero.armor).toBe(2);
    });

    it("§10.4 its aura gives every Felinor Unit you control +2/+2, itself included", () => {
      const s = rally({ radiant: true });
      s.play(FLAG);
      s.expectStats(FLAG, { attack: 10, health: 10 });
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 5, health: 6 });
      s.expectStats(s.unit("p1", 2) ?? "", { attack: 4, health: 4 });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 3, health: 4 });
    });

    it("§6.3 its Death shuffles in a base Prime", () => {
      const s = rally({ radiant: true });
      s.play(FLAG);
      destroyFlag(s);
      const prime = s.pile("p1", "library").find((card) => card.defId === PRIME);
      expect(prime?.radiant).toBe(false);
    });

    it("R386 the Radiant numbers step from 2", () => {
      const s = rally({ radiant: true });
      stepParam(s.card(FLAG), "armor", -1);
      stepParam(s.card(FLAG), "aura", 1);
      s.play(FLAG);
      expect(s.view("p1").you.hero.armor).toBe(1);
      s.expectStats(FLAG, { attack: 11, health: 11 });
    });
  });
});
