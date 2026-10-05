// The targeting point and what a declaration may pick (docs/classic-sets.md B5 E5, E9, E35; R450).
//
// "Targeting" is choosing: a declared `target` pick of a play, a cast or an activation, and a `target`
// prompt's answer (random picks, "all" effects, Tributes, hand and zone picks target nothing). At that
// point a targeting cost is owed (Classic #89 Paul Allen's Ghost: random discards at pay time, R662,
// and with too few other cards the card is no legal target at all), and an interceptor answers
// (Classic #33 Joro, from its owner's hand: summoned, no Cry, summoning sick, and the pick moves to
// it). A Spell's declarations never offer a card Immune to Spells, and the v0.2.0 filter fields — a
// graveyard pick, a cost range, damaged, Plague Tokens, a named predicate — narrow what a declaration
// offers.

import type { Action, ActionBody, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { HERO_HEALTH } from "../src/config";
import { declaredTargets, legalSelectionsFor, playActionsFor } from "../src/playChoices";
import { beginGame, legalActions, reduce } from "../src/reduce";
import type { CardInstance, GameState, PendingChoice } from "../src/state";
import { interceptTargeting, whyTargetAnswerRefused } from "../src/targetingPoint";
import { canPayToTarget, targetingDiscardsOf } from "../src/targeting";
import { viewFor } from "../src/viewFor";
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
    ready.players[player].hand = [];
  }
  return ready;
}

function attempt(state: GameState, playerId: PlayerId, body: ActionBody): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, playerId, nonce: `pa-tg-${nonce}` } as Action);
}

