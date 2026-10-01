// Copy the last Spell's text (docs/classic-sets.md B5 E14; Classic #57 Echo; SPEC §8.6 row 57, R399,
// R545–R547), proved on fixture cards (`fixtures/copiedText.ts`): a copier declares and resolves the
// last Spell's choices and script on the face it was played on, its X and Echo, the continuations of
// its prompts, its Cast on draw, its preview and glow; the copy is fixed as the play begins; it records
// the Spell it copied, never itself; and the opponent's view never names it in hand.

import type { Action, ActionBody, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { draw } from "../src/draw";
import { lastSpellPlayed } from "../src/query";
import { beginGame, legalActions, reduce } from "../src/reduce";
import { castCard } from "../src/resolve";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { COPIED_TEXT_KEY, copiedTextOf, runningScriptOf, textFaceOf } from "../src/subsystems/copiedText";
import { settle } from "../src/triggers";
import { viewFor } from "../src/viewFor";
import { CT, withCopiedText } from "./fixtures/copiedText";
import { eventsOfType, inHand, newGame, put, setLibrary, sinkFor, slot } from "./fixtures/harness";

let nonce = 0;

function game(seed: string, mana = 9): GameState {
  let ready = beginGame(withCopiedText(newGame(seed))).state;
  for (const player of ["p1", "p2"] as const) {
    ready = act(ready, player, { type: "mulligan", keep: ready.players[player].hand.map((card) => card.id) }).state;
  }
  for (const player of ["p1", "p2"] as const) {
    ready.players[player].mana.current = mana;
    ready.players[player].mana.max = mana;
  }
  return ready;
}

function attempt(state: GameState, playerId: PlayerId, body: ActionBody): { state: GameState; events: GameEvent[]; error?: string } {
  nonce += 1;
  return reduce(state, { ...body, playerId, nonce: `ct-${nonce}` } as Action);
}

function act(state: GameState, playerId: PlayerId, body: ActionBody): { state: GameState; events: GameEvent[] } {
  const result = attempt(state, playerId, body);
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

function hand(state: GameState, player: PlayerId, defId: string, radiant = false): CardInstance {
  const [card] = inHand(state, defId, player);
  if (card === undefined) throw new Error("no card");
  card.radiant = radiant;
  return card;
}

const AT_P2 = [{ pick: "hero", player: "p2" }] as const satisfies readonly Selection[];

/** Play a Spell from p1's hand at p2's hero (a bolt) or with no choices. */
function playSpell(state: GameState, defId: string, radiant = false, player: PlayerId = "p1"): GameState {
  const card = hand(state, player, defId, radiant);
  const targets = defId === CT.bolt.id ? [{ pick: "hero" as const, player: player === "p1" ? ("p2" as const) : ("p1" as const) }] : [];
  return act(state, player, { type: "play", instanceId: card.id, ...(targets.length === 0 ? {} : { targets }) }).state;
}

function heroHits(events: readonly GameEvent[], player: PlayerId): number[] {
  return eventsOfType(events, "damage").flatMap((event) => (event.targetId === `hero-${player}` ? [event.amount] : []));
}

function playsOf(state: GameState, player: PlayerId, instanceId: string): Extract<ActionBody, { type: "play" }>[] {
  return legalActions(state, player).filter(
    (action): action is Extract<ActionBody, { type: "play" }> => action.type === "play" && action.instanceId === instanceId,
  );
}

describe("E14 a copier has the last Spell's text (Classic #57 Echo)", () => {
  it("R399 with no Spell played yet it has no text: its play declares nothing, resolves nothing, records nothing", () => {
    const state = game("ct-empty");
    const echo = hand(state, "p1", CT.echo.id);
    expect(copiedTextOf(state, echo)).toBeNull();
    expect(playsOf(state, "p1", echo.id)).toEqual([{ type: "play", instanceId: echo.id }]);
    const health = state.players.p2.hero.health;
    const { state: after, events } = act(state, "p1", { type: "play", instanceId: echo.id });
    expect(after.players.p2.hero.health).toBe(health);
    expect(eventsOfType(events, "cardPlayed")).toHaveLength(1);
    expect(eventsOfType(events, "damage")).toHaveLength(0);
    expect(lastSpellPlayed(after)).toBeNull();
    expect(after.players.p1.graveyard.map((card) => card.id)).toContain(echo.id);
  });

  it("R399 it declares the copied Spell's targets, and legalActions offers exactly the copied Spell's", () => {
    let state = playSpell(game("ct-declare"), CT.bolt.id);
    put(state, CT.dummy.id, slot("p2", "units", 1));
    const echo = hand(state, "p1", CT.echo.id);
    const bolt = hand(state, "p1", CT.bolt.id);
    const strip = (plays: Extract<ActionBody, { type: "play" }>[]) => plays.map((play) => play.targets);
    expect(strip(playsOf(state, "p1", echo.id))).toEqual(strip(playsOf(state, "p1", bolt.id)));
    expect(playsOf(state, "p1", echo.id).length).toBeGreaterThan(1);
    // A play the copied text refuses is refused, by the same rule.
    const refused = attempt(state, "p1", { type: "play", instanceId: echo.id });
    expect(refused.error).toMatch(/target/);
    const health = state.players.p2.hero.health;
    state = act(state, "p1", { type: "play", instanceId: echo.id, targets: [...AT_P2] }).state;
    expect(state.players.p2.hero.health).toBe(health - 2);
  });

  it("R399 it resolves the copied face: a Radiant bolt's number through the copied definition's declared value (R386)", () => {
    const radiant = playSpell(game("ct-face-r"), CT.bolt.id, true);
    const echo = hand(radiant, "p1", CT.echo.id);
    const before = radiant.players.p2.hero.health;
    const { events } = act(radiant, "p1", { type: "play", instanceId: echo.id, targets: [...AT_P2] });
    expect(heroHits(events, "p2")).toEqual([4]);
    expect(radiant.players.p2.hero.health).toBe(before);

    const base = playSpell(game("ct-face-b"), CT.bolt.id, false);
    const echo2 = hand(base, "p1", CT.echo.id);
    const { events: baseEvents } = act(base, "p1", { type: "play", instanceId: echo2.id, targets: [...AT_P2] });
    expect(heroHits(baseEvents, "p2")).toEqual([2]);
  });

  it("R399 B5 E4 a played copier records the Spell it copied on its face, never itself, so a second one copies the same Spell", () => {
    let state = playSpell(game("ct-record"), CT.ping.id, true);
    state = playSpell(state, CT.echo.id);
    expect(lastSpellPlayed(state)).toEqual({ defId: CT.ping.id, radiant: true });
    const second = hand(state, "p1", CT.echo.id);
    expect(copiedTextOf(state, second)).toEqual({ defId: CT.ping.id, radiant: true });
    const { state: after, events } = act(state, "p1", { type: "play", instanceId: second.id });
    expect(heroHits(events, "p2")).toEqual([3]);
    expect(lastSpellPlayed(after)).toEqual({ defId: CT.ping.id, radiant: true });
  });

  it("R399 the copied modes are declared with the play and resolved", () => {
    const state = game("ct-modes");
    const modal = hand(state, "p1", CT.modal.id);
    const after = act(state, "p1", { type: "play", instanceId: modal.id, modes: ["hit"] }).state;
    const echo = hand(after, "p1", CT.echo.id);
    expect(playsOf(after, "p1", echo.id).map((play) => play.modes)).toEqual([["hit"], ["heal"]]);
    after.players.p1.hero.health = 20;
    const healed = act(after, "p1", { type: "play", instanceId: echo.id, modes: ["heal"] }).state;
    expect(healed.players.p1.hero.health).toBe(22);
  });

  it("R545 an X-cost text: X is chosen with the play, from 1 up to the mana left once the copier's own price is paid", () => {
    let state = game("ct-x", 4);
    const xBolt = hand(state, "p1", CT.xBolt.id);
    state = act(state, "p1", { type: "play", instanceId: xBolt.id, x: 1 }).state;
    expect(state.players.p1.mana.current).toBe(3);
    const echo = hand(state, "p1", CT.echo.id);
    // 3 mana: the copier costs 1, so X is 1 or 2.
    expect(playsOf(state, "p1", echo.id).map((play) => play.x)).toEqual([1, 2]);
    expect(attempt(state, "p1", { type: "play", instanceId: echo.id, x: 3 }).error).toMatch(/X is above/);
    expect(attempt(state, "p1", { type: "play", instanceId: echo.id }).error).toMatch(/X must be at least/);
    const { state: after, events } = act(state, "p1", { type: "play", instanceId: echo.id, x: 2 });
    expect(heroHits(events, "p2")).toEqual([2]);
    expect(after.players.p1.mana.current).toBe(2);
    expect(eventsOfType(events, "cardPlayed")[0]?.costPaid).toBe(1);
  });

  it("R545 with no mana left after the copier's price, an X-cost text cannot be played at all", () => {
    let state = game("ct-x-none", 2);
    const xBolt = hand(state, "p1", CT.xBolt.id);
    state = act(state, "p1", { type: "play", instanceId: xBolt.id, x: 1 }).state;
    expect(state.players.p1.mana.current).toBe(1);
    const echo = hand(state, "p1", CT.echo.id);
    expect(playsOf(state, "p1", echo.id)).toEqual([]);
    expect(attempt(state, "p1", { type: "play", instanceId: echo.id, x: 1 }).error).toMatch(/X is above/);
  });

  it("R545 B5 E12 a cast copier with an X-cost text asks its caster for X, as any cast X card", () => {
    let state = game("ct-x-cast", 3);
    const xBolt = hand(state, "p1", CT.xBolt.id);
    state = act(state, "p1", { type: "play", instanceId: xBolt.id, x: 1 }).state;
    const echo = newInstance(state, CT.echo.id, "p1", { z: "hand", player: "p1" });
    const sink = sinkFor(state);
    castCard(sink, echo);
    settle(sink);
    state.rngCursor = sink.rng.cursor;
    const pending = state.pending;
    expect(pending?.kind).toBe("number");
    // A cast pays nothing: X up to the caster's current mana (2).
    expect(pending?.options.map((option) => option.label)).toEqual(["1", "2"]);
    const { events } = act(state, "p1", { type: "answer", choiceId: pending?.id ?? "", selection: [{ pick: "mode", option: "2" }] });
    expect(heroHits(events, "p2")).toEqual([2]);
  });

  it("R545 an embiggen text resolves at its base price: the copier pays its own and offers no embiggen choice", () => {
    let state = game("ct-embiggen");
    const bigger = hand(state, "p1", CT.bigger.id);
    state = act(state, "p1", { type: "play", instanceId: bigger.id, embiggen: true }).state;
    const echo = hand(state, "p1", CT.echo.id);
    expect(playsOf(state, "p1", echo.id)).toEqual([{ type: "play", instanceId: echo.id }]);
    expect(attempt(state, "p1", { type: "play", instanceId: echo.id, embiggen: true }).error).toMatch(/embiggen/);
    const { events } = act(state, "p1", { type: "play", instanceId: echo.id });
    expect(heroHits(events, "p2")).toEqual([1]);
    expect(eventsOfType(events, "cardPlayed")[0]?.costPaid).toBe(1);
  });

  it("R546 a prompt in the copied text resumes into the copied definition, after a JSON round trip, with its declared number", () => {
    let state = game("ct-prompt");
    const dummy = put(state, CT.dummy.id, slot("p2", "units", 1));
    const asker = hand(state, "p1", CT.asker.id, true);
    state = act(state, "p1", { type: "play", instanceId: asker.id }).state;
    expect(state.pending?.resume.defId).toBe(CT.asker.id);
    state = act(state, "p1", { type: "answer", choiceId: state.pending?.id ?? "", selection: [{ pick: "instance", instanceId: dummy.id }] }).state;
    expect(state.players.p2.units[0]?.[0]?.damage).toBe(6);

    const echo = hand(state, "p1", CT.echo.id);
    state = act(state, "p1", { type: "play", instanceId: echo.id }).state;
    const pending = state.pending;
    expect(pending?.playerId).toBe("p1");
    // The continuation names the copied Spell's definition and face, with the copier as its card.
    expect(pending?.resume.defId).toBe(CT.asker.id);
    expect(pending?.resume.radiant).toBe(true);
    expect(pending?.resume.instanceId).toBe(echo.id);
    const round = JSON.parse(JSON.stringify(state)) as GameState;
    const { state: after, events } = act(round, "p1", {
      type: "answer",
      choiceId: pending?.id ?? "",
      selection: [{ pick: "instance", instanceId: dummy.id }],
    });
    expect(eventsOfType(events, "damage").map((event) => event.amount)).toEqual([6]);
    // 6 + 6 on a 1/9: the copied text's declared number killed it.
    expect(eventsOfType(events, "destroyed").map((event) => event.instanceId)).toEqual([dummy.id]);
    expect(after.players.p1.graveyard.map((card) => card.id)).toContain(echo.id);
  });

  it("R546 the copy is fixed as the play begins: a Spell its own resolution casts changes neither it nor its Echo repeat", () => {
    const state = playSpell(game("ct-fixed"), CT.caster.id);
    expect(lastSpellPlayed(state)).toEqual({ defId: CT.ping.id, radiant: false });
    // Every caster play leaves the ping it cast as the last Spell, so the record is set to the one a
    // caster that had cast nothing would leave; then a Radiant copier copies it.
    state.lastSpell = { defId: CT.caster.id, radiant: false };
    const echo = hand(state, "p1", CT.echo.id, true);
    const { state: after, events } = act(state, "p1", { type: "play", instanceId: echo.id });
    // Each resolution casts a ping (1) and then deals 5: twice, though the ping is last after the first.
    expect(heroHits(events, "p2")).toEqual([1, 5, 1, 5]);
    expect(lastSpellPlayed(after)).toEqual({ defId: CT.ping.id, radiant: false });
    expect(after.players.p1.graveyard.find((card) => card.id === echo.id)?.memory[COPIED_TEXT_KEY]).toBeUndefined();
  });

  it("R546 Radiant Echo 1 repeats the copied text, asking a fresh pick for the repeat", () => {
    let state = playSpell(game("ct-repeat"), CT.bolt.id);
    const dummy = put(state, CT.dummy.id, slot("p2", "units", 1));
    const echo = hand(state, "p1", CT.echo.id, true);
    state = act(state, "p1", { type: "play", instanceId: echo.id, targets: [...AT_P2] }).state;
    const pending = state.pending;
    expect(pending?.prompt).toMatch(/Echo/);
    const round = JSON.parse(JSON.stringify(state)) as GameState;
    const after = act(round, "p1", {
      type: "answer",
      choiceId: pending?.id ?? "",
      selection: [{ pick: "instance", instanceId: dummy.id }],
    }).state;
    expect(after.players.p2.units[0]?.[0]?.damage).toBe(2);
  });

  it("R546 a copied face's own Echo X adds to the copier's own", () => {
    const base = playSpell(game("ct-echo-sum-b"), CT.echoSpell.id);
    const echo = hand(base, "p1", CT.echo.id);
    expect(heroHits(act(base, "p1", { type: "play", instanceId: echo.id }).events, "p2")).toEqual([1, 1]);
    const radiant = playSpell(game("ct-echo-sum-r"), CT.echoSpell.id);
    const echoR = hand(radiant, "p1", CT.echo.id, true);
    expect(heroHits(act(radiant, "p1", { type: "play", instanceId: echoR.id }).events, "p2")).toEqual([1, 1, 1]);
  });

  it("R399 B5 E1 a countered Spell is never recorded, so it is not copied; nor is a countered copier", () => {
    let state = playSpell(game("ct-countered"), CT.ping.id);
    put(state, CT.counter.id, slot("p1", "backrow", 1));
    state = act(state, "p1", { type: "endTurn" }).state;
    state.players.p2.mana.current = 9;
    const bolt = hand(state, "p2", CT.bolt.id);
    state = act(state, "p2", { type: "play", instanceId: bolt.id, targets: [{ pick: "hero", player: "p1" }] }).state;
    expect(lastSpellPlayed(state)).toEqual({ defId: CT.ping.id, radiant: false });
    // The trap is spent; a second trap counters p2's copier, which records nothing either.
    put(state, CT.counter.id, slot("p1", "backrow", 2));
    const echo = hand(state, "p2", CT.echo.id);
    expect(copiedTextOf(state, echo)).toEqual({ defId: CT.ping.id, radiant: false });
    const { events } = act(state, "p2", { type: "play", instanceId: echo.id });
    expect(eventsOfType(events, "countered")).toHaveLength(1);
    expect(heroHits(events, "p1")).toEqual([]);
  });

  it("R399 a Field Spell is never the last Spell", () => {
    let state = playSpell(game("ct-field"), CT.ping.id);
    const field = hand(state, "p1", CT.field.id);
    state = act(state, "p1", { type: "play", instanceId: field.id }).state;
    expect(lastSpellPlayed(state)).toEqual({ defId: CT.ping.id, radiant: false });
  });

  it("R399 a cast copier copies the Spell that is last as the cast begins (R70)", () => {
    const state = playSpell(game("ct-cast"), CT.ping.id, true);
    const echo = newInstance(state, CT.echo.id, "p1", { z: "hand", player: "p1" });
    const sink = sinkFor(state);
    castCard(sink, echo);
    settle(sink);
    expect(heroHits(sink.events, "p2")).toEqual([3]);
    expect(lastSpellPlayed(state)).toEqual({ defId: CT.ping.id, radiant: true });
  });

  it("R547 a copier drawn while the last Spell casts on draw is cast, and the draw repeats", () => {
    const state = game("ct-draw-cast");
    const first = newInstance(state, CT.drawCast.id, "p1", { z: "hand", player: "p1" });
    const sink = sinkFor(state);
    castCard(sink, first);
    settle(sink);
    expect(lastSpellPlayed(state)).toEqual({ defId: CT.drawCast.id, radiant: false });
    const [echo, next] = setLibrary(state, "p1", [CT.echo.id, CT.ping.id]);
    const drawing = sinkFor(state);
    draw(drawing, "p1", 1);
    settle(drawing);
    expect(heroHits(drawing.events, "p2")).toEqual([1]);
    expect(state.players.p1.graveyard.map((card) => card.id)).toContain(echo?.id);
    expect(state.players.p1.hand.map((card) => card.id)).toContain(next?.id);
  });

  it("R547 a copier drawn while the last Spell does not cast on draw goes to the hand", () => {
    const state = playSpell(game("ct-draw-plain"), CT.ping.id);
    const [echo] = setLibrary(state, "p1", [CT.echo.id, CT.ping.id]);
    const drawing = sinkFor(state);
    draw(drawing, "p1", 1);
    settle(drawing);
    expect(state.players.p1.hand.map((card) => card.id)).toContain(echo?.id);
    expect(heroHits(drawing.events, "p2")).toEqual([]);
  });

  it("R547 the copied face's preview and yellow glow answer for the copier in hand, on that face's numbers", () => {
    let state = game("ct-glow");
    const glow = hand(state, "p1", CT.glow.id, true);
    state = act(state, "p1", { type: "play", instanceId: glow.id, targets: [...AT_P2] }).state;
    const echo = hand(state, "p1", CT.echo.id);
    expect(runningScriptOf(state, echo).preview).toBeTypeOf("function");
    expect(textFaceOf(state, echo)).toMatchObject({ id: echo.id, defId: CT.glow.id, radiant: true });
    const view = viewFor(state, "p1");
    const card = Array.isArray(view.you.hand) ? view.you.hand.find((held) => held.instanceId === echo.id) : undefined;
    expect(card?.preview).toEqual([{ label: "damage", value: 4 }]);
    expect(card?.conditionActive).toBe(true);
  });

  it("R399 R243 the owner's hand view carries the copied face and its numbers; the opponent's never names the card", () => {
    const empty = game("ct-view-empty");
    const blank = hand(empty, "p1", CT.echo.id);
    const blankView = viewFor(empty, "p1");
    const blankCard = Array.isArray(blankView.you.hand) ? blankView.you.hand.find((held) => held.instanceId === blank.id) : undefined;
    expect(blankCard).toBeDefined();
    expect(blankCard).not.toHaveProperty("copies");

    const state = playSpell(game("ct-view"), CT.bolt.id, true);
    const echo = hand(state, "p1", CT.echo.id);
    const own = viewFor(state, "p1");
    const card = Array.isArray(own.you.hand) ? own.you.hand.find((held) => held.instanceId === echo.id) : undefined;
    expect(card?.defId).toBe(CT.echo.id);
    expect(card?.cost).toBe(1);
    expect(card?.copies).toEqual({ defId: CT.bolt.id, radiant: true, params: { damage: 4 } });
    const theirs = viewFor(state, "p2");
    expect(theirs.opponent.hand).toEqual({ count: state.players.p1.hand.length });
    expect(JSON.stringify(theirs)).not.toContain(echo.id);
  });
});
