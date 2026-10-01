// C+ #28 Nuestro hogar, nuestras tumbas — SPEC §8.7 row 28, BUILD M9 Classic+ row C+ 28: "Taunt,
// Reborn; Death heals your hero 3 on both deaths (R8), past 30 allowed; exile or a bounce heals
// nothing; the heal reads through `param()`; radiant Divine Shield too, heal 8".

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/028-nuestro-hogar-nuestras-tumbas";

const HOGAR = "classicplus-028";
/** #19 Midrange Menace, 9/9 Taunt: an attacker that kills the 3/4 (and the 6/8) in one hit. */
const MENACE = "core-019";
/** #17 Flood: "Bounce all Units". */
const FLOOD = "core-017";
/** #100 Ceaseless Void: "Cry: Exile all other permanents" (played here at a `costOverride` of 0). */
const VOID = "core-100";
/** A card that keeps a hand from auto-ending the turn (§2.5). */
const FILLER = "core-005";

function keywords(s: Scenario, ref: string): string[] {
  return s.stats(ref).keywords.map((keyword) => keyword.kind);
}

function healed(s: Scenario): number[] {
  return s.events.flatMap((event) => (event.type === "healed" ? [event.amount] : []));
}

/** p1's two Menaces attack p2's #28 twice: its first death and its Reborn body's. */
function killTwice(radiantFace: boolean, health = 20): Scenario {
  const s = scenario({
    p1: { hand: [FILLER], field: [MENACE, MENACE] },
    p2: { hand: [FILLER], field: [{ def: HOGAR, radiant: radiantFace }], health },
  });
  const [first, second] = [s.unit("p1", 1), s.unit("p1", 2)];
  if (first === null || second === null) throw new Error("two Menaces");
  s.attack(first, HOGAR);
  if (radiantFace) s.attack(second, HOGAR); // the Divine Shield takes the first hit
  return s;
}

describe("C+ #28 Nuestro hogar, nuestras tumbas", () => {
  it("is a 3/4 Taunt, Reborn whose two faces run one script", () => {
    expect(def.id).toBe(HOGAR);
    expect(radiant).toBe(base);
    const s = scenario({ p1: { field: [HOGAR] } });
    s.expectStats(HOGAR, { attack: 3, health: 4, maxHealth: 4 });
    expect(keywords(s, HOGAR).sort()).toEqual(["Reborn", "Taunt"]);
  });

  describe("base", () => {
    it("R8 its Death heals your hero 3 on the first death, which Reborn answers", () => {
      const s = killTwice(false);
      s.expectHealth("p2", 23);
      expect(healed(s)).toEqual([3]);
      // Reborn brought it back at 1 health (§6.1).
      s.expectStats(s.unit("p2", 1) ?? HOGAR, { health: 1 });
    });

    it("R8 and heals 3 again when the Reborn body dies", () => {
      const s = killTwice(false);
      const second = s.unit("p1", 2);
      if (second === null) throw new Error("a second Menace");
      s.attack(second, s.unit("p2", 1) ?? HOGAR);
      s.expectHealth("p2", 26);
      expect(healed(s)).toEqual([3, 3]);
      s.expectInZone(HOGAR, "graveyard");
    });

    it("R19 heals your hero past 30", () => {
      const s = killTwice(false, 30);
      s.expectHealth("p2", 33);
    });

    it("§4.5 a bounce is no death: Flood returns it and nothing is healed", () => {
      const s = scenario({ p1: { hand: [FLOOD, FILLER], mana: 5 }, p2: { hand: [FILLER], field: [HOGAR], health: 20 } });
      s.play(FLOOD);
      s.expectInZone(HOGAR, "hand");
      s.expectHealth("p2", 20);
      expect(healed(s)).toEqual([]);
    });

    it("§4.5 an exile is no death: Ceaseless Void exiles it and nothing is healed", () => {
      const s = scenario({
        p1: { hand: [{ def: VOID, costOverride: 0 }, FILLER] },
        p2: { hand: [FILLER], field: [HOGAR], health: 20 },
      });
      s.play(VOID);
      s.expectInZone(HOGAR, "exile");
      s.expectHealth("p2", 20);
      expect(healed(s)).toEqual([]);
    });

    it("R386 an Upgrade moves heal by 1: it heals 4", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [MENACE] },
        p2: { hand: [FILLER], field: [HOGAR], health: 20 },
      });
      stepParam(s.card(HOGAR), "heal", 1);
      s.attack(s.unit("p1", 1) ?? MENACE, HOGAR);
      s.expectHealth("p2", 24);
    });

    it("R386 a Degrade moves it the other way: it heals 2", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [MENACE] },
        p2: { hand: [FILLER], field: [HOGAR], health: 20 },
      });
      stepParam(s.card(HOGAR), "heal", -1);
      s.attack(s.unit("p1", 1) ?? MENACE, HOGAR);
      s.expectHealth("p2", 22);
    });
  });

  describe("radiant", () => {
    it("is a 6/8 with Taunt, Reborn and Divine Shield", () => {
      const s = scenario({ p1: { field: [{ def: HOGAR, radiant: true }] } });
      s.expectStats(HOGAR, { attack: 6, health: 8, maxHealth: 8 });
      expect(keywords(s, HOGAR).sort()).toEqual(["Divine Shield", "Reborn", "Taunt"]);
    });

    it("R8 the Divine Shield takes the first hit; its Death heals your hero 8", () => {
      const s = killTwice(true);
      s.expectHealth("p2", 28);
      expect(healed(s)).toEqual([8]);
    });

    it("R386 an Upgrade steps from 8: it heals 9", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [MENACE, MENACE] },
        p2: { hand: [FILLER], field: [{ def: HOGAR, radiant: true }], health: 20 },
      });
      stepParam(s.card(HOGAR), "heal", 1);
      s.attack(s.unit("p1", 1) ?? MENACE, HOGAR);
      s.attack(s.unit("p1", 2) ?? MENACE, HOGAR);
      s.expectHealth("p2", 29);
    });
  });
});
