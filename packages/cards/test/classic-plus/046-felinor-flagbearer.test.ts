// C+ #46 Felinor Flagbearer — SPEC §8.7 row 46, BUILD M9 Classic+ row C+ 46: "Rush (no Cleave, no Cry,
// balance patch 1); Aura: your other Felinor Units have +1/+1 while it is on the field (not itself, not
// the opponent's); Death shuffles a Felinor Flagbearer Prime (C+ #46.1) into your deck (R80's cap),
// shown in your library list; the aura reads through `param()`; radiant +2/+2 to every Felinor Unit
// you control, itself included".

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
  it("is a (2) Felinor Catalyst Unit, 4/4 Rush (8/8 Radiant), no Cleave, naming its Prime", () => {
    expect(def.tags).toEqual(["Felinor", "Catalyst"]);
    expect(def.refs).toContain(PRIME);
    const s = rally();
    s.play(FLAG);
    s.expectStats(FLAG, { attack: 4, health: 4 });
    expect(kinds(s, FLAG)).toEqual(["Rush"]);
  });

  describe("base", () => {
    it("no Cry: playing grants no Armor to either hero, in any view", () => {
      const s = rally();
      s.play(FLAG);
      expect(s.state.players.p1.hero.armor).toBe(0);
      expect(s.view("p1").you.hero.armor).toBe(0);
      expect(s.view("p2").opponent.hero.armor).toBe(0);
      expect(s.state.players.p2.hero.armor).toBe(0);
    });

    it("R124 hero Armor 3 stays 3: there is no Cry to stack with it", () => {
      const s = rally({ armor: 3 });
      s.play(FLAG);
      expect(s.view("p1").you.hero.armor).toBe(3);
    });

    it("§4.4 step 2 with no Armor from it, a 4-attack hit deals its full 4", () => {
      const s = rally();
      s.play(FLAG);
      destroyFlag(s);
      s.expectInZone(FLAG, "graveyard");
      s.endTurn();
      s.attack(s.unit("p2", 2) ?? "", "hero");
      s.expectHealth("p1", 26);
      expect(s.view("p1").you.hero.armor).toBe(0);
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

    it("§3.2 R13 a Felinor dormant under a Stack pile gets no aura; the one on top does", () => {
      const s = scenario({
        p1: { hand: [FLAG, FILLER], field: [DUPE, { def: DUPE, stack: true }], mana: 10 },
        p2: { hand: [FILLER] },
      });
      const top = s.unit("p1", 1);
      const buried = s.state.players.p1.units[0]?.find((card) => card.id !== top?.id);
      if (top === null || buried === undefined) throw new Error("no pile");
      s.play(FLAG);
      s.expectStats(top, { attack: 4, health: 5 });
      s.expectStats(buried, { attack: 3, health: 4 });
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

    it("R386 an Upgrade makes the aura +2/+2 (no Armor left to tune)", () => {
      const s = rally();
      stepParam(s.card(FLAG), "aura", 1);
      s.play(FLAG);
      expect(s.view("p1").you.hero.armor).toBe(0);
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 5, health: 6 });
    });
  });

  describe("radiant", () => {
    it("§5.2 8/8 Rush, no Cleave; no Cry Armor on the Radiant face either", () => {
      const s = rally({ radiant: true });
      s.play(FLAG);
      expect(kinds(s, FLAG)).toEqual(["Rush"]);
      expect(s.view("p1").you.hero.armor).toBe(0);
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

    it("R386 the Radiant aura steps from 2", () => {
      const s = rally({ radiant: true });
      stepParam(s.card(FLAG), "aura", 1);
      s.play(FLAG);
      expect(s.view("p1").you.hero.armor).toBe(0);
      s.expectStats(FLAG, { attack: 11, health: 11 });
    });
  });
});
