// C+ #65.4 Golden Grape — SPEC §8.7 row 65.4, BUILD M9 Classic+ row C+ 65.4: "A card chosen with the play
// (R81), in your hand or a permanent you control (face-down included), becomes Radiant, no change if it
// already is (§5.2); a hand card's or face-down card's change is reported to the opponent by a redacted
// `radiantSet` whether or not the flag changed (R177); radiant also the cards next to it: its row
// neighbours on your side (lanes N−1, N+1), or in your hand the cards at the neighbouring indices".

import { legalActions } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/065-4-golden-grape";

const GOLDEN = "classicplus-065-4";
const FILLER = "core-005";
const TIMMY = "core-011"; // Unit 3/3 → 6/6
const DFENDER = "core-001"; // Unit 0/7 → 0/14
const FELINORS = "core-012"; // Unit 3/4 → 6/9
const SHEEPISH = "core-041"; // Trap

function pick(s: Scenario, ref: string | ReturnType<Scenario["card"]>): Selection[] {
  return [{ pick: "instance", instanceId: s.card(ref).id }];
}

function radiantSets(s: Scenario, viewer: "p1" | "p2"): { instanceId: string }[] {
  return s.view(viewer).events.flatMap((event) => (event.type === "radiantSet" ? [{ instanceId: event.instanceId }] : []));
}

const RADIANT = { def: GOLDEN, radiant: true };

