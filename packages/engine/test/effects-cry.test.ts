// Trigger a Cry (docs/classic-sets.md B5 E13, R467): a Unit's Cry run again, on the field or out of a
// graveyard, for the player whose card triggered it, who makes its choices (R70) — Classic #54 Rewind.
// On the field the Unit is the Cry's "this"; out of a graveyard "this" finds nothing. A Cry's declared
// choices (R81) are asked as prompts, because a triggered Cry has no play to carry them; nothing is
// played, so nothing counts a play; and a Cry that asks parks its own tail ahead of what the
// triggering list still owes (R113).

import type { PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { HERO_HEALTH } from "../src/config";
import { hasTriggerableCry, triggerCry } from "../src/effects";
import { TRIGGER_CRY_HOOK } from "../src/cryTrigger";
import { runHookResumable } from "../src/prompts";
import { makeContext } from "../src/resolve";
import { hashState } from "../src/replay";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { viewFor } from "../src/viewFor";
import { moveToZone, placeOnField } from "../src/zones";
import { plain } from "./fixtures/combat";
import { eventsOfType, put, sinkFor, slot } from "./fixtures/harness";
import {
  aimer,
  asker,
  crier,
  graveRewind,
  moder,
  quickdrawOf,
  rewind,
  spark,
  tributeCrier,
} from "./fixtures/prompts";
import {
  act,
  answerKeys,
  board,
  castNow,
  expectReplays,
  eventTypes,
  handCard,
  must,
  openAs,
  replayable,
  resolvingCard,
  roundTrip,
} from "./fixtures/promptHarness";

/** Rewind resolving with its declared target (R81), as the play pipeline hands it over. */
function rewindOn(state: GameState, target: CardInstance, player: PlayerId = "p1", radiant = false): string[] {
  const sink = sinkFor(state);
  const card = resolvingCard(state, rewind.id, player, radiant);
  const targets: Selection[] = [{ pick: "instance", instanceId: target.id }];
  runHookResumable(sink, card, "cry", { controller: player, targets });
  settle(sink);
  state.rngCursor = sink.rng.cursor;
  return eventTypes(sink.events);
}

function graveUnit(state: GameState, player: PlayerId, defId: string): CardInstance {
  const card = newInstance(state, defId, player, { z: "graveyard", player });
  state.players[player].graveyard.push(card);
  return card;
}

describe("E13: trigger a Cry", () => {
  it("R467 on the field the Cry runs as the Unit, for the triggering card's controller, and is no play", () => {
    const state = board("cry-field");
    const unit = put(state, crier.id, slot("p1", "units", 2));
    const played = state.counters.played;
    const log = { ...state.players.p1.turnLog };
    const events = rewindOn(state, unit);
    // The Cry: 2 to the enemy hero and +1/+1 on "this"; then Rewind's own heal.
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 2);
    expect(unit.buffs).toEqual({ attack: 1, health: 1 });
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH + 1);
    // Nothing was played, cast or announced.
    expect(events).not.toContain("cardPlayed");
    expect(events).not.toContain("cardAnnounced");
    expect(events).not.toContain("cardResolved");
    expect(state.counters.played).toBe(played);
    expect(state.players.p1.turnLog).toEqual(log);
  });

  it("R467 out of a graveyard the Cry runs, and \"this\" finds nothing", () => {
    const state = board("cry-grave");
    const dead = graveUnit(state, "p1", crier.id);
    castNow(state, graveRewind.id);
    openAs(state, "pick", "p1");
    answerKeys(state, `instance:${dead.id}`);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 2);
    expect(dead.buffs).toEqual({ attack: 0, health: 0 });
    expect(dead.zone.z).toBe("graveyard");
  });

  it("R467 the triggering card's controller runs the Cry of the other player's Unit", () => {
    const state = board("cry-enemy");
    const theirs = put(state, crier.id, slot("p2", "units", 1));
    rewindOn(state, theirs, "p1", true);
    // Radiant: twice. "The enemy hero" is the triggering controller's enemy both times.
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 4);
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH + 1);
    expect(theirs.buffs).toEqual({ attack: 2, health: 2 });
  });

  it("R467 a Cry's declared target is asked of the triggering controller, and the other seat sees only that", () => {
    const state = board("cry-target");
    const unit = put(state, aimer.id, slot("p1", "units", 1));
    const enemy = put(state, plain.id, slot("p2", "units", 3));
    rewindOn(state, unit);
    const pending = openAs(state, "target", "p1");
    expect(pending.resume.hook).toBe(TRIGGER_CRY_HOOK);
    expect(pending.options.map((option) => option.selection)).toEqual([
      { pick: "instance", instanceId: enemy.id },
      { pick: "hero", player: "p2" },
    ]);
    expect(pending.options.map((option) => option.label)).toEqual([plain.name, "Enemy hero"]);
    expect(pending.options.map((option) => option.key)).toEqual([`instance:${enemy.id}`, "hero:p2"]);
    expect(viewFor(state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    // Rewind's heal waits behind the Cry.
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH);

    const copy = roundTrip(state);
    answerKeys(state, "hero:p2");
    answerKeys(copy, "hero:p2");
    expect(hashState(copy)).toBe(hashState(state));
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 3);
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH + 1);
    expect(state.pending).toBeNull();
  });

  it("R467 Radiant triggers twice, each run with its own choices", () => {
    const state = board("cry-twice");
    const theirs = put(state, aimer.id, slot("p2", "units", 1));
    const other = put(state, plain.id, slot("p2", "units", 4));
    rewindOn(state, theirs, "p1", true);
    openAs(state, "target", "p1");
    answerKeys(state, `instance:${other.id}`);
    const second = openAs(state, "target", "p1");
    expect(second.resume.hook).toBe(TRIGGER_CRY_HOOK);
    answerKeys(state, "hero:p2");
    // The first run's 3 damage killed the unit it chose; the second run chose the hero.
    expect(other.zone.z).toBe("graveyard");
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 3);
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH + 1);
  });

  it("R467 R90 modes are asked first when a target belongs to a mode, and a target only for its mode", () => {
    const hit = board("cry-mode-hit");
    const unit = put(hit, moder.id, slot("p1", "units", 1));
    rewindOn(hit, unit);
    const mode = openAs(hit, "mode", "p1");
    expect(mode.options.map((option) => option.key)).toEqual(["mode:hit", "mode:heal"]);
    answerKeys(hit, "mode:hit");
    openAs(hit, "target", "p1");
    answerKeys(hit, "hero:p2");
    expect(hit.players.p2.hero.health).toBe(HERO_HEALTH - 4);

    const heal = board("cry-mode-heal");
    const healer = put(heal, moder.id, slot("p1", "units", 1));
    rewindOn(heal, healer);
    answerKeys(heal, "mode:heal");
    expect(heal.pending).toBeNull();
    expect(heal.players.p1.hero.health).toBe(HERO_HEALTH + 4 + 1);
  });

  it("R467 R113 a Cry that asks parks its own tail ahead of the triggering list's", () => {
    const state = board("cry-asks");
    const unit = put(state, asker.id, slot("p1", "units", 1));
    rewindOn(state, unit);
    openAs(state, "mode", "p1");
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 1);
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH);
    const { sink } = answerKeys(state, "mode:left");
    // The answered step (4), then the Cry's own tail (2), and only then Rewind's heal.
    const order = sink.events
      .filter((event) => event.type === "damage" || event.type === "healed")
      .map((event) => (event.type === "damage" ? `damage ${event.amount}` : "heal"));
    expect(order).toEqual(["damage 4", "damage 2", "heal"]);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 7);
  });

  it("R467 R90 R123 a Cry's Tribute is a play's price: nothing is asked or paid, and its slot stays empty", () => {
    const state = board("cry-tribute");
    const unit = put(state, tributeCrier.id, slot("p1", "units", 1));
    const mine = put(state, plain.id, slot("p1", "units", 2));
    rewindOn(state, unit);
    const pending = openAs(state, "target", "p1");
    expect(pending.options.map((option) => option.selection)).toEqual([{ pick: "hero", player: "p2" }]);
    answerKeys(state, "hero:p2");
    // The empty slot is what the script reads as "nothing tributed": 5, and the unit is untouched.
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 5);
    expect(state.players.p1.units[1]?.[0]?.id).toBe(mine.id);
  });

  it("R467 R174 a Unit that leaves the field while its Cry's choices are asked triggers nothing", () => {
    const state = board("cry-left");
    const unit = put(state, aimer.id, slot("p1", "units", 1));
    rewindOn(state, unit);
    openAs(state, "target", "p1");
    moveToZone(state, unit, "graveyard");
    answerKeys(state, "hero:p2");
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH);
    // The triggering list goes on.
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH + 1);
  });

  it("R467 no Cry is triggered for a dormant card, a card with no Cry, or a card that is not a Unit", () => {
    const state = board("cry-none");
    const buried = put(state, crier.id, slot("p1", "units", 1));
    const top = newInstance(state, plain.id, "p1", { z: "hand", player: "p1" });
    expect(placeOnField(state, top, slot("p1", "units", 1), { stack: true })).toBe(true);
    const plainUnit = put(state, plain.id, slot("p1", "units", 2));
    const spell = graveUnit(state, "p1", spark.id);
    const dead = graveUnit(state, "p1", crier.id);
    expect(hasTriggerableCry(state, buried)).toBe(false);
    expect(hasTriggerableCry(state, plainUnit)).toBe(false);
    expect(hasTriggerableCry(state, spell)).toBe(false);
    expect(hasTriggerableCry(state, dead)).toBe(true);
    for (const card of [buried, plainUnit, spell]) {
      const sink = sinkFor(state);
      triggerCry({ instanceId: card.id }).apply(makeContext(sink, null, { controller: "p1" }));
      settle(sink);
      expect(sink.events).toEqual([]);
    }
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH);
  });

  it("R467 a Rewind game replays from its log", () => {
    const aim = quickdrawOf(aimer).id;
    const back = quickdrawOf(rewind).id;
    const { state: dealt, log, decks } = replayable("cry-replay", [aim, back]);
    let state = dealt;
    const unit = handCard(state, "p1", aim);
    state = act(state, { type: "play", playerId: "p1", instanceId: unit.id, targets: [{ pick: "hero", player: "p2" }] }, log);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 3);
    state = act(
      state,
      { type: "play", playerId: "p1", instanceId: handCard(state, "p1", back).id, targets: [{ pick: "instance", instanceId: unit.id }] },
      log,
    );
    const pending = openAs(state, "target", "p1");
    state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [{ pick: "hero", player: "p2" }] }, log);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 6);
    expect(eventsOfType(state.applied.at(-1)?.events ?? [], "cardPlayed")).toEqual([]);
    expectReplays("cry-replay", decks, log, state);
    expect(must(state.players.p1.units[0], "the aimer's pile")[0]?.id).toBe(unit.id);
  });
});
