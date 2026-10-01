// A kill credited to another unit (R42, R412: `killCredit.ts`, `effects/killCredit.ts`'s
// `withKillCredit`), as Classic+ #19.2 Jungle Loser credits Classic+ #19.5 Bot Loser; and whether a card
// is being cast on draw (`castOnDrawNow.ts`, R58), which Classic+ #26 Tommy Tempo's ability reads.
//
// Pinned: with a credit in force, a lethal hit on a paired victim names the paired unit as R42's killer
// — in the `destroyed` event and for its kill triggers — while the striker stays the hit's source; the
// credit is gone once the watched effect is; a kill of an unpaired victim is the striker's own; without
// a transfer the striker keeps the kill and the follow-up runs for the pair whose victim died; a Death
// that asks during the watched attack loses nothing, and the paused game survives JSON and replays. A
// cast-on-draw card's Cry reads `isCastOnDraw` true; the same card played from hand reads it false.

import { describe, expect, it } from "vitest";
import { drawOne } from "../src/draw";
import { KILL_CREDIT_KEY, creditedKillerId } from "../src/killCredit";
import { isBerserk } from "../src/restrictions";
import { findInstance } from "../src/state";
import { answer, deathAsker, grunt, playing, recorder, replaysTo, roundTrip } from "./fixtures/damage-combat";
import { eventsOfType, inHand, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import { bot, jungle, registerKillCredit, tempo } from "./fixtures/killCredit";

function game(seed: string): ReturnType<typeof playing> {
  const state = playing(seed);
  registerKillCredit();
  return state;
}

describe("R42 R412 withKillCredit", () => {
  it("with a transfer, the paired unit is the killer: the destroyed event names it and its kill trigger fires", () => {
    const state = game("kc-transfer");
    const striker = put(state, jungle.id, slot("p1", "units", 2), { radiant: true });
    const credited = put(state, bot.id, slot("p1", "units", 4));
    const victim = put(state, grunt.id, slot("p2", "units", 4));
    const play = recorder(state);
    const ended = play.play({ type: "endTurn", playerId: "p1" });
    expect(eventsOfType(ended.events, "destroyed")).toEqual([expect.objectContaining({ instanceId: victim.id, killerId: credited.id })]);
    // The hit was the striker's: its damage event names it as the source.
    expect(eventsOfType(ended.events, "damage").some((hit) => hit.sourceId === striker.id && hit.targetId === victim.id)).toBe(true);
    const after = play.state();
    expect(findInstance(after, credited.id)?.buffs.attack).toBe(5);
    expect(findInstance(after, striker.id)?.buffs.attack).toBe(0);
    expect(findInstance(after, striker.id)?.memory[KILL_CREDIT_KEY]).toBeUndefined();
    expect(replaysTo(play.start, play.log, after)).toBe(true);
  });

  it("a kill of a victim with no pair stays the striker's", () => {
    const state = game("kc-unpaired");
    const striker = put(state, jungle.id, slot("p1", "units", 2), { radiant: true });
    const credited = put(state, bot.id, slot("p1", "units", 4));
    const victim = put(state, grunt.id, slot("p2", "units", 1));
    const result = recorder(state).play({ type: "endTurn", playerId: "p1" });
    expect(eventsOfType(result.events, "destroyed")).toEqual([expect.objectContaining({ instanceId: victim.id, killerId: striker.id })]);
    expect(findInstance(result.state, credited.id)?.buffs.attack).toBe(0);
  });

  it("without a transfer the striker keeps the kill, and the follow-up runs for the pair whose victim died", () => {
    const state = game("kc-follow");
    const striker = put(state, jungle.id, slot("p1", "units", 2));
    const credited = put(state, bot.id, slot("p1", "units", 4));
    const victim = put(state, grunt.id, slot("p2", "units", 4));
    const result = recorder(state).play({ type: "endTurn", playerId: "p1" });
    expect(eventsOfType(result.events, "destroyed")).toEqual([expect.objectContaining({ instanceId: victim.id, killerId: striker.id })]);
    const live = findInstance(result.state, credited.id);
    expect(live?.buffs.attack).toBe(0);
    expect(live !== undefined && isBerserk(live)).toBe(true);
  });

  it("R113 a Death that asks during the watched attack: the credit already landed, and the pause survives JSON", () => {
    const state = game("kc-pause");
    put(state, jungle.id, slot("p1", "units", 2), { radiant: true });
    const credited = put(state, bot.id, slot("p1", "units", 4));
    const victim = put(state, deathAsker.id, slot("p2", "units", 4));
    const play = recorder(state);
    const ended = play.play({ type: "endTurn", playerId: "p1" });
    expect(eventsOfType(ended.events, "destroyed")).toEqual([expect.objectContaining({ instanceId: victim.id, killerId: credited.id })]);
    const paused = play.state();
    expect(paused.pending).not.toBeNull();
    expect(roundTrip(paused)).toEqual(paused);
    const answered = answer(roundTrip(paused));
    expect(findInstance(answered.state, credited.id)?.buffs.attack).toBe(5);
    expect(answered.state.pending).toBeNull();
  });

  it("creditedKillerId reads the record and falls back to the striker", () => {
    const state = game("kc-read");
    const striker = put(state, jungle.id, slot("p1", "units", 2));
    const victim = put(state, grunt.id, slot("p2", "units", 1));
    expect(creditedKillerId(striker, victim)).toBe(striker.id);
    striker.memory[KILL_CREDIT_KEY] = [{ victimId: victim.id, toId: "c999" }];
    expect(creditedKillerId(striker, victim)).toBe("c999");
    expect(creditedKillerId(striker, { id: "c1000" })).toBe(striker.id);
  });
});

describe("R58 isCastOnDraw", () => {
  it("a cast-on-draw card's Cry reads true, and the draw is complete once the cast resolved", () => {
    const state = game("kc-cast");
    const [drawn] = setLibrary(state, "p1", [tempo.id, grunt.id]);
    if (drawn === undefined) throw new Error("no card");
    expect(drawOne(sinkFor(state), "p1")).toBe("cast");
    expect(findInstance(state, drawn.id)?.memory.castOnDraw).toBe(true);
    expect(state.heldDraws).toBeUndefined();
  });

  it("the same card played from hand reads false", () => {
    const state = game("kc-hand");
    const [card] = inHand(state, tempo.id, "p1");
    if (card === undefined) throw new Error("no card");
    const result = recorder(state).play({ type: "play", instanceId: card.id, zone: { row: "units", lane: 1 }, playerId: "p1" });
    expect(findInstance(result.state, card.id)?.memory.castOnDraw).toBe(false);
  });
});
