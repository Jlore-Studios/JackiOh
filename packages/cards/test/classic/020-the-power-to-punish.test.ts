// C #20 The Power to Punish — SPEC §8.6 row 20, BUILD M9 Classic row C 20: "Activate, once per turn
// (R384), the mode and its target carried in the `activate` action: deal 2 damage to a target; the
// opponent discards a card of their choice (R16; an empty hand: nothing), the prompt's options their
// own hand and named nowhere in your view; or a Unit, either side, is destroyed at the start of your
// next turn, a delayed effect keyed to that stay on the field that fizzles if the Unit has left it,
// even if it came back (R174), resolving with the start-of-turn delayed effects (R62, R68) and still
// firing if The Power to Punish has left the field (as R76); an Indestructible target survives (R46);
// activating is not a play; radiant: 4 damage; discards 2; or every enemy Unit on the field at the
// start of your next turn is destroyed, the Units there then rather than a list fixed at activation;
// its tuned numbers (damage, discards) read through `param()` (R386)".

import { describe, expect, it } from "vitest";
import { legalActions, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { PlayerId, Selection } from "@jackioh/shared";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/020-the-power-to-punish";

const PUNISH = "classic-020";
const DAMAGE = "deal damage";
const DISCARD = "opponent discards";
const DOOM = "destroy a Unit at the start of your next turn";
const DOOM_ALL = "destroy all enemy Units at the start of your next turn";

const FILLER = "core-005"; // (1) Spell Stockpile: draw 2, heal 2.
const VANILLA = "core-008"; // 4/4.
const TIMMY = "core-011"; // (1) 3/3.
const MENACE = "core-019"; // 9/9.
const POINTMASTER = "core-020"; // 7/1.
const ROCK = "core-066"; // 10/10 Indestructible.
const BIG = "core-025"; // 7/7 Armor 7; Radiant adds Reborn.
const CUBE = "core-022"; // (3) Cry: Tribute one of your other Units.
const COLLATERAL = "core-034"; // (4) Exile target permanent and a random card from their deck.

const AT_P2: readonly Selection[] = [{ pick: "hero", player: "p2" }];

function setup(p1: SideSetup = {}, p2: SideSetup = {}, radiantFace = false): Scenario {
  return scenario({
    p1: { hand: [FILLER, FILLER], library: [FILLER, FILLER, FILLER], backrow: [{ def: PUNISH, radiant: radiantFace }], ...p1 },
    p2: { hand: [FILLER, FILLER], library: [FILLER, FILLER, FILLER], ...p2 },
  });
}

function at(card: { id: string }): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

function punish(s: Scenario, mode: string, targets?: readonly Selection[]): Scenario {
  return s.activate(PUNISH, { modes: [mode], ...(targets === undefined ? {} : { targets }) });
}

/** p1 ends the turn, p2 takes theirs and ends it, and p1's next turn starts. */
function toNextTurn(s: Scenario): Scenario {
  s.endTurn();
  expect(s.state.active).toBe("p2");
  s.endTurn();
  expect(s.state.active).toBe("p1");
  return s;
}

function activations(s: Scenario, player: PlayerId): unknown[] {
  const card = s.card(PUNISH);
  return legalActions(s.state, player).filter((action) => action.type === "activate" && action.instanceId === card.id);
}

describe("C #20 The Power to Punish", () => {
  it("R384 declares one Activate, once per turn, with three modes on each face", () => {
    expect(def.id).toBe(PUNISH);
    expect(base.activations?.[0]?.uses).toBe(1);
    expect(base.activations?.[0]?.modes?.[0]?.options).toEqual([DAMAGE, DISCARD, DOOM]);
    expect(radiant.activations?.[0]?.uses).toBe(1);
    expect(radiant.activations?.[0]?.modes?.[0]?.options).toEqual([DAMAGE, DISCARD, DOOM_ALL]);
  });

  describe("base", () => {
    it("R81 deal damage: the mode and its target travel in the action; 2 damage to a hero", () => {
      const s = setup();

      punish(s, DAMAGE, AT_P2);

      s.expectHealth("p2", 28);
      s.expectEvents("activated", "damage");
    });

    it("deal damage reaches a Unit on either side, from this card", () => {
      const s = setup({ field: [VANILLA] }, { field: [POINTMASTER] });
      const point = s.card(POINTMASTER);

      punish(s, DAMAGE, at(point));

      s.expectInZone(point, "graveyard");
      const hit = s.events.find((event) => event.type === "damage");
      expect(hit).toMatchObject({ sourceId: s.card(PUNISH).id, amount: 2 });
    });

    it("R90 the damage mode refuses an activation with no target", () => {
      const s = setup();
      expect(() => punish(s, DAMAGE)).toThrow();
      s.expectHealth("p2", 30);
    });

    it("R16 the opponent discards a card of their choice: they hold the prompt, over their own hand", () => {
      const s = setup({}, { hand: [VANILLA, TIMMY, MENACE] });
      const menace = s.card(MENACE);

      punish(s, DISCARD);

      const pending = s.state.pending;
      expect(pending?.playerId).toBe("p2");
      expect(pending?.kind).toBe("hand");
      expect(pending?.options.map((option) => option.selection)).toEqual(
        s.hand("p2").map((card) => ({ pick: "instance", instanceId: card.id })),
      );

      s.answer([{ pick: "instance", instanceId: menace.id }]);

      s.expectInZone(menace, "graveyard");
      expect(s.hand("p2")).toHaveLength(2);
      expect(s.state.pending).toBeNull();
      s.expectEvents("promptOpened", "discarded");
    });

    it("R177 your view names none of the options of the opponent's discard pick", () => {
      const s = setup({}, { hand: [VANILLA, TIMMY, MENACE] });
      const ids = s.hand("p2").map((card) => card.id);

      punish(s, DISCARD);

      const seen = JSON.stringify(s.view("p1"));
      for (const defId of [VANILLA, TIMMY, MENACE]) expect(seen).not.toContain(defId);
      for (const id of ids) expect(seen).not.toContain(id);
      // The opponent's own view does carry its options.
      expect(JSON.stringify(s.view("p2"))).toContain(MENACE);
    });

    it("§9.3 the opponent's open discard pick survives a JSON round trip and resumes through reduce", () => {
      const s = setup({}, { hand: [VANILLA, TIMMY] });
      punish(s, DISCARD);
      const paused = s.state;
      const revived = JSON.parse(JSON.stringify(paused)) as GameState;
      expect(revived).toEqual(paused);

      const choiceId = revived.pending?.id ?? "";
      const pick = s.card(TIMMY).id;
      const result = reduce(revived, {
        type: "answer",
        playerId: "p2",
        choiceId,
        selection: [{ pick: "instance", instanceId: pick }],
        nonce: "punish-round-trip",
      });

      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.work).toEqual([]);
      expect(result.state.players.p2.graveyard.map((card) => card.id)).toContain(pick);
    });

    it("R16 an opponent with an empty hand discards nothing and is asked nothing", () => {
      const s = setup({}, { hand: [], field: [VANILLA] });

      punish(s, DISCARD);

      expect(s.state.pending).toBeNull();
      expect(s.pile("p2", "graveyard")).toHaveLength(0);
    });

    it("R62 the chosen enemy Unit is destroyed at the start of your next turn, and not before", () => {
      const s = setup({}, { field: [VANILLA] });
      const vanilla = s.card(VANILLA);

      punish(s, DOOM, at(vanilla));
      s.endTurn();
      s.expectInZone(vanilla, "field");
      s.endTurn();

      s.expectInZone(vanilla, "graveyard");
      s.expectEvents("turnStarted", "destroyed");
      // R62: start-of-turn delayed effects come before the turn's draw.
      const types = s.lastEvents.map((event) => event.type);
      const drawn = s.lastEvents.findIndex((event) => event.type === "drawn" && event.player === "p1");
      expect(drawn).toBeGreaterThan(types.indexOf("destroyed"));
    });

    it("the delayed destroy may name one of your own Units", () => {
      const s = setup({ field: [VANILLA] });
      const vanilla = s.card(VANILLA);

      punish(s, DOOM, at(vanilla));
      toNextTurn(s);

      s.expectInZone(vanilla, "graveyard");
    });

    it("R46 an Indestructible Unit survives it", () => {
      const s = setup({}, { field: [ROCK] });
      const rock = s.card(ROCK);

      punish(s, DOOM, at(rock));
      toNextTurn(s);

      s.expectInZone(rock, "field");
    });

    it("R174 it fizzles when the Unit has left the field, even though it came back (Reborn)", () => {
      const s = setup({ hand: [CUBE, FILLER], field: [{ def: BIG, radiant: true }] });
      const big = s.card(BIG);

      punish(s, DOOM, at(big));
      // The Cube's Cry tributes the 7/7, whose Reborn brings it back: a new stay (R83).
      s.play(CUBE, { targets: at(big) });
      s.expectInZone(big, "field");
      toNextTurn(s);

      s.expectInZone(big, "field");
      s.expectStats(big, { health: 1 });
    });

    it("R76 it still fires after The Power to Punish has left the field", () => {
      const s = setup({}, { field: [VANILLA], hand: [COLLATERAL, FILLER] });
      const vanilla = s.card(VANILLA);
      const card = s.card(PUNISH);

      punish(s, DOOM, at(vanilla));
      s.endTurn();
      s.play(COLLATERAL, { targets: at(card) });
      s.expectInZone(card, "exile");
      s.endTurn();

      s.expectInZone(vanilla, "graveyard");
    });

    it("R384 once per turn: a second activation is refused and not listed; it is back next turn", () => {
      const s = setup();

      punish(s, DAMAGE, AT_P2);
      expect(activations(s, "p1")).toHaveLength(0);
      expect(() => punish(s, DAMAGE, AT_P2)).toThrow();
      s.expectHealth("p2", 28);

      toNextTurn(s);
      expect(activations(s, "p1").length).toBeGreaterThan(0);
      punish(s, DAMAGE, AT_P2);
      s.expectHealth("p2", 26);
    });

    it("R384 only its controller in their own turn: on the opponent's turn it is not listed", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], backrow: [PUNISH] },
        p2: { hand: [FILLER] },
      });
      expect(activations(s, "p1")).toHaveLength(0);
      expect(() => punish(s, DAMAGE, AT_P2)).toThrow();
    });

    it("R384 activating is not a play", () => {
      const s = setup();
      const played = s.state.players.p1.turnLog.cardsPlayed;

      punish(s, DAMAGE, AT_P2);

      expect(s.events.some((event) => event.type === "cardPlayed")).toBe(false);
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(played);
    });

    it("R386 an Upgrade of its damage deals 3; an Upgrade of its discards takes 2 cards", () => {
      const hit = setup();
      stepParam(hit.card(PUNISH), "damage", 1);
      punish(hit, DAMAGE, AT_P2);
      hit.expectHealth("p2", 27);

      const two = setup({}, { hand: [VANILLA, TIMMY, MENACE] });
      stepParam(two.card(PUNISH), "discards", 1);
      punish(two, DISCARD);
      expect(two.state.pending?.min).toBe(2);
      two.answer([two.card(VANILLA).id, two.card(TIMMY).id]);
      expect(two.pile("p2", "graveyard").map((card) => card.defId).sort()).toEqual([TIMMY, VANILLA].sort());
    });
  });

  describe("radiant", () => {
    it("deals 4", () => {
      const s = setup({}, {}, true);

      punish(s, DAMAGE, AT_P2);

      s.expectHealth("p2", 26);
    });

    it("R16 the opponent discards 2 cards of their choice", () => {
      const s = setup({}, { hand: [VANILLA, TIMMY, MENACE] }, true);

      punish(s, DISCARD);
      expect(s.state.pending?.min).toBe(2);
      expect(s.state.pending?.max).toBe(2);
      s.answer([s.card(VANILLA).id, s.card(MENACE).id]);

      expect(s.hand("p2").map((card) => card.defId)).toEqual([TIMMY]);
    });

    it("R16 with one card in hand the opponent discards that one", () => {
      const s = setup({}, { hand: [TIMMY], field: [VANILLA] }, true);

      punish(s, DISCARD);
      expect(s.state.pending?.max).toBe(1);
      s.answer([s.card(TIMMY).id]);

      expect(s.hand("p2")).toHaveLength(0);
    });

    it("the third mode takes no target", () => {
      const s = setup({}, { field: [VANILLA] }, true);
      expect(() => punish(s, DOOM_ALL, at(s.card(VANILLA)))).toThrow();
    });

    it("destroys every enemy Unit on the field at the start of your next turn — the Units there then", () => {
      const s = setup({ field: [MENACE] }, { field: [VANILLA], hand: [TIMMY, FILLER] }, true);
      const vanilla = s.card(VANILLA);
      const menace = s.card(MENACE);

      punish(s, DOOM_ALL);
      s.endTurn();
      // A Unit the opponent plays after the activation is there when it resolves, so it goes too.
      s.play(TIMMY, { zone: 2 });
      const timmy = s.card(TIMMY);
      s.expectInZone(vanilla, "field");
      s.endTurn();

      s.expectInZone(vanilla, "graveyard");
      s.expectInZone(timmy, "graveyard");
      s.expectInZone(menace, "field");
    });

    it("R46 an Indestructible enemy Unit survives it", () => {
      const s = setup({}, { field: [ROCK, VANILLA] }, true);

      punish(s, DOOM_ALL);
      toNextTurn(s);

      s.expectInZone(ROCK, "field");
      s.expectInZone(VANILLA, "graveyard");
    });

    it("R386 an Upgrade of its damage deals 5", () => {
      const s = setup({}, {}, true);
      stepParam(s.card(PUNISH), "damage", 1);

      punish(s, DAMAGE, AT_P2);

      s.expectHealth("p2", 25);
    });
  });
});
