// C+ #31 Fusion Lab — SPEC §8.7 row 31, BUILD M9 Classic+ row C+ 31: "Field Spell: its Cry fuses a
// random non-token card of any set but Fusion Lab (R387) into a hand card chosen with the play (R81),
// and each end of your turn does the same through a hand prompt; the hand card is the kept instance,
// its type wins (R77) and it costs what it cost before (`costOverride`); an Immutable hand card is never
// offered (R23); an empty hand, no pick and nothing happens; the opponent's view names neither the hand
// card, the ingredient nor the fused id (R97, R179); radiant the random card is fused in on its Radiant
// face and lends it to both of the fusion's forms, so its rider shows whether or not the hand card is
// Radiant" (R561).

import type { Action, Selection } from "@jackioh/shared";
import { defOf, effectiveCost, fusedIdParts, fusedIdSpecs, hashState, reduce, type GameState } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
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
  it("declares one hand card with the play (R81), and its faces differ only in the ingredient's face", () => {
    expect(def.id).toBe(LAB);
    expect(base.targets).toEqual([{ kind: "hand", min: 1, max: 1, filter: { check: "fusable" } }]);
    expect(base).not.toBe(radiant);
  });

  describe("base", () => {
    it("R77 its Cry fuses a random card into the chosen hand card, which stays in hand, keeps its type and its cost", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] } });
      const kept = idOf(s, MENACE);
      const other = idOf(s, STOCKPILE);
      const costBefore = effectiveCost(s.state, s.card(kept));
      s.play(LAB, { targets: pick(kept) });
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
        s.play(LAB, { targets: pick(kept) });
        expect(partsOf(s, kept)[0]).not.toBe(LAB);
      }
    });

    it("R81 at each end of your turn it asks for a hand card and fuses a random card into it", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      const stockpile = s.hand("p1").find((card) => card.defId === STOCKPILE)?.id ?? "";
      s.play(LAB, { targets: pick(menace) });
      const once = s.card(menace).defId;
      s.endTurn();
      const pending = s.state.pending;
      expect(pending?.kind).toBe("hand");
      expect(pending?.playerId).toBe("p1");
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      s.answer(stockpile);
      expect(partsOf(s, stockpile)[1]).toBe(STOCKPILE);
      expect(defOf(s.state, s.card(stockpile).defId).type).toBe("Spell");
      expect(s.card(menace).defId).toBe(once);
      expect(s.state.active).toBe("p2");
    });

    it("R113 paused at the end of turn, the state survives JSON and the answer replays to the same hash", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      s.play(LAB, { targets: pick(menace) });
      s.endTurn();
      const pending = s.state.pending;
      if (pending === null) throw new Error("the end of turn asked nothing");
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(thawed).toEqual(s.state);
      const action = { type: "answer", choiceId: pending.id, selection: pick(menace), playerId: "p1", nonce: "lab-roundtrip" } as Action;
      const live = reduce(s.state, action);
      const again = reduce(thawed, action);
      expect(live.error).toBeUndefined();
      expect(hashState(again.state)).toBe(hashState(live.state));
      expect(again.events).toEqual(live.events);
    });

    it("R23 an Immutable hand card is never offered, by the play or by the end of turn", () => {
      // A Radiant Midrange Menace prints Immutable.
      const s = scenario({ p1: { hand: [LAB, { def: MENACE, radiant: true }, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      const stockpile = s.hand("p1").find((card) => card.defId === STOCKPILE)?.id ?? "";
      expect(() => s.play(LAB, { targets: pick(menace) })).toThrow(/not a legal target/);
      s.play(LAB, { targets: pick(stockpile) });
      s.endTurn();
      const offered = (s.state.pending?.options ?? []).map((option) => (option.selection.pick === "instance" ? option.selection.instanceId : ""));
      expect(offered).toEqual([stockpile]);
    });

    it("R23 with only an Immutable card in hand the Cry has no pick and the end of turn asks nothing", () => {
      const s = scenario({ p1: { hand: [LAB, { def: MENACE, radiant: true }] }, p2: { hand: [STOCKPILE] } });
      s.play(LAB);
      s.endTurn();
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
      expect(s.state.pending).toBeNull();
      expect(s.events.some((event) => event.type === "turnEnded" && event.player === "p1")).toBe(true);
    });

    it("§8.7 an empty hand: no pick and nothing happens, at the Cry and at the end of turn", () => {
      const s = scenario({ p1: { hand: [LAB] }, p2: { hand: [STOCKPILE] } });
      s.play(LAB);
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
      s.endTurn();
      expect(s.state.pending).toBeNull();
      expect(s.events.some((event) => event.type === "fused")).toBe(false);
    });

    it("R97 R179 the opponent's view names neither the hand card, the ingredient nor the fused id", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      s.play(LAB, { targets: pick(menace) });
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
      s.play(LAB, { targets: pick(menace) });
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
      s.play(LAB, { targets: pick(stockpile) });
      const kept = s.card(stockpile);
      expect(kept.radiant).toBe(true);
      const [ingredient] = fusedIdSpecs(kept.defId) ?? [];
      expect(ingredient?.radiant).toBe(true);
      const lent = defOf(s.state, ingredient?.defId ?? "").radiant.text.split("\n")[0] ?? "";
      expect(defOf(s.state, kept.defId).radiant.text).toContain(lent);
    });

    it("R77 its end of turn fuses a Radiant card too, keeping the cost", () => {
      const s = scenario({ p1: { hand: [{ def: LAB, radiant: true }, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      const menace = idOf(s, MENACE);
      const stockpile = s.hand("p1").find((card) => card.defId === STOCKPILE)?.id ?? "";
      s.play(LAB, { targets: pick(menace) });
      const cost = effectiveCost(s.state, s.card(stockpile));
      s.endTurn();
      s.answer(stockpile);
      expect(fusedIdSpecs(s.card(stockpile).defId)?.[0]?.radiant).toBe(true);
      expect(effectiveCost(s.state, s.card(stockpile))).toBe(cost);
    });
  });
});
