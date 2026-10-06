// C+ #46.1 Felinor Flagbearer Prime — SPEC §8.7 row 46.1, BUILD M9 Classic+ row C+ 46.1: "Rush; Cry
// fills every empty, unreserved unit zone of yours with copies of itself (R688: a Locked zone takes
// one; R57: face, buffs and
// granted keywords, not damage), none firing a Cry (R1), so nothing loops; each copy's aura gives your
// other Felinors +1/+1, so each of n Primes has +(n − 1)/+(n − 1) from the rest; a full board makes
// none; the copies are Tokens and cease to exist when they leave (R11); the aura reads through
// `param()`; radiant +2/+2 and the copies are Radiant".

import { lockZone, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/046-1-felinor-flagbearer-prime";

const PRIME = "classicplus-046-1";
const VANILLA = "core-008"; // Mr. Vanilla 4/4, no tag
const DUPE = "core-012"; // Duplicating Felinors 3/4, a Felinor
const FLOOD = "core-017"; // (4) Spell: bounce all Units
const FILLER = "core-005";

function primes(s: Scenario): string[] {
  const out: string[] = [];
  for (let lane = 1; lane <= 5; lane += 1) {
    const unit = s.unit("p1", lane);
    if (unit?.defId === PRIME) out.push(unit.id);
  }
  return out;
}

function muster(opts: { radiant?: boolean; field?: readonly (string | { def: string; lane: number })[] } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: PRIME, ...(opts.radiant === true ? { radiant: true } : {}) }, FLOOD, FILLER], field: opts.field ?? [], mana: 10 },
    p2: { hand: [FILLER], field: [DUPE] },
  });
}

describe("C+ #46.1 Felinor Flagbearer Prime", () => {
  it("is a (2) Felinor unit-token card (printed Legendary), 5/5 Rush; both faces run one script", () => {
    expect(def.token).toBe(true);
    expect(def.tags).toEqual(["Felinor", "Token"]);
    expect(def.printedRarity).toBe("Legendary");
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R64 its Cry fills every empty unit zone of yours, left to right, with copies of itself", () => {
      const s = muster({ field: [{ def: VANILLA, lane: 2 }] });
      s.play(PRIME, { zone: 4 });
      const lanes = [1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId);
      expect(lanes).toEqual([PRIME, VANILLA, PRIME, PRIME, PRIME]);
      const summons = s.lastEvents.filter((event) => event.type === "summoned").map((event) => (event.type === "summoned" ? event.lane : 0));
      expect(summons).toEqual([4, 1, 3, 5]); // the played Prime, then its copies
    });

    it("R1 no copy fires a Cry, so nothing loops: one Cry, four copies", () => {
      const s = muster();
      s.play(PRIME);
      expect(primes(s)).toHaveLength(5);
      expect(s.lastEvents.filter((event) => event.type === "cardResolved")).toHaveLength(1);
    });

    it("§10.4 each of n Primes has +(n − 1)/+(n − 1) from the rest; other Felinors get +n", () => {
      const s = muster({ field: [{ def: DUPE, lane: 5 }] });
      s.play(PRIME);
      const ids = primes(s);
      expect(ids).toHaveLength(4);
      for (const id of ids) s.expectStats(id, { attack: 5 + 3, health: 5 + 3 });
      s.expectStats(s.unit("p1", 5) ?? "", { attack: 3 + 4, health: 4 + 4 });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 3, health: 4 });
    });

    it("R688 a Locked zone takes a copy; a reserved zone is still skipped", () => {
      const s = muster();
      lockZone(s.state, { player: "p1", row: "units", lane: 3 });
      s.state.reserved.push({ player: "p1", row: "units", lane: 4 });
      s.play(PRIME);
      expect([1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId ?? null)).toEqual([PRIME, PRIME, PRIME, null, PRIME]);
    });

    it("§3.2 a full board makes no copy", () => {
      const s = muster({ field: [VANILLA, VANILLA, VANILLA, VANILLA] });
      s.play(PRIME);
      expect(primes(s)).toHaveLength(1);
      expect(s.lastEvents.filter((event) => event.type === "summoned")).toHaveLength(1);
    });

    it("R57 the copies keep its face, buffs and granted keywords, and are summoning sick with Rush", () => {
      const s = muster();
      const card = s.card(PRIME);
      card.buffs = { attack: 2, health: 1 };
      card.grantedKeywords.push({ kind: "Taunt" });
      s.play(PRIME);
      const ids = primes(s);
      expect(ids).toHaveLength(5);
      for (const id of ids) {
        expect(s.card(id).buffs).toEqual({ attack: 2, health: 1 });
        expect(s.stats(id).keywords.map((keyword) => keyword.kind)).toEqual(expect.arrayContaining(["Rush", "Taunt"]));
        s.expectStats(id, { attack: 5 + 2 + 4, health: 5 + 1 + 4 });
      }
    });

    it("R11 the copies are Tokens: bounced, they cease to exist", () => {
      const s = muster();
      s.play(PRIME);
      const ids = primes(s);
      expect(ids).toHaveLength(5);
      s.play(FLOOD);
      for (const id of ids) s.expectInZone(id, "gone");
      expect(s.hand("p1").some((card) => card.defId === PRIME)).toBe(false);
    });

    it("R387 copies are not generation: every copy is a Prime", () => {
      const s = muster();
      s.play(PRIME);
      expect(primes(s)).toHaveLength(5);
    });

    it("R386 an Upgrade makes each aura +2/+2, and the copies keep it", () => {
      const s = muster();
      stepParam(s.card(PRIME), "aura", 1);
      s.play(PRIME, { zone: 1 });
      expect(primes(s)).toHaveLength(5);
      for (const id of primes(s)) s.expectStats(id, { attack: 5 + 8, health: 5 + 8 });
    });
  });

  describe("radiant", () => {
    it("§5.2 10/10 Rush; the copies are Radiant and each aura gives +2/+2", () => {
      const s = muster({ radiant: true, field: [VANILLA, VANILLA, VANILLA] });
      s.play(PRIME);
      const ids = primes(s);
      expect(ids).toHaveLength(2);
      for (const id of ids) {
        expect(s.card(id).radiant).toBe(true);
        s.expectStats(id, { attack: 12, health: 12 });
      }
    });

    it("§10.4 a full Radiant board: each of five has +8/+8", () => {
      const s = muster({ radiant: true });
      s.play(PRIME);
      expect(primes(s)).toHaveLength(5);
      for (const id of primes(s)) s.expectStats(id, { attack: 18, health: 18 });
    });
  });
});
