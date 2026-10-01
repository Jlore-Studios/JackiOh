// C+ #30 Felinor Fuser — SPEC §8.7 row 30, BUILD M9 Classic+ row C+ 30: "Cry: two chained Discovers,
// each of 3 different non-token Felinor Units of any set but Felinor Fuser (R387, R405), the first
// answer carried in the second prompt; then both are fused into this unit (R77, R102): it keeps its
// instance, zone, damage and position, stats sum, keywords union, texts join, its cost becomes min(sum,
// 4), and its definition id names the three (R179); the fused-in Cries never run, it being on the field
// already; the options reach only the chooser (R177); paused between the prompts the state survives
// JSON and replays; radiant Discovers Radiant Felinor Units and its Radiant face sums the Radiant faces".

import type { Action } from "@jackioh/shared";
import { defOf, fusedIdParts, fusedIdSpecs, hashState, reduce, type GameState, type PendingChoice } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/030-felinor-fuser";

const FUSER = "classicplus-030";
/** R405: the non-token Felinor-tagged Units of every set, Felinor Fuser aside. */
const FELINOR_UNITS = ["core-012", "core-043", "core-086", "classic-047", "classicplus-046"];
/** #11 Tempo Timmy: a non-Felinor on the enemy side, which #43 Big Felinor's Cry would destroy. */
const TIMMY = "core-011";
const FILLER = "core-005";

function open(s: Scenario): PendingChoice {
  const pending = s.state.pending;
  if (pending === null) throw new Error("no prompt is open");
  return pending;
}

function offered(pending: PendingChoice): string[] {
  return pending.options.flatMap((option) => (option.selection.pick === "mode" ? [option.selection.option] : []));
}

/** Play the Fuser into lane 2 and answer both Discovers with their first option. */
function fused(options: { radiant?: boolean; seed?: string } = {}): { s: Scenario; picks: string[] } {
  const s = scenario({
    ...(options.seed === undefined ? {} : { seed: options.seed }),
    p1: { hand: [{ def: FUSER, radiant: options.radiant === true }, FILLER] },
    p2: { hand: [FILLER], field: [TIMMY] },
  });
  s.play(FUSER, { zone: 2 });
  const picks: string[] = [];
  for (let step = 0; step < 2; step += 1) {
    const pick = offered(open(s))[0] ?? "";
    picks.push(pick);
    s.answer(pick);
  }
  return { s, picks };
}

