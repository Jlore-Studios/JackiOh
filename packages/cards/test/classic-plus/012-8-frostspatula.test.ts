// C+ #12.8 Frostspatula — SPEC §8.7 row 12.8, R383, R409, BUILD M9 Classic+ row C+ 12.8: "Animated on
// your turn (R383): played on your turn it animates at once into its own lane's unit zone, else the
// leftmost open, unlocked, unreserved one, summoning sick on every animation, so its Rush reaches units
// and never the hero; it returns to its backrow zone at the end of your cleanup, after your end-of-turn
// steps, and animates again after your next mana refresh and Brittle tick; on the opponent's turn it
// can't be attacked, "all Units" effects skip it and backrow effects reach it; while animated its
// backrow zone is reserved; both moves keep damage, buffs and memory (R78 does not apply); with no open
// unit zone it stays in the backrow; […]; it remembers each Unit it killed (R42) as `{ defId, radiant }`,
// and its Death, as a Unit or destroyed in the backrow (§4.5), summons a fresh copy of each for you,
// tokens included, until your board is full, the originals staying in their owners' graveyards (R409);
// `animated` and `deanimated` are public; radiant 20/6 and the copies are Radiant".

import { HERO_HEALTH } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const SPATULA = "classicplus-012-8";
const TOKENS = "core-015"; // (1) 1/1, Cry: summon a Rush Token
const MENACE = "core-019"; // 9/9
const RUSH = "core-t-rush"; // 3/3 Rush
const MAGIC_JAMMED = "core-036"; // (1) destroy target backrow card
const POWDER = "classicplus-012-4"; // (1) 3 damage to each enemy
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

function played(radiant: boolean, lane: number, p1: SideSetup = {}, p2: SideSetup = {}): Scenario {
  const s = scenario({
    p1: { hand: [{ def: SPATULA, radiant }, FILLER, FILLER], library: DECK, mana: 8, ...p1 },
    p2: { hand: [FILLER], library: DECK, field: [TOKENS], ...p2 },
  });
  s.play(SPATULA, { zone: lane });
  return s;
}

const spatulaId = (s: Scenario): string => s.card(SPATULA).id;
const where = (s: Scenario): string => JSON.stringify(s.card(SPATULA).zone);
const eventsOf = <T extends GameEvent["type"]>(s: Scenario, type: T): Extract<GameEvent, { type: T }>[] =>
  s.events.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);

