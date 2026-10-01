// C #28 Second Wind — SPEC §8.6 row 28, BUILD M9 Classic row C 28: "Cry: exile your deck, then
// discard your hand (a discard; C #64 sees it); the discards land in your graveyard before the Aura
// starts, so they are playable (R393); Aura: `legalActions` offers `play` for every card in your
// graveyard, any type, at its cost with its choices and R65's player discounts, its Cry firing and the
// play counting as one from hand; base: cards you own that would go to your graveyard are exiled
// instead (a replacement: a Spell played from there is exiled after it resolves); with no deck every
// draw is fatigue (§2.4); both end when it leaves the field; no event carries a deck position;
// radiant: no exile replacement, and only cards whose price as it would be paid is (1) or more are
// offered, so a (0) Cost card never loops; its tuned number (radiant minimum price) reads through
// `param()` (R386)".
//
// The base face's replacement is B5 E5 (`Script.replacements`, the damage workstream's engine), not in
// this worktree's engine: the tests that watch a card of yours go to exile instead of your graveyard
// wait for its integration. Everything else runs here.

import { describe, expect, it } from "vitest";
import {
  cardAt,
  findInstance,
  legalActions,
  reduce,
  stepParam,
  unspentManaOf,
  zoneCards,
  type CardInstance,
  type GameState,
} from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import type { ActionBody } from "@jackioh/shared";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/028-second-wind";

const WIND = "classic-028";
const REPLENISH = "core-010"; // (0) Spell
const STOCKPILE = "core-005"; // (1) Spell: "Draw 2. Heal your hero 2."
const ECLIPSE = "core-035"; // (1) Spell: "Deal 3 damage to a target."
const MR_TOKEN = "core-015"; // (1) Unit 1/1: "Cry: Summon a Rush Token."
const VANILLA = "core-008"; // (1) Unit 4/4
const SHEEPISH = "core-041"; // (1) Trap
const MANA_WELL = "core-006"; // (3) Field Spell
const MENACE = "core-019"; // (3) Unit 9/9
const HIT_JOB = "core-016"; // (3) Spell: destroy target Unit
const COLLATERAL = "core-034"; // (4) Spell: exile target permanent and a random card from their deck
const TOE_CRACKER = "classic-006"; // (2) Unit: "Aura: Your Traps cost (0)."
const RUSH_TOKEN = "core-t-rush";

type Play = Extract<ActionBody, { type: "play" }>;

function playsOfCard(s: Scenario, instanceId: string, player: "p1" | "p2" = "p1"): Play[] {
  return legalActions(s.state, player).filter((action): action is Play => action.type === "play" && action.instanceId === instanceId);
}

function graveyardDefsOffered(s: Scenario): string[] {
  const offered = new Set(
    legalActions(s.state, "p1").flatMap((action) => (action.type === "play" ? [action.instanceId] : [])),
  );
  return s.pile("p1", "graveyard").filter((card) => offered.has(card.id)).map((card) => card.defId);
}

/**
 * A play of a graveyard card (B5 E11), sent to `reduce` as `legalActions` offers it: the harness's
 * `play` takes a card from a hand, so a graveyard play is reduced here and read back off its result.
 */
