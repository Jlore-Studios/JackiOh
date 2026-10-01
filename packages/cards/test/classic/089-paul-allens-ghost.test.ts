// C #89 Paul Allen's Ghost — SPEC §8.6 row 89, BUILD M9 Classic row C 89: "Divine Shield; targeting it
// with anything but an attack costs the targeting player two discards of their choice from their other
// hand cards: a declared target in a play or an activation carries the 2-card pick in the action (as a
// Tribute carries its set, R101), and a prompt answer naming it asks for the 2 cards next; with fewer
// than 2 other cards it is not a legal target, absent from `legalActions` and from the prompt's
// options; both players are bound, its controller too; attacks, random picks and "all" effects cost
// nothing; the discards are discards (C #64 sees them); radiant 10/12 Divine Shield, Reborn; its tuned
// number (discard) reads through `param()` (R386)".
//
// A play's discards travel in the `play` action (`discards`), which the harness's `play` does not
// send, so those plays go through `reduce` on the scenario's state. The prompt half uses a Radiant
// C #57 Echo copying C #55 Book of Wildfire, whose Echo repeat asks a fresh target pick (R81). An
// activation's declared target is the same targeting point (`targeting.ts`); no Activate card in this
// branch declares a target, and the engine's B5 E5 tests prove it through a fixture.

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

    it("B5 E5 a declared target naming it carries a pick of 2 of the targeting player's other cards, listed whole", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      const ghost = s.card(GHOST);
      const atGhost = wildfirePlays(s, "p1").filter((play) => play.targets?.[0]?.pick === "instance" && play.targets[0].instanceId === ghost.id);
      expect(atGhost).toHaveLength(3);
      expect(atGhost.every((play) => play.discards?.length === 2)).toBe(true);
      const atHero = wildfirePlays(s, "p1").filter((play) => play.targets?.[0]?.pick === "hero");
      expect(atHero.every((play) => play.discards === undefined)).toBe(true);
    });

    it("B5 E5 R16 the play pays them: the chosen two are discarded, and the hit meets its Divine Shield", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      const [a, b, c] = spares(s, "p1");
      const ghost = s.card(GHOST);
      const result = send(s.state, "p1", {
        type: "play",
        instanceId: s.card(WILDFIRE).id,
        targets: [at(ghost)],
        discards: [a ?? "", b ?? ""],
      });
      expect(result.error).toBeUndefined();
      expect(result.events.filter((event) => event.type === "discarded").map((event) => (event as { instanceId: string }).instanceId)).toEqual([a, b]);
      expect(result.state.players.p1.hand.map((card) => card.id)).toEqual([c]);
      expect(result.events.some((event) => event.type === "divineShieldLost")).toBe(true);
    });

    it("B5 E5 a play naming it with no discards, or one of them the card played, is refused", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      const [a] = spares(s, "p1");
      const ghost = s.card(GHOST);
      const wildfire = s.card(WILDFIRE).id;
      expect(() => s.play(WILDFIRE, { targets: [at(ghost)] })).toThrow();
      expect(send(s.state, "p1", { type: "play", instanceId: wildfire, targets: [at(ghost)], discards: [a ?? "", wildfire] }).error).toBeDefined();
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
      expect(own.every((play) => play.discards?.length === 2)).toBe(true);
    });

    it("B5 E5 a prompt answer naming it asks for the 2 cards next; after a JSON round trip the answer is the same", () => {
      const s = scenario({
        p1: { hand: [{ def: ECHO, radiant: true }, WILDFIRE, SPARE, SPARE] },
        p2: { field: [GHOST, MENACE] },
      });
      s.play(WILDFIRE, { targets: [{ pick: "hero", player: "p2" }] });
      s.play(ECHO, { targets: [at(s.card(MENACE))] });
      const pending = s.state.pending;
      expect(pending?.options.map((option) => option.selection)).toContainEqual(at(s.card(GHOST)));
      s.answer([at(s.card(GHOST))]);
      expect(s.state.pending?.kind).toBe("hand");
      expect(s.state.pending?.min).toBe(2);
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const [a, b] = spares(s, "p1");
      const choiceId = s.state.pending?.id ?? "";
      const action: Action = {
        type: "answer",
        playerId: "p1",
        nonce: "c89-round-trip",
        choiceId,
        selection: [
          { pick: "instance", instanceId: a ?? "" },
          { pick: "instance", instanceId: b ?? "" },
        ],
      };
      const live = reduce(s.state, action);
      const revived = reduce(round, action);
      expect(live.error).toBeUndefined();
      expect(revived.state).toEqual(live.state);
      expect(live.events.filter((event) => event.type === "discarded")).toHaveLength(2);
      expect(live.events.some((event) => event.type === "divineShieldLost")).toBe(true);
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
      const [a, b] = spares(s, "p1");
      const result = send(s.state, "p1", {
        type: "play",
        instanceId: s.card(WILDFIRE).id,
        targets: [at(s.card(GHOST))],
        discards: [a ?? "", b ?? ""],
      });
      expect(result.error).toBeUndefined();
      expect(result.events.filter((event) => event.type === "drawn" && event.player === "p1")).toHaveLength(2);
      expect(result.state.players.p1.hand).toHaveLength(2);
    });

    it("R386 its discard is the declared number: an Upgrade's step makes it 3, and 2 others no longer pay", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      stepParam(s.card(GHOST), "discard", 1);
      expect(wildfirePlays(s, "p1").map((play) => play.targets?.[0])).not.toContainEqual(at(s.card(GHOST)));
      const rich = scenario({ p1: { hand: [WILDFIRE, SPARE, SPARE, SPARE] }, p2: { field: [GHOST] } });
      stepParam(rich.card(GHOST), "discard", 1);
      const atGhost = wildfirePlays(rich, "p1").filter((play) => play.targets?.[0]?.pick === "instance");
      expect(atGhost.map((play) => play.discards?.length)).toEqual([3]);
    });
  });

  describe("radiant", () => {
    it("is a 10/12 with Divine Shield and Reborn", () => {
      const s = scenario({ p1: { hand: [{ def: GHOST, radiant: true }, FILLER] } });
      s.play(GHOST).expectStats(GHOST, { attack: 10, health: 12 });
      expect(s.stats(GHOST).keywords.map((keyword) => keyword.kind)).toEqual(["Divine Shield", "Reborn"]);
    });

    it("§4.5 Reborn: destroyed, it returns at 1 health, and targeting it still costs 2", () => {
      const s = scenario({ p1: { hand: [NETHER, WILDFIRE, SPARE, SPARE], mana: 9 }, p2: { field: [{ def: GHOST, radiant: true }] } });
      s.play(NETHER);
      s.expectInZone(GHOST, "field").expectStats(GHOST, { health: 1 });
      const ghost = s.card(GHOST);
      const atGhost = wildfirePlays(s, "p1").filter((play) => play.targets?.[0]?.pick === "instance" && play.targets[0].instanceId === ghost.id);
      expect(atGhost.length).toBeGreaterThan(0);
      expect(atGhost.every((play) => play.discards?.length === 2)).toBe(true);
    });

    it("B5 E5 the same cost binds on the Radiant face: too few other cards, no target", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, SPARE] }, p2: { field: [{ def: GHOST, radiant: true }] } });
      expect(wildfirePlays(s, "p1").map((play) => play.targets?.[0])).not.toContainEqual(at(s.card(GHOST)));
    });
  });
});
