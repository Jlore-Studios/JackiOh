// §10.5 step 3's two v0.2.0 rules (R449): a play replaced by another card (Classic #23 Devil's Pact)
// and plays made Radiant by tag (Classic+ #68 Organic Produce: R213's rule by tag, read at step 1 as
// R214 reads Gifted Program).
//
// R449: a live `replacePlays` modifier replaces each card its player plays — a cast included (R70) —
// by a new instance of the named definition (Radiant per the modifier). The old card ceases to exist,
// the price paid was the old card's, and the new card makes its own choices as a cast does, before
// the announce, so a Counter reads them; it takes its zone at step 4 by R64.

import type { Action, ActionBody, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { HERO_HEALTH } from "../src/config";
import { playActionsFor, resolvingFace } from "../src/playChoices";
import { beginGame, legalActions, reduce } from "../src/reduce";
import { hashState } from "../src/replay";
import { castCard } from "../src/resolve";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { viewFor, HIDDEN_ID } from "../src/viewFor";
import { cardAt } from "../src/zones";
import { eventsOfType, inHand, newGame, put, sinkFor, slot } from "./fixtures/harness";
import { PA, withPlayA } from "./fixtures/playPipelineA";

let nonce = 0;

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
  const result = reduce(state, { ...body, playerId, nonce: `pa-s3-${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

function one(state: GameState, player: PlayerId, defId: string, radiant = false): CardInstance {
  const [card] = inHand(state, defId, player);
  if (card === undefined) throw new Error("no card");
  card.radiant = radiant;
  return card;
}

const ENEMY_HERO: Selection = { pick: "hero", player: "p2" };

/** A game in which p1 has played a Devil's Pact this turn (its modifier live), and the state after. */
function pacted(seed: string, radiant = false): GameState {
  const state = game(seed);
  const pact = one(state, "p1", PA.pact.id, radiant);
  return must(state, "p1", { type: "play", instanceId: pact.id }).state;
}

describe("R449 a play replaced at step 3 (Classic #23 Devil's Pact)", () => {
  it("R449 the card played becomes the named card, which asks its target and resolves as that play", () => {
    const state = pacted("r449-replace");
    const crier = one(state, "p1", PA.crier.id);
    const mana = state.players.p1.mana.current;

    const { state: asked, events } = must(state, "p1", { type: "play", instanceId: crier.id, zone: { row: "units", lane: 3 } });

    const replaced = eventsOfType(events, "transformed")[0];
    expect(replaced).toMatchObject({ instanceId: crier.id, fromDefId: PA.crier.id, toDefId: PA.book.id, hiddenFrom: ["p2"] });
    const bookId = replaced?.newInstanceId ?? "";
    // The price paid was the old card's, and the old card has ceased to exist.
    expect(asked.players.p1.mana.current).toBe(mana - 1);
    expect(asked.players.p1.hand.some((card) => card.id === crier.id)).toBe(false);
    // The Book of Flame asks for its target now, before the announce.
    expect(asked.pending?.kind).toBe("target");
    expect(asked.pending?.prompt).toContain("Play:");
    expect(eventsOfType(events, "cardAnnounced")).toEqual([]);
    expect(asked.players.p1.resolving.map((card) => card.id)).toEqual([bookId]);

    const { state: after, events: resolved } = must(asked, "p1", {
      type: "answer",
      choiceId: asked.pending?.id ?? "",
      selection: [ENEMY_HERO],
    });
    expect(eventsOfType(resolved, "cardAnnounced")[0]).toMatchObject({
      instanceId: bookId,
      defId: PA.book.id,
      cardType: "Spell",
      costPaid: 1,
      targets: ["hero-p2"],
    });
    expect(eventsOfType(resolved, "cardPlayed").map((event) => [event.instanceId, event.costPaid])).toEqual([[bookId, 1]]);
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 4);
    // The crier's Cry never happened and nothing stands in lane 3.
    expect(cardAt(after, slot("p1", "units", 3))).toBeNull();
    expect(after.players.p1.turnLog.playedByType).toEqual({ Spell: 2 });
    expect(after.players.p1.graveyard.map((card) => card.id)).toContain(bookId);
  });

  it("R449 R177 the other player never reads the card that was replaced in the hand", () => {
    const state = pacted("r449-hidden");
    const crier = one(state, "p1", PA.crier.id);
    const { state: asked } = must(state, "p1", { type: "play", instanceId: crier.id });
    const theirs = eventsOfType(viewFor(asked, "p2").events, "transformed")[0];
    expect(theirs?.fromDefId).toBe(HIDDEN_ID);
    expect(JSON.stringify(viewFor(asked, "p2").events)).not.toContain(PA.crier.id);
  });

  it("R449 the Radiant modifier replaces with the Radiant card", () => {
    const state = pacted("r449-radiant", true);
    const ping = one(state, "p1", PA.ping.id);
    const { state: asked } = must(state, "p1", { type: "play", instanceId: ping.id });
    const { state: after } = must(asked, "p1", { type: "answer", choiceId: asked.pending?.id ?? "", selection: [ENEMY_HERO] });
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 8);
    expect(after.lastSpell).toEqual({ defId: PA.book.id, radiant: true });
  });

  it("R449 a cast is a play and is replaced too (R70)", () => {
    const state = pacted("r449-cast");
    const ping = newInstance(state, PA.ping.id, "p1", { z: "hand", player: "p1" });
    const sink = sinkFor(state);
    castCard(sink, ping);
    settle(sink);
    expect(eventsOfType(sink.events, "transformed")[0]?.toDefId).toBe(PA.book.id);
    expect(state.pending?.prompt).toContain("Cast:");
    const { state: after } = must(state, "p1", { type: "answer", choiceId: state.pending?.id ?? "", selection: [ENEMY_HERO] });
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 4);
  });

  it("R449 the named card itself is not replaced by itself", () => {
    const state = pacted("r449-self");
    const book = one(state, "p1", PA.book.id);
    const { state: after, events } = must(state, "p1", { type: "play", instanceId: book.id, targets: [ENEMY_HERO] });
    expect(eventsOfType(events, "transformed")).toEqual([]);
    expect(eventsOfType(events, "cardPlayed").map((event) => event.instanceId)).toEqual([book.id]);
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 4);
  });

  it("R449 a replacement that is a permanent takes the leftmost open zone at step 4 (R64)", () => {
    const state = game("r449-permanent");
    put(state, "fx-1", slot("p1", "units", 1));
    state.players.p1.mods.push({
      id: "m-test",
      kind: "replacePlays",
      defId: PA.crier.id,
      radiant: false,
      expiry: { until: "thisTurn", turn: state.turn },
    });
    const ping = one(state, "p1", PA.ping.id);
    const { state: after } = must(state, "p1", { type: "play", instanceId: ping.id });
    expect(cardAt(after, slot("p1", "units", 2))?.defId).toBe(PA.crier.id);
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 3);
  });

  it("R449 the modifier lasts this turn only", () => {
    const state = pacted("r449-expiry");
    let after = must(state, "p1", { type: "endTurn" }).state;
    after = must(after, "p2", { type: "endTurn" }).state;
    after.players.p1.mana.current = 8;
    const ping = one(after, "p1", PA.ping.id);
    const { events } = must(after, "p1", { type: "play", instanceId: ping.id });
    expect(eventsOfType(events, "transformed")).toEqual([]);
    expect(after.players.p1.mods.some((mod) => mod.kind === "replacePlays")).toBe(false);
  });

  it("R449 the replacement's own question pauses the play: JSON round trip and replay agree", () => {
    const run = (): { paused: string; done: string; revived: string } => {
      const state = pacted("r449-pause");
      const crier = one(state, "p1", PA.crier.id);
      const { state: asked } = must(state, "p1", { type: "play", instanceId: crier.id });
      const body: ActionBody = { type: "answer", choiceId: asked.pending?.id ?? "", selection: [ENEMY_HERO] };
      const round = JSON.parse(JSON.stringify(asked)) as GameState;
      const live = reduce(asked, { ...body, playerId: "p1", nonce: "pause-answer" } as Action);
      const revived = reduce(round, { ...body, playerId: "p1", nonce: "pause-answer" } as Action);
      return { paused: hashState(asked), done: hashState(live.state), revived: hashState(revived.state) };
    };
    const first = run();
    const second = run();
    expect(first.revived).toBe(first.done);
    expect(second).toEqual(first);
  });

  it("R449 a Counter reads the replacement's announce and cancels it", () => {
    const state = pacted("r449-counter");
    put(state, PA.refusal.id, slot("p2", "backrow", 1));
    const mine = put(state, "fx-1", slot("p2", "units", 1));
    const ping = one(state, "p1", PA.ping.id);
    const { state: asked } = must(state, "p1", { type: "play", instanceId: ping.id });
    const { state: after, events } = must(asked, "p1", {
      type: "answer",
      choiceId: asked.pending?.id ?? "",
      selection: [{ pick: "instance", instanceId: mine.id }],
    });
    expect(eventsOfType(events, "countered")[0]?.defId).toBe(PA.book.id);
    expect(cardAt(after, slot("p2", "units", 1))?.damage).toBe(0);
  });
});

describe("R449 by tag: plays made Radiant at step 3 (Classic+ #68 Organic Produce, R213's rule by tag)", () => {
  it("R449 every Fruit its controller plays becomes Radiant as it is played, and nothing else does", () => {
    const state = game("r213-produce");
    put(state, PA.produce.id, slot("p1", "backrow", 1));
    const pear = one(state, "p1", PA.pear.id);
    const ping = one(state, "p1", PA.ping.id);

    const first = must(state, "p1", { type: "play", instanceId: pear.id });
    expect(eventsOfType(first.events, "radiantSet").map((event) => event.instanceId)).toEqual([pear.id]);
    expect(cardAt(first.state, slot("p1", "units", 1))?.radiant).toBe(true);
    const second = must(first.state, "p1", { type: "play", instanceId: ping.id });
    expect(eventsOfType(second.events, "radiantSet")).toEqual([]);
  });

  it("R449 step 1 reads the Radiant face (R214), so the play carries that face's choices", () => {
    const state = game("r214-produce");
    put(state, PA.produce.id, slot("p1", "backrow", 1));
    const apple = one(state, "p1", PA.apple.id);
    expect(resolvingFace(state, "p1", apple, 1).radiant).toBe(true);
    const plays = playActionsFor(state, "p1", apple);
    expect(plays.every((play) => (play.targets ?? []).length === 1)).toBe(true);
    expect(legalActions(state, "p1")).toContainEqual({ type: "play", instanceId: apple.id, targets: [ENEMY_HERO] });
    const { state: after } = must(state, "p1", { type: "play", instanceId: apple.id, targets: [ENEMY_HERO] });
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 3);
  });

  it("R449 the opponent's Organic Produce does nothing for this player, and a cast Fruit is Radiant too (R70)", () => {
    const state = game("r213-produce-theirs");
    put(state, PA.produce.id, slot("p2", "backrow", 1));
    const pear = one(state, "p1", PA.pear.id);
    const { state: after } = must(state, "p1", { type: "play", instanceId: pear.id });
    expect(cardAt(after, slot("p1", "units", 1))?.radiant).toBe(false);

    put(after, PA.produce.id, slot("p1", "backrow", 2));
    const cast = newInstance(after, PA.pear.id, "p1", { z: "hand", player: "p1" });
    const sink = sinkFor(after);
    castCard(sink, cast);
    settle(sink);
    expect(eventsOfType(sink.events, "radiantSet").map((event) => event.instanceId)).toEqual([cast.id]);
  });
});