function playFromGraveyard(s: Scenario, card: CardInstance, extra: Partial<Play> = {}): { state: GameState; events: GameEvent[] } {
  const result = reduce(s.state, { type: "play", playerId: "p1", instanceId: card.id, ...extra, nonce: `graveyard-${card.id}` });
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

function zoneOf(state: GameState, card: CardInstance): string | undefined {
  return findInstance(state, card.id)?.zone.z;
}

/**
 * Second Wind standing, with a graveyard to play from. The Radiant face is set up standing (its Cry
 * not run). The base face's "would go to your graveyard" replacement would exile any card a setup put
 * in the graveyard once it stands, so there the graveyard comes the way the card makes it: Second Wind
 * is played and its Cry discards the hand, which lands before the Aura starts (R393).
 */
function standing(radiantFace: boolean, graveyard: readonly string[], extra: { hand?: readonly string[]; field?: readonly string[]; mana?: number } = {}): Scenario {
  const p2 = { hand: [STOCKPILE, HIT_JOB, COLLATERAL], field: [MENACE], library: [STOCKPILE, STOCKPILE] };
  if (!radiantFace && graveyard.length > 0) {
    const s = scenario({
      p1: {
        hand: [WIND, ...graveyard],
        field: [...(extra.field ?? [])],
        library: [VANILLA, VANILLA, VANILLA],
        ...(extra.mana === undefined ? {} : { mana: extra.mana }),
      },
      p2,
    });
    s.play(WIND, { zone: 1 });
    return s;
  }
  return scenario({
    p1: {
      hand: [...(extra.hand ?? [STOCKPILE])],
      backrow: [{ def: WIND, radiant: radiantFace }],
      field: [...(extra.field ?? [])],
      graveyard: [...graveyard],
      library: [VANILLA, VANILLA, VANILLA],
      ...(extra.mana === undefined ? {} : { mana: extra.mana }),
    },
    p2,
  });
}

describe("C #28 Second Wind", () => {
  it("declares its one number, the Radiant face's minimum price (R386)", () => {
    expect(def.params).toEqual([{ key: "minCost", base: 1, radiant: 1, better: "down", step: 1, min: 1 }]);
    expect(base.replacements?.map((entry) => entry.on)).toEqual(["toGraveyard"]);
    expect(radiant.replacements).toBeUndefined();
  });

  describe("base", () => {
    it("Cry: exiles your whole deck, then discards your hand", () => {
      const s = scenario({
        p1: { hand: [WIND, ECLIPSE, MR_TOKEN], library: [VANILLA, MENACE, STOCKPILE] },
        p2: { hand: [STOCKPILE] },
      });
      const library = s.pile("p1", "library");
      s.play(WIND, { zone: 1 });
      expect(s.pile("p1", "library")).toEqual([]);
      for (const card of library) s.expectInZone(card, "exile");
      expect(s.hand("p1")).toEqual([]);
      expect(s.events.filter((event) => event.type === "discarded")).toHaveLength(2);
      s.expectEvents("exiled", "discarded");
    });

    it("R393 the discards land in your graveyard before the Aura starts, so they are playable", () => {
      const s = scenario({
        p1: { hand: [WIND, ECLIPSE, MR_TOKEN], library: [VANILLA] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(WIND, { zone: 1 });
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([ECLIPSE, MR_TOKEN]);
      expect(graveyardDefsOffered(s)).toEqual([ECLIPSE, MR_TOKEN]);
    });

    it("E11 Aura: legalActions offers every card in your graveyard, any type, at its cost with its choices", () => {
      const s = standing(false, [STOCKPILE, MR_TOKEN, SHEEPISH, MANA_WELL, ECLIPSE, MENACE]);
      expect(graveyardDefsOffered(s)).toEqual([STOCKPILE, MR_TOKEN, SHEEPISH, MANA_WELL, ECLIPSE, MENACE]);
      // A targeted Spell carries its targets, each hero and each unit.
      const eclipse = s.pile("p1", "graveyard").find((card) => card.defId === ECLIPSE);
      if (eclipse === undefined) throw new Error("setup");
      const targets = playsOfCard(s, eclipse.id).map((action) => action.targets);
      expect(targets).toContainEqual([{ pick: "hero", player: "p2" }]);
    });

    it("R1 a Unit played from the graveyard fires its Cry, pays its cost and counts as played", () => {
      const s = standing(false, [MR_TOKEN]);
      const unit = s.card(MR_TOKEN);
      const after = playFromGraveyard(s, unit);
      expect(cardAt(after.state, { player: "p1", row: "units", lane: 1 })?.id).toBe(unit.id);
      expect(cardAt(after.state, { player: "p1", row: "units", lane: 2 })?.defId).toBe(RUSH_TOKEN);
      expect(unspentManaOf(after.state, "p1")).toBe(3);
      const played = after.events.find((event) => event.type === "cardPlayed" && event.instanceId === unit.id);
      expect(played).toMatchObject({ from: "graveyard", costPaid: 1 });
    });

    it("R65 a play from the graveyard takes the player's discounts: a Trap under Cloaked Toe Cracker costs (0)", () => {
      const s = standing(false, [SHEEPISH], { field: [TOE_CRACKER] });
      const trap = s.card(SHEEPISH);
      expect(playsOfCard(s, trap.id)).not.toEqual([]);
      const after = playFromGraveyard(s, trap, { zone: { row: "backrow", lane: 2 } });
      expect(unspentManaOf(after.state, "p1")).toBe(4);
      // R227: set face-down, it takes a fresh id.
      expect(cardAt(after.state, { player: "p1", row: "backrow", lane: 2 })?.defId).toBe(SHEEPISH);
      expect(zoneCards(after.state, "p1", "graveyard")).toEqual([]);
    });

    it("a card it can't afford is not offered", () => {
      const s = standing(false, [MENACE], { mana: 2 });
      expect(graveyardDefsOffered(s)).toEqual([]);
    });

    it("R3 with no deck every draw is fatigue", () => {
      const s = scenario({
        p1: { hand: [WIND, ECLIPSE], library: [VANILLA, VANILLA] },
        p2: { hand: [STOCKPILE, STOCKPILE], library: [STOCKPILE, STOCKPILE] },
      });
      s.play(WIND, { zone: 1 });
      s.endTurn();
      s.endTurn();
      expect(s.state.active).toBe("p1");
      s.expectHealth("p1", 29);
      expect(s.events.some((event) => event.type === "fatigue")).toBe(true);
    });

    it("its Aura ends when it leaves the field: nothing in the graveyard is offered any more", () => {
      const s = standing(false, [STOCKPILE, VANILLA]);
      expect(graveyardDefsOffered(s)).toEqual([STOCKPILE, VANILLA]);
      s.endTurn();
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(WIND).id }] });
      s.expectInZone(WIND, "exile");
      s.endTurn();
      expect(graveyardDefsOffered(s)).toEqual([]);
    });

    it("no event carries a deck position", () => {
      const s = scenario({
        p1: { hand: [WIND, ECLIPSE], library: [VANILLA, MENACE, STOCKPILE] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(WIND, { zone: 1 });
      const exiled = s.events.filter((event) => event.type === "exiled");
      expect(exiled).toHaveLength(3);
      for (const event of exiled) expect(Object.keys(event).sort()).toEqual(["defId", "instanceId", "owner", "type"]);
      for (const viewer of ["p1", "p2"] as const) {
        for (const event of s.view(viewer).events) expect(event).not.toHaveProperty("position");
      }
    });

    it("E5 a Spell played from your graveyard is exiled after it resolves", () => {
      const s = standing(false, [STOCKPILE]);
      const spell = s.pile("p1", "graveyard")[0];
      if (spell === undefined) throw new Error("setup");
      const after = playFromGraveyard(s, spell);
      expect(zoneOf(after.state, spell)).toBe("exile");
    });

    it("E5 a card of yours that would go to your graveyard is exiled instead: a discard, a destroyed Unit", () => {
      const s = standing(false, [], { hand: [STOCKPILE], field: [VANILLA] });
      const vanilla = s.card(VANILLA);
      s.endTurn();
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      s.expectInZone(vanilla, "exile");
      expect(s.pile("p1", "graveyard")).toEqual([]);
    });

    it("E5 the replacement ends when it leaves the field: your destroyed Unit goes to your graveyard again", () => {
      const s = standing(false, [], { field: [VANILLA] });
      const vanilla = s.card(VANILLA);
      s.endTurn();
      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: s.card(WIND).id }] });
      s.expectInZone(WIND, "exile");
      s.endTurn();
      s.endTurn();
      expect(s.state.active).toBe("p2");
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: vanilla.id }] });
      s.expectInZone(vanilla, "graveyard");
    });

    it("E5 the opponent's cards go to their graveyard as usual", () => {
      const s = standing(false, [], { field: [VANILLA] });
      s.endTurn();
      const hitJob = s.card(HIT_JOB);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(VANILLA).id }] });
      s.expectInZone(hitJob, "graveyard");
    });
  });

  describe("radiant", () => {
    it("the same Cry: exiles your deck, discards your hand into your graveyard", () => {
      const s = scenario({
        p1: { hand: [{ def: WIND, radiant: true }, ECLIPSE, MR_TOKEN], library: [VANILLA, MENACE] },
        p2: { hand: [STOCKPILE] },
      });
      s.play(WIND, { zone: 1 });
      expect(s.pile("p1", "library")).toEqual([]);
      expect(s.pile("p1", "exile").map((card) => card.defId)).toEqual([VANILLA, MENACE]);
      expect(s.pile("p1", "graveyard").map((card) => card.defId)).toEqual([ECLIPSE, MR_TOKEN]);
    });

    it("only cards whose price as it would be paid is (1) or more are offered", () => {
      const s = standing(true, [REPLENISH, STOCKPILE, VANILLA, MANA_WELL]);
      expect(graveyardDefsOffered(s)).toEqual([STOCKPILE, VANILLA, MANA_WELL]);
    });

    it("the price is read as it would be paid: a (1) Trap Toe Cracker makes (0) is not offered", () => {
      const s = standing(true, [SHEEPISH, STOCKPILE], { field: [TOE_CRACKER] });
      expect(graveyardDefsOffered(s)).toEqual([STOCKPILE]);
    });

    it("no exile replacement: a Spell played from the graveyard lands in it again", () => {
      const s = standing(true, [STOCKPILE]);
      const spell = s.pile("p1", "graveyard")[0];
      if (spell === undefined) throw new Error("setup");
      const after = playFromGraveyard(s, spell);
      expect(zoneOf(after.state, spell)).toBe("graveyard");
      // And the (1) Stockpile is offered again: paid (1) each time, it is no free loop.
      expect(zoneCards(after.state, "p1", "graveyard").map((card) => card.id)).toEqual([spell.id]);
      expect(
        legalActions(after.state, "p1").some((action) => action.type === "play" && action.instanceId === spell.id),
      ).toBe(true);
    });

    it("R1 a Unit played from the graveyard fires its Cry", () => {
      const s = standing(true, [MR_TOKEN]);
      const after = playFromGraveyard(s, s.card(MR_TOKEN));
      expect(cardAt(after.state, { player: "p1", row: "units", lane: 2 })?.defId).toBe(RUSH_TOKEN);
    });

    it("R386 a Degrade of the minimum price makes it (2): a (1) Cost card is no longer offered", () => {
      const s = standing(true, [STOCKPILE, MANA_WELL]);
      stepParam(s.card(WIND), "minCost", 1);
      expect(graveyardDefsOffered(s)).toEqual([MANA_WELL]);
    });
  });
});
