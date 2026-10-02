// Cost rules (docs/classic-sets.md B5 E15, E39; R65, R363, R396, R455): the auras and player
// modifiers that price a play, the ladder `mana.effectiveCost` climbs, the ban on (N)+ Cost plays, the
// return-after-resolve enchantment's return and floor, and R396's cost reader.

import { describe, expect, it } from "vitest";
import { HAND_CAP } from "../src/config";
import { addCostRule, cast, castNew, discard } from "../src/effects";
import { costNow, effectiveCost, playCost } from "../src/mana";
import { addModifier } from "../src/modifiers";
import { applyEffects, makeContext, type EngineSink } from "../src/resolve";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { viewFor } from "../src/viewFor";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";
import {
  castTrap,
  embiggenField,
  fiveUnit,
  forever,
  graveSpell,
  inGraveyard,
  lobbyist,
  monkey,
  only,
  pbAct,
  pbPlaying,
  pbReduce,
  playsOf,
  tax,
  threeSpell,
  toeCracker,
  trickster,
  twoField,
  xSpell,
  xUnit,
  zeroSpell,
} from "./fixtures/playPipelineB";

/** R345: no automatic turn ends, so a test walks the turns it names. */
function manualTurns(state: GameState): GameState {
  const one = pbAct(state, { type: "setAutoEndTurn", enabled: false, playerId: "p1" });
  return pbAct(one, { type: "setAutoEndTurn", enabled: false, playerId: "p2" });
}

function handCard(state: GameState, defId: string, player: "p1" | "p2" = "p1"): CardInstance {
  return only(inHand(state, defId, player));
}

function sinkOf(state: GameState): EngineSink {
  return { state, events: [], rng: sinkFor(state).rng };
}

function run(state: GameState, effects: Parameters<typeof applyEffects>[0], controller: "p1" | "p2" = "p1"): EngineSink {
  const sink = sinkFor(state);
  applyEffects(effects, makeContext(sink, null, { controller }));
  settle(sink);
  state.rngCursor = sink.rng.cursor;
  return sink;
}

