// C #88 Siphon Squad — SPEC §8.6 row 88, BUILD M9 Classic row C 88: "Live while face-down (R403): its aura
// works from the moment it is set and it stays face-down until something reveals it; enemy Units have −X
// Attack, X twice the number of Units the opponent controls, recomputed on every change and floored at 0;
// whenever the opponent controls no Units, at any state check including the one right after it is set,
// it Tributes itself; the opponent's view shows their attack drop and never names the card (R33); its
// preview is X, for its controller only while it is face-down and for both players once it is face-up
// (R280, §10.8); radiant: enemy Units have 0 Attack, set after every other layer (§10.4), so their hits
// are no hits (R63); its tuned number (multiplier) reads through `param()` (R386)".
//
// The preview's proofs are in `test/preview.test.ts`.

import { legalActions, stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/088-siphon-squad";

const SIPHON = "classic-088";
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const TIMMY = "core-011"; // (1) Unit 3/3 Rush, First Strike.
const WEAPONS = "core-014"; // (4) Field Spell: Aura: your Units have +4 attack, Rush and First Strike.
const FIENDER = "core-092"; // (2) Unit 5/7 Stack.
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const ANCHOR = "core-010"; // (0) Spell (§2.5).

function attackOf(s: Scenario, player: "p1" | "p2", lane: number): number {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return s.stats(unit).attack;
}

/** p1 sets Siphon Squad against p2's board. */
function setAgainst(enemies: readonly string[], radiantFace = false, mine: readonly string[] = []): Scenario {
  const s = scenario({
    p1: { hand: [{ def: SIPHON, radiant: radiantFace }, ANCHOR, HIT_JOB], field: [...mine], mana: 10 },
    p2: { hand: [ANCHOR], field: enemies.map((enemy, at) => ({ def: enemy, lane: at + 1 })) },
  });
  s.play(SIPHON);
  return s;
}

describe("C #88 Siphon Squad", () => {
  it("is a Field Trap with an aura, a self-Tribute condition and one number; the base face previews X", () => {
    expect(def.id).toBe(SIPHON);
    expect(def.type).toBe("Field Trap");
    expect(def.params).toEqual([{ key: "multiplier", base: 2, radiant: 2, better: "up", step: 1, min: 1 }]);
    for (const script of [base, radiant]) {
      expect(script.aura).toBeTypeOf("function");
      expect(script.tributeWhen).toBeTypeOf("function");
    }
  });

  describe("base", () => {
    it("R403 live while face-down: set, it stays face-down and its aura works at once, −2 × 2 on each enemy Unit", () => {
      const s = setAgainst([VANILLA, MENACE]);
      const siphon = s.card(SIPHON);

      expect(s.card(siphon).faceUp).not.toBe(true);
      expect(s.events.some((event) => event.type === "trapFired")).toBe(false);
      expect(attackOf(s, "p2", 1)).toBe(0);
      expect(attackOf(s, "p2", 2)).toBe(5);
    });

    it("its own Units are untouched", () => {
      const s = setAgainst([VANILLA], false, [MENACE]);
      expect(attackOf(s, "p1", 1)).toBe(9);
      expect(attackOf(s, "p2", 1)).toBe(2);
    });

    it("X is recomputed on every change: one enemy Unit fewer, X drops from 4 to 2", () => {
      const s = setAgainst([VANILLA, MENACE]);
      expect(attackOf(s, "p2", 2)).toBe(5);

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(VANILLA).id }] });

      expect(attackOf(s, "p2", 2)).toBe(7);
    });

    it("§10.4 attack floors at 0: three enemy Units, X = 6, a 3-attack Unit reads 0", () => {
      const s = setAgainst([TIMMY, VANILLA, MENACE]);
      expect(attackOf(s, "p2", 1)).toBe(0);
      expect(attackOf(s, "p2", 3)).toBe(3);
    });

    it("R403 right after it is set, with the opponent holding no Units, it Tributes itself", () => {
      const s = setAgainst([]);
      const siphon = s.card(SIPHON);

      s.expectInZone(siphon, "graveyard");
      expect(s.events.some((event) => event.type === "destroyed" && event.instanceId === siphon.id)).toBe(true);
    });

    it("R403 whenever the opponent's last Unit leaves, the next state check Tributes it", () => {
      const s = setAgainst([VANILLA]);
      const siphon = s.card(SIPHON);
      s.expectInZone(siphon, "field");

      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(VANILLA).id }] });

      s.expectInZone(siphon, "graveyard");
    });

    it("R33 the opponent's view shows their attack drop and never names the card", () => {
      const s = setAgainst([VANILLA, MENACE]);
      const theirs = s.view("p2");

      expect(JSON.stringify(theirs)).not.toContain(SIPHON);
      expect(JSON.stringify(theirs)).not.toContain("Siphon Squad");
      const menace = theirs.you.units[1];
      expect(menace).toMatchObject({ defId: MENACE, attack: 5 });
    });

    it("§3.2 R13 a Stack pile is one Unit: only its top counts toward X and takes the −X", () => {
      const s = scenario({
        p1: { hand: [SIPHON, ANCHOR] },
        p2: { hand: [ANCHOR], field: [VANILLA, { def: FIENDER, stack: true }] },
      });
      s.play(SIPHON);

      expect(attackOf(s, "p2", 1)).toBe(5 - 2);
    });

    it("its own controller's Units never count toward X", () => {
      const s = setAgainst([MENACE], false, [VANILLA, TIMMY]);
      expect(attackOf(s, "p2", 1)).toBe(7);
    });

    it("R386 an Upgrade makes it 3× the count", () => {
      const s = scenario({ p1: { hand: [SIPHON, ANCHOR] }, p2: { hand: [ANCHOR], field: [MENACE, { def: VANILLA, lane: 2 }] } });
      stepParam(s.card(SIPHON), "multiplier", 1);
      s.play(SIPHON);

      expect(attackOf(s, "p2", 1)).toBe(3);
    });
  });

  describe("radiant", () => {
    it("R403 enemy Units have 0 Attack from the moment it is set, face-down", () => {
      const s = setAgainst([VANILLA, MENACE], true);

      expect(s.card(SIPHON).faceUp).not.toBe(true);
      expect(attackOf(s, "p2", 1)).toBe(0);
      expect(attackOf(s, "p2", 2)).toBe(0);
    });

    it("§10.4 the 0 is set after every other layer: a buffed Unit under a +4 attack aura still reads 0", () => {
      const s = scenario({
        p1: { hand: [{ def: SIPHON, radiant: true }, ANCHOR] },
        p2: { hand: [ANCHOR], field: [MENACE], backrow: [WEAPONS] },
      });
      s.card(MENACE).buffs.attack += 5;
      expect(attackOf(s, "p2", 1)).toBe(9 + 5 + 4);

      s.play(SIPHON);

      expect(attackOf(s, "p2", 1)).toBe(0);
    });

    it("R63 their hits are no hits: a 9/9's strike back deals nothing, and a 0-attack Unit can't attack at all", () => {
      const s = setAgainst([MENACE], true, [VANILLA]);
      const mine = s.card(VANILLA);

      s.attack(mine, s.card(MENACE));

      expect(s.lastEvents.filter((event) => event.type === "damage" && event.targetId === mine.id)).toEqual([]);
      s.expectStats(mine, { health: 4 });
      s.endTurn();
      expect(legalActions(s.state, "p2").some((action) => action.type === "attack")).toBe(false);
      expect(() => s.attack(s.card(MENACE), "hero")).toThrow(/0 attack/);
    });

    it("R403 it Tributes itself when the opponent controls no Units", () => {
      const empty = setAgainst([], true);
      empty.expectInZone(SIPHON, "graveyard");

      const s = setAgainst([VANILLA], true);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(VANILLA).id }] });
      s.expectInZone(SIPHON, "graveyard");
    });

    it("its own Units keep their attack", () => {
      const s = setAgainst([VANILLA], true, [MENACE]);
      expect(attackOf(s, "p1", 1)).toBe(9);
    });

    it("R386 the multiplier changes nothing on the Radiant face", () => {
      const s = scenario({ p1: { hand: [{ def: SIPHON, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [MENACE] } });
      stepParam(s.card(SIPHON), "multiplier", 1);
      s.play(SIPHON);

      expect(attackOf(s, "p2", 1)).toBe(0);
    });
  });
});
