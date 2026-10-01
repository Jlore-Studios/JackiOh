// C #32 Felinor Feelings — SPEC §8.6 row 32, BUILD M9 Classic row C 32: "The target is declared with
// the play (R81): any enemy permanent (the top of a unit pile, a backrow card, a face-down card) in a
// lane where a Unit you control costs exactly (1) (R65, its `costMod` counting; an X Unit its X, 0 with
// none chosen, R396); no such target → the play is still legal and the steal fizzles (§8's
// conventions, R90); stolen per §6.3 (its own lane if free, else the first free zone; none → it stays,
// R15), an entry that leaves it summoning sick (R171); a stolen face-down trap is read by you from then
// on and no longer by its owner (R33); a face-down option carries only its id (R177); radiant: first
// summon a Felinor Token into your leftmost open zone, then pick the steal in a prompt, so the token's
// lane counts; a full board summons no token and the prompt reads the rest; no target then → nothing;
// no tuned numbers".

import { describe, expect, it } from "vitest";
import { findInstance, legalActions, reduce, type GameState } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/032-felinor-feelings";

const FEELINGS = "classic-032";
const TIMMY = "core-011"; // (1) Unit 3/3.
const VANILLA = "core-008"; // (1) Unit 4/4.
const POINTMASTER = "core-020"; // (2) Unit 7/1.
const MENACE = "core-019"; // (3) Unit 9/9.
const BEAR = "core-060"; // (1) Trap.
const BILLY = "classicplus-069"; // (X) Unit.
const TOKEN = "core-t-felinor";
const FILLER = "core-010";

function setup(p1: SideSetup, p2: SideSetup, radiantFace = false): Scenario {
  return scenario({
    p1: { hand: [{ def: FEELINGS, radiant: radiantFace }, FILLER], ...p1 },
    p2: { hand: [FILLER], ...p2 },
  });
}

function at(card: { id: string }): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

function mustUnit(s: Scenario, player: "p1" | "p2", lane: number) {
  const unit = s.unit(player, lane);
  if (unit === null) throw new Error(`no unit in ${player} lane ${lane}`);
  return unit;
}

/** The targets `legalActions` offers this play, by instance id. */
function offered(s: Scenario): string[] {
  const card = s.card(FEELINGS);
  return legalActions(s.state, "p1").flatMap((action) =>
    action.type === "play" && action.instanceId === card.id
      ? (action.targets ?? []).flatMap((selection) => (selection.pick === "instance" ? [selection.instanceId] : []))
      : [],
  );
}

