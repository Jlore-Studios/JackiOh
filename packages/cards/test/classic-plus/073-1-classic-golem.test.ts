// C+ #73.1 Classic Golem — SPEC §8.7 row 73.1, BUILD M9 Classic+ row C+ 73.1: "Rush, First Strike,
// Trample; after the combat of an attack it declared on a Unit, if it survived, it is transformed (§6.3)
// into a random non-token Unit of Classic or Classic+ (never Core; a Unit, R35), its own stats having
// fought and its Trample excess landed first (R424); the new Unit fires no Cry (R1); if the Golem
// destroyed the defender (R42) the new Unit has a fresh exertion and no summoning sickness this turn, so
// it may attack again; a Golem that dies in the combat, or attacks the hero, transforms nothing; radiant
// 20/20 with Divine Shield for First Strike, transforming into a random Radiant non-token Unit of
// Classic or Classic+".

import { legalActions } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/catalog-data";
import { scenario, type FieldSetup, type Scenario } from "../_harness";
import { createInvariantMonitor } from "../_invariants";
import { base, def, radiant } from "../../src/scripts/classic-plus/073-1-classic-golem";

const GOLEM = "classicplus-073-1";
const VANILLA = "core-008"; // (1) Unit 4/4
const FAUCI = { def: "core-091", radiant: true }; // 2/12: survives a 10-attack First Strike
const BIG_MENACE = { def: "core-019", radiant: true }; // 18/18 Taunt: kills a 10/10 back

function golem(defender: FieldSetup | null, opts: { radiantFace?: boolean; seed?: string } = {}): Scenario {
  return scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: { hand: [VANILLA], field: [{ def: GOLEM, radiant: opts.radiantFace === true }], library: [VANILLA] },
    p2: { hand: [VANILLA], field: defender === null ? [] : [defender], library: [VANILLA] },
  });
}

function transformed(s: Scenario): Extract<GameEvent, { type: "transformed" }>[] {
  return s.events.filter((event): event is Extract<GameEvent, { type: "transformed" }> => event.type === "transformed");
}

function attackWith(s: Scenario): Scenario {
  const target = s.unit("p2", 1);
  return s.attack(GOLEM, target ?? "hero");
}