describe("C+ #30 Felinor Fuser", () => {
  it("is a 3/3 Felinor Unit whose faces differ only in the Discovered cards' face", () => {
    expect(def.id).toBe(FUSER);
    expect(base).not.toBe(radiant);
  });

  describe("base", () => {
    it("R405 R387 each Discover offers 3 different non-token Felinor Units of any set, never the Fuser", () => {
      const s = scenario({ p1: { hand: [FUSER, FILLER] } });
      s.play(FUSER);
      const first = open(s);
      expect(first.kind).toBe("discover");
      expect(first.playerId).toBe("p1");
      const ids = offered(first);
      expect(ids).toHaveLength(3);
      expect(new Set(ids).size).toBe(3);
      for (const id of ids) expect(FELINOR_UNITS).toContain(id);
      s.answer(ids[0] ?? "");
      const second = offered(open(s));
      expect(second).toHaveLength(3);
      for (const id of second) expect(FELINOR_UNITS).toContain(id);
    });

    it("R352 the first answer is carried in the second prompt, and the second answer fuses both", () => {
      const { s, picks } = fused();
      expect(s.state.pending).toBeNull();
      const fuser = s.unit("p1", 2);
      if (fuser === null) throw new Error("the Fuser left its zone");
      expect(fusedIdParts(fuser.defId)).toEqual([...picks, FUSER]);
    });

    it("R77 it keeps its instance, zone, damage and position; stats sum and texts join", () => {
      const s = scenario({ p1: { hand: [FUSER, FILLER] }, p2: { hand: [FILLER] } });
      const card = s.card(FUSER);
      s.play(FUSER, { zone: 2 });
      const picks: string[] = [];
      for (let step = 0; step < 2; step += 1) {
        picks.push(offered(open(s))[0] ?? "");
        s.answer(picks[step] ?? "");
      }
      const kept = s.unit("p1", 2);
      if (kept === null) throw new Error("the Fuser left its zone");
      expect(kept.id).toBe(card.id);
      const fusedDef = defOf(s.state, kept.defId);
      const ingredients = [...picks, FUSER].map((id) => defOf(s.state, id));
      const attack = ingredients.reduce((sum, entry) => sum + (entry.base.attack ?? 0), 0);
      const health = ingredients.reduce((sum, entry) => sum + (entry.base.health ?? 0), 0);
      s.expectStats(kept, { attack, health, maxHealth: health });
      expect(fusedDef.type).toBe("Unit");
      for (const entry of ingredients) expect(fusedDef.base.text).toContain(entry.base.text.split("\n")[0] ?? "");
      // R77: min(sum of printed costs, 4) — the Fuser's 3 alone with any Felinor reaches the cap.
      expect(fusedDef.cost).toBe(4);
    });

    it("R77 the fused-in Cries never run: the Fuser is on the field already", () => {
      const { s } = fused({ seed: "fuser-cries" });
      // #43 Big Felinor's Cry would destroy the enemy Timmy; #12's would summon a copy.
      s.expectInZone(TIMMY, "field");
      expect(s.events.filter((event) => event.type === "summoned")).toHaveLength(1);
    });

    it("R177 the options reach only the chooser", () => {
      const s = scenario({ p1: { hand: [FUSER, FILLER] }, p2: { hand: [FILLER] } });
      s.play(FUSER);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      expect(JSON.stringify(s.view("p2"))).not.toContain(offered(open(s))[0] ?? "no option");
      s.answer(offered(open(s))[0] ?? "");
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    });

    it("R113 paused between the prompts the state survives JSON and the answer replays to the same hash", () => {
      const s = scenario({ p1: { hand: [FUSER, FILLER] }, p2: { hand: [FILLER] } });
      s.play(FUSER);
      s.answer(offered(open(s))[0] ?? "");
      const pending = open(s);
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(hashState(thawed)).toBe(hashState(s.state));
      const action = {
        type: "answer",
        choiceId: pending.id,
        selection: [pending.options[1]?.selection],
        playerId: "p1",
        nonce: "fuser-roundtrip",
      } as Action;
      const live = reduce(s.state, action);
      const again = reduce(thawed, action);
      expect(live.error).toBeUndefined();
      expect(hashState(again.state)).toBe(hashState(live.state));
      expect(again.events).toEqual(live.events);
    });
  });

  describe("radiant", () => {
    it("R77 Discovers Radiant Felinor Units: each goes in on its Radiant face and the 6/6 sums the Radiant faces", () => {
      const { s, picks } = fused({ radiant: true });
      const kept = s.unit("p1", 2);
      if (kept === null) throw new Error("the Fuser left its zone");
      expect(kept.radiant).toBe(true);
      expect(fusedIdSpecs(kept.defId)).toEqual([...picks.map((id) => ({ defId: id, radiant: true })), { defId: FUSER }]);
      const faces = [...picks, FUSER].map((id) => defOf(s.state, id).radiant);
      const attack = faces.reduce((sum, face) => sum + (face.attack ?? 0), 0);
      const health = faces.reduce((sum, face) => sum + (face.health ?? 0), 0);
      s.expectStats(kept, { attack, health, maxHealth: health });
    });

    it("R387 the Radiant Discovers never offer the Fuser either", () => {
      const s = scenario({ p1: { hand: [{ def: FUSER, radiant: true }, FILLER] } });
      s.play(FUSER);
      for (const id of offered(open(s))) expect(FELINOR_UNITS).toContain(id);
    });
  });
});