describe("C #32 Felinor Feelings", () => {
  it("has no declared numbers; the base face declares its target, the Radiant face asks later", () => {
    expect(def.id).toBe(FEELINGS);
    expect(def.params).toBeUndefined();
    expect(base.targets?.[0]?.filter?.check).toBe("felinorLane");
    expect(radiant.targets).toBeUndefined();
  });

  describe("base", () => {
    it("R81 steals the declared enemy Unit in a lane where you control a (1) Cost Unit", () => {
      const s = setup({ field: [{ def: TIMMY, lane: 2 }] }, { field: [{ def: VANILLA, lane: 2 }] });
      const vanilla = mustUnit(s, "p2", 2);

      s.play(FEELINGS, { targets: at(vanilla) });

      expect(s.card(vanilla).controller).toBe("p1");
      expect(s.unit("p2", 2)).toBeNull();
    });

    it("R15 the stolen Unit takes its own lane when free, else your first free zone", () => {
      // Lane 2 on p1's side holds the Timmy, so the Vanilla lands in p1's lane 1.
      const s = setup({ field: [{ def: TIMMY, lane: 2 }] }, { field: [{ def: VANILLA, lane: 2 }] });
      const vanilla = mustUnit(s, "p2", 2);

      s.play(FEELINGS, { targets: at(vanilla) });

      expect(s.unit("p1", 1)?.id).toBe(vanilla.id);
    });

    it("§10.6 only permanents in such lanes are offered; another lane's is refused", () => {
      const s = setup(
        { field: [{ def: TIMMY, lane: 2 }, { def: POINTMASTER, lane: 3 }] },
        { field: [{ def: VANILLA, lane: 2 }, { def: MENACE, lane: 3 }] },
      );
      const vanilla = mustUnit(s, "p2", 2);
      const menace = mustUnit(s, "p2", 3);

      expect(offered(s)).toContain(vanilla.id);
      expect(offered(s)).not.toContain(menace.id);
      expect(() => s.play(FEELINGS, { targets: at(menace) })).toThrow();
    });

    it("R65 a Unit's cost changes count: a (2) Cost Unit made (1) opens its lane", () => {
      const s = setup({ field: [{ def: POINTMASTER, lane: 3, costMod: -1 }] }, { field: [{ def: MENACE, lane: 3 }] });
      const menace = mustUnit(s, "p2", 3);

      s.play(FEELINGS, { targets: at(menace) });

      expect(s.card(menace).controller).toBe("p1");
    });

    it("R396 an X Unit costs the X it was played for, and 0 with none chosen", () => {
      const s = setup(
        { field: [{ def: BILLY, lane: 1, statsOverride: { attack: 3, health: 3 } }] },
        { field: [{ def: MENACE, lane: 1 }] },
      );
      const billy = mustUnit(s, "p1", 1);
      const menace = mustUnit(s, "p2", 1);

      delete billy.x;
      expect(offered(s)).not.toContain(menace.id);
      billy.x = 1;
      expect(offered(s)).toContain(menace.id);
    });

    it("a backrow card in that lane may be taken, face-down included, and lands in your backrow", () => {
      const s = setup({ field: [{ def: TIMMY, lane: 4 }] }, { backrow: [{ def: BEAR, faceUp: false, lane: 4 }] });
      const bear = s.card(BEAR);

      s.play(FEELINGS, { targets: at(bear) });

      expect(s.backrow("p1", 4)?.id).toBe(bear.id);
      expect(s.card(bear).controller).toBe("p1");
    });

    it("R33 a stolen face-down trap is read by you from then on, and no longer by its owner", () => {
      const s = setup({ field: [{ def: TIMMY, lane: 4 }] }, { backrow: [{ def: BEAR, faceUp: false, lane: 4 }] });
      const bear = s.card(BEAR);
      expect(JSON.stringify(s.view("p1").opponent.backrow)).not.toContain(BEAR);

      s.play(FEELINGS, { targets: at(bear) });

      expect(JSON.stringify(s.view("p1").you.backrow)).toContain(BEAR);
      expect(JSON.stringify(s.view("p2").opponent.backrow)).not.toContain(BEAR);
    });

    it("R177 a face-down option is offered by its id alone: the play's targets name no hidden card", () => {
      const s = setup({ field: [{ def: TIMMY, lane: 4 }] }, { backrow: [{ def: BEAR, faceUp: false, lane: 4 }] });
      const bear = s.card(BEAR);

      expect(offered(s)).toEqual([bear.id]);
      // The option is the bare id: nothing p1 is shown names the card beneath it.
      expect(offered(s).every((id) => id === bear.id)).toBe(true);
      expect(JSON.stringify(s.view("p1"))).not.toContain(BEAR);
    });

    it("R13 a (1) Cost Unit of yours dormant under a Stack pile opens no lane", () => {
      const s = setup(
        { field: [{ def: TIMMY, lane: 2 }, { def: POINTMASTER, stack: true }] },
        { field: [{ def: MENACE, lane: 2 }] },
      );

      expect(offered(s)).toEqual([]);
      expect(() => s.play(FEELINGS, { targets: at(mustUnit(s, "p2", 2)) })).toThrow();
    });

    it("R13 an enemy Stack pile offers its top, never the card dormant beneath it", () => {
      const s = setup(
        { field: [{ def: TIMMY, lane: 2 }] },
        { field: [{ def: VANILLA, lane: 2 }, { def: MENACE, stack: true }] },
      );
      const dormant = s.card(VANILLA);
      const top = mustUnit(s, "p2", 2);

      expect(offered(s)).toEqual([top.id]);
      expect(() => s.play(FEELINGS, { targets: at(dormant) })).toThrow();
    });

    it("R90 with no such target the play is still legal and the steal fizzles", () => {
      const s = setup({ field: [{ def: POINTMASTER, lane: 1 }] }, { field: [{ def: MENACE, lane: 1 }] });

      s.play(FEELINGS);

      expect(mustUnit(s, "p2", 1).controller).toBe("p2");
      s.expectInZone(FEELINGS, "graveyard");
    });

    it("R15 with no free zone on your side the stolen card stays with its owner", () => {
      const s = setup(
        { field: [TIMMY, VANILLA, VANILLA, VANILLA, VANILLA] },
        { field: [{ def: MENACE, lane: 1 }] },
      );
      const menace = mustUnit(s, "p2", 1);

      s.play(FEELINGS, { targets: at(menace) });

      expect(s.card(menace).controller).toBe("p2");
      expect(s.unit("p2", 1)?.id).toBe(menace.id);
    });

    it("R171 the steal is an entry: the stolen Unit is summoning sick", () => {
      const s = setup({ field: [{ def: TIMMY, lane: 2 }] }, { field: [{ def: POINTMASTER, lane: 2 }] });
      const point = mustUnit(s, "p2", 2);

      s.play(FEELINGS, { targets: at(point) });

      expect(() => s.attack(point, "hero")).toThrow();
    });
  });

  describe("radiant", () => {
    it("summons a Felinor Token first, then asks, so the token's lane counts", () => {
      const s = setup({}, { field: [{ def: MENACE, lane: 1 }] }, true);
      const menace = mustUnit(s, "p2", 1);

      s.play(FEELINGS);

      expect(s.unit("p1", 1)?.defId).toBe(TOKEN);
      expect(s.state.pending?.playerId).toBe("p1");
      expect(s.state.pending?.options.map((option) => option.selection)).toEqual(at(menace));
      s.answer(at(menace));

      expect(s.card(menace).controller).toBe("p1");
      expect(s.unit("p1", 2)?.id).toBe(menace.id);
    });

    it("§9.3 the open steal prompt survives a JSON round trip and resumes through reduce", () => {
      const s = setup({}, { field: [{ def: MENACE, lane: 1 }] }, true);
      const menace = mustUnit(s, "p2", 1);
      s.play(FEELINGS);
      const paused = s.state;
      const revived = JSON.parse(JSON.stringify(paused)) as GameState;
      expect(revived).toEqual(paused);

      const result = reduce(revived, {
        type: "answer",
        playerId: "p1",
        choiceId: revived.pending?.id ?? "",
        selection: at(menace),
        nonce: "feelings-round-trip",
      });

      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
      expect(findInstance(result.state, menace.id)?.controller).toBe("p1");
    });

    it("offers only permanents in such lanes", () => {
      const s = setup({ field: [{ def: POINTMASTER, lane: 1 }] }, { field: [{ def: MENACE, lane: 2 }, { def: VANILLA, lane: 3 }] }, true);

      s.play(FEELINGS);

      // The token lands in lane 2 (lane 1 is taken): Menace's lane counts, Vanilla's does not.
      expect(s.unit("p1", 2)?.defId).toBe(TOKEN);
      expect(s.state.pending?.options.map((option) => option.selection)).toEqual(at(mustUnit(s, "p2", 2)));
    });

    it("a full board summons no token, and the prompt reads the lanes as they are", () => {
      const s = setup(
        { field: [POINTMASTER, POINTMASTER, { def: TIMMY, lane: 3 }, POINTMASTER, POINTMASTER] },
        { field: [{ def: MENACE, lane: 3 }] },
        true,
      );

      s.play(FEELINGS);

      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
      expect(s.state.pending?.options.map((option) => option.selection)).toEqual(at(mustUnit(s, "p2", 3)));
      s.answer(at(mustUnit(s, "p2", 3)));
      // No free zone: it stays with its owner (R15).
      expect(mustUnit(s, "p2", 3).controller).toBe("p2");
    });

    it("with no such permanent it asks nothing", () => {
      const s = setup({}, { field: [{ def: MENACE, lane: 4 }] }, true);

      s.play(FEELINGS);

      expect(s.unit("p1", 1)?.defId).toBe(TOKEN);
      expect(s.state.pending).toBeNull();
      expect(mustUnit(s, "p2", 4).controller).toBe("p2");
    });

    it("R177 a face-down option in the prompt names no card in your view", () => {
      const s = setup({}, { backrow: [{ def: BEAR, faceUp: false, lane: 1 }] }, true);

      s.play(FEELINGS);

      expect(s.state.pending).not.toBeNull();
      const pending = s.view("p1").pending;
      expect(JSON.stringify(pending)).toContain(`"${s.card(BEAR).id}"`);
      expect(JSON.stringify(s.view("p1"))).not.toContain(BEAR);
      s.answer(at(s.card(BEAR)));
      expect(s.backrow("p1", 1)?.defId).toBe(BEAR);
    });
  });
});
