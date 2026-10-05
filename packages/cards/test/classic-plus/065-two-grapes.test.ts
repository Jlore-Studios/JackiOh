// C+ #65 Two Grapes — SPEC §8.7 row 65, BUILD M9 Classic+ row C+ 65: "Add {grapes} to your hand"
// (exactly two, balance patch 1; each rolled on its own from `GRAPE_ODDS` in code, never in player
// text); a fixed seed gives fixed Grapes and many seeded rolls match the odds; a full hand burns;
// hidden from the opponent (R97); the count reads through `param()`; radiant Lucky 1, two Radiant
// Grapes, each rolled twice with the better kept in the order Rotten, Normal, Large, Golden, Mythic".
//
// The odds themselves are proved on 20,000 rolls of the verb in packages/engine/test/effects-fruit.test.ts;
// here the card is played, many times over, and its Grapes counted.

import { GRAPE_ODDS, createRng, numbersOn, stepParam } from "@jackioh/engine";
import { rollGrape } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CATALOG } from "../../src/index";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/065-two-grapes";

const GRAPES = "classicplus-065";
const FILLER = "core-005";
const GRAPE_IDS = GRAPE_ODDS.map((grape) => grape.defId);

function added(s: Scenario): Extract<GameEvent, { type: "addedToHand" }>[] {
  return s.lastEvents.flatMap((event) => (event.type === "addedToHand" && event.player === "p1" ? [event] : []));
}

function played(opts: { radiant?: boolean; seed?: string; hand?: number } = {}): Scenario {
  const s = scenario({
    seed: opts.seed ?? "two-grapes",
    p1: {
      hand: [{ def: GRAPES, ...(opts.radiant === true ? { radiant: true } : {}) }, ...Array.from({ length: opts.hand ?? 1 }, () => FILLER)],
    },
    p2: { hand: [FILLER] },
  });
  return s;
}