describe("E15 price rules from auras (R455)", () => {
  it("R455 Classic #6's aura: its controller's Traps and Field Traps cost (0), no other card and nobody else's", () => {
    const state = pbPlaying("r455-traps");
    put(state, toeCracker.id, slot("p1", "units", 1));
    const trap = handCard(state, castTrap.id);
    const spell = handCard(state, graveSpell.id);
    const theirs = handCard(state, castTrap.id, "p2");
    expect(effectiveCost(state, trap)).toBe(0);
    expect(effectiveCost(state, spell)).toBe(1);
    expect(effectiveCost(state, theirs)).toBe(1);
    // In a library it is its own cost (R65): an aura prices a play.
    const inLibrary = newInstance(state, castTrap.id, "p1", { z: "library", player: "p1" });
    expect(effectiveCost(state, inLibrary)).toBe(1);

    const mana = state.players.p1.mana.current;
    const after = pbAct(state, { type: "play", instanceId: trap.id, zone: { row: "backrow", lane: 2 }, playerId: "p1" });
    expect(after.players.p1.mana.current).toBe(mana);
  });

  it("R455 Classic #77's aura: both players' Spells cost (1) more, (2) on its Radiant face; an X card is untouched (R65)", () => {
    const state = pbPlaying("r455-spells");
    const monkeyCard = put(state, monkey.id, slot("p2", "units", 1));
    const mine = handCard(state, graveSpell.id);
    const theirs = handCard(state, graveSpell.id, "p2");
    const unit = handCard(state, fiveUnit.id);
    const x = handCard(state, xSpell.id);
    expect(effectiveCost(state, mine)).toBe(2);
    expect(effectiveCost(state, theirs)).toBe(2);
    expect(effectiveCost(state, unit)).toBe(5);
    expect(effectiveCost(state, { ...x, x: 2 })).toBe(2);
    monkeyCard.radiant = true;
    expect(effectiveCost(state, mine)).toBe(3);
    // A Vanilla card lays no aura (§6.3, R115).
    monkeyCard.vanilla = true;
    expect(effectiveCost(state, mine)).toBe(1);
  });

  it("R455 Classic #68's (3)+ surcharge reads the price the flat rules left, as R363 reads Curvature", () => {
    const state = pbPlaying("r455-threshold");
    put(state, lobbyist.id, slot("p1", "units", 1));
    const two = handCard(state, twoField.id);
    const three = handCard(state, threeSpell.id);
    const five = handCard(state, fiveUnit.id, "p2");
    expect(effectiveCost(state, two)).toBe(2);
    expect(effectiveCost(state, three)).toBe(4);
    expect(effectiveCost(state, five)).toBe(6);

    // A flat surcharge first: #77's Radiant face takes a (1) Spell to 3, which then meets (3)+.
    const monkeyCard = put(state, monkey.id, slot("p2", "units", 2));
    monkeyCard.radiant = true; // Spells +2
    const one = handCard(state, graveSpell.id);
    expect(effectiveCost(state, one)).toBe(4); // 1 + 2 = 3, then (3)+: +1
    // A flat discount first: the (3) Spell taken to 2 no longer meets it.
    addModifier(sinkOf(state), "p1", { kind: "costDiscount", amount: 3, expiry: { until: "thisTurn", turn: state.turn } });
    expect(effectiveCost(state, three)).toBe(2); // 3 + 2 − 3 = 2, under the threshold
    // And Curvature and #68 each read the one number the flat rules left (R363): a (3) card meets
    // #68's (3)+ and not Curvature's (4)+, even though #68 then takes it to 4.
    const fresh = pbPlaying("r455-threshold-curvature");
    put(fresh, lobbyist.id, slot("p1", "units", 1));
    addModifier(sinkOf(fresh), "p1", {
      kind: "costDiscount",
      amount: 1,
      minCurrentCost: 4,
      expiry: { until: "thisTurn", turn: fresh.turn },
    });
    expect(effectiveCost(fresh, handCard(fresh, threeSpell.id))).toBe(4); // 3 → +1
    expect(effectiveCost(fresh, handCard(fresh, fiveUnit.id))).toBe(5); // 5 → −1 +1
  });

  it("R455 Classic #68's Radiant ban: the opponent is offered no play whose price is (3) or more, and refused one; casts go on", () => {
    const state = pbPlaying("r455-ban");
    put(state, lobbyist.id, slot("p2", "units", 1), { radiant: true });
    const three = handCard(state, threeSpell.id);
    const one = handCard(state, graveSpell.id);
    expect(playsOf(state, three.id)).toEqual([]);
    expect(playsOf(state, one.id)).toHaveLength(1);
    expect(pbReduce(state, { type: "play", instanceId: three.id, playerId: "p1" }).error).toMatch(/can't play \(3\)\+ Cost cards/);
    // An X card: X below 3 only.
    const x = handCard(state, xSpell.id);
    expect(playsOf(state, x.id).map((play) => play.x)).toEqual([1, 2]);
    // The price is what bans: a discount under 3 lifts it.
    addModifier(sinkOf(state), "p1", { kind: "costDiscount", amount: 1, expiry: { until: "thisTurn", turn: state.turn } });
    expect(playsOf(state, three.id)).toHaveLength(1);
    // Its controller is not banned, and a cast is not a play from hand (R70).
    const theirs = handCard(state, threeSpell.id, "p2");
    expect(playCost(state, theirs)).toBe(3);
    const before = state.counters.played;
    run(state, [castNew({ def: threeSpell.id })]);
    expect(state.counters.played).toBe(before + 1);
  });
});

describe("E15 price rules on a player (R455)", () => {
  it("R455 Classic #2: the next Trap or Field Spell costs (2) less until one is played, across turns; a Spell or a cast leaves it", () => {
    const state = manualTurns(pbPlaying("r455-next"));
    const card = handCard(state, trickster.id);
    let after = pbAct(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 1 }, playerId: "p1" });
    const rule = only(after.players.p1.mods.filter((mod) => mod.kind === "costRule"));
    expect(only(viewFor(after, "p2").opponent.modifiers.filter((mod) => mod.id === rule.id)).label).toBe(
      "Your next Trap or Field Spell costs (2) less",
    );
    const field = handCard(after, twoField.id);
    const trap = handCard(after, castTrap.id);
    const spell = handCard(after, graveSpell.id);
    expect(effectiveCost(after, field)).toBe(0);
    expect(effectiveCost(after, trap)).toBe(0);
    expect(effectiveCost(after, spell)).toBe(1);

    // A Spell played leaves it; a cast Trap leaves it (a cast never uses a discount, R70).
    after = pbAct(after, { type: "play", instanceId: spell.id, playerId: "p1" });
    run(after, [castNew({ def: castTrap.id })]);
    expect(after.players.p1.mods.some((mod) => mod.id === rule.id)).toBe(true);
    // It waits across turns ("next" has no "this turn").
    after = pbAct(after, { type: "endTurn", playerId: "p1" });
    after = pbAct(after, { type: "endTurn", playerId: "p2" });
    expect(after.players.p1.mods.some((mod) => mod.id === rule.id)).toBe(true);
    // The Field Spell it prices spends it.
    const mana = after.players.p1.mana.current;
    after = pbAct(after, { type: "play", instanceId: field.id, zone: { row: "backrow", lane: 3 }, playerId: "p1" });
    expect(after.players.p1.mana.current).toBe(mana);
    expect(after.players.p1.mods.some((mod) => mod.id === rule.id)).toBe(false);
    expect(effectiveCost(after, trap)).toBe(1);
  });

  it("R455 Classic #2's Radiant face: the next Trap or Field Spell costs (0), which wins over every add", () => {
    const state = pbPlaying("r455-next-zero");
    put(state, lobbyist.id, slot("p2", "units", 1));
    const card = handCard(state, trickster.id);
    card.radiant = true;
    const after = pbAct(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 1 }, playerId: "p1" });
    const big = newInstance(after, twoField.id, "p1", { z: "hand", player: "p1" });
    big.costMod = 3; // (5), and (3)+ would add 1
    after.players.p1.hand.push(big);
    expect(effectiveCost(after, big)).toBe(0);
    expect(only(viewFor(after, "p1").you.modifiers.filter((mod) => mod.label.includes("next"))).label).toBe(
      "Your next Trap or Field Spell costs (0)",
    );
  });

  it("R455 AI Alignment Tax: the opponent's cards cost (1) more during their next turn only (R48's timing, turned outward)", () => {
    const state = manualTurns(pbPlaying("r455-tax"));
    const card = handCard(state, tax.id);
    const theirs = handCard(state, graveSpell.id, "p2");
    let after = pbAct(state, { type: "play", instanceId: card.id, playerId: "p1" });
    // Not on the turn it was cast.
    expect(effectiveCost(after, only(after.players.p2.hand.filter((held) => held.id === theirs.id)))).toBe(1);
    expect(only(viewFor(after, "p1").opponent.modifiers).label).toBe("Your cards cost (1) more (next turn)");
    after = pbAct(after, { type: "endTurn", playerId: "p1" });
    // On their next turn: +1.
    expect(after.active).toBe("p2");
    expect(effectiveCost(after, only(after.players.p2.hand.filter((held) => held.id === theirs.id)))).toBe(2);
    after = pbAct(after, { type: "endTurn", playerId: "p2" });
    // Gone at their cleanup.
    expect(after.players.p2.mods.filter((mod) => mod.kind === "costRule")).toEqual([]);
  });

  it("R455 an added cost rule's spans: this turn ends at cleanup, never stays", () => {
    const state = manualTurns(pbPlaying("r455-spans"));
    run(state, [
      addCostRule({ rule: { amount: 2 }, lasts: "thisTurn" }),
      addCostRule({ rule: { types: ["Spell"], amount: -1 }, lasts: "never" }),
    ]);
    const spell = handCard(state, graveSpell.id);
    expect(effectiveCost(state, spell)).toBe(2);
    const after = pbAct(state, { type: "endTurn", playerId: "p1" });
    expect(after.players.p1.mods.filter((mod) => mod.kind === "costRule")).toHaveLength(1);
    expect(effectiveCost(after, only(after.players.p1.hand.filter((held) => held.id === spell.id)))).toBe(0);
  });
});

