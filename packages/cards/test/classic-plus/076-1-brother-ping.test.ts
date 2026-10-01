// C+ #76.1 Brother Ping — SPEC §8.7 row 76.1, BUILD M9 Classic+ row C+ 76.1: "Pierce; Activate (R384):
// its controller, in their main phase with no prompt open, deals 1 damage to a target declared in the
// `activate` action, once per turn; sickness and exertion don't apply, so it activates the turn it
// arrives; the hit has Pierce (R346) and no Spell Damage; activating is no play (Combo, Quickstriker,
// Ceaseless Void ignore it); a second use that turn and any use on the opponent's turn are refused by the
// same check `legalActions` lists; its count resets on leaving the field (R78); `activated` is public; an
// Upgrade makes it Activate 2 and a Degrade never takes it below 1 (R386); Activate count and damage read
// through `param()`; radiant 8/8 and Activate 2: two uses a turn, a third refused".
//
// "Its count resets on leaving the field": Brother Ping is a unit token, so leaving the field it ceases
// to exist (R11) and no instance comes back with a count; the reset itself (R78's `memory`) is the
// activate subsystem's, proved in packages/engine/test/activate.test.ts.

import { legalActions, stepParam } from "@jackioh/engine";
import type { PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/076-1-brother-ping";

const PING = "classicplus-076-1";
const LAR = "classicplus-076";
const FILLER = "core-005";
const MENACE = "core-019"; // 9/9 Taunt
const ARMORED = "core-025"; // 7/7, Armor 7
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const CEASELESS_VOID = "core-100"; // costs (1) less for each card played this game, R55's pattern
const HIT_JOB = "core-016";

const ENEMY_HERO: Selection[] = [{ pick: "hero", player: "p2" }];

function listed(s: Scenario, player: PlayerId, instanceId: string): unknown[] {
  return legalActions(s.state, player).filter(
    (action) => (action.type === "activate" || action.type === "activatePower") && action.instanceId === instanceId,
  );
}

function setup(opts: { radiant?: boolean; p1Field?: readonly string[]; p2Field?: readonly string[] } = {}): Scenario {
  return scenario({
    p1: {
      hand: [FILLER, FILLER],
      field: [{ def: PING, ...(opts.radiant === true ? { radiant: true } : {}) }, ...(opts.p1Field ?? [])],
      library: [FILLER, FILLER],
    },
    p2: { hand: [FILLER], field: [...(opts.p2Field ?? [])], library: [FILLER, FILLER] },
  });
}

describe("C+ #76.1 Brother Ping", () => {
  it("is a (2) 4/4 Pierce token (printed Rare) with one Activate on each face: once, Radiant twice", () => {
    expect(def.id).toBe(PING);
    expect(def.printedRarity).toBe("Rare");
    expect([def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([4, 4, 8, 8]);
    expect(def.base.keywords).toEqual([{ kind: "Pierce" }]);
    expect(base.activations?.map((ability) => ability.uses)).toEqual([1]);
    expect(radiant.activations?.map((ability) => ability.uses)).toEqual([2]);
    const decl = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];
    expect(base.activations?.[0]?.targets).toEqual(decl);
  });

  describe("base", () => {
    it("R384 activating deals 1 damage to the target the action declares; `activated` is public", () => {
      const s = setup({ p2Field: [MENACE] });
      s.activate(PING, { targets: [{ pick: "instance", instanceId: s.card(MENACE).id }] });
      s.expectStats(MENACE, { health: 8 });
      const theirs = s.view("p2").events.filter((event) => event.type === "activated");
      expect(theirs).toEqual([expect.objectContaining({ type: "activated", instanceId: s.card(PING).id, defId: PING })]);
    });

    it("R384 it may hit either hero or a Unit of its own side", () => {
      const s = setup({ p1Field: [MENACE] });
      s.activate(PING, { targets: ENEMY_HERO });
      s.expectHealth("p2", 29);
      s.endTurn().endTurn();
      s.activate(PING, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", 29);
      s.endTurn().endTurn();
      s.activate(PING, { targets: [{ pick: "instance", instanceId: s.card(MENACE).id }] });
      s.expectStats(MENACE, { health: 8 });
    });

    it("R346 the hit has Pierce: Armor 7 takes none of it", () => {
      const s = setup({ p2Field: [ARMORED] });
      s.activate(PING, { targets: [{ pick: "instance", instanceId: s.card(ARMORED).id }] });
      s.expectStats(ARMORED, { health: 6 });
    });

    it("E6 it is no Spell's hit, so Spell Damage never raises it", () => {
      const s = setup({ p1Field: [SOLARIUS] });
      s.activate(PING, { targets: ENEMY_HERO });
      s.expectHealth("p2", 29);
    });

    it("R384 it activates the turn it arrives: Brother Lar's Death leaves a Ping that can be used at once", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [LAR] }, p2: { hand: [FILLER] } });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(LAR).id }] });
      const ping = s.unit("p1", 1);
      if (ping === null) throw new Error("no Ping");
      expect(listed(s, "p1", ping.id).length).toBeGreaterThan(0);
      s.activate(ping, { targets: ENEMY_HERO });
      s.expectHealth("p2", 29);
    });

    it("R384 activating spends no exertion: it may still attack that turn", () => {
      const s = setup();
      s.activate(PING, { targets: ENEMY_HERO }).attack(PING, "hero");
      s.expectHealth("p2", 25);
    });

    it("R384 activating is not a play: no cardPlayed, the play count and Ceaseless Void's price stand still", () => {
      const s = scenario({ p1: { hand: [CEASELESS_VOID, FILLER], field: [PING] }, p2: { hand: [FILLER] } });
      const played = s.state.players.p1.turnLog.cardsPlayed;
      const price = s.view("p1").you.hand;
      s.activate(PING, { targets: ENEMY_HERO });
      expect(s.lastEvents.some((event) => event.type === "cardPlayed")).toBe(false);
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(played);
      expect(s.view("p1").you.hand).toEqual(price);
    });

    it("R384 a second use that turn is refused and not listed; it is back the next turn", () => {
      const s = setup();
      const ping = s.card(PING);
      s.activate(ping, { targets: ENEMY_HERO });
      expect(listed(s, "p1", ping.id)).toHaveLength(0);
      expect(() => s.activate(ping, { targets: ENEMY_HERO })).toThrow();
      s.expectHealth("p2", 29);
      s.endTurn().endTurn();
      expect(listed(s, "p1", ping.id).length).toBeGreaterThan(0);
      s.activate(ping, { targets: ENEMY_HERO });
      s.expectHealth("p2", 28);
    });

    it("R78 its count resets on leaving the field: used, destroyed and back by a granted Reborn, it may activate again that turn", () => {
      const s = scenario({ p1: { hand: [HIT_JOB, FILLER], field: [PING], library: [FILLER] }, p2: { hand: [FILLER] } });
      s.card(PING).grantedKeywords = [{ kind: "Reborn" }];
      s.activate(PING, { targets: ENEMY_HERO });
      expect(listed(s, "p1", s.card(PING).id)).toHaveLength(0);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.unit("p1", 1)!.id }] });
      const back = s.unit("p1", 1);
      expect(back?.defId).toBe(PING);
      expect(listed(s, "p1", back!.id).length).toBeGreaterThan(0);
      s.activate(back!, { targets: ENEMY_HERO });
      s.expectHealth("p2", 28);
    });

    it("R384 on the opponent's turn it is neither listed nor accepted", () => {
      const s = scenario({ active: "p2", p1: { hand: [FILLER], field: [PING] }, p2: { hand: [FILLER] } });
      const ping = s.card(PING);
      expect(listed(s, "p1", ping.id)).toHaveLength(0);
      expect(() => s.activate(ping, { targets: ENEMY_HERO })).toThrow();
    });

    it("R384 legalActions lists one action per legal target, the heroes and the Units", () => {
      const s = setup({ p2Field: [MENACE] });
      const targets = listed(s, "p1", s.card(PING).id).map((action) => JSON.stringify((action as { targets?: unknown }).targets));
      expect(targets).toContain(JSON.stringify(ENEMY_HERO));
      expect(targets).toContain(JSON.stringify([{ pick: "instance", instanceId: s.card(MENACE).id }]));
    });

    it("R386 an Upgrade of its X makes it Activate 2; a Degrade never takes it below 1", () => {
      const up = setup();
      up.card(PING).tuning = { x: { Activate: 1 } };
      up.activate(PING, { targets: ENEMY_HERO }).activate(PING, { targets: ENEMY_HERO });
      up.expectHealth("p2", 28);
      expect(() => up.activate(PING, { targets: ENEMY_HERO })).toThrow();

      const down = setup();
      down.card(PING).tuning = { x: { Activate: -3 } };
      down.activate(PING, { targets: ENEMY_HERO });
      down.expectHealth("p2", 29);
      expect(() => down.activate(PING, { targets: ENEMY_HERO })).toThrow();
    });

    it("R386 an Upgrade of its damage deals 2; a Degrade never below 1", () => {
      const up = setup();
      stepParam(up.card(PING), "damage", 1);
      up.activate(PING, { targets: ENEMY_HERO });
      up.expectHealth("p2", 28);

      const down = setup();
      stepParam(down.card(PING), "damage", -1);
      down.activate(PING, { targets: ENEMY_HERO });
      down.expectHealth("p2", 29);
    });
  });

  describe("radiant", () => {
    it("R384 an 8/8 with Activate 2: two uses a turn, a third refused and not listed", () => {
      const s = setup({ radiant: true });
      const ping = s.card(PING);
      s.expectStats(ping, { attack: 8, maxHealth: 8 });
      s.activate(ping, { targets: ENEMY_HERO });
      expect(listed(s, "p1", ping.id).length).toBeGreaterThan(0);
      s.activate(ping, { targets: ENEMY_HERO });
      s.expectHealth("p2", 28);
      expect(listed(s, "p1", ping.id)).toHaveLength(0);
      expect(() => s.activate(ping, { targets: ENEMY_HERO })).toThrow();
    });

    it("R346 the Radiant hits pierce too", () => {
      const s = setup({ radiant: true, p2Field: [ARMORED] });
      const armored = [{ pick: "instance" as const, instanceId: s.card(ARMORED).id }];
      s.activate(PING, { targets: armored }).activate(PING, { targets: armored });
      s.expectStats(ARMORED, { health: 5 });
    });

    it("R386 an Upgrade of the Radiant X makes it Activate 3", () => {
      const s = setup({ radiant: true });
      s.card(PING).tuning = { x: { Activate: 1 } };
      s.activate(PING, { targets: ENEMY_HERO }).activate(PING, { targets: ENEMY_HERO }).activate(PING, { targets: ENEMY_HERO });
      s.expectHealth("p2", 27);
      expect(() => s.activate(PING, { targets: ENEMY_HERO })).toThrow();
    });
  });
});
