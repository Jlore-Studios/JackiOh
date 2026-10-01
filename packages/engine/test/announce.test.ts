// The announce window and Counter (docs/classic-sets.md B5 E1, E2; R448): §10.5 gains a step between 3
// and 4 — the paid-for card moves to its player's resolving zone, `cardAnnounced` goes out, and a
// window runs in which Counters answer it, traps first and then the other triggers the announce woke.
// The first Counter cancels the play, which then never resolves and is never counted; the card goes to
// its owner's graveyard, to exile, or to the countering player's hand as theirs (E2).
//
// Pinned here, on fixture cards of the shapes Classic #4, #10, #17, #72 and #87 and AI Refusal have
// (`fixtures/playPipelineA.ts`): the timing, what a countered play does not do, the second Counter that
// stays set, a non-trap response, a cast's announce, the card out of a discard's reach, a question in
// the window (pause, JSON round trip, replay from the log), the steal and its hand cap, and the
// redaction of a face-down card's announce.

import type { Action, ActionBody, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { openAnnounces } from "../src/announce";
import { DECK_SIZE, HAND_CAP, HERO_HEALTH } from "../src/config";
import { beginGame, legalActions, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { castCard } from "../src/resolve";
import { createGame, newInstance, type CardInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { cardAt } from "../src/zones";
import { vanillaDeck } from "./fixtures/catalog";
import { eventsOfType, inHand, newGame, put, setupCatalog, sinkFor, slot } from "./fixtures/harness";
import { PA, registerPlayA, withPlayA } from "./fixtures/playPipelineA";

let nonce = 0;

/** A game in p1's first main phase, both mulligans kept, with this file's fixtures registered. */
function game(seed: string): GameState {
  let ready = beginGame(withPlayA(newGame(seed))).state;
  for (const player of ["p1", "p2"] as const) {
    ready = must(ready, player, { type: "mulligan", keep: ready.players[player].hand.map((card) => card.id) }).state;
  }
  for (const player of ["p1", "p2"] as const) {
    ready.players[player].mana.current = 8;
    ready.players[player].mana.max = 8;
  }
  return ready;
}

function must(state: GameState, playerId: PlayerId, body: ActionBody): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, playerId, nonce: `pa-an-${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

function typesOf(events: readonly GameEvent[]): string[] {
  return events.map((event) => event.type);
}

function hand(state: GameState, player: PlayerId, defId: string): CardInstance {
  const [card] = inHand(state, defId, player);
  if (card === undefined) throw new Error("no card");
  return card;
}

describe("R448 the announce: between §10.5 steps 3 and 4", () => {
  it("R448 announces a paid-for play before it moves, and a play nobody answers goes on as before", () => {
    const state = game("r448-plain");
    const crier = hand(state, "p1", PA.crier.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: crier.id, zone: { row: "units", lane: 2 } });

    const order = typesOf(events);
    expect(order.indexOf("manaChanged")).toBeLessThan(order.indexOf("cardAnnounced"));
    expect(order.indexOf("cardAnnounced")).toBeLessThan(order.indexOf("cardPlayed"));
    expect(eventsOfType(events, "cardAnnounced")).toEqual([
      {
        type: "cardAnnounced",
        player: "p1",
        instanceId: crier.id,
        defId: PA.crier.id,
        cardType: "Unit",
        costPaid: 1,
        targets: [],
        row: "units",
        lane: 2,
      },
    ]);
    // The Cry resolved, the card stands where it was played, and no window is left open.
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 3);
    expect(cardAt(after, slot("p1", "units", 2))?.id).toBe(crier.id);
    expect(openAnnounces(after)).toEqual([]);
    expect(after.announcing).toBeUndefined();
  });

  it("R448 names the declared targets, a hero as hero-<player>", () => {
    const state = game("r448-targets");
    const bolt = hand(state, "p1", PA.bolt.id);
    const unit = put(state, "fx-3", slot("p2", "units", 1));

    const first = must(state, "p1", { type: "play", instanceId: bolt.id, targets: [{ pick: "hero", player: "p2" }] });
    expect(eventsOfType(first.events, "cardAnnounced")[0]?.targets).toEqual(["hero-p2"]);

    const second = hand(first.state, "p1", PA.bolt.id);
    const next = must(first.state, "p1", {
      type: "play",
      instanceId: second.id,
      targets: [{ pick: "instance", instanceId: unit.id }],
    });
    expect(eventsOfType(next.events, "cardAnnounced")[0]?.targets).toEqual([unit.id]);
    expect(eventsOfType(next.events, "cardAnnounced")[0]?.cardType).toBe("Spell");
  });
});

describe("R448 a countered play never resolves and counts for nothing", () => {
  it("R448 a Counter trap cancels the play: no Cry, no cardPlayed, no cardResolved, mana spent, nothing counted", () => {
    const state = game("r448-counter");
    const trap = put(state, PA.counterTrap.id, slot("p2", "backrow", 1));
    const crier = hand(state, "p1", PA.crier.id);
    const mana = state.players.p1.mana.current;

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: crier.id });

    expect(eventsOfType(events, "countered")).toEqual([
      { type: "countered", player: "p1", instanceId: crier.id, defId: PA.crier.id, byInstanceId: trap.id, to: "graveyard" },
    ]);
    for (const absent of ["cardPlayed", "summoned", "cardResolved"] as const) {
      expect(eventsOfType(events, absent)).toEqual([]);
    }
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH);
    expect(after.players.p1.graveyard.map((card) => card.id)).toContain(crier.id);
    expect(after.players.p1.resolving).toEqual([]);
    expect(after.players.p1.mana.current).toBe(mana - 1);
    // §10.5 step 4 never ran, so no count moved: the turn log, the game counter, E4's records.
    expect(after.players.p1.turnLog.cardsPlayed).toBe(0);
    expect(after.players.p1.turnLog.playedIds).toEqual([]);
    expect(after.players.p1.turnLog.playedByType).toBeUndefined();
    expect(after.counters.played).toBe(0);
    expect(after.players.p1.gameLog).toBeUndefined();
    // The trap fired and was consumed (§5.1).
    expect(after.players.p2.graveyard.map((card) => card.id)).toContain(trap.id);
  });

  it("R448 the first Counter cancels the play, and a second finds no card and stays set", () => {
    const state = game("r448-second");
    const first = put(state, PA.counterTrap.id, slot("p2", "backrow", 1));
    const second = put(state, PA.counterTrap2.id, slot("p2", "backrow", 2));
    const ping = hand(state, "p1", PA.ping.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: ping.id });

    expect(eventsOfType(events, "trapFired").map((event) => event.instanceId)).toEqual([first.id]);
    expect(eventsOfType(events, "countered")).toHaveLength(1);
    const standing = cardAt(after, slot("p2", "backrow", 2));
    expect(standing?.id).toBe(second.id);
    expect(standing?.faceUp).not.toBe(true);
  });

  it("R448 a countered Spell's Echo repeats never happen", () => {
    const state = game("r448-echo");
    put(state, PA.counterTrap.id, slot("p2", "backrow", 1));
    const target = put(state, "fx-3", slot("p2", "units", 1));
    const echo = hand(state, "p1", PA.echoBolt.id);

    const { state: after, events } = must(state, "p1", {
      type: "play",
      instanceId: echo.id,
      targets: [{ pick: "instance", instanceId: target.id }],
    });

    expect(eventsOfType(events, "damage")).toEqual([]);
    expect(after.echoQueue).toEqual([]);
    expect(after.pending).toBeNull();
  });

  it("R448 a Counter that is not a trap answers in the window, after the traps (Classic #87's shape)", () => {
    const state = game("r448-chalice");
    put(state, PA.chalice.id, slot("p2", "backrow", 1));
    const crier = hand(state, "p1", PA.crier.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: crier.id });

    expect(eventsOfType(events, "countered").map((event) => event.instanceId)).toEqual([crier.id]);
    expect(eventsOfType(events, "cardPlayed")).toEqual([]);
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH);

    // A play that paid 0 is not the chalice's: it resolves.
    const field = hand(after, "p1", PA.field.id);
    const next = must(after, "p1", { type: "play", instanceId: field.id });
    expect(eventsOfType(next.events, "countered")).toEqual([]);
    expect(eventsOfType(next.events, "cardPlayed").map((event) => event.instanceId)).toEqual([field.id]);
  });

  it("R448 a countered card goes to exile when the Counter says so (Classic #10), and exile counts it (R55)", () => {
    const state = game("r448-exile");
    put(state, PA.exileTrap.id, slot("p2", "backrow", 1));
    const ping = hand(state, "p1", PA.ping.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: ping.id });

    expect(eventsOfType(events, "countered")[0]?.to).toBe("exile");
    expect(after.players.p1.exile.map((card) => card.id)).toEqual([ping.id]);
    expect(after.counters.exiled).toBe(1);
    expect(eventsOfType(events, "exiled").map((event) => event.instanceId)).toEqual([ping.id]);
  });

  it("R448 a response that counters a Spell by its declared targets reads them off the announce (AI Refusal)", () => {
    const state = game("r448-refusal");
    put(state, PA.refusal.id, slot("p2", "backrow", 1));
    const mine = put(state, "fx-3", slot("p2", "units", 1));
    const bolt = hand(state, "p1", PA.bolt.id);
    const other = hand(state, "p1", PA.bolt.id);

    // At the hero: not a Spell that targets one of p2's units, so it resolves.
    const first = must(state, "p1", { type: "play", instanceId: bolt.id, targets: [{ pick: "hero", player: "p2" }] });
    expect(eventsOfType(first.events, "countered")).toEqual([]);
    const second = must(first.state, "p1", {
      type: "play",
      instanceId: other.id,
      targets: [{ pick: "instance", instanceId: mine.id }],
    });
    expect(eventsOfType(second.events, "countered").map((event) => event.instanceId)).toEqual([other.id]);
    expect(second.state.players.p2.units[0]?.[0]?.damage ?? 0).toBe(0);
  });
});

describe("R448 where the card waits: the resolving zone, out of every hand's reach", () => {
  it("R448 a discard of the player's hand in the window does not reach the card being played", () => {
    const state = game("r448-shred");
    put(state, PA.shredder.id, slot("p2", "backrow", 1));
    const crier = hand(state, "p1", PA.crier.id);
    const others = state.players.p1.hand.filter((card) => card.id !== crier.id).map((card) => card.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: crier.id });

    const discarded = eventsOfType(events, "discarded").map((event) => event.instanceId);
    expect(discarded.sort()).toEqual([...others].sort());
    expect(discarded).not.toContain(crier.id);
    expect(after.players.p1.hand).toEqual([]);
    expect(cardAt(after, slot("p1", "units", 1))?.id).toBe(crier.id);
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 3);
  });
});

describe("R448 a cast is announced, and can be countered (R70)", () => {
  it("R448 a card an effect casts is announced from the resolving zone, and a Counter cancels it", () => {
    const state = game("r448-cast");
    const trap = put(state, PA.counterTrap.id, slot("p2", "backrow", 1));
    const ping = newInstance(state, PA.ping.id, "p1", { z: "hand", player: "p1" });

    const sink = sinkFor(state);
    castCard(sink, ping);
    settle(sink);

    expect(eventsOfType(sink.events, "cardAnnounced").map((event) => [event.instanceId, event.costPaid])).toEqual([
      [ping.id, 0],
    ]);
    expect(eventsOfType(sink.events, "countered").map((event) => event.byInstanceId)).toEqual([trap.id]);
    expect(eventsOfType(sink.events, "cardPlayed")).toEqual([]);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH);
    expect(state.players.p1.graveyard.map((card) => card.id)).toContain(ping.id);
    expect(state.players.p1.turnLog.cardsPlayed).toBe(0);
    expect(state.announcing).toBeUndefined();
  });

  it("R448 an uncountered cast is announced and then played as before", () => {
    const state = game("r448-cast-plain");
    const ping = newInstance(state, PA.ping.id, "p1", { z: "hand", player: "p1" });

    const sink = sinkFor(state);
    castCard(sink, ping);
    settle(sink);

    expect(typesOf(sink.events).filter((type) => type === "cardAnnounced" || type === "cardPlayed")).toEqual([
      "cardAnnounced",
      "cardPlayed",
    ]);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 1);
    expect(state.players.p1.turnLog.cardsPlayed).toBe(1);
  });
});

describe("E2 a Counter that steals: the card goes to the thief's hand, and the thief owns it", () => {
  it("R448 the stolen card is the thief's own, in their hand (Classic #72 Radiant)", () => {
    const state = game("r448-steal");
    put(state, PA.stealTrap.id, slot("p2", "backrow", 1));
    const crier = hand(state, "p1", PA.crier.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: crier.id });

    expect(eventsOfType(events, "countered")[0]?.to).toBe("hand");
    expect(eventsOfType(events, "stolen")).toEqual([
      {
        type: "stolen",
        instanceId: crier.id,
        defId: PA.crier.id,
        from: "p1",
        to: "p2",
        zone: "resolving",
        // R466: announced face up, so both seats read the card it was.
        readableFrom: ["p1", "p2"],
      },
    ]);
    const taken = after.players.p2.hand.find((card) => card.id === crier.id);
    expect(taken?.owner).toBe("p2");
    expect(taken?.controller).toBe("p2");
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH);
    // R97: the thief reads its own hand card. (What the victim reads of a card taken off the stack
    // is the `stolen` redaction's own rule, which the prompts workstream settles.)
    expect(eventsOfType(viewFor(after, "p2").events, "stolen")[0]?.instanceId).toBe(crier.id);
  });

  it("R448 a thief with a full hand burns the stolen card into their own graveyard (§2.4)", () => {
    const state = game("r448-steal-burn");
    put(state, PA.stealTrap.id, slot("p2", "backrow", 1));
    while (state.players.p2.hand.length < HAND_CAP) inHand(state, "fx-5", "p2");
    const crier = hand(state, "p1", PA.crier.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: crier.id });

    expect(eventsOfType(events, "countered")[0]?.to).toBe("graveyard");
    expect(eventsOfType(events, "burned").map((event) => event.instanceId)).toEqual([crier.id]);
    expect(after.players.p2.graveyard.find((card) => card.id === crier.id)?.owner).toBe("p2");
    expect(after.players.p1.graveyard.some((card) => card.id === crier.id)).toBe(false);
  });
});

describe("R448 a question in the window (Classic #4 Palantir's shape)", () => {
  function paused(seed: string): { state: GameState; bolt: CardInstance; palantir: CardInstance } {
    const state = game(seed);
    const palantir = put(state, PA.palantir.id, slot("p2", "backrow", 1));
    const bolt = hand(state, "p1", PA.bolt.id);
    const { state: waiting } = must(state, "p1", { type: "play", instanceId: bolt.id, targets: [{ pick: "hero", player: "p2" }] });
    return { state: waiting, bolt, palantir };
  }

  it("R448 pauses the play with its card in the resolving zone and the rest of the play owed", () => {
    const { state, bolt } = paused("r448-pause");
    expect(state.pending?.playerId).toBe("p2");
    expect(state.players.p1.resolving.map((card) => card.id)).toEqual([bolt.id]);
    expect(state.announcing).toEqual([{ instanceId: bolt.id, player: "p1" }]);
    expect(state.work.some((item) => item.resume.hook === "play" && item.resume.step === "announce")).toBe(true);
    // p1 may only wait; p2 answers.
    expect(legalActions(state, "p1").map((action) => action.type)).toEqual(["concede"]);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH);
  });

  it("R448 the answer 'steal' sacrifices the Field Spell and takes the Spell, which never resolves", () => {
    const { state, bolt, palantir } = paused("r448-steal-answer");
    const { state: after, events } = must(state, "p2", {
      type: "answer",
      choiceId: state.pending?.id ?? "",
      selection: [{ pick: "mode", option: "steal" }],
    });
    expect(after.pending).toBeNull();
    expect(after.players.p2.graveyard.map((card) => card.id)).toContain(palantir.id);
    expect(after.players.p2.hand.find((card) => card.id === bolt.id)?.owner).toBe("p2");
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH);
    expect(eventsOfType(events, "cardPlayed")).toEqual([]);
    expect(after.work).toEqual([]);
    expect(after.announcing).toBeUndefined();
  });

  it("R448 the answer 'pass' lets the play go on to step 4 and resolve", () => {
    const { state, bolt } = paused("r448-pass");
    const { state: after, events } = must(state, "p2", {
      type: "answer",
      choiceId: state.pending?.id ?? "",
      selection: [{ pick: "mode", option: "pass" }],
    });
    expect(eventsOfType(events, "cardPlayed").map((event) => event.instanceId)).toEqual([bolt.id]);
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 2);
    expect(after.players.p1.graveyard.map((card) => card.id)).toContain(bolt.id);
    expect(after.players.p1.turnLog.cardsPlayed).toBe(1);
  });

  it("R448 the paused window survives JSON.parse(JSON.stringify(state)) and answers the same way", () => {
    const { state } = paused("r448-json");
    const round = JSON.parse(JSON.stringify(state)) as GameState;
    expect(round).toEqual(state);
    expect(hashState(round)).toBe(hashState(state));
    const answer: ActionBody = { type: "answer", choiceId: state.pending?.id ?? "", selection: [{ pick: "mode", option: "steal" }] };
    const live = reduce(state, { ...answer, playerId: "p2", nonce: "json-live" } as Action);
    const revived = reduce(round, { ...answer, playerId: "p2", nonce: "json-live" } as Action);
    expect(revived.error).toBeUndefined();
    expect(hashState(revived.state)).toBe(hashState(live.state));
    expect(revived.events).toEqual(live.events);
  });

  it("R448 a game with a question in a window replays from its log to the same hash", () => {
    const seed = "r448-fold";
    const decks: [string[], string[]] = [
      [PA.qBolt.id, ...vanillaDeck(DECK_SIZE - 1, 1)],
      [PA.qPalantir.id, ...vanillaDeck(DECK_SIZE - 1, 21)],
    ];
    setupCatalog();
    registerPlayA();
    const start = createGame({ seed, decks });
    let state = beginGame(start).state;
    const log: Action[] = [];
    const step = (playerId: PlayerId, body: ActionBody): void => {
      const action = { ...body, playerId, nonce: `fold-${log.length}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(`${body.type}: ${result.error}`);
      log.push(action);
      state = result.state;
    };
    for (const player of ["p1", "p2"] as const) {
      step(player, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id) });
    }
    // p1's first turn passes; p2 sets down its Palantir; p1 plays the Bolt into its question.
    step("p1", { type: "endTurn" });
    const palantir = state.players.p2.hand.find((card) => card.defId === PA.qPalantir.id);
    step("p2", { type: "play", instanceId: palantir?.id ?? "" });
    step("p2", { type: "endTurn" });
    const bolt = state.players.p1.hand.find((card) => card.defId === PA.qBolt.id);
    step("p1", { type: "play", instanceId: bolt?.id ?? "", targets: [{ pick: "hero", player: "p2" }] });
    expect(state.pending?.playerId).toBe("p2");
    const pausedHash = hashState(state);
    step("p2", { type: "answer", choiceId: state.pending?.id ?? "", selection: [{ pick: "mode", option: "steal" }] });
    expect(state.players.p2.hand.some((card) => card.id === bolt?.id)).toBe(true);

    const replayed = fold({ seed, decks, log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
    const partial = fold({ seed, decks, log: log.slice(0, -1) });
    expect(hashState(partial.state)).toBe(pausedHash);
  });
});

