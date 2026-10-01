// C #48 Hired Shrimp — SPEC §8.6 row 48, BUILD M9 Classic row C 48: "Reads `loc` (§5), public in both
// views; base: the Cry's target is any permanent, either side, face-down cards included, unfiltered,
// never Hired Shrimp itself, which is still in hand when the Cry's target is chosen (R397, R70, R90); at
// resolution the target is destroyed only if its `loc` is greater than Hired Shrimp's own, else the Cry
// fizzles; an Indestructible target survives (R46); no permanent → it enters anyway; a face-down option
// carries only its id (R177); a fused card's `loc` is its ingredients' sum; the test reads both values
// from the catalog, never literals; radiant 8/6: only permanents whose `loc` is greater are offered
// (the highlight) and the target is destroyed, except a face-down card the chooser may not read, which
// is always offered and judged at resolution as on the base face, so the option list never reveals its
// `loc` (R397, R177); none qualifies and no such face-down card → no target; no tuned numbers".
//
// Every `loc` is read off the catalog (`cardDef(id).loc`) and each case first states the comparison it
// relies on. The permanents are Core cards with their own tests — Carnivorous Cube, Mr. Vanilla, Mana
// Well, Bear Honeypot, The Rock — and this set's C #52 Final Gambit, a Trap with fewer lines. Fused
// cards come from Unlicensed Experimentation, which fuses the opponent's played permanent onto one of
// its controller's of that type (R77, R102).