describe("C+ #12.8 Frostspatula", () => {
  describe("base", () => {
    it("R383 played on your turn it animates at once into its own lane's unit zone, its backrow zone reserved", () => {
      const s = played(false, 2);
      expect(s.unit("p1", 2)?.id).toBe(spatulaId(s));
      expect(s.view("p1").you.reserved.backrow[1]).toBe(true);
      expect(eventsOf(s, "animated")).toHaveLength(1);
      expect(s.view("p2").events.some((event) => event.type === "animated")).toBe(true);
      s.expectStats(SPATULA, { attack: 10, health: 3 });
    });

    it("R383 with its lane's unit zone taken it goes to the leftmost open one; with none it stays in the backrow", () => {
      const taken = played(false, 2, { field: [{ def: MENACE, lane: 2 }] });
      expect(taken.unit("p1", 1)?.id).toBe(spatulaId(taken));
      const full = played(false, 2, { field: [MENACE, MENACE, MENACE, MENACE, MENACE] });
      expect(full.backrow("p1", 2)?.id).toBe(spatulaId(full));
    });

    it("R83 summoning sick on arrival: its Rush reaches units, never the hero", () => {
      const s = played(false, 1);
      expect(() => s.attack(SPATULA, "hero")).toThrow();
      s.attack(SPATULA, TOKENS);
      expect(s.pile("p2", "graveyard").some((card) => card.defId === TOKENS)).toBe(true);
    });

    it("R383 it returns to its backrow zone at your cleanup, keeping its damage; on the opponent's turn it hides there", () => {
      const s = played(false, 2, {}, { hand: [POWDER, FILLER] });
      s.attack(SPATULA, TOKENS); // takes 1 damage
      s.endTurn();
      expect(s.backrow("p1", 2)?.id).toBe(spatulaId(s));
      expect(s.unit("p1", 2)).toBeNull();
      expect(eventsOf(s, "deanimated")).toHaveLength(1);
      // The opponent can't attack it, and their "all Units" sweep skips it.
      s.play(POWDER);
      expect(s.card(SPATULA).zone).toMatchObject({ z: "field", row: "backrow" });
      expect(s.card(SPATULA).damage ?? 0).toBe(1);
      s.endTurn();
      // Back on your turn, after the refresh: animated again, damage kept.
      expect(s.unit("p1", 2)?.id).toBe(spatulaId(s));
      s.expectStats(SPATULA, { health: 2 });
    });

    it("R383 a home zone Locked meanwhile keeps it a Unit through its cleanup", () => {
      const s = played(false, 2);
      // A Lock on its reserved backrow zone, as Lock effects leave one (§3.2).
      s.state.players.p1.locks.backrow[1] = true;
      s.endTurn();
      expect(s.unit("p1", 2)?.id).toBe(spatulaId(s));
    });

    it("backrow effects reach it on the opponent's turn", () => {
      const s = played(false, 2, {}, { hand: [MAGIC_JAMMED, FILLER] });
      s.endTurn();
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: spatulaId(s) }] });
      s.expectInZone(SPATULA, "graveyard");
    });

    it("R409 it remembers each Unit it killed, and its Death summons a fresh copy of each for you, with no Cry", () => {
      const s = played(false, 1, {}, { hand: [MAGIC_JAMMED, FILLER] });
      const victim = s.card(TOKENS).id;
      s.attack(SPATULA, TOKENS);
      expect(s.card(SPATULA).memory.kills).toMatchObject([{ defId: TOKENS, radiant: false }]);
      s.endTurn();
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: spatulaId(s) }] });
      // Destroyed in the backrow: its Death fires. A fresh Me and Mr Token for p1, no Cry (no Rush Token).
      const copies = [1, 2, 3, 4, 5].flatMap((lane) => (s.unit("p1", lane)?.defId === TOKENS ? [s.unit("p1", lane)] : []));
      expect(copies).toHaveLength(1);
      expect(copies[0]?.id).not.toBe(victim);
      expect(s.card(victim).zone.z).toBe("graveyard");
      expect(s.card(victim).owner).toBe("p2");
      expect(eventsOf(s, "summoned").some((event) => event.defId === RUSH)).toBe(false);
    });

    it("R409 tokens included, and a kill in the same combat as its own death counts", () => {
      const s = played(false, 1, {}, { field: [RUSH] });
      const id = spatulaId(s);
      s.attack(SPATULA, RUSH); // 10 kills the 3/3; the 3/3 kills the 10/3
      // A token that dies as a Unit ceases to exist (R11).
      s.expectInZone(id, "gone");
      expect(s.unit("p1", 1)?.defId).toBe(RUSH);
    });

    it("R409 the copies go in until your board is full", () => {
      const s = played(false, 1, { field: [{ def: MENACE, lane: 2 }, { def: MENACE, lane: 3 }, { def: MENACE, lane: 4 }, { def: MENACE, lane: 5 }] }, { field: [RUSH] });
      s.attack(SPATULA, RUSH);
      // Its own zone opened as it died: one copy fits.
      expect(s.unit("p1", 1)?.defId).toBe(RUSH);
    });

    it("its kills survive its moves between the rows (R383: memory is kept)", () => {
      const s = played(false, 1, {}, { field: [TOKENS, TOKENS] });
      s.attack(SPATULA, s.unit("p2", 1)?.id ?? "");
      s.endTurn().endTurn();
      s.attack(SPATULA, s.unit("p2", 2)?.id ?? "");
      expect(s.card(SPATULA).memory.kills).toHaveLength(2);
    });

    it("its hero never takes its attack while it is summoning sick", () => {
      const s = played(false, 1);
      s.expectHealth("p2", HERO_HEALTH);
      expect(where(s)).toContain("units");
    });
  });

  describe("radiant", () => {
    it("20/6, and the copies are Radiant", () => {
      const s = played(true, 1, {}, { hand: [MAGIC_JAMMED, FILLER] });
      s.expectStats(SPATULA, { attack: 20, health: 6 });
      s.attack(SPATULA, TOKENS);
      s.endTurn();
      s.play(MAGIC_JAMMED, { targets: [{ pick: "instance", instanceId: spatulaId(s) }] });
      const copy = [1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)).find((unit) => unit?.defId === TOKENS);
      expect(copy?.radiant).toBe(true);
    });
  });
});
