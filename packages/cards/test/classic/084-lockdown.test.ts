// C #84 Lockdown (SPEC §8.6 row 84; BUILD M9 row C 84). (2) Field Spell, Rare: Indestructible; after
// a permanent is played, Lock its zone; Activate: Tribute this. Radiant: after your opponent plays a
// permanent.
//
// A countered card is shown with the real C #72 Grand Counterspell. No card in the catalog casts a
// permanent by its effect (C #7, #47 and #56 cast Spells only; no Cast-on-draw card with a script is a
// permanent), so "a cast counts" uses a fixture Spell that casts a Mr. Vanilla (`castNew`, R70), as
// `test/combat-windows.test.ts` does.

import { describe, expect, it } from "vitest";
import { isLocked, legalActions, newInstance, registerScripts, registeredScripts, type Script } from "@jackioh/engine";
import { castNew } from "@jackioh/engine/effects";
import type { CardDef, CardType, PlayerId, Row } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const LOCKDOWN = "classic-084";
const VANILLA = "core-008"; // 4/4
const TOKEN_MAKER = "core-015"; // Me and Mr Token: Cry: summon a Rush Token
const DEFENDER = "core-003"; // Right-house defender: Reborn
const CALL_TO_ARMS = "core-069"; // Spell 2: Recruit 3 (1) Cost or less Units
const HIT_JOB = "core-016";
const MAGIC_JAMMED = "core-036"; // destroy target backrow card, Lock its zone
const COLLATERAL = "core-034"; // exile target permanent
const MANA_WELL = "core-006"; // Field Spell
const SHEEPISH = "core-041"; // Trap
const STOCKPILE = "core-005";
const TIMMY = "core-011";
const GRAND_COUNTERSPELL = "classic-072"; // Trap: when your opponent plays a non-Unit card, Counter it.

const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA, VANILLA] };

function locked(s: Scenario, player: PlayerId, row: Row, lane: number): boolean {
  return isLocked(s.state, { player, row, lane });
}

function lockEvents(s: Scenario): string[] {
  return s.lastEvents.flatMap((e) => (e.type === "locked" ? [`${e.player} ${e.row} ${e.lane}`] : []));
}

/** A fixture card: a transient def in the match state and its script in the registry. */
function fixture(s: Scenario, id: string, type: CardType, script: Script): string {
  const face = { keywords: [], text: id };
  const def: CardDef = { id, index: id, name: id, set: "Core", type, tags: [], rarity: "Common", token: false, cost: 0, base: face, radiant: face };
  s.state.transientDefs[id] = def;
  registerScripts({ ...registeredScripts(), [id]: { base: script, radiant: script } });
  return id;
}