import { defOf, legalActions } from "@jackioh/engine";
import type { PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { cardDef } from "../../src/catalog-data";
import { base, def, radiant } from "../../src/scripts/classic/048-hired-shrimp";

const SHRIMP = "classic-048";
const GAMBIT = "classic-052"; // a Trap with fewer lines than Hired Shrimp
const CUBE = "core-022"; // Carnivorous Cube: a Unit with many more lines
const VANILLA = "core-008"; // Mr. Vanilla: a Unit with very few
const MANA_WELL = "core-006"; // a Field Spell with few
const HONEYPOT = "core-060"; // Bear Honeypot: a Trap with more
const ROCK = "core-066"; // The Rock: Indestructible, few lines
const BIGOT = "core-002";
const GARY = "core-004";
const FIENDER = "core-092"; // Felinor Fiender: Stack
const EXPERIMENT = "core-085"; // Unlicensed Experimentation: fuses an enemy's played permanent onto yours
const FILLER = "core-005";

const loc = (id: string): number => {
  const value = cardDef(id).loc;
  if (value === undefined) throw new Error(`${id} has no loc in the catalog`);
  return value;
};

const SHRIMP_LOC = loc(SHRIMP);

function pick(id: string): Selection[] {
  return [{ pick: "instance", instanceId: id }];
}

/** The target ids `legalActions` offers Hired Shrimp's play, any zone. */
function offered(s: Scenario, player: PlayerId = "p1"): Set<string> {
  const shrimp = s.hand(player).find((card) => card.defId === SHRIMP);
  if (shrimp === undefined) throw new Error("Hired Shrimp should be in hand");
  const out = new Set<string>();
  for (const action of legalActions(s.state, player)) {
    if (action.type !== "play" || action.instanceId !== shrimp.id) continue;
    for (const selection of action.targets ?? []) {
      expect(Object.keys(selection).sort()).toEqual(["instanceId", "pick"]);
      if (selection.pick === "instance") out.add(selection.instanceId);
    }
  }
  return out;
}

/**
 * p1 controls `mine` (a Unit) and Unlicensed Experimentation; p2 plays `theirs` (a Unit), which is fused
 * onto `mine`; then p2's turn ends and p1 holds Hired Shrimp. Returns the scenario and the fused card.
 */
function fused(mine: string, theirs: string, isRadiant = false): { s: Scenario; fusedId: string } {
  const s = scenario({
    p1: {
      hand: [{ def: SHRIMP, radiant: isRadiant }, FILLER],
      field: [mine],
      backrow: [{ def: EXPERIMENT, faceUp: false }],
      library: [FILLER, FILLER],
    },
    p2: { hand: [theirs, FILLER], library: [FILLER, FILLER], mana: 4 },
    active: "p2",
  });
  s.play(theirs);
  s.expectEvents("trapFired");
  s.endTurn();
  const card = s.unit("p1", 1);
  if (card === null) throw new Error("the fused card should stand in p1's lane 1");
  return { s, fusedId: card.id };
}

describe("C #48 Hired Shrimp", () => {
  it("its lines of code are catalog data, public: the entry carries `loc`, and it declares no numbers", () => {
    expect(def.id).toBe(SHRIMP);
    expect(def.loc).toBe(SHRIMP_LOC);
    expect(def.params).toBeUndefined();
    expect(base.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"] } }]);
    expect(radiant.targets).toEqual([
      { kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"], check: "longer" } },
    ]);
  });

  describe("base", () => {
    it("R397 offers any permanent on either side — face-down cards included, unfiltered — and never itself", () => {
      const s = scenario({
        p1: { hand: [SHRIMP, FILLER], field: [VANILLA], backrow: [MANA_WELL] },
        p2: { hand: [FILLER], field: [CUBE], backrow: [{ def: GAMBIT, faceUp: false }] },
      });
      const shrimp = s.card(SHRIMP);
      const want = [s.card(VANILLA).id, s.card(MANA_WELL).id, s.card(CUBE).id, s.card(GAMBIT).id];
      expect(offered(s)).toEqual(new Set(want));
      expect(offered(s).has(shrimp.id)).toBe(false);
    });

    it("R13 a card dormant under a Stack pile is no permanent on the field: only the pile's top is offered", () => {
      const s = scenario({ p1: { hand: [SHRIMP, FILLER] }, p2: { hand: [FILLER], field: [CUBE, { def: FIENDER, stack: true }] } });
      expect(offered(s)).toEqual(new Set([s.card(FIENDER).id]));
      expect(() => s.play(SHRIMP, { targets: pick(s.card(CUBE).id) })).toThrow();
    });

    it("R397 destroys a target whose loc is greater than its own", () => {
      expect(loc(CUBE)).toBeGreaterThan(SHRIMP_LOC);
      const s = scenario({ p1: { hand: [SHRIMP, FILLER] }, p2: { hand: [FILLER], field: [CUBE] } });
      s.play(SHRIMP, { targets: pick(s.card(CUBE).id) });
      s.expectInZone(CUBE, "graveyard").expectInZone(SHRIMP, "field");
    });

    it("R397 a wrong guess fizzles: a target with fewer lines stays", () => {
      expect(loc(VANILLA)).toBeLessThan(SHRIMP_LOC);
      const s = scenario({ p1: { hand: [SHRIMP, FILLER] }, p2: { hand: [FILLER], field: [VANILLA] } });
      s.play(SHRIMP, { targets: pick(s.card(VANILLA).id) });
      s.expectInZone(VANILLA, "field");
      expect(s.events.some((event) => event.type === "destroyed")).toBe(false);
    });

    it("R397 \"more\" is strictly more: another Hired Shrimp has as many lines and stays", () => {
      const s = scenario({ p1: { hand: [SHRIMP, FILLER] }, p2: { hand: [FILLER], field: [SHRIMP] } });
      const theirs = s.unit("p2", 1);
      if (theirs === null) throw new Error("p2's Hired Shrimp should be on the board");
      s.play(SHRIMP, { targets: pick(theirs.id) });
      s.expectInZone(theirs, "field");
    });

    it("your own permanents may be the target too", () => {
      expect(loc(HONEYPOT)).toBeGreaterThan(SHRIMP_LOC);
      const s = scenario({ p1: { hand: [SHRIMP, FILLER], backrow: [{ def: HONEYPOT, faceUp: false }] }, p2: { hand: [FILLER] } });
      s.play(SHRIMP, { targets: pick(s.card(HONEYPOT).id) });
      s.expectInZone(HONEYPOT, "graveyard");
    });

    it("R177 a face-down card is offered by its id alone and judged at resolution: a longer Trap is destroyed", () => {
      expect(loc(HONEYPOT)).toBeGreaterThan(SHRIMP_LOC);
      const s = scenario({ p1: { hand: [SHRIMP, FILLER] }, p2: { hand: [FILLER], backrow: [{ def: HONEYPOT, faceUp: false }] } });
      const trap = s.backrow("p2", 1);
      if (trap === null) throw new Error("p2's trap should be set");
      expect(offered(s)).toEqual(new Set([trap.id]));
      expect(JSON.stringify(s.view("p1"))).not.toContain(HONEYPOT);
      s.play(SHRIMP, { targets: pick(trap.id) });
      s.expectInZone(trap, "graveyard");
    });

    it("R177 ... and a shorter face-down Trap survives the guess, still face-down and unnamed", () => {
      expect(loc(GAMBIT)).toBeLessThan(SHRIMP_LOC);
      const s = scenario({ p1: { hand: [SHRIMP, FILLER] }, p2: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }] } });
      const trap = s.backrow("p2", 1);
      if (trap === null) throw new Error("p2's trap should be set");
      s.play(SHRIMP, { targets: pick(trap.id) });
      s.expectInZone(trap, "field");
      expect(s.card(trap).faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p1"))).not.toContain(GAMBIT);
    });

    it("R46 an Indestructible target with more lines survives: a fused The Rock and Carnivorous Cube", () => {
      const { s, fusedId } = fused(ROCK, CUBE);
      const fusedLoc = defOf(s.state, s.card(fusedId).defId).loc;
      expect(fusedLoc).toBe(loc(ROCK) + loc(CUBE));
      expect(fusedLoc).toBeGreaterThan(SHRIMP_LOC);
      s.play(SHRIMP, { targets: pick(fusedId) });
      s.expectInZone(fusedId, "field");
      expect(s.stats(fusedId).position).toBe("ATK");
    });

    it("a fused card's loc is its ingredients' sum: Bigot and Gary each have fewer lines, together more", () => {
      expect(loc(BIGOT)).toBeLessThan(SHRIMP_LOC);
      expect(loc(GARY)).toBeLessThan(SHRIMP_LOC);
      expect(loc(BIGOT) + loc(GARY)).toBeGreaterThan(SHRIMP_LOC);
      const { s, fusedId } = fused(BIGOT, GARY);
      const fusedDef = defOf(s.state, s.card(fusedId).defId);
      expect(fusedDef.loc).toBe(loc(BIGOT) + loc(GARY));
      // Public in both views: each seat's view carries the fused definition with its loc (R243).
      for (const viewer of ["p1", "p2"] as const) {
        expect(s.view(viewer).defs?.[fusedDef.id]?.loc).toBe(loc(BIGOT) + loc(GARY));
      }
      s.play(SHRIMP, { targets: pick(fusedId) });
      s.expectInZone(fusedId, "graveyard");
    });

    it("R90 with no permanent on the field it enters anyway, its Cry finding nothing", () => {
      const s = scenario({ p1: { hand: [SHRIMP, FILLER] }, p2: { hand: [FILLER] } });
      expect(offered(s)).toEqual(new Set());
      s.play(SHRIMP);
      s.expectInZone(SHRIMP, "field").expectStats(SHRIMP, { attack: 4, health: 3 });
    });
  });

  describe("radiant", () => {
    it("R397 offers only the permanents with more lines, plus a face-down card you may not read whatever its lines", () => {
      expect(loc(CUBE)).toBeGreaterThan(SHRIMP_LOC);
      expect(loc(VANILLA)).toBeLessThan(SHRIMP_LOC);
      expect(loc(MANA_WELL)).toBeLessThan(SHRIMP_LOC);
      expect(loc(GAMBIT)).toBeLessThan(SHRIMP_LOC);
      expect(loc(HONEYPOT)).toBeGreaterThan(SHRIMP_LOC);
      const s = scenario({
        p1: {
          hand: [{ def: SHRIMP, radiant: true }, FILLER],
          field: [VANILLA],
          backrow: [{ def: GAMBIT, faceUp: false }, { def: HONEYPOT, faceUp: false }],
        },
        p2: { hand: [FILLER], field: [CUBE, VANILLA], backrow: [MANA_WELL, { def: GAMBIT, faceUp: false }] },
      });
      const mine = { gambit: s.backrow("p1", 1), honeypot: s.backrow("p1", 2) };
      const theirs = { cube: s.unit("p2", 1), gambit: s.backrow("p2", 2) };
      // Your own face-down cards you read, so they are judged now; the opponent's is offered unread.
      expect(offered(s)).toEqual(new Set([mine.honeypot?.id, theirs.cube?.id, theirs.gambit?.id]));
    });

    it("R397 an offered target with more lines is destroyed", () => {
      const s = scenario({ p1: { hand: [{ def: SHRIMP, radiant: true }, FILLER] }, p2: { hand: [FILLER], field: [CUBE, VANILLA] } });
      s.play(SHRIMP, { targets: pick(s.card(CUBE).id) });
      s.expectInZone(CUBE, "graveyard");
      s.expectStats(SHRIMP, { attack: 8, health: 6 });
    });

    it("R90 a target that is not highlighted is refused, and legalActions agrees", () => {
      const s = scenario({ p1: { hand: [{ def: SHRIMP, radiant: true }, FILLER] }, p2: { hand: [FILLER], field: [VANILLA, CUBE] } });
      const vanilla = s.card(VANILLA);
      expect(offered(s).has(vanilla.id)).toBe(false);
      expect(() => s.play(SHRIMP, { targets: pick(vanilla.id) })).toThrow();
      s.expectInZone(SHRIMP, "hand");
    });

    it("R177 a face-down card it may not read is judged at resolution: a shorter Trap survives, unnamed", () => {
      const s = scenario({
        p1: { hand: [{ def: SHRIMP, radiant: true }, FILLER] },
        p2: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }] },
      });
      const trap = s.backrow("p2", 1);
      if (trap === null) throw new Error("p2's trap should be set");
      s.play(SHRIMP, { targets: pick(trap.id) });
      s.expectInZone(trap, "field");
      expect(JSON.stringify(s.view("p1"))).not.toContain(GAMBIT);
    });

    it("R177 ... and a longer one is destroyed", () => {
      const s = scenario({
        p1: { hand: [{ def: SHRIMP, radiant: true }, FILLER] },
        p2: { hand: [FILLER], backrow: [{ def: HONEYPOT, faceUp: false }] },
      });
      const trap = s.backrow("p2", 1);
      if (trap === null) throw new Error("p2's trap should be set");
      s.play(SHRIMP, { targets: pick(trap.id) });
      s.expectInZone(trap, "graveyard");
    });

    it("R177 the offer never reveals a face-down card's lines: a shorter and a longer Trap are offered alike", () => {
      const short = scenario({ p1: { hand: [{ def: SHRIMP, radiant: true }, FILLER] }, p2: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }] } });
      const long = scenario({ p1: { hand: [{ def: SHRIMP, radiant: true }, FILLER] }, p2: { hand: [FILLER], backrow: [{ def: HONEYPOT, faceUp: false }] } });
      expect([...offered(short)]).toEqual([short.backrow("p2", 1)?.id]);
      expect([...offered(long)]).toEqual([long.backrow("p2", 1)?.id]);
    });

    it("none qualifies and no face-down card: no target, and it enters anyway", () => {
      const s = scenario({ p1: { hand: [{ def: SHRIMP, radiant: true }, FILLER], field: [VANILLA] }, p2: { hand: [FILLER], field: [VANILLA] } });
      expect(offered(s)).toEqual(new Set());
      s.play(SHRIMP);
      s.expectInZone(SHRIMP, "field");
      expect(s.events.some((event) => event.type === "destroyed")).toBe(false);
    });

    it("a fused card qualifies by its ingredients' sum, and is destroyed", () => {
      const { s, fusedId } = fused(BIGOT, GARY, true);
      expect(offered(s).has(fusedId)).toBe(true);
      s.play(SHRIMP, { targets: pick(fusedId) });
      s.expectInZone(fusedId, "graveyard");
    });
  });
});
