// Playing cards from the graveyard (docs/classic-sets.md B5 E11; R454): the permissions, what
// `legalActions` offers under them, §10.5 taking the card out of the graveyard, R65's prices, the
// Plague Token payment, a pause mid-play surviving JSON, and what the other seat sees.

import { describe, expect, it } from "vitest";
import { effectiveCost } from "../src/mana";
import { addModifier } from "../src/modifiers";
import { legalActions } from "../src/reduce";
import { hashState } from "../src/replay";
import { findInstance, type GameState } from "../src/state";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { activeUnitsOf, moveToZone } from "../src/zones";
import { registerScripts, registeredScripts } from "../src/scripts";
import { eventsOfType, put, slot } from "./fixtures/harness";
import {
  QUEST_REWARD_KEY,
  discoverSpell,
  graveSpell,
  graveTrap,
  graveUnit,
  inGraveyard,
  lobbyist,
  monkey,
  only,
  pbAct,
  pbPlaying,
  pbReduce,
  plantation,
  playsOf,
  questCard,
  roundTrip,
  secondWind,
  threeSpell,
  titan,
  zeroSpell,
} from "./fixtures/playPipelineB";

function heroHealth(state: GameState, player: "p1" | "p2"): number {
  return state.players[player].hero.health;
}

describe("E11 play from the graveyard (R454)", () => {
  it("R454 a permission on the field offers and plays a graveyard card as a play: counted, its script run, the card out of the graveyard", () => {
    const state = pbPlaying("r454-offer");
    const spell = inGraveyard(state, graveSpell.id);

    // No permission: nothing offered, and the play is refused.
    expect(playsOf(state, spell.id)).toEqual([]);
    expect(pbReduce(state, { type: "play", instanceId: spell.id, playerId: "p1" }).error).toMatch(
      /may not play that card from your graveyard/,
    );

    put(state, secondWind.id, slot("p1", "backrow", 1));
    const offered = playsOf(state, spell.id);
    expect(offered).toEqual([{ type: "play", instanceId: spell.id }]);

    const before = heroHealth(state, "p2");
    const played = state.counters.played;
    const result = pbReduce(state, { ...only(offered), playerId: "p1" });
    expect(result.error).toBeUndefined();
    const after = result.state;
    // Its script ran (1 damage), it was counted as a play, and it paid its price.
    expect(heroHealth(after, "p2")).toBe(before - 1);
    expect(after.counters.played).toBe(played + 1);
    expect(after.players.p1.turnLog.playedIds).toContain(spell.id);
    expect(after.players.p1.mana.current).toBe(3);
    // The play said where it came from; the Spell resolved and landed in the graveyard again.
    const cardPlayed = only(eventsOfType(result.events, "cardPlayed"));
    expect(cardPlayed.from).toBe("graveyard");
    expect(cardPlayed.costPaid).toBe(1);
    expect(eventsOfType(result.events, "cardResolved").map((event) => event.instanceId)).toEqual([spell.id]);
    expect(after.players.p1.graveyard.map((card) => card.id)).toContain(spell.id);
  });

  it("R454 a Unit played from the graveyard is placed, summoning sick, and its Cry fires (R1: played, from anywhere)", () => {
    const state = pbPlaying("r454-unit");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const body = inGraveyard(state, graveUnit.id);
    const before = heroHealth(state, "p2");

    const after = pbAct(state, { type: "play", instanceId: body.id, zone: { row: "units", lane: 3 }, playerId: "p1" });
    const placed = only(activeUnitsOf(after, "p1").filter((unit) => unit.id === body.id));
    expect(placed.zone).toEqual({ z: "field", player: "p1", row: "units", lane: 3 });
    expect(placed.summonedTurn).toBe(after.turn);
    expect(heroHealth(after, "p2")).toBe(before - 2);
    expect(after.players.p1.graveyard.map((card) => card.id)).not.toContain(body.id);
  });

  it("R454 only the player's own graveyard is reached, and only in their main phase with nothing open", () => {
    const state = pbPlaying("r454-own");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const theirs = inGraveyard(state, graveSpell.id, "p2");
    expect(playsOf(state, theirs.id)).toEqual([]);
    expect(pbReduce(state, { type: "play", instanceId: theirs.id, playerId: "p1" }).error).toMatch(/no card .* in p1's hand/);

    // p2's own Second Wind would let p2 — on p2's turn, not p1's.
    put(state, secondWind.id, slot("p2", "backrow", 1));
    expect(playsOf(state, theirs.id, "p2")).toEqual([]);
  });

  it("R454 R65's player prices reach a graveyard play: a next-Spell discount prices and is spent, and an aura's surcharge prices it", () => {
    const state = pbPlaying("r454-prices");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const spell = inGraveyard(state, threeSpell.id);
    state.players.p1.mana.current = 2;
    // (3) is more than 2 mana: not offered.
    expect(playsOf(state, spell.id)).toEqual([]);

    addModifier({ state, events: [] }, "p1", {
      kind: "costDiscount",
      amount: 1,
      onlyType: "Spell",
      expiry: { until: "used" },
    });
    const offered = playsOf(state, spell.id);
    expect(offered).toHaveLength(1);
    const result = pbReduce(state, { ...only(offered), playerId: "p1" });
    expect(result.error).toBeUndefined();
    expect(only(eventsOfType(result.events, "cardPlayed")).costPaid).toBe(2);
    expect(result.state.players.p1.mods.filter((mod) => mod.kind === "costDiscount")).toEqual([]);

    // An aura's surcharge prices it too (Classic #77's shape: Spells cost (1) more).
    const taxed = pbPlaying("r454-aura");
    put(taxed, secondWind.id, slot("p1", "backrow", 1));
    put(taxed, monkey.id, slot("p2", "units", 1));
    const again = inGraveyard(taxed, graveSpell.id);
    const paid = pbReduce(taxed, { type: "play", instanceId: again.id, playerId: "p1" });
    expect(only(eventsOfType(paid.events, "cardPlayed")).costPaid).toBe(2);
  });

  it("R454 R65 a graveyard card is priced as a play wherever it is read while a permission lets its player play it, and at its own cost otherwise", () => {
    const state = pbPlaying("r454-read");
    put(state, monkey.id, slot("p2", "units", 1)); // Spells cost (1) more
    const spell = inGraveyard(state, graveSpell.id);
    expect(effectiveCost(state, spell)).toBe(1);
    const wind = put(state, secondWind.id, slot("p1", "backrow", 1));
    expect(effectiveCost(state, spell)).toBe(2);
    // The view prints that price.
    expect(only(viewFor(state, "p1").you.graveyard.filter((card) => card.instanceId === spell.id)).cost).toBe(2);
    wind.vanilla = true;
    expect(effectiveCost(state, spell)).toBe(1);
  });

  it("R454 Second Wind's Radiant face needs a price of (1) or more, as it would be paid", () => {
    const state = pbPlaying("r454-min-price");
    put(state, secondWind.id, slot("p1", "backrow", 1), { radiant: true });
    const free = inGraveyard(state, zeroSpell.id);
    const one = inGraveyard(state, graveSpell.id);
    expect(playsOf(state, free.id)).toEqual([]);
    expect(pbReduce(state, { type: "play", instanceId: free.id, playerId: "p1" }).error).toMatch(/must cost \(1\) or more/);
    expect(playsOf(state, one.id)).toHaveLength(1);

    // A discount that takes the (1) Spell to (0) takes it below the floor: the price as it would be paid.
    addModifier({ state, events: [] }, "p1", {
      kind: "costDiscount",
      amount: 1,
      expiry: { until: "thisTurn", turn: state.turn },
    });
    expect(playsOf(state, one.id)).toEqual([]);
  });

  it("R454 Corpse Plantation: Units only, paid with its Plague Tokens — at least 1, at most the tokens and the price, the rest in mana", () => {
    const state = pbPlaying("r454-plague");
    const field = put(state, plantation.id, slot("p1", "backrow", 2));
    field.counters.plague = 2;
    const body = inGraveyard(state, graveUnit.id); // (2)
    const spell = inGraveyard(state, graveSpell.id);
    state.players.p1.mana.current = 1;

    // Units only.
    expect(playsOf(state, spell.id)).toEqual([]);
    // 1 token + 1 mana, or 2 tokens; never mana alone under this permission.
    const offered = playsOf(state, body.id).filter((play) => play.zone?.lane === 1);
    expect(offered.map((play) => play.plague)).toEqual([
      { from: field.id, tokens: 1 },
      { from: field.id, tokens: 2 },
    ]);
    expect(pbReduce(state, { type: "play", instanceId: body.id, playerId: "p1" }).error).toMatch(/spending Plague Tokens/);
    expect(pbReduce(state, { type: "play", instanceId: body.id, plague: { from: field.id, tokens: 0 }, playerId: "p1" }).error).toMatch(
      /at least 1 Plague Token/,
    );
    expect(pbReduce(state, { type: "play", instanceId: body.id, plague: { from: field.id, tokens: 3 }, playerId: "p1" }).error).toMatch(
      /not that many/,
    );
    // A hand card never spends tokens.
    const handPlay = pbReduce(state, {
      type: "play",
      instanceId: only(state.players.p1.hand).id,
      plague: { from: field.id, tokens: 1 },
      playerId: "p1",
    });
    expect(handPlay.error).toMatch(/only a play from your graveyard can spend Plague Tokens/);

    const result = pbReduce(state, { type: "play", instanceId: body.id, plague: { from: field.id, tokens: 1 }, playerId: "p1" });
    expect(result.error).toBeUndefined();
    const after = result.state;
    // The price was the whole (2): one token and one mana.
    expect(only(eventsOfType(result.events, "cardPlayed")).costPaid).toBe(2);
    expect(after.players.p1.mana.current).toBe(0);
    expect(findInstance(after, field.id)?.counters.plague).toBe(1);
    expect(eventsOfType(result.events, "counterChanged")).toContainEqual({
      type: "counterChanged",
      instanceId: field.id,
      counter: "plague",
      value: 1,
    });
    expect(activeUnitsOf(after, "p1").map((unit) => unit.id)).toContain(body.id);
  });

  it("R454 two permissions: a Unit may be paid in mana under one and with tokens under the other", () => {
    const state = pbPlaying("r454-both");
    const field = put(state, plantation.id, slot("p1", "backrow", 2));
    field.counters.plague = 1;
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const body = inGraveyard(state, graveUnit.id);
    const payments = playsOf(state, body.id)
      .filter((play) => play.zone?.lane === 1)
      .map((play) => play.plague ?? null);
    expect(payments).toEqual([null, { from: field.id, tokens: 1 }]);
  });

  it("R454 In Too Deep's reward L is the same permission, held while the card says it was earned", () => {
    const state = pbPlaying("r454-quest");
    const quest = put(state, questCard.id, slot("p1", "backrow", 1));
    const spell = inGraveyard(state, graveSpell.id);
    expect(playsOf(state, spell.id)).toEqual([]);
    quest.memory[QUEST_REWARD_KEY] = true;
    expect(playsOf(state, spell.id)).toHaveLength(1);
  });

  it("R454 a Trap played from the graveyard is set face-down under a fresh id, and the other seat reads no more than a zone", () => {
    const state = pbPlaying("r454-trap");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const trap = inGraveyard(state, graveTrap.id);
    const result = pbReduce(state, { type: "play", instanceId: trap.id, zone: { row: "backrow", lane: 4 }, playerId: "p1" });
    expect(result.error).toBeUndefined();
    const placed = result.state.players.p1.backrow[3];
    expect(placed?.defId).toBe(graveTrap.id);
    expect(placed?.id).not.toBe(trap.id);

    const theirs = viewFor(result.state, "p2");
    expect(theirs.opponent.backrow[3]).toMatchObject({ faceDown: true });
    // The raw event names the card and the id it had; p2's copy is the face-down redaction (R97,
    // R227) — no identity, no former id — with the pile it came from, which the public graveyard
    // already showed.
    expect(only(eventsOfType(result.events, "cardPlayed")).formerId).toBe(trap.id);
    const shown = only(eventsOfType(theirs.events, "cardPlayed"));
    expect(shown).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID, from: "graveyard" });
    expect(shown.formerId).toBeUndefined();
    // Its controller reads it (R33).
    const mine = only(eventsOfType(viewFor(result.state, "p1").events, "cardPlayed"));
    expect(mine.defId).toBe(graveTrap.id);
  });

  it("R454 a card no longer in the graveyard at step 4 is not played (R226's rule, from the graveyard)", () => {
    const state = pbPlaying("r454-lost");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const titanCard = inGraveyard(state, titan.id);
    // A body whose Death exiles its owner's graveyard: the Tribute paid at step 2 takes the Titan
    // out of the graveyard before step 4 can.
    const exiler = put(state, "fx-1", slot("p1", "units", 1));
    const scripts = registeredScripts();
    registerScripts({
      ...scripts,
      "fx-1": {
        base: {
          death: () => [
            {
              kind: "test:exileGraveyard",
              apply(ctx): void {
                for (const card of [...ctx.state.players.p1.graveyard]) {
                  if (card.id === titanCard.id) moveToZone(ctx.state, card, "exile");
                }
              },
            },
          ],
        },
        radiant: {},
      },
    });
    try {
      const result = pbReduce(state, {
        type: "play",
        instanceId: titanCard.id,
        zone: { row: "units", lane: 2 },
        tributes: [exiler.id],
        playerId: "p1",
      });
      expect(result.error).toBeUndefined();
      expect(eventsOfType(result.events, "cardPlayed")).toEqual([]);
      expect(result.state.players.p1.exile.map((card) => card.id)).toContain(titanCard.id);
      expect(result.state.counters.played).toBe(state.counters.played);
    } finally {
      registerScripts(scripts);
    }
  });

  it("R454 R455 a ban on the field refuses a graveyard play as it refuses a hand play", () => {
    const state = pbPlaying("r454-ban");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    put(state, lobbyist.id, slot("p2", "units", 1), { radiant: true });
    const spell = inGraveyard(state, threeSpell.id);
    expect(playsOf(state, spell.id)).toEqual([]);
    expect(pbReduce(state, { type: "play", instanceId: spell.id, playerId: "p1" }).error).toMatch(/can't play \(3\)\+ Cost cards/);
  });

  it("R454 a graveyard play that asks pauses, survives JSON, and finishes the same way live and round-tripped", () => {
    const state = pbPlaying("r454-pause");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const asker = inGraveyard(state, discoverSpell.id);
    const paused = pbAct(state, { type: "play", instanceId: asker.id, playerId: "p1" });
    const pending = paused.pending;
    expect(pending?.kind).toBe("discover");
    expect(pending?.playerId).toBe("p1");
    // The other seat sees a prompt is open, never its options (§10.8).
    expect(viewFor(paused, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });

    const round = roundTrip(paused);
    expect(hashState(round)).toBe(hashState(paused));
    const answer = only(legalActions(paused, "p1").filter((action) => action.type === "answer"));
    const live = pbAct(paused, { ...answer, playerId: "p1" });
    const replayed = pbAct(round, { ...answer, playerId: "p1" });
    expect(hashState(replayed)).toBe(hashState(live));
    expect(live.pending).toBeNull();
    expect(live.players.p1.graveyard.map((card) => card.id)).toContain(asker.id);
  });

  it("R454 a graveyard play's cardPlayed reaches the other seat with its source and nothing hidden", () => {
    const state = pbPlaying("r454-view");
    put(state, secondWind.id, slot("p1", "backrow", 1));
    const spell = inGraveyard(state, graveSpell.id);
    const result = pbReduce(state, { type: "play", instanceId: spell.id, playerId: "p1" });
    const event = only(eventsOfType(viewFor(result.state, "p2").events, "cardPlayed"));
    expect(event).toMatchObject({ instanceId: spell.id, defId: graveSpell.id, from: "graveyard" });
    // Engine bookkeeping never travels (R119, R174).
    expect(event.exitsFrom).toBeUndefined();
  });
});
