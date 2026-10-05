// C+ #31 Fusion Lab — SPEC §8.7 row 31, BUILD M9 Classic+ row C+ 31: "Field Spell (balance patch 1: the
// Cry and the end-of-turn trigger became one Activate, once): Activate fuses a random non-token card of
// any set but Fusion Lab (R387) into a hand card chosen with the activation (R81); the hand card is the
// kept instance, its type wins (R77) and it costs what it cost before (`costOverride`); an Immutable
// hand card is never offered (R23); an empty hand offers no activation; the opponent's view names
// neither the hand card, the ingredient nor the fused id (R97, R179); radiant the random card is fused
// in on its Radiant face and lends it to both of the fusion's forms, so its rider shows whether or not
// the hand card is Radiant" (R561).

import type { Action, Selection } from "@jackioh/shared";
import { defOf, effectiveCost, fusedIdParts, fusedIdSpecs, hashState, reduce, type GameState } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { expectAnimated } from "../_animated";
import { base, def, radiant } from "../../src/scripts/classic-plus/031-fusion-lab";

const LAB = "classicplus-031";
/** #19 Midrange Menace, (3) Unit: the hand card the fusion keeps. */
const MENACE = "core-019";
/** #5 Stockpile, (1) Spell: a second hand card. */
const STOCKPILE = "core-005";
const HIDDEN = "hidden";

/** An instance id, read before a Fuse renames the card's definition. */
const idOf = (s: Scenario, ref: string): string => s.card(ref).id;
const pick = (id: string): Selection[] => [{ pick: "instance", instanceId: id }];

/** The kept card's fused id read back into [random ingredient, kept card]. */
function partsOf(s: Scenario, id: string): string[] {
  const parts = fusedIdParts(s.card(id).defId);
  if (parts === null) throw new Error(`${id} was not fused`);
  return parts;
}

function fusedEvents(s: Scenario, viewer: "p1" | "p2") {
  return s.view(viewer).events.flatMap((event) => (event.type === "fused" ? [event] : []));
}

