// C+ #66 Vine of Grapes — SPEC §8.7 row 66, BUILD M9 Classic+ row C+ 66: "As Two Grapes with 5 Grapes;
// the count reads through `param()`; radiant 5 Radiant Grapes, each rolled with Lucky 1".
//
// The roll is C+ #65's (`addRolledGrapes`), whose odds are proved in packages/engine/test/
// effects-fruit.test.ts and through Two Grapes in 065-two-grapes.test.ts.

import { GRAPE_ODDS, createRng, stepParam } from "@jackioh/engine";
import { rollGrape } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CATALOG } from "../../src/index";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/066-vine-of-grapes";

const VINE = "classicplus-066";
const FILLER = "core-005";
const GRAPE_IDS = GRAPE_ODDS.map((grape) => grape.defId);

function added(s: Scenario): Extract<GameEvent, { type: "addedToHand" }>[] {
  return s.lastEvents.flatMap((event) => (event.type === "addedToHand" && event.player === "p1" ? [event] : []));
}

function played(opts: { radiant?: boolean; seed?: string; hand?: number } = {}): Scenario {
  return scenario({
    seed: opts.seed ?? "vine",
    p1: {
      hand: [{ def: VINE, ...(opts.radiant === true ? { radiant: true } : {}) }, ...Array.from({ length: opts.hand ?? 1 }, () => FILLER)],
    },
    p2: { hand: [FILLER] },
  });
}

describe("C+ #66 Vine of Grapes", () => {
  it("is a (3) Fruit Spell naming the five Grapes; the Radiant face prints Lucky 1", () => {
    expect(def.id).toBe(VINE);
    expect(def.cost).toBe(3);
    expect(def.tags).toContain("Fruit");
    expect(def.refs).toEqual(GRAPE_IDS);
    expect(def.radiant.keywords).toEqual([{ kind: "Lucky", n: 1 }]);
    expect(typeof base.cry).toBe("function");
    expect(typeof radiant.cry).toBe("function");
  });

  describe("base", () => {
    it("R382 adds 5 Grapes, each its own roll of GRAPE_ODDS from the match rng", () => {
      const s = played({ seed: "five" });
      const cursor = s.state.rngCursor;
      s.play(VINE);

      const rng = createRng("five", cursor);
      expect(added(s).map((event) => event.defId)).toEqual([0, 1, 2, 3, 4].map(() => rollGrape(rng)));
      expect(added(s).every((event) => !s.card(event.instanceId).radiant)).toBe(true);
      expect(s.state.rngCursor - cursor).toBe(5);
    });

    it("§2.4 R4 a full hand burns the Grapes that don't fit", () => {
      const s = played({ hand: 7 });
      s.play(VINE);
      expect(added(s)).toHaveLength(3);
      expect(s.lastEvents.filter((event) => event.type === "burned")).toHaveLength(2);
      expect(s.hand("p1")).toHaveLength(10);
    });

    it("R97 the opponent learns only that five cards reached the hand", () => {
      const s = played();
      s.play(VINE);
      const theirs = s.view("p2").events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(theirs).toHaveLength(5);
      expect(theirs.every((event) => event.type === "addedToHand" && event.defId === "hidden")).toBe(true);
    });

    it("R386 an Upgrade of its count adds 6; a Degrade 4", () => {
      const up = played();
      stepParam(up.card(VINE), "grapes", 1);
      up.play(VINE);
      expect(added(up)).toHaveLength(6);

      const down = played();
      stepParam(down.card(VINE), "grapes", -1);
      down.play(VINE);
      expect(added(down)).toHaveLength(4);
    });
  });

  describe("radiant", () => {
    it("R74 §6.1 adds 5 Radiant Grapes, each two rolls with the better kept", () => {
      const s = played({ radiant: true, seed: "lucky-vine" });
      const cursor = s.state.rngCursor;
      s.play(VINE);

      const rng = createRng("lucky-vine", cursor);
      const expected = [0, 1, 2, 3, 4].map(() => {
        const first = GRAPE_IDS.indexOf(rollGrape(rng));
        const second = GRAPE_IDS.indexOf(rollGrape(rng));
        return GRAPE_IDS[Math.max(first, second)];
      });
      expect(added(s).map((event) => event.defId)).toEqual(expected);
      expect(added(s).every((event) => s.card(event.instanceId).radiant)).toBe(true);
      expect(s.state.rngCursor - cursor).toBe(10);
    });

    it("R276 the Radiant face keeps five Grapes, not the designer's three", () => {
      expect(CATALOG[VINE]?.params?.find((entry) => entry.key === "grapes")).toMatchObject({ base: 5, radiant: 5 });
      const s = played({ radiant: true });
      s.play(VINE);
      expect(added(s)).toHaveLength(5);
    });

    it("R386 the Radiant count steps the same way", () => {
      const s = played({ radiant: true });
      stepParam(s.card(VINE), "grapes", -1);
      s.play(VINE);
      expect(added(s)).toHaveLength(4);
    });
  });
});