describe("E39 return after resolving, and its floor (R455)", () => {
  it("R455 Classic+ #14: the next Spell played gains the return — it comes back after it resolves, every time, and can't cost less than (2)", () => {
    const state = pbPlaying("r455-forever");
    const card = handCard(state, forever.id);
    let after = pbAct(state, { type: "play", instanceId: card.id, playerId: "p1" });
    expect(only(viewFor(after, "p1").you.modifiers).label).toBe(
      "Your next Spell returns to your hand after it resolves (it can't cost less than (2))",
    );
    const spell = handCard(after, graveSpell.id);
    after.players.p1.mana.current = 4;
    const before = after.players.p2.hero.health;
    after = pbAct(after, { type: "play", instanceId: spell.id, playerId: "p1" });
    // Resolved (1 damage), back in hand, carrying the enchantment; the rider is spent.
    expect(after.players.p2.hero.health).toBe(before - 1);
    const back = only(after.players.p1.hand.filter((held) => held.id === spell.id));
    expect(back.enchantments).toEqual([{ kind: "returnAfterResolve", floor: 2 }]);
    expect(after.players.p1.mods.filter((mod) => mod.kind === "enchantNextSpell")).toEqual([]);
    // The floor: a (1) Spell costs (2), after every discount.
    expect(effectiveCost(after, back)).toBe(2);
    addModifier(sinkOf(after), "p1", { kind: "costDiscount", amount: 5, expiry: { until: "thisTurn", turn: after.turn } });
    expect(effectiveCost(after, back)).toBe(2);
    // And it comes back again, and in any pile it keeps the floor.
    after = pbAct(after, { type: "play", instanceId: back.id, playerId: "p1" });
    expect(after.players.p1.hand.map((held) => held.id)).toContain(back.id);
  });

  it("R410 R455 a Spell with the return comes back from the exile it lands in, and not when discarded; a full hand burns it", () => {
    const state = pbPlaying("r455-no-return");
    const spell = handCard(state, graveSpell.id);
    spell.enchantments = [{ kind: "returnAfterResolve", floor: 1 }];
    // Discarded: not played, so nothing resolves and nothing returns.
    const sink = sinkFor(state);
    applyEffects([discard({ target: { of: "instance", instanceId: spell.id } })], makeContext(sink, null, { controller: "p1" }));
    expect(state.players.p1.graveyard.map((card) => card.id)).toContain(spell.id);

    // Cast with "then exile it": it resolves, lands in exile, and comes back from there (R410).
    const other = inGraveyard(state, graveSpell.id);
    other.enchantments = [{ kind: "returnAfterResolve", floor: 1 }];
    const back = run(state, [cast({ target: { of: "instance", instanceId: other.id }, afterward: "exile" })]);
    expect(eventsOfType(back.events, "exiled").map((event) => event.instanceId)).toContain(other.id);
    expect(state.players.p1.hand.map((card) => card.id)).toContain(other.id);
    expect(state.players.p1.exile.map((card) => card.id)).not.toContain(other.id);

    // A full hand burns it on its way back (§2.4).
    const full = pbPlaying("r455-burn");
    const burnt = handCard(full, graveSpell.id);
    burnt.enchantments = [{ kind: "returnAfterResolve", floor: 1 }];
    const filler = HAND_CAP - full.players.p1.hand.length + 1;
    inHand(full, zeroSpell.id, "p1", Math.max(0, filler));
    const played = pbReduce(full, { type: "play", instanceId: burnt.id, playerId: "p1" });
    expect(played.error).toBeUndefined();
    expect(eventsOfType(played.events, "burned").map((event) => event.instanceId)).toContain(burnt.id);
  });

  it("R455 Forever&'s Radiant face floors at (1), and a cast Spell takes the rider too (R70)", () => {
    const state = pbPlaying("r455-forever-radiant");
    const card = handCard(state, forever.id);
    card.radiant = true;
    const after = pbAct(state, { type: "play", instanceId: card.id, playerId: "p1" });
    run(after, [castNew({ def: zeroSpell.id })]);
    const back = only(after.players.p1.hand.filter((held) => held.defId === zeroSpell.id && held.enchantments !== undefined));
    expect(back.enchantments).toEqual([{ kind: "returnAfterResolve", floor: 1 }]);
    expect(effectiveCost(after, back)).toBe(1);
  });
});