describe("C #84 Lockdown", () => {
  describe("base", () => {
    it("§8.6 after a Unit is played its zone Locks, and the Unit stays", () => {
      const s = scenario({ p1: { backrow: [LOCKDOWN], hand: [VANILLA, STOCKPILE], library: SPARE.library }, p2: SPARE });
      s.play(VANILLA, { zone: 3 });
      expect(lockEvents(s)).toEqual(["p1 units 3"]);
      expect(s.unit("p1", 3)?.defId).toBe(VANILLA);
    });

    it("§8.6 a Field Spell's zone and a face-down Trap's zone lock too, the other seat never told which Trap (R33)", () => {
      const s = scenario({ p1: { backrow: [LOCKDOWN], hand: [MANA_WELL, SHEEPISH, STOCKPILE], library: SPARE.library, mana: 10 }, p2: SPARE });
      s.play(MANA_WELL, { zone: 2 });
      expect(lockEvents(s)).toEqual(["p1 backrow 2"]);
      s.play(SHEEPISH, { zone: 3 });
      expect(lockEvents(s)).toEqual(["p1 backrow 3"]);
      expect(s.backrow("p1", 3)?.defId).toBe(SHEEPISH);
      const seen = s.view("p2").events;
      expect(seen.filter((e) => e.type === "locked")).toContainEqual({ type: "locked", player: "p1", row: "backrow", lane: 3 });
      // No event p2 is shown names the Trap.
      const trap = s.backrow("p1", 3)!;
      expect(JSON.stringify(seen)).not.toContain(SHEEPISH);
      expect(JSON.stringify(seen)).not.toContain(`"${trap.id}"`);
    });

    it("§8.6 either player's plays: the opponent's Unit Locks its zone on their turn", () => {
      const s = scenario({ active: "p2", p1: { backrow: [LOCKDOWN], ...SPARE }, p2: { hand: [VANILLA, STOCKPILE], library: SPARE.library } });
      s.play(VANILLA, { zone: 4 });
      expect(locked(s, "p2", "units", 4)).toBe(true);
    });

    it("§8.6 a Spell stands in no zone and locks nothing", () => {
      const s = scenario({ p1: { backrow: [LOCKDOWN], hand: [STOCKPILE, TIMMY], library: SPARE.library }, p2: SPARE });
      s.play(STOCKPILE);
      expect(lockEvents(s)).toEqual([]);
    });

    it("R119 Lockdown does not answer its own arrival", () => {
      const s = scenario({ p1: { hand: [LOCKDOWN, STOCKPILE], library: SPARE.library }, p2: SPARE });
      s.play(LOCKDOWN, { zone: 1 });
      expect(lockEvents(s)).toEqual([]);
      expect(locked(s, "p1", "backrow", 1)).toBe(false);
    });

    it("R70 a summon that is no play locks nothing: a Cry's token, a Recruit, a Reborn", () => {
      const s = scenario({
        p1: { backrow: [LOCKDOWN], field: [{ def: DEFENDER, lane: 5 }], hand: [TOKEN_MAKER, CALL_TO_ARMS, HIT_JOB, STOCKPILE], library: [TIMMY, VANILLA], mana: 20 },
        p2: SPARE,
      });
      // Me and Mr Token is played (lane 1, Locked); the Rush Token it summons (lane 2) is not.
      s.play(TOKEN_MAKER, { zone: 1 });
      expect(lockEvents(s)).toEqual(["p1 units 1"]);
      expect(s.unit("p1", 2)?.defId).toBe("core-t-rush");
      // Call to Arms recruits Timmy into lane 3: no play, no Lock.
      s.play(CALL_TO_ARMS);
      expect(s.unit("p1", 3)?.defId).toBe(TIMMY);
      expect(lockEvents(s)).toEqual([]);
      // The Defender dies and comes back through Reborn: no play, no Lock.
      const defender = s.unit("p1", 5)!;
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: defender.id }] });
      expect(s.unit("p1", 5)?.id).toBe(defender.id);
      expect(locked(s, "p1", "units", 5)).toBe(false);
    });

    it("R70 a cast by an effect counts as a play: the cast Unit's zone Locks, the casting Spell's nothing", () => {
      const s = scenario({ p1: { backrow: [LOCKDOWN], hand: [STOCKPILE, TIMMY], library: SPARE.library }, p2: SPARE });
      const caster = fixture(s, "fx-cast-vanilla", "Spell", { cry: () => [castNew({ def: VANILLA })] });
      const spell = newInstance(s.state, caster, "p1", { z: "hand", player: "p1" });
      s.state.players.p1.hand.push(spell);
      s.play(spell);
      const cast = s.unit("p1", 1)!;
      expect(cast.defId).toBe(VANILLA);
      expect(s.lastEvents.some((e) => e.type === "cardPlayed" && e.instanceId === cast.id)).toBe(true);
      expect(lockEvents(s)).toEqual(["p1 units 1"]);
    });

    it("§8.6 a countered card locks nothing: C #72 Grand Counterspell stops a Field Spell before it reaches a zone", () => {
      const s = scenario({
        p1: { backrow: [LOCKDOWN], hand: [MANA_WELL, STOCKPILE], library: SPARE.library },
        p2: { ...SPARE, backrow: [{ def: GRAND_COUNTERSPELL, faceUp: false }] },
      });
      s.play(MANA_WELL, { zone: 3 });
      expect(s.lastEvents.some((e) => e.type === "countered")).toBe(true);
      s.expectInZone(MANA_WELL, "graveyard");
      expect(lockEvents(s)).toEqual([]);
      expect(locked(s, "p1", "backrow", 3)).toBe(false);
    });

    it("R13 a Lockdown dormant under a backrow pile does not act: a play locks nothing", () => {
      const s = scenario({ p1: { backrow: [LOCKDOWN, { def: MANA_WELL, stack: true }], hand: [VANILLA, STOCKPILE], library: SPARE.library }, p2: SPARE });
      s.play(VANILLA, { zone: 2 });
      expect(lockEvents(s)).toEqual([]);
    });

    it("R46 Indestructible: a destroy leaves it; an exile removes it", () => {
      const s = scenario({ active: "p2", p1: { backrow: [LOCKDOWN], ...SPARE }, p2: { hand: [MAGIC_JAMMED, COLLATERAL, STOCKPILE], library: SPARE.library, mana: 10 } });
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: s.card(LOCKDOWN).id }] });
      s.expectInZone(LOCKDOWN, "field");
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(LOCKDOWN).id }] });
      s.expectInZone(LOCKDOWN, "exile");
    });

    it("R384 Activate: Tribute this, which bypasses Indestructible; the Locks it made stay", () => {
      const s = scenario({ p1: { backrow: [LOCKDOWN], hand: [VANILLA, STOCKPILE], library: SPARE.library }, p2: SPARE });
      s.play(VANILLA, { zone: 2 });
      const lockdown = s.card(LOCKDOWN);
      expect(legalActions(s.state, "p1").some((a) => a.type === "activate" && a.instanceId === lockdown.id)).toBe(true);
      expect(legalActions(s.state, "p2").some((a) => a.type === "activate")).toBe(false);
      s.activate(LOCKDOWN, { ability: "tribute" });
      s.expectInZone(lockdown, "graveyard");
      expect(locked(s, "p1", "units", 2)).toBe(true);
    });
  });

  describe("radiant", () => {
    it("§8.6 only the opponent's plays: your own Unit locks nothing, theirs locks its zone", () => {
      const s = scenario({
        p1: { backrow: [{ def: LOCKDOWN, radiant: true }], hand: [VANILLA, STOCKPILE], library: SPARE.library },
        p2: { hand: [VANILLA, STOCKPILE], library: SPARE.library },
      });
      s.play(VANILLA, { zone: 1 });
      expect(lockEvents(s)).toEqual([]);
      s.endTurn();
      s.play(VANILLA, { zone: 1 });
      expect(locked(s, "p2", "units", 1)).toBe(true);
      expect(locked(s, "p1", "units", 1)).toBe(false);
    });

    it("R384 the Radiant face keeps Indestructible and its Activate", () => {
      const s = scenario({ p1: { backrow: [{ def: LOCKDOWN, radiant: true }], hand: [MAGIC_JAMMED, STOCKPILE], library: SPARE.library }, p2: SPARE });
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: s.card(LOCKDOWN).id }] });
      s.expectInZone(LOCKDOWN, "field");
      s.activate(LOCKDOWN, { ability: "tribute" });
      s.expectInZone(LOCKDOWN, "graveyard");
    });
  });
});