function must(state: GameState, playerId: PlayerId, body: ActionBody): { state: GameState; events: GameEvent[] } {
  const result = attempt(state, playerId, body);
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

function hand(state: GameState, player: PlayerId, defId: string, count = 1): CardInstance[] {
  return inHand(state, defId, player, count);
}

function one(state: GameState, player: PlayerId, defId: string): CardInstance {
  const [card] = hand(state, player, defId);
  if (card === undefined) throw new Error("no card");
  return card;
}

function at(card: CardInstance): Selection {
  return { pick: "instance", instanceId: card.id };
}

function offered(state: GameState, player: PlayerId, card: CardInstance): Selection[] {
  const decl = declaredTargets(card)[0];
  if (decl === undefined) throw new Error("no declaration");
  return legalSelectionsFor(state, player, card, decl);
}

function answer(state: GameState, playerId: PlayerId, selection: Selection[]): { state: GameState; events: GameEvent[] } {
  return must(state, playerId, { type: "answer", choiceId: state.pending?.id ?? "", selection });
}

describe("R450 the v0.2.0 filter fields (§10.6)", () => {
  it("R450 a graveyard pick offers either side's graveyard cards the filter admits", () => {
    const state = game("r450-graveyard");
    const mine = put(state, "fx-1", slot("p1", "units", 1));
    const theirs = put(state, "fx-2", slot("p2", "units", 1));
    // Move them to graveyards, with a Spell beside them that the type filter refuses.
    state.players.p1.units[0] = null;
    state.players.p2.units[0] = null;
    mine.zone = { z: "graveyard", player: "p1" };
    theirs.zone = { z: "graveyard", player: "p2" };
    state.players.p1.graveyard.push(mine);
    state.players.p2.graveyard.push(theirs);
    const spell = one(state, "p1", PA.ping.id);
    state.players.p1.hand = state.players.p1.hand.filter((card) => card.id !== spell.id);
    spell.zone = { z: "graveyard", player: "p1" };
    state.players.p1.graveyard.push(spell);
    const raiser = one(state, "p1", PA.graveRaiser.id);

    expect(offered(state, "p1", raiser)).toEqual([at(mine), at(theirs)]);
  });

  it("R450 a cost range reads R65's cost where the card is now, and no hero has a cost", () => {
    const state = game("r450-cost");
    const cheap = one(state, "p1", "fx-3");
    const dear = one(state, "p1", PA.ghost.id);
    const discounted = one(state, "p1", PA.ghost.id);
    discounted.costMod = -1;
    const field = put(state, "fx-4", slot("p2", "units", 2));
    const hunter = one(state, "p1", PA.cheapHunter.id);

    const picks = offered(state, "p1", hunter);
    expect(picks).toContainEqual(at(cheap));
    expect(picks).toContainEqual(at(discounted));
    expect(picks).toContainEqual(at(field));
    expect(picks).not.toContainEqual(at(dear));
    expect(picks.some((pick) => pick.pick === "hero")).toBe(false);
  });

  it("R450 damaged and Plague Token filters offer only the cards that match", () => {
    const state = game("r450-damaged");
    const hurt = put(state, "fx-1", slot("p2", "units", 1));
    const whole = put(state, "fx-2", slot("p2", "units", 2));
    hurt.damage = 1;
    const plagued = put(state, PA.field.id, slot("p2", "backrow", 1));
    plagued.counters.plague = 2;
    const clean = put(state, PA.chalice.id, slot("p2", "backrow", 2));
    const medic = one(state, "p1", PA.medic.id);
    const hunter = one(state, "p1", PA.plagueHunter.id);

    expect(offered(state, "p1", medic)).toEqual([at(hurt)]);
    expect(offered(state, "p1", hunter)).toEqual([at(plagued)]);
    expect(offered(state, "p1", hunter)).not.toContainEqual(at(clean));
    expect(offered(state, "p1", hunter)).not.toContainEqual(at(whole));
  });

  it("R450 a named predicate is asked for cards and zones, and a hero it does not admit is not offered", () => {
    const state = game("r450-check");
    const lane1 = put(state, "fx-1", slot("p2", "units", 1));
    const lane2 = put(state, "fx-2", slot("p2", "units", 2));
    const hunter = one(state, "p1", PA.laneHunter.id);

    const picks = offered(state, "p1", hunter);
    expect(picks).toContainEqual(at(lane2));
    expect(picks).not.toContainEqual(at(lane1));
    expect(picks).toContainEqual({ pick: "zone", player: "p1", row: "units", lane: 2 });
    expect(picks).toContainEqual({ pick: "zone", player: "p1", row: "backrow", lane: 2 });
    expect(picks.every((pick) => pick.pick !== "hero")).toBe(true);
    expect(picks.every((pick) => pick.pick !== "zone" || pick.lane === 2)).toBe(true);
  });
});

describe("R450 Immune to Spells (E35): a Spell's declarations never offer it", () => {
  it("R450 a Spell cannot name an immune unit, and a Unit's Cry can", () => {
    const state = game("r450-immune");
    const immune = put(state, PA.immune.id, slot("p2", "units", 1));
    const bolt = one(state, "p1", PA.bolt.id);
    const zapper = one(state, "p1", PA.zapper.id);

    expect(offered(state, "p1", bolt)).not.toContainEqual(at(immune));
    expect(offered(state, "p1", zapper)).toContainEqual(at(immune));
    expect(attempt(state, "p1", { type: "play", instanceId: bolt.id, targets: [at(immune)] }).error).toBeDefined();
    const { state: after } = must(state, "p1", { type: "play", instanceId: zapper.id, targets: [at(immune)] });
    expect(after.players.p2.units[0]?.[0]?.damage).toBe(1);
  });
});

describe("R450 a targeting cost (Classic #89 Paul Allen's Ghost)", () => {
  it("R450 R662 a declared target naming it lists one play with no discards carried, offered only when payable", () => {
    const state = game("r450-ghost-list");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const other = put(state, "fx-1", slot("p2", "units", 2));
    const bolt = one(state, "p1", PA.bolt.id);
    hand(state, "p1", "fx-5", 3);

    expect(targetingDiscardsOf(state, ghost)).toBe(2);
    const plays = playActionsFor(state, "p1", bolt);
    const atGhost = plays.filter((play) => play.targets?.[0]?.pick === "instance" && play.targets[0].instanceId === ghost.id);
    const atOther = plays.filter((play) => play.targets?.[0]?.pick === "instance" && play.targets[0].instanceId === other.id);
    // R662: the discards are random at pay time, so one play, carrying none.
    expect(atGhost).toHaveLength(1);
    expect(atOther).toHaveLength(1);
    expect(atGhost.every((play) => !("discards" in play))).toBe(true);
    expect(atOther.every((play) => !("discards" in play))).toBe(true);
    expect(legalActions(state, "p1").filter((action) => action.type === "play" && action.instanceId === bolt.id)).toHaveLength(
      plays.length,
    );
  });

  it("R662 a targeting cost is never a choice: with exactly the cost held, the play pays both with no prompt", () => {
    const state = game("r640-exact-cost");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const bolt = one(state, "p1", PA.bolt.id);
    const [a, b] = hand(state, "p1", "fx-5", 2);
    const { state: after, events } = must(state, "p1", { type: "play", instanceId: bolt.id, targets: [at(ghost)] });
    expect(after.pending).toBeNull();
    expect(eventsOfType(events, "discarded").map((event) => event.instanceId).sort()).toEqual([a?.id, b?.id].sort());
    expect(after.players.p1.hand).toHaveLength(0);
  });

  it("R450 R662 refuses a play naming it when too few other cards are held, and takes none otherwise", () => {
    const state = game("r450-ghost-refuse");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const bolt = one(state, "p1", PA.bolt.id);
    hand(state, "p1", "fx-5", 1);
    // Bolt plus one other: only one card outside the played card, fewer than the two owed — no legal target.
    expect(attempt(state, "p1", { type: "play", instanceId: bolt.id, targets: [at(ghost)] }).error).toBeDefined();
    expect(attempt(state, "p1", { type: "play", instanceId: bolt.id, targets: [{ pick: "hero", player: "p2" }] }).error).toBeUndefined();

    const rich = game("r450-ghost-afford");
    const richGhost = put(rich, PA.ghost.id, slot("p2", "units", 1));
    const richBolt = one(rich, "p1", PA.bolt.id);
    hand(rich, "p1", "fx-5", 2);
    expect(attempt(rich, "p1", { type: "play", instanceId: richBolt.id, targets: [at(richGhost)] }).error).toBeUndefined();
  });

  it("R450 R662 two costly picks each payable alone but not together are refused as a cost", () => {
    const state = game("r450-ghost-sum");
    const first = put(state, PA.ghost.id, slot("p2", "units", 1));
    const second = put(state, PA.ghost.id, slot("p2", "units", 2));
    const twin = one(state, "p1", PA.twin.id);
    hand(state, "p1", "fx-5", 3);
    // Each pick is offered alone (three others pay either two), but the pair owes four.
    expect(offered(state, "p1", twin)).toContainEqual(at(first));
    expect(offered(state, "p1", twin)).toContainEqual(at(second));
    expect(attempt(state, "p1", { type: "play", instanceId: twin.id, targets: [at(first), at(second)] }).error).toMatch(/discard/);
  });

  it("R450 with fewer than two other cards in hand it is no legal target", () => {
    const state = game("r450-ghost-few");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const bolt = one(state, "p1", PA.bolt.id);
    hand(state, "p1", "fx-5", 1);
    expect(canPayToTarget(state, "p1", ghost, bolt.id)).toBe(false);
    expect(offered(state, "p1", bolt)).not.toContainEqual(at(ghost));
  });

  it("R450 R662 two random other cards are paid at step 2 with the mana, and the play resolves at the card", () => {
    const state = game("r450-ghost-pay");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const bolt = one(state, "p1", PA.bolt.id);
    const [a, b, c] = hand(state, "p1", "fx-5", 3);
    const spareIds = [a?.id, b?.id, c?.id];

    const { state: after, events } = must(state, "p1", {
      type: "play",
      instanceId: bolt.id,
      targets: [at(ghost)],
    });

    const order = events.map((event) => event.type);
    const discarded = eventsOfType(events, "discarded").map((event) => event.instanceId);
    expect(discarded).toHaveLength(2);
    // Random, but never the card being played: both come from the three spares.
    for (const id of discarded) expect(spareIds).toContain(id);
    expect(new Set(discarded).size).toBe(2);
    expect(order.indexOf("manaChanged")).toBeLessThan(order.indexOf("discarded"));
    expect(order.lastIndexOf("discarded")).toBeLessThan(order.indexOf("cardAnnounced"));
    expect(after.players.p1.hand).toHaveLength(1);
    expect(spareIds).toContain(after.players.p1.hand[0]?.id);
    expect(cardAt(after, slot("p2", "units", 1))?.damage).toBe(2);
  });

  it("R450 R662 it binds its own controller too", () => {
    const state = game("r450-ghost-own");
    const ghost = put(state, PA.ghost.id, slot("p1", "units", 1));
    const bolt = one(state, "p1", PA.bolt.id);
    hand(state, "p1", "fx-5", 1);
    // Bolt plus one other: unpayable even for its controller — no legal target.
    expect(attempt(state, "p1", { type: "play", instanceId: bolt.id, targets: [at(ghost)] }).error).toBeDefined();
    expect(offered(state, "p1", bolt)).not.toContainEqual(at(ghost));

    const rich = game("r450-ghost-own-rich");
    const ownGhost = put(rich, PA.ghost.id, slot("p1", "units", 1));
    const richBolt = one(rich, "p1", PA.bolt.id);
    hand(rich, "p1", "fx-5", 2);
    // Payable — and the two random discards land on its controller all the same.
    const { events } = must(rich, "p1", { type: "play", instanceId: richBolt.id, targets: [at(ownGhost)] });
    expect(eventsOfType(events, "discarded")).toHaveLength(2);
  });

  it("R450 R662 a prompt answer naming it pays two random discards at once and goes on", () => {
    const state = game("r450-ghost-prompt");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const chooser = one(state, "p1", PA.chooser.id);
    const [a, b] = hand(state, "p1", "fx-5", 2);
    const spareIds = [a?.id, b?.id];

    const { state: asked } = must(state, "p1", { type: "play", instanceId: chooser.id });
    expect(asked.pending?.kind).toBe("target");
    expect(asked.pending?.options.map((option) => option.selection)).toContainEqual(at(ghost));

    const { state: done, events } = answer(asked, "p1", [at(ghost)]);
    // R662: no follow-up hand prompt — the cost is paid at once, at random.
    expect(done.pending).toBeNull();
    expect(done.work).toEqual([]);
    const discarded = eventsOfType(events, "discarded").map((event) => event.instanceId);
    expect(discarded).toHaveLength(2);
    for (const id of discarded) expect(spareIds).toContain(id);
    expect(cardAt(done, slot("p2", "units", 1))?.damage).toBe(2);
  });

  it("R450 a prompt never offers it to a chooser who cannot pay", () => {
    const state = game("r450-ghost-prompt-few");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const other = put(state, "fx-1", slot("p2", "units", 2));
    const chooser = one(state, "p1", PA.chooser.id);
    hand(state, "p1", "fx-5", 1);
    const { state: asked } = must(state, "p1", { type: "play", instanceId: chooser.id });
    const picks = asked.pending?.options.map((option) => option.selection) ?? [];
    expect(picks).toContainEqual(at(other));
    expect(picks).not.toContainEqual(at(ghost));
  });

  it("R450 R662 an Echo repeat's fresh pick of it pays the discards too (the pipeline's own prompt)", () => {
    const state = game("r450-ghost-echo");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const other = put(state, "fx-1", slot("p2", "units", 2));
    const echo = one(state, "p1", PA.echoBolt.id);
    hand(state, "p1", "fx-5", 2);

    const { state: repeat } = must(state, "p1", { type: "play", instanceId: echo.id, targets: [at(other)] });
    expect(repeat.pending?.resume.hook).toBe("play");
    const { state: done, events } = answer(repeat, "p1", [at(ghost)]);
    expect(done.pending).toBeNull();
    expect(eventsOfType(events, "discarded")).toHaveLength(2);
    expect(cardAt(done, slot("p2", "units", 1))?.damage).toBe(1);
    expect(cardAt(done, slot("p2", "units", 2))?.damage).toBe(1);
    expect(done.players.p1.graveyard.map((card) => card.id)).toContain(echo.id);
  });

  it("R450 an answer whose picks cost more cards than its chooser holds is refused", () => {
    const state = game("r450-ghost-sum");
    const first = put(state, PA.ghost.id, slot("p2", "units", 1));
    const second = put(state, PA.ghost.id, slot("p2", "units", 2));
    hand(state, "p1", "fx-5", 3);
    const pending = {
      id: "q-test",
      playerId: "p1",
      kind: "target",
      prompt: "two",
      options: [],
      min: 2,
      max: 2,
      resume: { defId: "", hook: "resume", step: "", radiant: false, data: {} },
    } as PendingChoice;
    expect(whyTargetAnswerRefused(state, pending, [at(first), at(second)])).not.toBeNull();
    expect(whyTargetAnswerRefused(state, pending, [at(first)])).toBeNull();
  });
});

describe("R450 an interception (Classic #33 Joro)", () => {
  it("R450 summons it from its owner's hand, no Cry, summoning sick, and moves the declared pick to it", () => {
    const state = game("r450-joro");
    const unit = put(state, "fx-1", slot("p2", "units", 1));
    const joro = one(state, "p2", PA.joro.id);
    const bolt = one(state, "p1", PA.bolt.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: bolt.id, targets: [at(unit)] });

    expect(eventsOfType(events, "redirected")).toEqual([
      { type: "redirected", what: "target", fromId: unit.id, toId: joro.id, byInstanceId: joro.id },
    ]);
    const summoned = eventsOfType(events, "summoned").find((event) => event.instanceId === joro.id);
    expect(summoned).toMatchObject({ player: "p2", row: "units", lane: 2 });
    // Its Cry would have hit p1's hero for 5: a summon fires none (R1).
    expect(after.players.p1.hero.health).toBe(HERO_HEALTH);
    expect(eventsOfType(events, "cardAnnounced")[0]?.targets).toEqual([joro.id]);
    expect(eventsOfType(events, "damage").map((event) => event.targetId)).toEqual([joro.id]);
    expect(cardAt(after, slot("p2", "units", 1))?.damage).toBe(0);
    // The redirect is public on both seats.
    expect(eventsOfType(viewFor(after, "p1").events, "redirected")[0]?.toId).toBe(joro.id);
  });

  it("R450 the interceptor is summoning sick", () => {
    const state = game("r450-joro-sick");
    const unit = put(state, "fx-1", slot("p2", "units", 1));
    const joro = one(state, "p2", PA.joro.id);
    const zapper = one(state, "p1", PA.zapper.id);
    const { state: after } = must(state, "p1", { type: "play", instanceId: zapper.id, targets: [at(unit)] });
    const standing = cardAt(after, slot("p2", "units", 2));
    expect(standing?.id).toBe(joro.id);
    expect(standing?.summonedTurn).toBe(after.turn);
    expect(standing?.damage).toBe(1);
    expect(after.players.p2.units[1]?.[0]?.id).toBe(joro.id);
  });

  it("R450 nothing answers a hero target, a player's own unit, a full row, or a declaration it does not fit", () => {
    const hero = game("r450-joro-hero");
    put(hero, "fx-1", slot("p2", "units", 1));
    one(hero, "p2", PA.joro.id);
    const bolt = one(hero, "p1", PA.bolt.id);
    const toHero = must(hero, "p1", { type: "play", instanceId: bolt.id, targets: [{ pick: "hero", player: "p2" }] });
    expect(eventsOfType(toHero.events, "redirected")).toEqual([]);

    const own = game("r450-joro-own");
    const mine = put(own, "fx-1", slot("p1", "units", 1));
    one(own, "p1", PA.joro.id);
    const zap = one(own, "p1", PA.zapper.id);
    expect(eventsOfType(must(own, "p1", { type: "play", instanceId: zap.id, targets: [at(mine)] }).events, "redirected")).toEqual([]);

    const full = game("r450-joro-full");
    const units = [1, 2, 3, 4, 5].map((lane) => put(full, "fx-1", slot("p2", "units", lane)));
    one(full, "p2", PA.joro.id);
    const fullBolt = one(full, "p1", PA.bolt.id);
    const target = units[0] as CardInstance;
    const blocked = must(full, "p1", { type: "play", instanceId: fullBolt.id, targets: [at(target)] });
    expect(eventsOfType(blocked.events, "redirected")).toEqual([]);
    expect(eventsOfType(blocked.events, "damage").map((event) => event.targetId)).toEqual([target.id]);

    const unfit = game("r450-joro-unfit");
    const hurt = put(unfit, "fx-1", slot("p2", "units", 1));
    hurt.damage = 1;
    one(unfit, "p2", PA.joro.id);
    const medic = one(unfit, "p1", PA.medic.id);
    expect(eventsOfType(must(unfit, "p1", { type: "play", instanceId: medic.id, targets: [at(hurt)] }).events, "redirected")).toEqual([]);
  });

  it("R450 one interceptor answers one targeting: a play naming two units redirects the first only", () => {
    const state = game("r450-joro-first");
    const first = put(state, "fx-1", slot("p2", "units", 1));
    const second = put(state, "fx-2", slot("p2", "units", 2));
    const [joro, spare] = hand(state, "p2", PA.joro.id, 2);
    const twin = one(state, "p1", PA.twin.id);

    const { state: after, events } = must(state, "p1", { type: "play", instanceId: twin.id, targets: [at(first), at(second)] });

    expect(eventsOfType(events, "redirected").map((event) => [event.fromId, event.toId])).toEqual([[first.id, joro?.id]]);
    expect(after.players.p2.hand.filter((card) => card.defId === PA.joro.id).map((card) => card.id)).toEqual([spare?.id]);
    expect(cardAt(after, slot("p2", "units", 2))?.damage).toBe(1);
  });

  it("R450 answers a prompted pick too", () => {
    const state = game("r450-joro-prompt");
    const unit = put(state, "fx-1", slot("p2", "units", 1));
    const joro = one(state, "p2", PA.joro.id);
    const chooser = one(state, "p1", PA.chooser.id);
    const { state: asked } = must(state, "p1", { type: "play", instanceId: chooser.id });
    const { state: after, events } = answer(asked, "p1", [at(unit)]);
    expect(eventsOfType(events, "redirected").map((event) => event.toId)).toEqual([joro.id]);
    expect(eventsOfType(events, "damage").map((event) => event.targetId)).toEqual([joro.id]);
    expect(cardAt(after, slot("p2", "units", 1))?.damage).toBe(0);
  });

  it("R450 R662 a cost already owed for the first pick stays paid when an interceptor takes the pick", () => {
    const state = game("r450-joro-ghost");
    const ghost = put(state, PA.ghost.id, slot("p2", "units", 1));
    const joro = one(state, "p2", PA.joro.id);
    const bolt = one(state, "p1", PA.bolt.id);
    hand(state, "p1", "fx-5", 2);
    const { state: after, events } = must(state, "p1", {
      type: "play",
      instanceId: bolt.id,
      targets: [at(ghost)],
    });
    expect(eventsOfType(events, "discarded")).toHaveLength(2);
    expect(eventsOfType(events, "redirected")[0]?.toId).toBe(joro.id);
    expect(cardAt(after, slot("p2", "units", 1))?.damage).toBe(0);
  });

  it("R450 the attack half calls the same interception and reports an attack moved", () => {
    const state = game("r450-joro-attack");
    const unit = put(state, "fx-1", slot("p2", "units", 1));
    const joro = one(state, "p2", PA.joro.id);
    const sink = sinkFor(state);
    const picks = interceptTargeting(sink, { chooser: "p1", picks: [at(unit)], what: "attack" });
    expect(picks).toEqual([at(joro)]);
    expect(eventsOfType(sink.events, "redirected")).toEqual([
      { type: "redirected", what: "attack", fromId: unit.id, toId: joro.id, byInstanceId: joro.id },
    ]);
  });
});