describe("C+ #31 Fusion Lab", () => {
  it("declares one hand card with the activation (R81), once, and its faces differ only in the ingredient's face", () => {
    expect(def.id).toBe(LAB);
    expect(base.activations ?? []).toHaveLength(1);
    expect(base.activations?.[0]?.targets).toEqual([{ kind: "hand", min: 1, max: 1, filter: { check: "fusable" } }]);
    expect(base.activations?.[0]?.uses).toBe(1);
    expect(base).not.toBe(radiant);
  });

  describe("base", () => {
    it("R77 its Activate fuses a random card into the chosen hand card, which stays in hand, keeps its type and its cost", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] } });
      const kept = idOf(s, MENACE);
      const other = idOf(s, STOCKPILE);
      const costBefore = effectiveCost(s.state, s.card(kept));
      s.play(LAB);
      s.activate(LAB, { targets: pick(kept) });
      const now = s.card(kept);
      expect(now.zone).toEqual({ z: "hand", player: "p1" });
      const [ingredient, held] = partsOf(s, kept);
      expect(held).toBe(MENACE);
      expect(ingredient).not.toBe(LAB);
      expect(defOf(s.state, ingredient ?? "").token).toBe(false);
      expect(defOf(s.state, now.defId).type).toBe("Unit");
      expect(effectiveCost(s.state, now)).toBe(costBefore);
      expect(now.costOverride).toBe(costBefore);
      expect(s.card(other).defId).toBe(STOCKPILE);
      s.expectInZone(LAB, "field");
    });

    it("R387 the random card is never Fusion Lab, whatever the seed", () => {
      for (const seed of ["lab-a", "lab-b", "lab-c", "lab-d", "lab-e", "lab-f"]) {
        const s = scenario({ seed, p1: { hand: [LAB, MENACE] } });
        const kept = idOf(s, MENACE);
        s.play(LAB);
        s.activate(LAB, { targets: pick(kept) });
        expect(partsOf(s, kept)[0]).not.toBe(LAB);
      }
    });

    it("the end of turn fuses nothing: the Cry and the trigger are gone", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(LAB);
      const menace = idOf(s, MENACE);
      s.endTurn();
      expect(s.state.pending).toBeNull();
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
      expect(s.card(menace).defId).toBe(MENACE);
      expect(s.state.active).toBe("p2");
    });

    it("the Activate is once: a second activation is refused", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] } });
      const menace = idOf(s, MENACE);
      const stockpile = s.hand("p1").find((card) => card.defId === STOCKPILE)?.id ?? "";
      s.play(LAB);
      s.activate(LAB, { targets: pick(menace) });
      expect(() => s.activate(LAB, { targets: pick(stockpile) })).toThrow();
      expect(s.card(stockpile).defId).toBe(STOCKPILE);
    });

    it("R113 the played state survives JSON and the activation replays to the same hash", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      s.play(LAB);
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(thawed).toEqual(s.state);
      const declared = { type: "activate", playerId: "p1", instanceId: s.card(LAB).id, ability: "fuse", targets: pick(menace), nonce: "lab-roundtrip" } as Action;
      const live = reduce(s.state, declared);
      const again = reduce(thawed, declared);
      expect(live.error).toBeUndefined();
      expect(hashState(again.state)).toBe(hashState(live.state));
      expect(again.events).toEqual(live.events);
    });

    it("R23 an Immutable hand card is never offered: its declared pick is refused, the legal card still fuses", () => {
      // A Radiant Midrange Menace prints Immutable.
      const s = scenario({ p1: { hand: [LAB, { def: MENACE, radiant: true }, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      const stockpile = s.hand("p1").find((card) => card.defId === STOCKPILE)?.id ?? "";
      s.play(LAB);
      expect(() => s.activate(LAB, { targets: pick(menace) })).toThrow(/not a legal target/);
      s.activate(LAB, { targets: pick(stockpile) });
      expect(partsOf(s, stockpile)[1]).toBe(STOCKPILE);
      expect(s.card(menace).defId).toBe(MENACE);
    });

    it("R23 with only an Immutable card in hand the activation has no pick", () => {
      const s = scenario({ p1: { hand: [LAB, { def: MENACE, radiant: true }] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      s.play(LAB);
      expect(() => s.activate(LAB, { targets: pick(menace) })).toThrow(/not a legal target/);
      s.endTurn();
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
      expect(s.state.pending).toBeNull();
      expect(s.events.some((event) => event.type === "turnEnded" && event.player === "p1")).toBe(true);
    });

    it("§8.7 an empty hand: activating picks nothing and fuses nothing, and the end of turn asks nothing", () => {
      const s = scenario({ p1: { hand: [LAB] }, p2: { hand: [STOCKPILE] } });
      s.play(LAB);
      s.activate(LAB);
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
      s.endTurn();
      expect(s.state.pending).toBeNull();
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
    });

    it("R97 R179 the opponent's view names neither the hand card, the ingredient nor the fused id", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      s.play(LAB);
      s.activate(LAB, { targets: pick(menace) });
      const theirs = fusedEvents(s, "p2");
      expect(theirs).toHaveLength(1);
      expect(theirs[0]?.defId).toBe(HIDDEN);
      expect(theirs[0]?.resultInstanceId).toBe(HIDDEN);
      expect(theirs[0]?.instanceIds.every((id) => id === HIDDEN)).toBe(true);
      const view = JSON.stringify(s.view("p2"));
      const fusedId = s.card(menace).defId;
      expect(view).not.toContain(fusedId);
      expect(view).not.toContain(partsOf(s, menace)[0] ?? "none");
      expect(view).not.toContain(menace);
      expect(fusedEvents(s, "p1")[0]?.defId).toBe(fusedId);
    });
  });

  describe("radiant", () => {
    it("R561 the random card goes in on its Radiant face, lent to both forms: a base hand card shows its Radiant text", () => {
      const s = scenario({ p1: { hand: [{ def: LAB, radiant: true }, MENACE] } });
      const menace = idOf(s, MENACE);
      s.play(LAB);
      s.activate(LAB, { targets: pick(menace) });
      const kept = s.card(menace);
      expect(kept.radiant).toBe(false);
      const [ingredient, held] = fusedIdSpecs(kept.defId) ?? [];
      expect(ingredient?.radiant).toBe(true);
      expect(held).toEqual({ defId: MENACE });
      const fusedDef = defOf(s.state, kept.defId);
      const lent = defOf(s.state, ingredient?.defId ?? "").radiant.text.split("\n")[0] ?? "";
      expect(fusedDef.base.text).toContain(lent);
      expect(fusedDef.radiant.text).toContain(lent);
    });

    it("R561 a Radiant hand card shows the same Radiant ingredient", () => {
      const s = scenario({ p1: { hand: [{ def: LAB, radiant: true }, { def: STOCKPILE, radiant: true }] } });
      const stockpile = idOf(s, STOCKPILE);
      s.play(LAB);
      s.activate(LAB, { targets: pick(stockpile) });
      const kept = s.card(stockpile);
      expect(kept.radiant).toBe(true);
      const [ingredient] = fusedIdSpecs(kept.defId) ?? [];
      expect(ingredient?.radiant).toBe(true);
      const lent = defOf(s.state, ingredient?.defId ?? "").radiant.text.split("\n")[0] ?? "";
      expect(defOf(s.state, kept.defId).radiant.text).toContain(lent);
    });

    it("R77 its Activate fuses a Radiant card too, keeping the cost", () => {
      const s = scenario({ p1: { hand: [{ def: LAB, radiant: true }, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      const stockpile = s.hand("p1").find((card) => card.defId === STOCKPILE)?.id ?? "";
      s.play(LAB);
      const cost = effectiveCost(s.state, s.card(stockpile));
      s.activate(LAB, { targets: pick(stockpile) });
      expect(fusedIdSpecs(s.card(stockpile).defId)?.[0]?.radiant).toBe(true);
      expect(effectiveCost(s.state, s.card(stockpile))).toBe(cost);
    });
  });
});

describe("C+ #31 Fusion Lab: Animated (patch v0.2.10)", () => {
  /** Its Cry's hand pick, declared with the play: the Stockpile beside it. */
  const play = (s: Scenario): { targets: Selection[] } => {
    const kept = s.hand("p1").find((card) => card.defId === "core-005");
    return { targets: kept === undefined ? [] : pick(kept.id) };
  };

  it("R383 played, it animates into its lane's unit zone, else the leftmost open one, a 1/3 Unit; with none open it stays a Field Spell", () => {
    expectAnimated({ def: "classicplus-031", stats: { attack: 1, health: 3 }, play });
  });

  it("R383 radiant: a 2/6 Unit", () => {
    expectAnimated({ def: "classicplus-031", radiant: true, stats: { attack: 2, health: 6 }, play });
  });
});