describe("C+ #65.4 Golden Grape", () => {
  it("is a (1) Fruit Spell token (printed Legendary) whose pick is your hand or your side of the field", () => {
    expect(def.id).toBe(GOLDEN);
    expect(def.cost).toBe(1);
    expect(def.printedRarity).toBe("Legendary");
    const decl = [{ kind: "target", min: 1, max: 1, filter: { side: "ally", of: ["unit", "backrow", "hand"] } }];
    expect(base.targets).toEqual(decl);
    expect(radiant.targets).toEqual(decl);
  });

  describe("base", () => {
    it("§5.2 a card in your hand becomes Radiant", () => {
      const s = scenario({ p1: { hand: [GOLDEN, TIMMY] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, TIMMY) });
      expect(s.card(TIMMY).radiant).toBe(true);
      expect(s.card(TIMMY).zone.z).toBe("hand");
    });

    it("§5.2 R22 a Unit of yours converts in place: Radiant stats at once, damage kept, no Cry", () => {
      const s = scenario({ p1: { hand: [GOLDEN, FILLER], field: [{ def: FELINORS, damage: 2 }] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, FELINORS) });
      s.expectStats(FELINORS, { attack: 6, maxHealth: 9, health: 7 });
      expect(s.lastEvents.filter((event) => event.type === "summoned")).toEqual([]);
    });

    it("R33 a face-down trap of yours is a legal pick and becomes Radiant face-down", () => {
      const s = scenario({ p1: { hand: [GOLDEN, FILLER], backrow: [SHEEPISH] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, SHEEPISH) });
      expect(s.card(SHEEPISH).radiant).toBe(true);
      expect(s.card(SHEEPISH).faceUp).not.toBe(true);
    });

    it("R81 the opponent's cards are never offered, and a play naming one is refused", () => {
      const s = scenario({ p1: { hand: [GOLDEN, FILLER] }, p2: { hand: [FILLER], field: [TIMMY] } });
      const offered = JSON.stringify(legalActions(s.state, "p1"));
      expect(offered).not.toContain(s.card(TIMMY).id);
      expect(() => s.play(GOLDEN, { targets: pick(s, TIMMY) })).toThrow();
    });

    it("§6.1 Immutable never blocks it: Radiant is the card's own text", () => {
      const s = scenario({ p1: { hand: [GOLDEN, FILLER], field: [FELINORS] }, p2: { hand: [FILLER] } });
      s.card(FELINORS).grantedKeywords = [{ kind: "Immutable" }];
      s.play(GOLDEN, { targets: pick(s, FELINORS) });
      expect(s.card(FELINORS).radiant).toBe(true);
      s.expectStats(FELINORS, { attack: 6, maxHealth: 9 });
    });

    it("§6.3 a card that is already Radiant is left as it is", () => {
      const s = scenario({ p1: { hand: [GOLDEN], field: [{ def: TIMMY, radiant: true }] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, TIMMY) });
      expect(s.card(TIMMY).radiant).toBe(true);
      expect(s.lastEvents.filter((event) => event.type === "radiantSet")).toEqual([]);
    });

    it("R177 a hand card's change reaches the opponent redacted — and so does a no-change on a Radiant one", () => {
      const fresh = scenario({ p1: { hand: [GOLDEN, TIMMY] }, p2: { hand: [FILLER] } });
      fresh.play(GOLDEN, { targets: pick(fresh, TIMMY) });
      expect(radiantSets(fresh, "p2")).toEqual([{ instanceId: "hidden" }]);
      expect(radiantSets(fresh, "p1")).toEqual([{ instanceId: fresh.card(TIMMY).id }]);

      const already = scenario({ p1: { hand: [GOLDEN, { def: TIMMY, radiant: true }] }, p2: { hand: [FILLER] } });
      already.play(GOLDEN, { targets: pick(already, TIMMY) });
      expect(radiantSets(already, "p2")).toEqual([{ instanceId: "hidden" }]);
    });

    it("R177 R33 a face-down card's change reaches the opponent redacted", () => {
      const s = scenario({ p1: { hand: [GOLDEN, FILLER], backrow: [{ def: SHEEPISH, radiant: true }] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, SHEEPISH) });
      expect(radiantSets(s, "p2")).toEqual([{ instanceId: "hidden" }]);
      expect(JSON.stringify(s.view("p2"))).not.toContain(SHEEPISH);
    });

    it("the base face changes only the pick: its neighbours stay as they are", () => {
      const s = scenario({ p1: { hand: [GOLDEN, FILLER], field: [DFENDER, TIMMY, FELINORS] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, TIMMY) });
      expect([s.card(DFENDER).radiant, s.card(TIMMY).radiant, s.card(FELINORS).radiant]).toEqual([false, true, false]);
    });
  });

  describe("radiant", () => {
    it("§3.1 a Unit and its row neighbours on your side (lanes N−1 and N+1) become Radiant", () => {
      const s = scenario({
        p1: { hand: [RADIANT, FILLER], field: [{ def: DFENDER, lane: 1 }, { def: TIMMY, lane: 2 }, { def: FELINORS, lane: 3 }, { def: "core-019", lane: 4 }] },
        p2: { hand: [FILLER], field: [{ def: "core-043", lane: 2 }] },
      });
      s.play(GOLDEN, { targets: pick(s, TIMMY) });
      expect([s.card(DFENDER).radiant, s.card(TIMMY).radiant, s.card(FELINORS).radiant]).toEqual([true, true, true]);
      expect(s.card("core-019").radiant).toBe(false);
      expect(s.card("core-043").radiant).toBe(false);
    });

    it("§3.1 an empty neighbouring zone is nothing; the edge lane has one neighbour", () => {
      const s = scenario({ p1: { hand: [RADIANT, FILLER], field: [{ def: TIMMY, lane: 1 }, { def: FELINORS, lane: 3 }] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, TIMMY) });
      expect(s.card(TIMMY).radiant).toBe(true);
      expect(s.card(FELINORS).radiant).toBe(false);
    });

    it("§3.1 a backrow pick's neighbours are its backrow neighbours, not the Units in front", () => {
      const s = scenario({
        p1: { hand: [RADIANT, FILLER], field: [{ def: TIMMY, lane: 2 }], backrow: [{ def: SHEEPISH, lane: 2 }, { def: "core-060", lane: 3 }] },
        p2: { hand: [FILLER] },
      });
      s.play(GOLDEN, { targets: pick(s, SHEEPISH) });
      expect(s.card(SHEEPISH).radiant).toBe(true);
      expect(s.card("core-060").radiant).toBe(true);
      expect(s.card(TIMMY).radiant).toBe(false);
    });

    it("R13 a card dormant under a Stack pile is no neighbour: only the top of the pile beside it", () => {
      const s = scenario({
        p1: { hand: [RADIANT, FILLER], field: [{ def: TIMMY, lane: 1 }, { def: DFENDER, lane: 2 }, { def: FELINORS, lane: 2, stack: true }] },
        p2: { hand: [FILLER] },
      });
      s.play(GOLDEN, { targets: pick(s, TIMMY) });
      expect(s.card(FELINORS).radiant).toBe(true);
      expect(s.card(DFENDER).radiant).toBe(false);
    });

    it("in your hand the cards at the neighbouring indices become Radiant, and no others", () => {
      const s = scenario({ p1: { hand: [RADIANT, DFENDER, TIMMY, FELINORS, "core-019"] }, p2: { hand: [FILLER] } });
      // The Grape leaves the hand as it is played, so the hand reads D-fender, Timmy, Felinors, Menace.
      s.play(GOLDEN, { targets: pick(s, TIMMY) });
      expect([s.card(DFENDER).radiant, s.card(TIMMY).radiant, s.card(FELINORS).radiant, s.card("core-019").radiant]).toEqual([
        true,
        true,
        true,
        false,
      ]);
    });

    it("the first hand card has one neighbour", () => {
      const s = scenario({ p1: { hand: [RADIANT, DFENDER, TIMMY, FELINORS] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, DFENDER) });
      expect([s.card(DFENDER).radiant, s.card(TIMMY).radiant, s.card(FELINORS).radiant]).toEqual([true, true, false]);
    });

    it("R177 every hand change reaches the opponent redacted", () => {
      const s = scenario({ p1: { hand: [RADIANT, DFENDER, TIMMY, FELINORS] }, p2: { hand: [FILLER] } });
      s.play(GOLDEN, { targets: pick(s, TIMMY) });
      expect(radiantSets(s, "p2")).toEqual([{ instanceId: "hidden" }, { instanceId: "hidden" }, { instanceId: "hidden" }]);
    });
  });
});