describe("C+ #73.1 Classic Golem", () => {
  it("is a (4) Unit token printed Legendary: 10/10 Rush, First Strike, Trample; radiant 20/20 trades First Strike for Divine Shield", () => {
    expect(def.token).toBe(true);
    expect(def.printedRarity).toBe("Legendary");
    expect(def.cost).toBe(4);
    expect([def.base.attack, def.base.health]).toEqual([10, 10]);
    expect(def.base.keywords.map((k) => k.kind)).toEqual(["Rush", "First Strike", "Trample"]);
    expect([def.radiant.attack, def.radiant.health]).toEqual([20, 20]);
    expect(def.radiant.keywords.map((k) => k.kind)).toEqual(["Rush", "Trample", "Divine Shield"]);
    expect(base.afterAttack).toBeTypeOf("function");
    expect(radiant.afterAttack).toBeTypeOf("function");
    expect(radiant).not.toBe(base);
  });

  describe("base", () => {
    it("R424 its own stats fight first: First Strike kills the defender unhurt and Trample's excess hits the hero", () => {
      const s = golem(VANILLA);
      const defender = s.unit("p2", 1);
      attackWith(s);
      s.expectInZone(defender ?? "", "graveyard");
      s.expectHealth("p2", 24);
      // The fight ended before the transform: the defender died to the Golem's hit.
      const order = s.events.map((event) => event.type);
      expect(order.lastIndexOf("destroyed")).toBeLessThan(order.indexOf("transformed"));
    });

    it("R424 R380 R35 after attacking a Unit it becomes a random non-token Classic or Classic+ Unit, on its base face", () => {
      for (let seed = 0; seed < 12; seed += 1) {
        const s = attackWith(golem(VANILLA, { seed: `golem-${seed}` }));
        const [event] = transformed(s);
        expect(event?.fromDefId).toBe(GOLEM);
        const made = cardDef(event?.toDefId ?? "");
        expect(made.type).toBe("Unit");
        expect(made.token).toBe(false);
        expect(["Classic", "Classic+"]).toContain(made.set);
        const unit = s.unit("p1", 1);
        // B2.7: an X-stats Unit (C+ #69 Buff Billy) made outside a play has no X, so it is 0/0 and dies.
        if (made.base.xStats !== undefined) {
          expect(unit).toBeNull();
          continue;
        }
        expect(unit?.id, `${seed}: ${event?.toDefId}`).toBe(event?.newInstanceId);
        expect(unit?.radiant).toBe(false);
        expect(unit?.position).toBe("ATK");
      }
    });

    it("R1 the new Unit fires no Cry: it is transformed, never summoned or played", () => {
      const s = attackWith(golem(VANILLA));
      const after = s.events.slice(s.events.findIndex((event) => event.type === "transformed"));
      expect(after.some((event) => event.type === "summoned" || event.type === "cardPlayed")).toBe(false);
    });

    it("R42 it destroyed the defender: the new Unit has a fresh exertion and no sickness, so it may attack again", () => {
      const s = attackWith(golem(VANILLA));
      const made = s.unit("p1", 1);
      expect(made?.defId).not.toBe(GOLEM);
      expect(made?.summonedTurn).toBeUndefined();
      expect(made?.exertion).toEqual({ attacked: false, switched: false });
      expect(made).not.toBeNull();
      expect(s.stats(made ?? "").attack).toBeGreaterThan(0);
      const attacks = legalActions(s.state, "p1").filter((action) => action.type === "attack" && action.attackerId === made?.id);
      expect(attacks.length).toBeGreaterThan(0);
      s.attack(made ?? "", "hero");
      expect(s.events.filter((event) => event.type === "attackDeclared")).toHaveLength(2);
    });

    it("R424 the fuzz monitor agrees: the readied new Unit is no sick attack (I1) and its missing summonedTurn no I4 mismatch", () => {
      // The cards-plus-d simulation found the monitor reading R424's lifted sickness as a lost entry.
      const s = golem(VANILLA);
      const monitor = createInvariantMonitor(s.state);
      const from = s.events.length;
      attackWith(s);
      expect(monitor.after(s.events.slice(from), s.state)).toEqual([]);
      const made = s.unit("p1", 1)?.id ?? "";
      const again = monitor.before(s.state, "p1", { type: "attack", attackerId: made, targetId: "hero-p2" });
      // I3 names the cards the scenario placed without an entry event; nothing else may be found.
      expect(again.filter((finding) => !finding.startsWith("I3"))).toEqual([]);
    });

    it("R424 a defender that survives: it still transforms, but the new Unit is summoning sick", () => {
      const s = attackWith(golem(FAUCI));
      expect(s.unit("p2", 1)?.defId).toBe("core-091");
      expect(transformed(s)).toHaveLength(1);
      const made = s.unit("p1", 1);
      expect(made?.summonedTurn).toBe(s.state.turn);
      expect(legalActions(s.state, "p1").some((action) => action.type === "attack" && action.attackerId === made?.id)).toBe(false);
    });

    it("R424 a Golem that dies in the combat transforms into nothing", () => {
      const s = golem(BIG_MENACE);
      const it = s.unit("p1", 1);
      attackWith(s);
      s.expectInZone(it ?? "", "gone");
      expect(transformed(s)).toEqual([]);
      expect(s.unit("p1", 1)).toBeNull();
    });

    it("R424 a Golem that attacks the hero transforms nothing", () => {
      const s = attackWith(golem(null));
      s.expectHealth("p2", 20);
      expect(transformed(s)).toEqual([]);
      expect(s.unit("p1", 1)?.defId).toBe(GOLEM);
    });

    it("R53 R424 a forced attack is no attack it declared: it transforms nothing", () => {
      // Core #9 Moths to the Flame (1/14): "Start of turn: Every enemy Unit attacks this." Damaged to 9,
      // so the Golem's 10 kills it and the forced attack is one that destroyed its target.
      const s = scenario({
        p1: { hand: [VANILLA], field: [GOLEM], library: [VANILLA] },
        p2: { hand: [VANILLA], field: [{ def: "core-009", damage: 5 }], library: [VANILLA] },
      });
      s.endTurn();
      expect(s.events.some((event) => event.type === "attackDeclared" && event.forced)).toBe(true);
      expect(s.pile("p2", "graveyard").some((card) => card.defId === "core-009")).toBe(true);
      expect(transformed(s)).toEqual([]);
      expect(s.unit("p1", 1)?.defId).toBe(GOLEM);
    });

    it("R23 R424 an Immutable Golem is not transformed, and takes no draw (R129)", () => {
      const s = golem(VANILLA);
      s.card(GOLEM).grantedKeywords.push({ kind: "Immutable" });
      const defender = s.unit("p2", 1);
      const cursor = s.state.rngCursor;
      attackWith(s);
      s.expectInZone(defender ?? "", "graveyard");
      expect(transformed(s)).toEqual([]);
      expect(s.unit("p1", 1)?.defId).toBe(GOLEM);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("R129 a fixed seed makes the same Unit; the draw is one rng pick", () => {
      const one = attackWith(golem(VANILLA, { seed: "golem-fixed" }));
      const two = attackWith(golem(VANILLA, { seed: "golem-fixed" }));
      expect(transformed(one)[0]?.toDefId).toBe(transformed(two)[0]?.toDefId);
    });
  });

  describe("radiant", () => {
    it("20/20 kills a 2/12, its Divine Shield taking the hit, and transforms into a Radiant Unit, ready to attack again", () => {
      const s = attackWith(golem(FAUCI, { radiantFace: true }));
      s.expectInZone("core-091", "graveyard");
      // No First Strike on the Radiant face: the defender hits back, and the Shield absorbs it.
      expect(s.events.some((event) => event.type === "divineShieldLost")).toBe(true);
      const [event] = transformed(s);
      expect(cardDef(event?.toDefId ?? "").set).not.toBe("Core");
      expect(s.unit("p1", 1)?.radiant).toBe(true);
      expect(s.unit("p1", 1)?.summonedTurn).toBeUndefined();
    });

    it("Divine Shield absorbs the hit of a defender that survives the Golem's 20; it still transforms, summoning sick", () => {
      const s = golem({ def: VANILLA, statsOverride: { attack: 5, health: 30 } }, { radiantFace: true });
      expect(s.stats(GOLEM).keywords.map((keyword) => keyword.kind)).toContain("Divine Shield");
      attackWith(s);
      expect(s.unit("p2", 1)?.damage).toBe(20);
      expect(s.events.some((event) => event.type === "divineShieldLost")).toBe(true);
      expect(transformed(s)).toHaveLength(1);
      expect(s.unit("p1", 1)?.summonedTurn).toBe(s.state.turn);
    });

    it("R424 attacking the hero transforms nothing", () => {
      const s = attackWith(golem(null, { radiantFace: true }));
      s.expectHealth("p2", 10);
      expect(transformed(s)).toEqual([]);
    });
  });
});
