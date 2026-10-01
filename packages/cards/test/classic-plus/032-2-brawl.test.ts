// C+ #32.2 Brawl — SPEC §8.7 row 32.2, BUILD M9 Classic+ row C+ 32.2: "Destroys every Unit on both
// sides but one survivor drawn at random (R60); Indestructible and Immune to Spells Units stay as well;
// Reborn units come back; no Units, nothing; radiant the survivor is a Unit you choose with the play
// (R81)".

import type { Action, Selection } from "@jackioh/shared";
import { hashState, reduce, type GameState } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/032-2-brawl";

const BRAWL = "classicplus-032-2";
const MENACE = "core-019";
const TIMMY = "core-011";
/** #56 Jilliax: Divine Shield, which a destroy ignores. */
const JILLIAX = "core-056";
/** C #41 State of the Game, Indestructible. */
const UNBREAKABLE = "classic-041";
/** #3 Right-house defender: Taunt, Divine Shield, Reborn. */
const DEFENDER = "core-003";
const FILLER = "core-005";
/** C+ #19.1 Top Loser, whose Radiant face is Immune to Spells. */
const TOP_LOSER = "classicplus-019-1";

const units = (s: Scenario): string[] =>
  (["p1", "p2"] as const).flatMap((player) =>
    [1, 2, 3, 4, 5].flatMap((lane) => {
      const unit = s.unit(player, lane);
      return unit === null ? [] : [unit.id];
    }),
  );

const destroyed = (s: Scenario): string[] => s.events.flatMap((event) => (event.type === "destroyed" ? [event.instanceId] : []));

function brawl(seed: string, radiantFace = false, targets?: (s: Scenario) => Selection[]) {
  const s = scenario({
    seed,
    p1: { hand: [{ def: BRAWL, radiant: radiantFace }, FILLER], field: [TIMMY, MENACE] },
    p2: { field: [TIMMY, JILLIAX] },
  });
  const before = units(s);
  s.play(BRAWL, targets === undefined ? {} : { targets: targets(s) });
  return { s, before };
}

describe("C+ #32.2 Brawl", () => {
  it("declares nothing on the base face and one Unit of either side on the Radiant face (R81)", () => {
    expect(def.id).toBe(BRAWL);
    expect(base.targets).toBeUndefined();
    expect(radiant.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] } }]);
  });

  describe("base", () => {
    it("R60 destroys every Unit on both sides but one survivor drawn at random, all in one check (R59)", () => {
      const { s, before } = brawl("brawl-one");
      const after = units(s);
      expect(after).toHaveLength(1);
      expect(before).toContain(after[0]);
      expect([...destroyed(s)].sort()).toEqual(before.filter((id) => id !== after[0]).sort());
      s.expectInZone(BRAWL, "graveyard");
    });

    it("R60 the survivor is the match rng's: the same seed spares the same Unit, other seeds others", () => {
      expect(units(brawl("brawl-seed").s)).toEqual(units(brawl("brawl-seed").s));
      const spared = new Set(
        ["a", "b", "c", "d", "e", "f", "g", "h"].map((seed) => {
          const { s, before } = brawl(`brawl-${seed}`);
          return before.indexOf(units(s)[0] ?? "");
        }),
      );
      expect(spared.size).toBeGreaterThan(1);
    });

    it("§9.3 the random survivor replays from a JSON copy to the same hash", () => {
      const s = scenario({ seed: "brawl-replay", p1: { hand: [BRAWL, FILLER], field: [TIMMY, MENACE] }, p2: { field: [TIMMY, JILLIAX] } });
      const action = { type: "play", instanceId: s.card(BRAWL).id, playerId: "p1", nonce: "brawl-replay" } as Action;
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const live = reduce(s.state, action);
      expect(live.error).toBeUndefined();
      expect(hashState(reduce(thawed, action).state)).toBe(hashState(live.state));
    });

    it("R46 an Indestructible Unit stays as well; a Reborn Unit comes back", () => {
      const s = scenario({
        seed: "brawl-keywords",
        p1: { hand: [BRAWL, FILLER], field: [UNBREAKABLE, MENACE] },
        p2: { field: [DEFENDER, TIMMY] },
      });
      s.play(BRAWL);
      s.expectInZone(UNBREAKABLE, "field");
      s.expectInZone(DEFENDER, "field");
      const left = units(s).map((id) => s.card(id).defId);
      expect(left).toContain(UNBREAKABLE);
      expect(left).toContain(DEFENDER);
      expect(left.length).toBeLessThanOrEqual(3);
    });

    it("§6.1 an Immune to Spells Unit is not among the candidates and stays anyway", () => {
      for (const seed of ["immune-a", "immune-b", "immune-c", "immune-d"]) {
        const s = scenario({
          seed,
          p1: { hand: [BRAWL, FILLER], field: [TIMMY, MENACE] },
          p2: { field: [{ def: TOP_LOSER, radiant: true }] },
        });
        s.play(BRAWL);
        const left = units(s).map((id) => s.card(id).defId);
        expect(left).toContain(TOP_LOSER);
        expect(left).toHaveLength(2);
        expect(destroyed(s)).toHaveLength(1);
      }
    });

    it("R129 with no Unit there is nothing to destroy and nothing is drawn", () => {
      const s = scenario({ p1: { hand: [BRAWL, FILLER] } });
      const cursor = s.state.rngCursor;
      s.play(BRAWL);
      expect(destroyed(s)).toEqual([]);
      expect(s.state.rngCursor).toBe(cursor);
      s.expectInZone(BRAWL, "graveyard");
    });

    it("§8.7 a lone Unit is the survivor", () => {
      const s = scenario({ p1: { hand: [BRAWL, FILLER] }, p2: { field: [MENACE] } });
      s.play(BRAWL);
      s.expectInZone(MENACE, "field");
      expect(destroyed(s)).toEqual([]);
    });
  });

  describe("radiant", () => {
    it("R81 the survivor is the Unit you choose with the play, an enemy one included", () => {
      const { s, before } = brawl("brawl-radiant", true, (at) => [{ pick: "instance", instanceId: at.unit("p2", 2)?.id ?? "" }]);
      expect(units(s).map((id) => s.card(id).defId)).toEqual([JILLIAX]);
      expect(destroyed(s)).toHaveLength(before.length - 1);
    });

    it("R81 an Immune to Spells Unit can't be the chosen survivor, and stays all the same", () => {
      const s = scenario({
        p1: { hand: [{ def: BRAWL, radiant: true }, FILLER], field: [TIMMY] },
        p2: { field: [{ def: TOP_LOSER, radiant: true }, MENACE] },
      });
      const loser = s.card(TOP_LOSER).id;
      expect(() => s.play(BRAWL, { targets: [{ pick: "instance", instanceId: loser }] })).toThrow(/not a legal target/);
      s.play(BRAWL, { targets: [{ pick: "instance", instanceId: s.card(TIMMY).id }] });
      expect(units(s).map((id) => s.card(id).defId).sort()).toEqual([TIMMY, TOP_LOSER].sort());
    });

    it("R81 or one of your own", () => {
      const { s } = brawl("brawl-radiant-own", true, (at) => [{ pick: "instance", instanceId: at.unit("p1", 2)?.id ?? "" }]);
      expect(units(s).map((id) => s.card(id).defId)).toEqual([MENACE]);
    });
  });
});