describe("C+ #65 Two Grapes", () => {
  it("is a (1) Fruit Spell with no refs ('Grape' names the five Grapes, R480), and both faces run one shape of hook", () => {
    expect(def.id).toBe(GRAPES);
    expect(def.cost).toBe(1);
    expect(def.tags).toContain("Fruit");
    expect(def.refs).toBeUndefined();
    expect(def.params).toEqual([{ key: "grapes", base: 2, radiant: 2, better: "up", step: 1, min: 1 }]);
    expect(typeof base.cry).toBe("function");
    expect(typeof radiant.cry).toBe("function");
    expect(def.radiant.keywords).toEqual([{ kind: "Lucky", n: 1 }]);
  });

  describe("base", () => {
    it("R382 adds exactly 2 Grapes, each one of the five", () => {
      const s = played();
      s.play(GRAPES);

      const grapes = added(s);
      expect(grapes).toHaveLength(2);
      expect(grapes.every((event) => GRAPE_IDS.includes(event.defId))).toBe(true);
      expect(grapes.every((event) => !s.card(event.instanceId).radiant)).toBe(true);
      expect(s.card(GRAPES).zone.z).toBe("graveyard");
    });

    it("R382 each Grape is its own roll: one draw of the rng each, two in all", () => {
      const s = played();
      const before = s.state.rngCursor;
      s.play(GRAPES);
      expect(s.state.rngCursor - before).toBe(2);
    });

    it("§10.7 a fixed seed gives fixed Grapes, the very ones the roll gives from the same cursor", () => {
      const a = played({ seed: "fixed" });
      const cursor = a.state.rngCursor;
      a.play(GRAPES);
      const b = played({ seed: "fixed" });
      b.play(GRAPES);
      expect(added(a).map((event) => event.defId)).toEqual(added(b).map((event) => event.defId));

      const rng = createRng("fixed", cursor);
      expect(added(a).map((event) => event.defId)).toEqual([rollGrape(rng), rollGrape(rng)]);
    });

    it("R382 many seeded plays match the odds: Normal is the commonest, and every Grape comes up", () => {
      const counts = new Map<string, number>();
      const plays = 250;
      for (let i = 0; i < plays; i += 1) {
        const s = played({ seed: `odds-${i}` });
        s.play(GRAPES);
        for (const event of added(s)) counts.set(event.defId, (counts.get(event.defId) ?? 0) + 1);
      }
      const total = plays * 2;
      const share = (at: number): number => ((counts.get(GRAPE_IDS[at] ?? "") ?? 0) / total) * 100;
      expect(Math.abs(share(1) - 60)).toBeLessThan(6);
      expect(Math.abs(share(0) - 12)).toBeLessThan(4);
      expect(Math.abs(share(2) - 20)).toBeLessThan(5);
      expect(share(3)).toBeGreaterThan(3);
      expect(share(3)).toBeLessThan(11);
      expect(share(4)).toBeLessThan(4);
      expect(GRAPE_IDS.slice(0, 4).every((id) => (counts.get(id) ?? 0) > 0)).toBe(true);
    });

    it("§2.4 R4 a full hand burns the Grapes that don't fit", () => {
      const s = played({ hand: 9 });
      s.play(GRAPES);
      expect(s.hand("p1")).toHaveLength(10);
      const burned = s.lastEvents.filter((event) => event.type === "burned");
      expect(burned).toHaveLength(1);
      expect(added(s)).toHaveLength(1);
    });

    it("R97 the opponent sees two cards reach a hand under the sentinel, never which Grapes", () => {
      const s = played();
      s.play(GRAPES);
      const theirs = s.view("p2").events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(theirs).toHaveLength(2);
      for (const event of theirs) {
        if (event.type !== "addedToHand") continue;
        expect(event.defId).toBe("hidden");
        expect(event.instanceId).toBe("hidden");
      }
      const hand = s.view("p2").opponent.hand;
      expect(hand).toEqual({ count: 3 });
    });

    it("R386 an Upgrade of its count adds 3 Grapes; a Degrade 1, never fewer than 1", () => {
      const up = played();
      stepParam(up.card(GRAPES), "grapes", 1);
      up.play(GRAPES);
      expect(added(up)).toHaveLength(3);

      const down = played();
      stepParam(down.card(GRAPES), "grapes", -1);
      down.play(GRAPES);
      expect(added(down)).toHaveLength(1);

      const floor = played();
      stepParam(floor.card(GRAPES), "grapes", -5);
      floor.play(GRAPES);
      expect(added(floor)).toHaveLength(1);
    });
  });

  describe("radiant", () => {
    it("R74 adds exactly 2 Radiant Grapes", () => {
      const s = played({ radiant: true });
      s.play(GRAPES);
      const grapes = added(s);
      expect(grapes).toHaveLength(2);
      expect(grapes.every((event) => s.card(event.instanceId).radiant)).toBe(true);
    });

    it("§6.1 Lucky 1: each Grape is two rolls with the better kept — four draws of the rng", () => {
      const s = played({ radiant: true, seed: "lucky" });
      const cursor = s.state.rngCursor;
      s.play(GRAPES);
      expect(s.state.rngCursor - cursor).toBe(4);

      const rng = createRng("lucky", cursor);
      const expected = [0, 1].map(() => {
        const first = GRAPE_IDS.indexOf(rollGrape(rng));
        const second = GRAPE_IDS.indexOf(rollGrape(rng));
        return GRAPE_IDS[Math.max(first, second)];
      });
      expect(added(s).map((event) => event.defId)).toEqual(expected);
    });

    it("§6.1 over many plays Lucky shifts the odds toward the better Grapes", () => {
      let rotten = 0;
      let better = 0;
      const plays = 200;
      for (let i = 0; i < plays; i += 1) {
        const s = played({ radiant: true, seed: `lucky-${i}` });
        s.play(GRAPES);
        for (const event of added(s)) {
          if (event.defId === GRAPE_IDS[0]) rotten += 1;
          if (GRAPE_IDS.indexOf(event.defId) >= 2) better += 1;
        }
      }
      // Rotten falls from 12% to 1.44%; Large or better rises from 28% to about 48%.
      expect(rotten / (plays * 2)).toBeLessThan(0.05);
      expect(better / (plays * 2)).toBeGreaterThan(0.38);
    });

    it("R386 B3.4 its Lucky is a numbered keyword: one X step up makes it Lucky 2, three draws a Grape", () => {
      const s = played({ radiant: true, seed: "lucky-two" });
      const card = s.card(GRAPES);
      const luckyNow = (): number | undefined =>
        numbersOn(s.state, s.card(GRAPES)).find((number) => number.ref.kind === "keyword" && number.ref.key === "Lucky")?.value;
      expect(luckyNow()).toBe(1);
      // An Upgrade's X change (B3.4 rule 3) is one step of `tuning.x` under the keyword's name.
      card.tuning = { x: { Lucky: 1 } };
      expect(luckyNow()).toBe(2);

      const cursor = s.state.rngCursor;
      s.play(GRAPES);
      expect(added(s)).toHaveLength(2);
      expect(s.state.rngCursor - cursor).toBe(6);
    });

    it("R386 B3.4 a Degrade's X change never takes its Lucky below 1", () => {
      const s = played({ radiant: true, seed: "lucky-floor" });
      s.card(GRAPES).tuning = { x: { Lucky: -3 } };
      const cursor = s.state.rngCursor;
      s.play(GRAPES);
      expect(s.state.rngCursor - cursor).toBe(4);
    });

    it("R386 the Radiant count steps the same way", () => {
      const s = played({ radiant: true });
      stepParam(s.card(GRAPES), "grapes", 1);
      s.play(GRAPES);
      expect(added(s)).toHaveLength(3);
    });
  });

  it("the catalog hides the odds from both faces' text", () => {
    for (const face of [CATALOG[GRAPES]?.base.text ?? "", CATALOG[GRAPES]?.radiant.text ?? ""]) {
      expect(face).not.toContain("%");
      expect(face).not.toContain("Rotten");
      expect(face).not.toContain("Mythic");
    }
  });
});