describe("R396 an X card on the field costs its X", () => {
  it("R396 costNow: an X card played for X costs X on the field, 0 elsewhere and when it arrived with no X; an embiggen card its base", () => {
    const state = pbPlaying("r396-x");
    const inHandX = handCard(state, xUnit.id);
    expect(costNow(state, inHandX)).toBe(0);
    state.players.p1.mana.current = 3;
    const after = pbAct(state, { type: "play", instanceId: inHandX.id, x: 3, zone: { row: "units", lane: 1 }, playerId: "p1" });
    const onField = only(after.players.p1.units[0] ?? []);
    expect(onField.x).toBe(3);
    expect(costNow(after, onField)).toBe(3);
    // A summon with no chosen X: 0 (R65).
    const summoned = put(after, xUnit.id, slot("p1", "units", 2));
    expect(costNow(after, summoned)).toBe(0);
    // An embiggen card on the field paid its bigger price, and still costs its base (R65).
    const big = put(after, embiggenField.id, slot("p1", "backrow", 1));
    big.embiggened = true;
    expect(costNow(after, big)).toBe(2);
    // Any other card: R65's cost as it stands, a hand card at its hand cost.
    const spell = handCard(after, graveSpell.id);
    put(after, monkey.id, slot("p2", "units", 1));
    expect(costNow(after, spell)).toBe(2);
    // The graveyard and the library read their own cost.
    const grave = inGraveyard(after, xUnit.id);
    expect(costNow(after, grave)).toBe(0);
  });
});
