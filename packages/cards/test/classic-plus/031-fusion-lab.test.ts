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

const pick = (s: Scenario, ref: string): Selection[] => [{ pick: "instance", instanceId: s.card(ref).id }];

/** The kept card's fused id read back into [random ingredient, kept card]. */
function partsOf(s: Scenario, ref: string): string[] {
  const parts = fusedIdParts(s.card(ref).defId);
  if (parts === null) throw new Error(`${ref} was not fused`);
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
      const kept = s.card(MENACE);
      const costBefore = effectiveCost(s.state, kept);
      s.play(LAB, { targets: pick(s, MENACE) });
      const now = s.card(kept);
      expect(now.zone).toEqual({ z: "hand", player: "p1" });
      const [ingredient, held] = partsOf(s, kept.id);
      expect(held).toBe(MENACE);
      expect(ingredient).not.toBe(LAB);
      expect(defOf(s.state, ingredient ?? "").token).toBe(false);
      expect(defOf(s.state, now.defId).type).toBe("Unit");
      expect(effectiveCost(s.state, now)).toBe(costBefore);
      expect(now.costOverride).toBe(costBefore);
      // The other hand card is untouched.
      expect(s.card(STOCKPILE).defId).toBe(STOCKPILE);
      s.expectInZone(LAB, "field");
    });

    it("R387 the random card is never Fusion Lab, whatever the seed", () => {
      for (const seed of ["lab-a", "lab-b", "lab-c", "lab-d", "lab-e", "lab-f"]) {
        const s = scenario({ seed, p1: { hand: [LAB, MENACE] } });
        s.play(LAB, { targets: pick(s, MENACE) });
        expect(partsOf(s, MENACE)[0]).not.toBe(LAB);
      }
    });

    it("R81 at each end of your turn it asks for a hand card and fuses a random card into it", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(LAB, { targets: pick(s, MENACE) });
      const once = s.card(MENACE).defId;
      s.endTurn();
      const pending = s.state.pending;
      expect(pending?.kind).toBe("hand");
      expect(pending?.playerId).toBe("p1");
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      s.answer(s.card(STOCKPILE).id);
      const parts = partsOf(s, STOCKPILE);
      expect(parts[1]).toBe(STOCKPILE);
      expect(defOf(s.state, s.card(STOCKPILE).defId).type).toBe("Spell");
      expect(s.card(MENACE).defId).toBe(once);
      expect(s.state.active).toBe("p2");
    });

    it("R113 paused at the end of turn, the state survives JSON and the answer replays to the same hash", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(LAB, { targets: pick(s, MENACE) });
      s.endTurn();
      const pending = s.state.pending;
      if (pending === null) throw new Error("the end of turn asked nothing");
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action = {
        type: "answer",
        choiceId: pending.id,
        selection: pick(s, MENACE),
        playerId: "p1",
        nonce: "lab-roundtrip",
      } as Action;
      const live = reduce(s.state, action);
      const again = reduce(thawed, action);
      expect(live.error).toBeUndefined();
      expect(hashState(again.state)).toBe(hashState(live.state));
      expect(again.events).toEqual(live.events);
    });

    it("R23 an Immutable hand card is never offered, by the play or by the end of turn", () => {
      const s = scenario({ p1: { hand: [LAB, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.card(MENACE).grantedKeywords.push({ kind: "Immutable" });
      expect(() => s.play(LAB, { targets: pick(s, MENACE) })).toThrow();
      s.play(LAB, { targets: pick(s, STOCKPILE) });
      s.endTurn();
      const ids = (s.state.pending?.options ?? []).map((option) => (option.selection.pick === "instance" ? option.selection.instanceId : ""));
      expect(ids).toEqual([s.card(STOCKPILE).id]);
      expect(ids).not.toContain(s.card(MENACE).id);
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
      s.play(LAB, { targets: pick(s, MENACE) });
      const theirs = fusedEvents(s, "p2");
      expect(theirs).toHaveLength(1);
      expect(theirs[0]?.defId).toBe(HIDDEN);
      expect(theirs[0]?.resultInstanceId).toBe(HIDDEN);
      expect(theirs[0]?.instanceIds.every((id) => id === HIDDEN)).toBe(true);
      const view = JSON.stringify(s.view("p2"));
      expect(view).not.toContain(s.card(MENACE).defId);
      expect(view).not.toContain(s.card(MENACE).id);
      expect(fusedEvents(s, "p1")[0]?.defId).toBe(s.card(MENACE).defId);
    });
  });

  describe("radiant", () => {
    it("R561 the random card goes in on its Radiant face, lent to both forms: a base hand card shows its Radiant text", () => {
      const s = scenario({ p1: { hand: [{ def: LAB, radiant: true }, MENACE] } });
      s.play(LAB, { targets: pick(s, MENACE) });
      const kept = s.card(MENACE);
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
      const s = scenario({ p1: { hand: [{ def: LAB, radiant: true }, { def: MENACE, radiant: true }] } });
      s.play(LAB, { targets: pick(s, MENACE) });
      const kept = s.card(MENACE);
      expect(kept.radiant).toBe(true);
      const [ingredient] = fusedIdSpecs(kept.defId) ?? [];
      expect(ingredient?.radiant).toBe(true);
      const lent = defOf(s.state, ingredient?.defId ?? "").radiant.text.split("\n")[0] ?? "";
      expect(defOf(s.state, kept.defId).radiant.text).toContain(lent);
    });

    it("R77 its end of turn fuses a Radiant card too, keeping the cost", () => {
      const s = scenario({ p1: { hand: [{ def: LAB, radiant: true }, MENACE, STOCKPILE] }, p2: { hand: [STOCKPILE] } });
      s.play(LAB, { targets: pick(s, MENACE) });
      const cost = effectiveCost(s.state, s.card(STOCKPILE));
      s.endTurn();
      s.answer(s.card(STOCKPILE).id);
      expect(fusedIdSpecs(s.card(STOCKPILE).defId)?.[0]?.radiant).toBe(true);
      expect(effectiveCost(s.state, s.card(STOCKPILE))).toBe(cost);
    });
  });
});
