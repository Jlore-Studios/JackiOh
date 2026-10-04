// C #89 Paul Allen's Ghost — SPEC §8.6 row 89, BUILD M9 Classic row C 89: "Divine Shield; targeting it
// with anything but an attack costs the targeting player two random discards from their other hand
// cards (R654: no "of your choice"), paid at once with the price; with fewer than 2 other cards it is
// not a legal target, absent from `legalActions` and from the prompt's options; both players are bound,
// its controller too; attacks, random picks and "all" effects cost nothing; the discards are discards
// (C #64 sees them); radiant 10/12 Divine Shield, Reborn; its tuned number (discard) reads through
// `param()` (R386)".
//
// A costly play goes through `reduce` on the scenario's state, since the harness's `play` sends only
// the listed actions. The prompt half uses a Radiant C #57 Echo copying C #55 Book of Wildfire, whose
// Echo repeat asks a fresh target pick (R81). An activation's declared target is shown with C #78
// Mutate Spell's Activate (a tokened permanent), and a random pick with C #22 Mid Runner's random
// bounces.

import { legalActions, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, ActionBody, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic/089-paul-allens-ghost";
import { scenario, type Scenario } from "../_harness";

const GHOST = "classic-089";
const WILDFIRE = "classic-055"; // (1) Spell: Deal 4 damage (a declared target).
const ECHO = "classic-057";
const RECYCLER = "classic-064"; // Whenever you discard cards, draw that many.
const NETHER = "core-088"; // (4) Spell: Destroy all permanents.
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9
const FILLER = "core-005"; // (1) Spell
const SPARE = "core-010"; // (0) Spell, a card to discard
const MUTATE = "classic-078"; // Activate: remove a Plague Token from a permanent; an enemy one is exiled.
const MID_RUNNER = "classic-022"; // Cry: with 4 or more mana as it was played, bounce 2 random enemy permanents.

let nonce = 0;

function at(card: { id: string }): Selection {
  return { pick: "instance", instanceId: card.id };
}

function send(state: GameState, playerId: PlayerId, body: ActionBody): { state: GameState; events: GameEvent[]; error?: string } {
  nonce += 1;
  return reduce(state, { ...body, playerId, nonce: `c89-${nonce}` } as Action);
}

function wildfirePlays(s: Scenario, player: PlayerId): Extract<ActionBody, { type: "play" }>[] {
  const id = s.card(WILDFIRE).id;
  return legalActions(s.state, player).filter(
    (action): action is Extract<ActionBody, { type: "play" }> => action.type === "play" && action.instanceId === id,
  );
}

function spares(s: Scenario, player: PlayerId): string[] {
  return s.hand(player).filter((card) => card.defId === SPARE).map((card) => card.id);
}

describe("C #89 Paul Allen's Ghost", () => {
  it("is a (2) 5/6 Divine Shield Unit (10/12 Divine Shield, Reborn), its discard a declared number", () => {
    expect(def.cost).toBe(2);
    expect([def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([5, 6, 10, 12]);
    expect(def.base.keywords).toEqual([{ kind: "Divine Shield" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Divine Shield" }, { kind: "Reborn" }]);
    expect(def.params).toEqual([{ key: "discard", base: 2, radiant: 2, better: "up", step: 1, min: 1 }]);
    expect(base.targetingDiscards).toBeTypeOf("function");
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("is a 5/6 with Divine Shield", () => {
      const s = scenario({ p1: { hand: [GHOST, FILLER] } });
      s.play(GHOST).expectStats(GHOST, { attack: 5, health: 6 });
      expect(s.stats(GHOST).keywords.map((keyword) => keyword.kind)).toEqual(["Divine Shield"]);
    });

    it("B5 E5 R654 a declared target naming it lists one play carrying no discards", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      const ghost = s.card(GHOST);
      const atGhost = wildfirePlays(s, "p1").filter((play) => play.targets?.[0]?.pick === "instance" && play.targets[0].instanceId === ghost.id);
      expect(atGhost).toHaveLength(1);
      expect(atGhost.every((play) => !("discards" in play))).toBe(true);
      const atHero = wildfirePlays(s, "p1").filter((play) => play.targets?.[0]?.pick === "hero");
      expect(atHero.every((play) => !("discards" in play))).toBe(true);
    });

    it("B5 E5 R654 the play pays two random others: two spares go, never the card played, and the hit meets its Divine Shield", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      const spareIds = spares(s, "p1");
      const ghost = s.card(GHOST);
      const wildfire = s.card(WILDFIRE).id;
      const result = send(s.state, "p1", {
        type: "play",
        instanceId: wildfire,
        targets: [at(ghost)],
      });
      expect(result.error).toBeUndefined();
      const discarded = result.events.filter((event) => event.type === "discarded").map((event) => (event as { instanceId: string }).instanceId);
      expect(discarded).toHaveLength(2);
      for (const id of discarded) expect(spareIds).toContain(id);
      expect(discarded).not.toContain(wildfire);
      expect(result.state.players.p1.hand).toHaveLength(1);
      expect(spareIds).toContain(result.state.players.p1.hand[0]?.id);
      expect(result.events.some((event) => event.type === "divineShieldLost")).toBe(true);
    });

    it("B5 E5 a play naming it with too few other cards held is refused", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE] }, p2: { field: [GHOST, VANILLA] } });
      const ghost = s.card(GHOST);
      // Wildfire plus one spare: only one card outside the played card, fewer than the two owed.
      expect(() => s.play(WILDFIRE, { targets: [at(ghost)] })).toThrow();
    });

    it("B5 E5 with fewer than 2 other cards it is no legal target: absent from legalActions", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE] }, p2: { field: [GHOST, VANILLA] } });
      const ghost = s.card(GHOST);
      const targets = wildfirePlays(s, "p1").map((play) => play.targets?.[0]);
      expect(targets).not.toContainEqual(at(ghost));
      expect(targets).toContainEqual(at(s.card(VANILLA)));
    });

    it("B5 E5 it binds both players, its controller too", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE], field: [GHOST] } });
      const targets = wildfirePlays(s, "p1").map((play) => play.targets?.[0]);
      expect(targets).not.toContainEqual(at(s.card(GHOST)));
      const rich = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE], field: [GHOST] } });
      const own = wildfirePlays(rich, "p1").filter((play) => play.targets?.[0]?.pick === "instance");
      // Offered now that two others are held — and carrying no discards, paid at random.
      expect(own.map((play) => play.targets?.[0])).toContainEqual(at(rich.card(GHOST)));
      expect(own.every((play) => !("discards" in play))).toBe(true);
    });

    it("B5 E5 R654 a prompt answer naming it pays the 2 random cards at once, with no follow-up prompt", () => {
      const s = scenario({
        p1: { hand: [{ def: ECHO, radiant: true }, WILDFIRE, SPARE, SPARE] },
        p2: { field: [GHOST, MENACE] },
      });
      s.play(WILDFIRE, { targets: [{ pick: "hero", player: "p2" }] });
      s.play(ECHO, { targets: [at(s.card(MENACE))] });
      const pending = s.state.pending;
      expect(pending?.options.map((option) => option.selection)).toContainEqual(at(s.card(GHOST)));
      const spareIds = spares(s, "p1");
      s.answer([at(s.card(GHOST))]);
      expect(s.state.pending).toBeNull();
      const discarded = s.events.filter((event) => event.type === "discarded").map((event) => (event as { instanceId: string }).instanceId);
      expect(discarded).toHaveLength(2);
      for (const id of discarded) expect(spareIds).toContain(id);
      expect(s.events.some((event) => event.type === "divineShieldLost")).toBe(true);
    });

    it("B5 E5 a prompt never offers it to a chooser with fewer than 2 other cards", () => {
      const s = scenario({
        p1: { hand: [{ def: ECHO, radiant: true }, WILDFIRE, SPARE] },
        p2: { field: [GHOST, MENACE] },
      });
      s.play(WILDFIRE, { targets: [{ pick: "hero", player: "p2" }] });
      s.play(ECHO, { targets: [at(s.card(MENACE))] });
      const picks = s.state.pending?.options.map((option) => option.selection) ?? [];
      expect(picks).toContainEqual(at(s.card(MENACE)));
      expect(picks).not.toContainEqual(at(s.card(GHOST)));
    });

    it("B5 E5 R654 an activation's declared target naming it pays 2 random cards too: C #78 Mutate Spell", () => {
      const s = scenario({
        p1: { hand: [SPARE, SPARE, SPARE], backrow: [MUTATE] },
        p2: { hand: [FILLER], field: [{ def: GHOST, counters: { plague: 1 } }] },
      });
      const ghost = s.card(GHOST);
      const mutate = s.card(MUTATE).id;
      const spareIds = spares(s, "p1");
      const offered = legalActions(s.state, "p1").filter(
        (action): action is Extract<ActionBody, { type: "activate" }> => action.type === "activate" && action.instanceId === mutate,
      );
      // R654: one action, carrying no discards.
      expect(offered).toHaveLength(1);
      expect(offered.every((action) => !("discards" in action))).toBe(true);
      const result = send(s.state, "p1", { type: "activate", instanceId: mutate, targets: [at(ghost)] });
      expect(result.error).toBeUndefined();
      const discarded = result.events.filter((event) => event.type === "discarded").map((event) => (event as { instanceId: string }).instanceId);
      expect(discarded).toHaveLength(2);
      for (const id of discarded) expect(spareIds).toContain(id);
      expect(result.state.players.p1.hand).toHaveLength(1);
      expect(spareIds).toContain(result.state.players.p1.hand[0]?.id);
      expect(result.state.players.p2.exile.map((card) => card.id)).toEqual([ghost.id]);
    });

    it("B5 E5 with fewer than 2 other cards an activation can't name it", () => {
      const s = scenario({
        p1: { hand: [SPARE], backrow: [MUTATE] },
        p2: { hand: [FILLER], field: [{ def: GHOST, counters: { plague: 1 } }] },
      });
      const mutate = s.card(MUTATE).id;
      const ghost = s.card(GHOST);
      const named = legalActions(s.state, "p1").filter(
        (action) => action.type === "activate" && action.instanceId === mutate && (action.targets ?? []).some((pick) => pick.pick === "instance" && pick.instanceId === ghost.id),
      );
      expect(named).toEqual([]);
      expect(send(s.state, "p1", { type: "activate", instanceId: mutate, targets: [at(ghost)] }).error).toBeDefined();
    });

    it("R394 a random pick targets nothing: C #22 Mid Runner's random bounce returns it with no discard", () => {
      const s = scenario({ p1: { hand: [MID_RUNNER, FILLER] }, p2: { field: [GHOST], hand: [FILLER] } });
      s.play(MID_RUNNER, { zone: 1 });
      s.expectInZone(GHOST, "hand");
      expect(s.events.some((event) => event.type === "discarded")).toBe(false);
      expect(s.state.pending).toBeNull();
    });

    it("R394 an attack is no targeting: it is attacked with no discard", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [MENACE] }, p2: { field: [GHOST], hand: [FILLER] } });
      s.attack(MENACE, GHOST);
      expect(s.lastEvents.some((event) => event.type === "discarded")).toBe(false);
      expect(s.lastEvents.some((event) => event.type === "divineShieldLost")).toBe(true);
    });

    it("R394 an \"all\" effect targets nothing: Twisting Nether destroys it with no discard", () => {
      const s = scenario({ p1: { hand: [NETHER, FILLER] }, p2: { field: [GHOST] } });
      s.play(NETHER);
      s.expectInZone(GHOST, "graveyard");
      expect(s.lastEvents.some((event) => event.type === "discarded")).toBe(false);
    });

    it("the discards are discards: C #64 Malzahar's Recycler draws for them", () => {
      const s = scenario({
        p1: { hand: [WILDFIRE, SPARE, SPARE], backrow: [RECYCLER], library: [VANILLA, VANILLA, VANILLA] },
        p2: { field: [GHOST] },
      });
      const result = send(s.state, "p1", {
        type: "play",
        instanceId: s.card(WILDFIRE).id,
        targets: [at(s.card(GHOST))],
      });
      expect(result.error).toBeUndefined();
      expect(result.events.filter((event) => event.type === "drawn" && event.player === "p1")).toHaveLength(2);
      expect(result.state.players.p1.hand).toHaveLength(2);
    });

    it("R386 its discard is the declared number: an Upgrade's step makes it 3, and 2 others no longer pay", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      stepParam(s.card(GHOST), "discard", 1);
      expect(wildfirePlays(s, "p1").map((play) => play.targets?.[0])).not.toContainEqual(at(s.card(GHOST)));
      const rich = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      stepParam(rich.card(GHOST), "discard", 1);
      // Three others now owed: offered with four others held — still carrying no discards.
      const atGhost = wildfirePlays(rich, "p1").filter((play) => play.targets?.[0]?.pick === "instance" && play.targets[0].instanceId === rich.card(GHOST).id);
      expect(atGhost).toHaveLength(1);
      expect(atGhost.every((play) => !("discards" in play))).toBe(true);
    });
  });

  describe("radiant", () => {
    it("is a 10/12 with Divine Shield and Reborn", () => {
      const s = scenario({ p1: { hand: [{ def: GHOST, radiant: true }, FILLER] } });
      s.play(GHOST).expectStats(GHOST, { attack: 10, health: 12 });
      expect(s.stats(GHOST).keywords.map((keyword) => keyword.kind)).toEqual(["Divine Shield", "Reborn"]);
    });

    it("§4.5 Reborn: destroyed, it returns at 1 health, and targeting it still costs 2 random cards", () => {
      const s = scenario({ p1: { hand: [NETHER, WILDFIRE, SPARE, SPARE], mana: 9 }, p2: { field: [{ def: GHOST, radiant: true }] } });
      s.play(NETHER);
      s.expectInZone(GHOST, "field").expectStats(GHOST, { health: 1 });
      const ghost = s.card(GHOST);
      const atGhost = wildfirePlays(s, "p1").filter((play) => play.targets?.[0]?.pick === "instance" && play.targets[0].instanceId === ghost.id);
      expect(atGhost.length).toBeGreaterThan(0);
      expect(atGhost.every((play) => !("discards" in play))).toBe(true);
    });

    it("B5 E5 the same cost binds on the Radiant face: too few other cards, no target", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE] }, p2: { field: [{ def: GHOST, radiant: true }] } });
      expect(wildfirePlays(s, "p1").map((play) => play.targets?.[0])).not.toContainEqual(at(s.card(GHOST)));
    });
  });
});
