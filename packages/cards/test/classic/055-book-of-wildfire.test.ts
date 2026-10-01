// C #55 Book of Wildfire — SPEC §8.6 row 55, BUILD M9 Classic row C 55: "`classic-055`, a Book: one
// targeted hit of 4 on any Unit or hero, either side; nothing names it, so C #23 and C #29 never make
// it (R381); C #4's base face answers it as a Book; radiant 8; its tuned number (damage) reads through
// `param()` (R386)".
//
// The hit is one §4.4 damage instance from the Spell, so Divine Shield, Armor and Spell Damage meet it
// as they meet any Spell's. C #4 Palantir's answer to a Book is proved in C #4's own test file; here
// the play is shown to be a Book play, which is what Palantir reads (`playedThisGameWithTag`).

import { legalActions, playedThisGameWithTag, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/catalog-data";
import { base, def, radiant } from "../../src/scripts/classic/055-book-of-wildfire";
import { scenario, type Scenario } from "../_harness";

const WILDFIRE = "classic-055";
const VANILLA = "core-008"; // 4/4, no text
const MENACE = "core-019"; // 9/9 Taunt
const DEFENDER = "core-003"; // 1/1 Taunt, Divine Shield, Reborn
const TOP_LOSER = "classicplus-019-1"; // Radiant: Immune to Spells
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const FILLER = "core-005"; // a hand card, so a turn never auto-ends (§2.5)

function hero(player: PlayerId): Selection {
  return { pick: "hero", player };
}

function at(s: Scenario, ref: string): Selection {
  return { pick: "instance", instanceId: s.card(ref).id };
}

function wildfirePlays(s: Scenario, player: PlayerId = "p1"): Selection[][] {
  const id = s.card(WILDFIRE).id;
  return legalActions(s.state, player).flatMap((action) =>
    action.type === "play" && action.instanceId === id ? [action.targets ?? []] : [],
  );
}

function hitsOn(s: Scenario, targetId: string): number[] {
  return s.events.flatMap((event) => (event.type === "damage" && event.targetId === targetId ? [event.amount] : []));
}

describe("C #55 Book of Wildfire", () => {
  it("is a (1) Spell with the Book tag, declaring one target, its damage a declared number on both faces", () => {
    expect(def.id).toBe(WILDFIRE);
    expect(def.type).toBe("Spell");
    expect(def.cost).toBe(1);
    expect(def.tags).toEqual(["Book"]);
    expect(def.params).toEqual([{ key: "damage", base: 4, radiant: 8, better: "up", step: 1, min: 1 }]);
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
    expect(radiant).toBe(base);
  });

  it("R381 nothing names it: C #23 and C #29 name Book of Flame (C #16), never Book of Wildfire", () => {
    expect(cardDef("classic-023").refs).toEqual(["classic-016"]);
    expect(cardDef("classic-029").refs).toEqual(["classic-016"]);
    expect(cardDef("classic-016").name).toBe("Book of Flame");
    expect(def.name).toBe("Book of Wildfire");
  });

  describe("base", () => {
    it("deals 4 damage to a target enemy Unit", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { field: [MENACE] } });
      s.play(WILDFIRE, { targets: [at(s, MENACE)] });
      s.expectStats(MENACE, { health: 5 });
      s.expectInZone(WILDFIRE, "graveyard");
    });

    it("deals 4 damage to the enemy hero, or to your own hero: any target, either side", () => {
      const enemy = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      enemy.play(WILDFIRE, { targets: [hero("p2")] }).expectHealth("p2", 26);
      const own = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      own.play(WILDFIRE, { targets: [hero("p1")] }).expectHealth("p1", 26);
    });

    it("may hit one of your own Units, and kills a 4-health one", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER], field: [VANILLA] } });
      s.play(WILDFIRE, { targets: [at(s, VANILLA)] });
      s.expectInZone(VANILLA, "graveyard");
    });

    it("legalActions offers every Unit and both heroes, one target each, and a play with none is refused", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER], field: [VANILLA] }, p2: { field: [MENACE] } });
      const offered = wildfirePlays(s);
      expect(offered).toHaveLength(4);
      expect(offered.every((targets) => targets.length === 1)).toBe(true);
      expect(offered).toContainEqual([hero("p1")]);
      expect(offered).toContainEqual([hero("p2")]);
      expect(() => s.play(WILDFIRE)).toThrow(/target/);
      expect(() => s.play(WILDFIRE, { targets: [hero("p1"), hero("p2")] })).toThrow(/at most/);
    });

    it("E35 a Unit Immune to Spells is never offered, and a play naming it is refused", () => {
      const s = scenario({
        p1: { hand: [WILDFIRE, FILLER] },
        p2: { field: [{ def: TOP_LOSER, radiant: true }] },
      });
      const loser = s.card(TOP_LOSER);
      expect(wildfirePlays(s)).not.toContainEqual([{ pick: "instance", instanceId: loser.id }]);
      expect(() => s.play(WILDFIRE, { targets: [{ pick: "instance", instanceId: loser.id }] })).toThrow(/not a legal target/);
    });

    it("§4.4 step 1: Divine Shield takes the whole hit, and the unit stays", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { field: [DEFENDER] } });
      s.play(WILDFIRE, { targets: [at(s, DEFENDER)] });
      s.expectInZone(DEFENDER, "field").expectStats(DEFENDER, { health: 1 });
      s.expectEvents("divineShieldLost");
    });

    it("§4.4 step 2: Armor reduces it — a Unit in Defense Position takes 3", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { field: [{ def: MENACE, position: "DEF" }] } });
      s.play(WILDFIRE, { targets: [at(s, MENACE)] });
      s.expectStats(MENACE, { health: 6 });
    });

    it("§4.4 step 0: Spell Damage on your side raises the hit — Solarius's +2 makes it 6", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER], field: [SOLARIUS] } });
      s.play(WILDFIRE, { targets: [hero("p2")] });
      expect(hitsOn(s, "hero-p2")).toEqual([6]);
    });

    it("is a Book play: the game counts its Book tag for its player (what C #4 Palantir answers)", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      s.play(WILDFIRE, { targets: [hero("p2")] });
      expect(playedThisGameWithTag(s.state, "p1", "Book")).toBe(1);
    });

    it("R386 its damage is the declared number: an Upgrade's step makes it 5, a Degrade's 3", () => {
      const up = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      stepParam(up.card(WILDFIRE), "damage", 1);
      up.play(WILDFIRE, { targets: [hero("p2")] }).expectHealth("p2", 25);
      const down = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      stepParam(down.card(WILDFIRE), "damage", -1);
      down.play(WILDFIRE, { targets: [hero("p2")] }).expectHealth("p2", 27);
      expect(down.view("p1").you.graveyard.find((card) => card.defId === WILDFIRE)?.params).toEqual({ damage: 3 });
    });

    it("R97 in its owner's hand the opponent's view never names it", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] } });
      const theirs = s.view("p2");
      expect(theirs.opponent.hand).toEqual({ count: 2 });
      expect(JSON.stringify(theirs)).not.toContain(WILDFIRE);
      const own = s.view("p1");
      expect(Array.isArray(own.you.hand) && own.you.hand.some((card) => card.defId === WILDFIRE)).toBe(true);
    });

    it("its play replays exactly from the log: a JSON round trip of the state plays the same", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, FILLER] }, p2: { field: [MENACE] } });
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action: Action = {
        type: "play",
        playerId: "p1",
        nonce: "c55-json",
        instanceId: s.card(WILDFIRE).id,
        targets: [at(s, MENACE)],
      };
      const a = reduce(s.state, action);
      const b = reduce(round, action);
      expect(a.error).toBeUndefined();
      expect(b.state).toEqual(a.state);
    });
  });

  describe("radiant", () => {
    it("deals 8 damage to a target", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, FILLER] }, p2: { field: [MENACE] } });
      s.play(WILDFIRE, { targets: [at(s, MENACE)] });
      s.expectStats(MENACE, { health: 1 });
    });

    it("deals 8 to a hero, either side", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, FILLER] } });
      s.play(WILDFIRE, { targets: [hero("p1")] }).expectHealth("p1", 22);
    });

    it("R386 its declared damage steps from 8: an Upgrade makes it 9, enough to kill the 9/9", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, FILLER] }, p2: { field: [MENACE] } });
      stepParam(s.card(WILDFIRE), "damage", 1);
      s.play(WILDFIRE, { targets: [at(s, MENACE)] });
      s.expectInZone(MENACE, "graveyard");
    });

    it("§4.4: the Spell Damage of your side raises the Radiant hit too (8 + 2)", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, FILLER], field: [SOLARIUS] } });
      s.play(WILDFIRE, { targets: [hero("p2")] });
      expect(hitsOn(s, "hero-p2")).toEqual([10]);
    });
  });
});