describe("R448 a card being set face-down is its player's alone while it waits (R97, R227)", () => {
  function setting(seed: string, trapDef: string): { state: GameState; trap: CardInstance } {
    const state = game(seed);
    put(state, PA.watcher.id, slot("p2", "backrow", 1));
    const trap = hand(state, "p1", trapDef);
    const { state: waiting } = must(state, "p1", { type: "play", instanceId: trap.id, zone: { row: "backrow", lane: 3 } });
    return { state: waiting, trap };
  }

  it("R448 the other player reads only the zone of the announce, a card back in the resolving zone, and no type", () => {
    const { state, trap } = setting("r448-hidden", PA.hiddenFieldTrap.id);
    expect(state.pending?.playerId).toBe("p2");

    const theirs = viewFor(state, "p2");
    expect(eventsOfType(theirs.events, "cardAnnounced")).toEqual([
      {
        type: "cardAnnounced",
        player: "p1",
        instanceId: HIDDEN_ID,
        defId: HIDDEN_ID,
        cardType: "Trap",
        costPaid: 0,
        targets: [],
        row: "backrow",
        lane: 3,
        faceDown: true,
      },
    ]);
    expect(theirs.opponent.resolving).toEqual([{ instanceId: HIDDEN_ID, defId: HIDDEN_ID, radiant: false, cost: -1 }]);
    expect(JSON.stringify(theirs)).not.toContain(trap.id);
    expect(JSON.stringify(theirs)).not.toContain(PA.hiddenFieldTrap.id);

    const own = viewFor(state, "p1");
    expect(eventsOfType(own.events, "cardAnnounced")[0]?.defId).toBe(PA.hiddenFieldTrap.id);
    expect(eventsOfType(own.events, "cardAnnounced")[0]?.cardType).toBe("Field Trap");
    expect(own.you.resolving.map((card) => card.instanceId)).toEqual([trap.id]);
  });

  it("R448 once set, the trap is face-down under a fresh id, and the announce stays unread", () => {
    const { state, trap } = setting("r448-hidden-set", PA.hiddenTrap.id);
    const { state: after } = must(state, "p2", {
      type: "answer",
      choiceId: state.pending?.id ?? "",
      selection: [{ pick: "mode", option: "noted" }],
    });
    const set = cardAt(after, slot("p1", "backrow", 3));
    expect(set?.defId).toBe(PA.hiddenTrap.id);
    expect(set?.id).not.toBe(trap.id);
    const theirs = viewFor(after, "p2");
    expect(eventsOfType(theirs.events, "cardAnnounced")[0]?.instanceId).toBe(HIDDEN_ID);
    expect(JSON.stringify(theirs.events)).not.toContain(PA.hiddenTrap.id);
    expect(theirs.opponent.resolving).toEqual([]);
  });

  it("R448 a face-up play waiting in the window is public to both players", () => {
    const state = game("r448-public");
    put(state, PA.watcher.id, slot("p2", "backrow", 1));
    const crier = hand(state, "p1", PA.crier.id);
    const { state: waiting } = must(state, "p1", { type: "play", instanceId: crier.id });
    const theirs = viewFor(waiting, "p2");
    expect(theirs.opponent.resolving.map((card) => card.defId)).toEqual([PA.crier.id]);
    expect(eventsOfType(theirs.events, "cardAnnounced")[0]?.defId).toBe(PA.crier.id);
  });
});
