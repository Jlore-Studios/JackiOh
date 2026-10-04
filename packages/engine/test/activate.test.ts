// Activate (docs/classic-sets.md B3.2, R384): a card's "Activate:", "Activate N:" and "Activate ♾️:"
// abilities, the `activate` action and its alias `activatePower`, the refusal that is also the list,
// the costs, the choices, the pauses, and the view.
//
// Every rule of B3.2 is pinned here through the fixtures of `fixtures/activate.ts`, by observable
// behaviour: what the action does to the board, what `legalActions` offers, what `viewFor` shows,
// and what survives a prompt, a JSON round trip and a replay.

import type { Action, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { ACTIVATE_UNLIMITED_CAP, DECK_SIZE, HERO_HEALTH } from "../src/config";
import { DELAYED_DESTROY_HOOK } from "../src/effects/delay";
import { beginGame, legalActions, reduce, seatToAct } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import {
  ACTIVATION_HOOK_PREFIX,
  activationDecls,
  activationHook,
  type ActivationDecl,
  type Script,
} from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { DEATHS_WORK } from "../src/stateCheck";
import { createGame, newInstance, type CardInstance, type GameState } from "../src/state";
import {
  ACTIVATION_WORK,
  abilitiesOf,
  activateAbility,
  activateActionsFor,
  isActingOnField,
  usesAllowed,
  whyCannotActivateAbility,
} from "../src/subsystems/activate";
import { POWER_KEY, POWER_USED_KEY } from "../src/subsystems/heroPower";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { scriptStepFor } from "../src/work";
import { moveToZone, placeOnField } from "../src/zones";
import {
  ACTIVATE_SCRIPTS,
  LOG_LANE,
  MERCHANT_PRICE,
  PICK_KEY,
  SCEPTER_KEY,
  activateCatalog,
  asker,
  chooser,
  endless,
  ghost,
  heroic,
  lockdown,
  logCard,
  merchant,
  mourner,
  nose,
  notes,
  pinger,
  punisher,
  scepter,
  sentry,
  trapper,
  turtle,
} from "./fixtures/activate";
import { vanillaDeck } from "./fixtures/catalog";
import { eventsOfType, inHand, newGame, put, setLibrary, setupCatalog, sinkFor, slot } from "./fixtures/harness";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

function register(): void {
  registerCatalog(activateCatalog(registeredCatalog()));
  registerScripts({ ...registeredScripts(), ...ACTIVATE_SCRIPTS });
}

function game(seed: string, decks?: [string[], string[]]): GameState {
  const state = newGame(`activate-${seed}`, decks);
  register();
  return state;
}

let nonce = 0;

function actResult(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `act${nonce}` } as Action);
}

function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  const result = actResult(state, body);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

/** Past both mulligans, in p1's main phase on turn 1, with the note log in p2's backrow. */
function playing(seed: string): GameState {
  let state = beginGame(game(seed)).state;
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
  }
  put(state, logCard.id, slot("p2", "backrow", LOG_LANE));
  // R345: the tests end turns themselves, so nothing ends one behind their back (R82).
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  return state;
}

function activate(
  player: PlayerId,
  instanceId: string,
  extra: Omit<Extract<ActionInput, { type: "activate" }>, "type" | "instanceId" | "playerId"> = {},
): ActionInput {
  return { type: "activate", instanceId, playerId: player, ...extra };
}

const roundTrip = (state: GameState): GameState => JSON.parse(JSON.stringify(state)) as GameState;

/** Answer the one open prompt with its first option, whoever holds it. */
function answer(state: GameState): { state: GameState; events: GameEvent[] } {
  const pending = state.pending;
  if (pending === null) throw new Error("expected a prompt");
  const option = pending.options[0];
  if (option === undefined) throw new Error("the prompt offers nothing");
  return act(state, { type: "answer", choiceId: pending.id, selection: [option.selection], playerId: pending.playerId });
}

// ---------------------------------------------------------------------------

describe("B3.2 Activate: using an ability (R384)", () => {
  it("R384 the activate action runs the ability with its declared target, announces it, and is not a play", () => {
    const state = playing("ping");
    const card = put(state, pinger.id, slot("p1", "backrow", 1));

    const { state: after, events } = act(
      state,
      activate("p1", card.id, { ability: "ping", targets: [{ pick: "hero", player: "p2" }] }),
    );

    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 1);
    expect(eventsOfType(events, "activated")).toEqual([
      { type: "activated", player: "p1", instanceId: card.id, defId: pinger.id, ability: "ping" },
    ]);
    // B3.2 rule 6: nothing that counts plays sees it.
    expect(eventsOfType(events, "cardPlayed")).toEqual([]);
    expect(after.counters.played).toBe(state.counters.played);
    expect(after.players.p1.turnLog).toEqual(state.players.p1.turnLog);
    // Its use is counted on the instance, for this turn.
    expect(after.players.p1.backrow[0]?.memory.activations).toEqual({ turn: state.turn, count: 1 });
  });

  it("R384 an ability named by nothing is the card's only one; a card with several needs it named", () => {
    const state = playing("naming");
    const card = put(state, pinger.id, slot("p1", "backrow", 1));
    const { state: after } = act(state, activate("p1", card.id, { targets: [{ pick: "hero", player: "p2" }] }));
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 1);

    const many = put(after, chooser.id, slot("p1", "backrow", 2));
    many.memory[PICK_KEY] = "alpha";
    expect(whyCannotActivateAbility(after, "p1", many.id)).toBe("that card has several abilities: name the one to activate");
    expect(whyCannotActivateAbility(after, "p1", many.id, "gamma")).toBeNull();
  });
});

describe("B3.2 rules 1, 3, 7, 9: how many uses (R384)", () => {
  it("R384 Activate is once per turn and Activate N is N times, each counted per card per turn", () => {
    let state = playing("uses");
    const once = put(state, pinger.id, slot("p1", "backrow", 1));
    const twice = put(state, sentry.id, slot("p1", "units", 1));
    const hit = { targets: [{ pick: "hero" as const, player: "p2" as const }] };

    state = act(state, activate("p1", once.id, hit)).state;
    expect(actResult(state, activate("p1", once.id, hit)).error).toBe("that ability has already been used this turn");

    state = act(state, activate("p1", twice.id)).state;
    state = act(state, activate("p1", twice.id)).state;
    expect(actResult(state, activate("p1", twice.id)).error).toBe("that ability has been used 2 times this turn");
    // Two "Gain 1 mana" uses on top of the refreshed crystal.
    expect(state.players.p1.mana.current).toBe(3);

    // The Radiant face's "Activate 2" is the face's own count.
    const radiant = put(state, pinger.id, slot("p1", "backrow", 2), { radiant: true });
    expect(usesAllowed(radiant, activationDecls(ACTIVATE_SCRIPTS[pinger.id]?.radiant ?? {})[0] as ActivationDecl)).toBe(2);
  });

  it("R384 uses are counted per card: a card with several abilities counts every use of any of them", () => {
    const state = playing("per-card");
    const card = put(state, chooser.id, slot("p1", "backrow", 1));
    card.memory[PICK_KEY] = "alpha";
    const after = act(state, activate("p1", card.id, { ability: "gamma" })).state;
    expect(after.players.p1.backrow[0]?.memory.activations).toEqual({ turn: state.turn, count: 1 });
    expect(whyCannotActivateAbility(after, "p1", card.id, "alpha")).toBe("that ability has already been used this turn");
  });

  it("R384 uses reset on the controller's next turn, and on the opponent's turn nothing can be activated", () => {
    let state = playing("reset");
    const card = put(state, pinger.id, slot("p1", "backrow", 1));
    const hit = { targets: [{ pick: "hero" as const, player: "p2" as const }] };
    state = act(state, activate("p1", card.id, hit)).state;

    state = act(state, { type: "endTurn", playerId: "p1" }).state;
    expect(state.active).toBe("p2");
    expect(whyCannotActivateAbility(state, "p1", card.id)).toBe("it is not your turn");
    expect(actResult(state, activate("p1", card.id, hit)).error).toBe("it is not your turn");

    state = act(state, { type: "endTurn", playerId: "p2" }).state;
    expect(state.active).toBe("p1");
    expect(whyCannotActivateAbility(state, "p1", card.id)).toBeNull();
    expect(act(state, activate("p1", card.id, hit)).state.players.p2.hero.health).toBe(HERO_HEALTH - 2);
  });

  it("R384 Activate ♾️ is any number of uses, bounded by ACTIVATE_UNLIMITED_CAP", () => {
    const state = playing("unlimited");
    const card = put(state, endless.id, slot("p1", "backrow", 1));
    const sink = sinkFor(state);
    for (let use = 0; use < ACTIVATE_UNLIMITED_CAP; use += 1) {
      expect(activateAbility(sink, "p1", { type: "activate", instanceId: card.id })).toBeNull();
    }
    expect(notes(state)).toHaveLength(ACTIVATE_UNLIMITED_CAP);
    expect(activateAbility(sink, "p1", { type: "activate", instanceId: card.id })).toBe(
      `that ability has been used ${ACTIVATE_UNLIMITED_CAP} times this turn`,
    );
  });

  it("R384 Degrade and Upgrade move Activate N by the tuning key Activate, never below 1, and never ♾️", () => {
    const state = playing("tuned");
    const once = put(state, pinger.id, slot("p1", "backrow", 1));
    const twice = put(state, sentry.id, slot("p1", "units", 1));
    const always = put(state, endless.id, slot("p1", "backrow", 2));
    const [pingDecl] = abilitiesOf(state, once);
    const [surgeDecl] = abilitiesOf(state, twice);
    const [againDecl] = abilitiesOf(state, always);
    if (pingDecl === undefined || surgeDecl === undefined || againDecl === undefined) throw new Error("no ability");

    // An Upgrade turns "Activate" into "Activate 2".
    once.tuning = { x: { Activate: 1 } };
    expect(usesAllowed(once, pingDecl)).toBe(2);
    // A Degrade takes "Activate 2" to 1, and no further.
    twice.tuning = { x: { Activate: -1 } };
    expect(usesAllowed(twice, surgeDecl)).toBe(1);
    twice.tuning = { x: { Activate: -5 } };
    expect(usesAllowed(twice, surgeDecl)).toBe(1);
    // KY's Constant sets the number outright.
    twice.tuning = { set: { Activate: 3 } };
    expect(usesAllowed(twice, surgeDecl)).toBe(3);
    // ♾️ has no number to move.
    always.tuning = { x: { Activate: -3 } };
    expect(usesAllowed(always, againDecl)).toBe(ACTIVATE_UNLIMITED_CAP);

    // The tuned count is the one the refusal reads.
    let next = act(state, activate("p1", once.id, { targets: [{ pick: "hero", player: "p2" }] })).state;
    next = act(next, activate("p1", once.id, { targets: [{ pick: "hero", player: "p2" }] })).state;
    expect(next.players.p2.hero.health).toBe(HERO_HEALTH - 2);
    expect(whyCannotActivateAbility(next, "p1", once.id)).toBe("that ability has been used 2 times this turn");
  });

  it("R384 leaving the field resets the uses (R78), so a card bounced and played again starts fresh", () => {
    let state = playing("bounced");
    const card = put(state, pinger.id, slot("p1", "backrow", 1));
    const hit = { targets: [{ pick: "hero" as const, player: "p2" as const }] };
    state = act(state, activate("p1", card.id, hit)).state;
    const used = state.players.p1.backrow[0] as CardInstance;
    expect(whyCannotActivateAbility(state, "p1", used.id)).toBe("that ability has already been used this turn");

    moveToZone(state, used, "hand");
    expect(used.memory.activations).toBeUndefined();
    expect(placeOnField(state, used, slot("p1", "backrow", 1))).toBe(true);
    expect(whyCannotActivateAbility(state, "p1", used.id)).toBeNull();
  });
});

describe("B3.2 rule 2: who and when (R384)", () => {
  it("R384 only the controller, in their main phase, with no prompt open and the game not over", () => {
    const state = playing("when");
    const card = put(state, pinger.id, slot("p1", "backrow", 1));

    expect(whyCannotActivateAbility(state, "p2", card.id)).toBe("that card is not yours");
    expect(whyCannotActivateAbility(state, "p1", "c99999")).toBe("no card c99999");

    const inPhase = roundTrip(state);
    inPhase.phase = "start";
    expect(whyCannotActivateAbility(inPhase, "p1", card.id)).toBe("an ability is activated in the main phase");

    const asking = roundTrip(state);
    asking.pending = {
      id: "q-test",
      playerId: "p1",
      kind: "target",
      prompt: "a question",
      options: [{ key: "none", label: "none", selection: { pick: "none" } }],
      min: 1,
      max: 1,
      resume: { defId: "", hook: "resume", step: "x", radiant: false, data: {} },
    };
    expect(whyCannotActivateAbility(asking, "p1", card.id)).toBe("answer the open prompt first");

    const over = roundTrip(state);
    over.result = { winner: "p1", reason: "concede" };
    expect(whyCannotActivateAbility(over, "p1", card.id)).toBe("the game is over");
  });

  it("R384 summoning sickness and exertion do not stop an activation: activating is not attacking", () => {
    const state = playing("sick");
    const unit = put(state, sentry.id, slot("p1", "units", 1));
    unit.summonedTurn = state.turn;
    unit.exertion = { attacked: true, switched: true };
    expect(whyCannotActivateAbility(state, "p1", unit.id)).toBeNull();
    expect(act(state, activate("p1", unit.id)).state.players.p1.mana.current).toBe(state.players.p1.mana.current + 1);
  });

  it("R384 a card acts only on the field: not in a hand, not dormant under a pile (R13), not face-down", () => {
    const state = playing("acting");
    const held = inHand(state, pinger.id, "p1")[0] as CardInstance;
    expect(whyCannotActivateAbility(state, "p1", held.id)).toBe("that card is not on the field");

    const under = put(state, sentry.id, slot("p1", "units", 2));
    expect(isActingOnField(state, under)).toBe(true);
    const top = newInstance(state, sentry.id, "p1", { z: "hand", player: "p1" });
    expect(placeOnField(state, top, slot("p1", "units", 2), { stack: true })).toBe(true);
    expect(isActingOnField(state, under)).toBe(false);
    expect(whyCannotActivateAbility(state, "p1", under.id)).toBe("that card is under a pile and does not act");
    expect(whyCannotActivateAbility(state, "p1", top.id)).toBeNull();

    const trap = put(state, trapper.id, slot("p1", "backrow", 3));
    expect(whyCannotActivateAbility(state, "p1", trap.id)).toBe("a face-down card has no ability to use");
    trap.faceUp = true;
    expect(whyCannotActivateAbility(state, "p1", trap.id)).toBeNull();

    // §6.3 Vanilla: a card with no text has no ability.
    const blank = put(state, pinger.id, slot("p1", "backrow", 4));
    blank.vanilla = true;
    expect(whyCannotActivateAbility(state, "p1", blank.id)).toBe("that card has no Activate ability");
  });

  it("R384 a condition the text sets is part of the refusal (Classic #7: nothing remembered, no activation)", () => {
    const state = playing("condition");
    const card = put(state, scepter.id, slot("p1", "backrow", 1));
    expect(whyCannotActivateAbility(state, "p1", card.id)).toBe("that ability can't be activated now");
    expect(legalActions(state, "p1").some((body) => body.type === "activate" && body.instanceId === card.id)).toBe(false);
    card.memory[SCEPTER_KEY] = { defId: "fx-1", radiant: false };
    expect(whyCannotActivateAbility(state, "p1", card.id)).toBeNull();
    expect(notes(act(state, activate("p1", card.id)).state)).toEqual(["cast"]);
  });
});

describe("B3.2 rule 4: costs (R384)", () => {
  it("R384 a mana price is paid as it is activated, and an ability the player cannot pay for is refused", () => {
    const state = playing("mana");
    const card = put(state, merchant.id, slot("p1", "backrow", 1));
    setLibrary(state, "p1", ["fx-5", "fx-6"]);
    state.players.p1.mana.current = MERCHANT_PRICE - 1;
    expect(whyCannotActivateAbility(state, "p1", card.id)).toBe(`that ability costs ${MERCHANT_PRICE}, more than your mana`);

    state.players.p1.mana.current = MERCHANT_PRICE + 1;
    const hand = state.players.p1.hand.length;
    const { state: after, events } = act(state, activate("p1", card.id));
    expect(after.players.p1.mana.current).toBe(1);
    expect(after.players.p1.hand.length).toBe(hand + 1);
    expect(eventsOfType(events, "manaChanged").at(0)).toMatchObject({ player: "p1", current: 1 });
  });

  it("R384 a random discard is paid from the hand, and an empty hand cannot pay it", () => {
    const state = playing("discard");
    const card = put(state, nose.id, slot("p1", "units", 1));
    state.players.p1.hand = [];
    expect(whyCannotActivateAbility(state, "p1", card.id)).toBe("that ability needs a card in your hand to discard");

    const [first, second] = inHand(state, "fx-3", "p1", 2);
    const { state: after, events } = act(state, activate("p1", card.id));
    expect(after.players.p1.hand).toHaveLength(1);
    expect(eventsOfType(events, "discarded")).toHaveLength(1);
    expect([first?.id, second?.id]).toContain(eventsOfType(events, "discarded")[0]?.instanceId);
    expect(notes(after)).toEqual(["sniff"]);
  });

  it("R384 Tribute this bypasses Indestructible, and the ability's effect runs after it with the card in its graveyard", () => {
    const state = playing("tribute-self");
    const card = put(state, lockdown.id, slot("p1", "backrow", 1));
    const { state: after, events } = act(state, activate("p1", card.id));
    expect(after.players.p1.backrow[0]).toBeNull();
    expect(after.players.p1.graveyard.map((held) => held.id)).toContain(card.id);
    expect(eventsOfType(events, "destroyed").map((event) => event.instanceId)).toEqual([card.id]);
    expect(notes(after)).toEqual(["left:graveyard"]);
  });

  it("R384 a Tribute cost takes the controller's units, the card itself allowed, and one unit is one Tribute", () => {
    const state = playing("tribute");
    const eater = put(state, turtle.id, slot("p1", "units", 1));
    const enemy = put(state, sentry.id, slot("p2", "units", 1));
    const toP2 = { targets: [{ pick: "hero" as const, player: "p2" as const }] };

    // With no other unit, the card pays with itself: 5 damage, its Attack as it stood.
    const alone = act(state, activate("p1", eater.id, { ...toP2, tributes: [eater.id] })).state;
    expect(alone.players.p2.hero.health).toBe(HERO_HEALTH - 5);
    expect(alone.players.p1.units[0]).toBeNull();

    // The refusals: none, two, an enemy unit, the same unit twice.
    const ally = put(state, sentry.id, slot("p1", "units", 2));
    expect(actResult(state, activate("p1", eater.id, toP2)).error).toBe("that ability tributes a Unit");
    expect(actResult(state, activate("p1", eater.id, { ...toP2, tributes: [ally.id, eater.id] })).error).toBe(
      "that ability tributes a Unit",
    );
    expect(actResult(state, activate("p1", eater.id, { ...toP2, tributes: [enemy.id] })).error).toBe(
      `${enemy.id} cannot be tributed for that ability`,
    );
    expect(actResult(state, activate("p1", eater.id, { ...toP2, tributes: [ally.id, ally.id] })).error).toBe(
      "that ability cannot tribute the same Unit twice",
    );
  });

  it("R384 the tributed unit's Attack is read as it stood before it died (R78, R89)", () => {
    const state = playing("last-known");
    const eater = put(state, turtle.id, slot("p1", "units", 1));
    const fed = put(state, sentry.id, slot("p1", "units", 2));
    fed.buffs = { attack: 3, health: 0 };
    const { state: after } = act(state, activate("p1", eater.id, { targets: [{ pick: "hero", player: "p2" }], tributes: [fed.id] }));
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - 5);
    // The eater stays, and ♾️ lets it go again while it has something to eat.
    expect(after.players.p1.units[0]?.[0]?.id).toBe(eater.id);
  });

  it("R384 a target the Tribute took off the field is gone for the effect (R174), and nothing else is hit", () => {
    const state = playing("target-tributed");
    const eater = put(state, turtle.id, slot("p1", "units", 1));
    const fed = put(state, sentry.id, slot("p1", "units", 2));
    const { state: after, events } = act(
      state,
      activate("p1", eater.id, { targets: [{ pick: "instance", instanceId: fed.id }], tributes: [fed.id] }),
    );
    expect(after.players.p1.graveyard.map((card) => card.id)).toContain(fed.id);
    expect(eventsOfType(events, "damage")).toEqual([]);
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH);
  });
});

describe("B3.2 rules 5, 8: choices and the list (R384, R81, R90)", () => {
  it("R384 modes and the targets they bind travel in the action and are checked like a play's", () => {
    const state = playing("modes");
    const card = put(state, punisher.id, slot("p1", "backrow", 1));
    const enemy = put(state, sentry.id, slot("p2", "units", 1));
    const name = "punisher (activate)";

    expect(actResult(state, activate("p1", card.id)).error).toBe(`${name} needs a mode choice for each of its 1 mode declaration`);
    expect(actResult(state, activate("p1", card.id, { modes: ["boom"] })).error).toBe(`"boom" is not a mode of ${name}`);
    expect(actResult(state, activate("p1", card.id, { modes: ["damage"] })).error).toBe(`${name} needs 1 target for that choice`);
    expect(actResult(state, activate("p1", card.id, { modes: ["discard"], targets: [{ pick: "hero", player: "p2" }] })).error).toBe(
      `${name} takes no targets for that choice`,
    );
    expect(actResult(state, activate("p1", card.id, { modes: ["doom"], targets: [{ pick: "hero", player: "p2" }] })).error).toBe(
      `that is not a legal target for ${name}`,
    );

    const damaged = act(state, activate("p1", card.id, { modes: ["damage"], targets: [{ pick: "hero", player: "p2" }] })).state;
    expect(damaged.players.p2.hero.health).toBe(HERO_HEALTH - 2);

    const hand = state.players.p2.hand.length;
    expect(act(state, activate("p1", card.id, { modes: ["discard"] })).state.players.p2.hand).toHaveLength(hand - 1);

    const doomed = act(state, activate("p1", card.id, { modes: ["doom"], targets: [{ pick: "instance", instanceId: enemy.id }] })).state;
    expect(doomed.delayed.map((entry) => [entry.resume.hook, entry.watch])).toEqual([[DELAYED_DESTROY_HOOK, enemy.id]]);
  });

  it("R384 legalActions lists activate for exactly the usable abilities, each Tribute set crossed with each choice", () => {
    const state = playing("listing");
    const eater = put(state, turtle.id, slot("p1", "units", 1));
    const allyA = put(state, sentry.id, slot("p1", "units", 2));
    put(state, sentry.id, slot("p1", "units", 3));
    put(state, sentry.id, slot("p2", "units", 1));

    const listed = legalActions(state, "p1").filter(
      (body): body is Extract<typeof body, { type: "activate" }> => body.type === "activate" && body.instanceId === eater.id,
    );
    // Three ways to pay (itself or either ally) times six targets (four units, two heroes). The two
    // sentries' "Activate 2" abilities are listed too, but not under the eater's id.
    expect(listed).toHaveLength(3 * 6);
    expect(listed).toEqual(activateActionsFor(state, "p1", eater));
    expect(listed[0]).toEqual({
      type: "activate",
      instanceId: eater.id,
      ability: "eat",
      tributes: [eater.id],
      targets: [{ pick: "instance", instanceId: eater.id }],
    });
    // Every listed action is one the reducer accepts.
    for (const body of listed) expect(actResult(state, { ...body, playerId: "p1" }).error).toBeUndefined();
    // And each ability of each card acting on the field is listed as its own entry.
    expect(legalActions(state, "p1")).toContainEqual({ type: "activate", instanceId: allyA.id, ability: "surge" });

    // Used up, it is listed no more; and on the opponent's turn not at all.
    const merchantCard = put(state, merchant.id, slot("p1", "backrow", 1));
    state.players.p1.mana.current = 0;
    expect(legalActions(state, "p1").some((body) => body.type === "activate" && body.instanceId === merchantCard.id)).toBe(false);
  });

  it("R450 R654 a declared target that costs discards lists one action carrying none, paid random with the costs", () => {
    const state = playing("ghost");
    const card = put(state, pinger.id, slot("p1", "backrow", 1));
    const costly = put(state, ghost.id, slot("p2", "units", 1));
    state.players.p1.hand = [];
    const [a, b, c] = inHand(state, sentry.id, "p1", 3);
    const spareIds = [a?.id, b?.id, c?.id];
    const atGhost = { pick: "instance", instanceId: costly.id } as const;

    const listed = activateActionsFor(state, "p1", card).filter((body) => body.targets?.[0]?.pick === "instance" && body.targets[0].instanceId === costly.id);
    // R654: the discards are random at pay time, so one action, carrying none.
    expect(listed).toHaveLength(1);
    expect(listed.every((body) => !("discards" in body))).toBe(true);
    expect(activateActionsFor(state, "p1", card).filter((body) => body.targets?.[0]?.pick === "hero").every((body) => !("discards" in body))).toBe(true);
    // Refused when too few other cards are held; a hero target asks nothing and succeeds.
    const poor = playing("ghost-poor");
    const poorCard = put(poor, pinger.id, slot("p1", "backrow", 1));
    const poorCostly = put(poor, ghost.id, slot("p2", "units", 1));
    poor.players.p1.hand = [];
    inHand(poor, sentry.id, "p1", 1);
    const poorGhost = { pick: "instance", instanceId: poorCostly.id } as const;
    expect(actResult(poor, activate("p1", poorCard.id, { ability: "ping", targets: [poorGhost] })).error).toBeDefined();
    expect(actResult(poor, activate("p1", poorCard.id, { ability: "ping", targets: [{ pick: "hero", player: "p2" }] })).error).toBeUndefined();

    const { state: after, events } = act(state, activate("p1", card.id, { ability: "ping", targets: [atGhost] }));
    const order = events.map((event) => event.type);
    const discarded = eventsOfType(events, "discarded").map((event) => event.instanceId);
    expect(discarded).toHaveLength(2);
    for (const id of discarded) expect(spareIds).toContain(id);
    expect(order.lastIndexOf("discarded")).toBeLessThan(order.indexOf("damage"));
    expect(after.players.p1.hand).toHaveLength(1);
    expect(spareIds).toContain(after.players.p1.hand[0]?.id);
    expect(after.players.p2.units[0]?.[0]?.damage).toBe(1);

    // With fewer than two cards in hand it is no legal target of the ability.
    state.players.p1.hand = [];
    inHand(state, sentry.id, "p1", 1);
    expect(activateActionsFor(state, "p1", card).some((body) => body.targets?.[0]?.pick === "instance" && body.targets[0].instanceId === costly.id)).toBe(false);
  });

  it("R384 an ability a card does not have now is neither listed nor accepted (v0.2.1's rolled power)", () => {
    const state = playing("has");
    const card = put(state, chooser.id, slot("p1", "backrow", 1));
    card.memory[PICK_KEY] = "beta";
    expect(abilitiesOf(state, card).map((decl) => decl.id)).toEqual(["beta", "gamma"]);
    expect(whyCannotActivateAbility(state, "p1", card.id, "alpha")).toBe('that card has no ability "alpha"');
    expect(
      legalActions(state, "p1").flatMap((body) => (body.type === "activate" && body.instanceId === card.id ? [body.ability] : [])),
    ).toEqual(["beta", "gamma"]);
    expect(notes(act(state, activate("p1", card.id, { ability: "beta" })).state)).toEqual(["beta"]);
  });

  it("R384 a fused card has every ingredient's abilities, the second of two with one id named <id>#2 (R102)", () => {
    const run = (): [] => [];
    const decl = (id: string): ActivationDecl => ({ id, label: id, uses: 1, run });
    const script: Script = { activations: [decl("ping"), decl("ping"), decl("surge")] };
    const ids = activationDecls(script).map((entry) => entry.id);
    expect(ids).toEqual(["ping", "ping#2", "surge"]);
    // A tail its prompt parked comes back to the same ability by that id.
    const second = script.activations?.[1];
    const resume = { defId: "", hook: activationHook("ping#2"), step: "", radiant: false, data: {} };
    expect(resume.hook.startsWith(ACTIVATION_HOOK_PREFIX)).toBe(true);
    expect(scriptStepFor(script, resume)).toBe(second?.run);
  });
});

describe("B3.2 and §9.3: an activation across a prompt (R384, R113, R117)", () => {
  it("R384 a prompt inside the ability parks the rest of its list under the ability's own hook, and the answer finishes it", () => {
    const state = playing("asker");
    const card = put(state, asker.id, slot("p1", "backrow", 1));
    const paused = act(state, activate("p1", card.id)).state;

    expect(notes(paused)).toEqual(["ask"]);
    expect(paused.pending?.playerId).toBe("p1");
    expect(paused.work.map((item) => item.resume.hook)).toEqual([activationHook("ask")]);
    // §9.3: plain data, so the paused game survives a JSON round trip and finishes identically.
    const copy = roundTrip(paused);
    expect(copy).toEqual(paused);

    const live = answer(paused).state;
    const restored = answer(copy).state;
    expect(notes(live)).toEqual(["ask", "answered", "ask:tail"]);
    expect(hashState(restored)).toBe(hashState(live));
    // The use was spent before the pause, so the answer cannot buy another.
    expect(whyCannotActivateAbility(live, "p1", card.id)).toBe("that ability has already been used this turn");
  });

  it("R384 a Tribute whose Death asks owes the ability's effect on state.work, behind the Death pass's own remainder", () => {
    const state = playing("tribute-pause");
    const eater = put(state, turtle.id, slot("p1", "units", 1));
    const fed = put(state, mourner.id, slot("p1", "units", 2));
    const paused = act(state, activate("p1", eater.id, { targets: [{ pick: "hero", player: "p2" }], tributes: [fed.id] })).state;

    expect(notes(paused)).toEqual(["death"]);
    expect(paused.players.p2.hero.health).toBe(HERO_HEALTH);
    // R113: the Death pass's remainder (its hook's tail with it) first, then the ability it paid for.
    expect(paused.work.map((item) => item.resume.hook)).toEqual([DEATHS_WORK, ACTIVATION_WORK]);

    const copy = roundTrip(paused);
    const live = answer(paused).state;
    expect(notes(live)).toEqual(["death", "mourned", "death:tail"]);
    // The effect ran once the cost was whole: the mourner's Attack, 1.
    expect(live.players.p2.hero.health).toBe(HERO_HEALTH - 1);
    expect(live.work).toEqual([]);
    expect(hashState(answer(copy).state)).toBe(hashState(live));
  });

  it("R384 a game with activations replays from its log to the same state (§9.3)", () => {
    const deck = [asker.id, ...vanillaDeck(DECK_SIZE - 1, 1)];
    const decks: [string[], string[]] = [deck, vanillaDeck(DECK_SIZE, 21)];
    const seed = "activate-replay";
    setupCatalog();
    register();
    let state = beginGame(createGame({ seed, decks })).state;
    const log: Action[] = [];
    const step = (body: ActionInput): void => {
      const action = { ...body, nonce: `r${log.length}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(result.error);
      log.push(action);
      state = result.state;
    };
    for (const player of ["p1", "p2"] as const) {
      step({ type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
    }
    const held = state.players.p1.hand.find((card) => card.defId === asker.id);
    if (held === undefined) throw new Error("Quickdraw put the asker in the opening hand");
    step({ type: "play", instanceId: held.id, playerId: "p1" });
    step({ type: "activate", instanceId: held.id, playerId: "p1" });
    expect(state.pending).not.toBeNull();
    const pending = state.pending;
    if (pending === null) throw new Error("expected a prompt");
    step({ type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: seatToAct(state) });

    const replayed = fold({ seed, decks, log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
  });
});

describe("B3.2 rule 10: activatePower is an alias of activate (R384, R43)", () => {
  function withPower(state: GameState, lane: number): CardInstance {
    const card = put(state, heroic.id, slot("p1", "backrow", lane));
    // R43: "burn" is "deal 2 damage to each opposing hero" for (1).
    card.memory[POWER_KEY] = "burn";
    return card;
  }

  it("R384 an activate that names no ability uses a Heroic Power's power, and activatePower still does", () => {
    const state = playing("alias");
    const card = withPower(state, 1);
    const listed = legalActions(state, "p1").filter((body) => "instanceId" in body && body.instanceId === card.id);
    // The power is listed as the activatePower it has always been, and only once.
    expect(listed).toEqual([{ type: "activatePower", instanceId: card.id }]);

    const viaActivate = act(state, activate("p1", card.id)).state;
    expect(viaActivate.players.p2.hero.health).toBe(HERO_HEALTH - 2);
    expect(viaActivate.players.p1.backrow[0]?.memory[POWER_USED_KEY]).toBe(state.turn);
    expect(actResult(viaActivate, { type: "activatePower", instanceId: card.id, playerId: "p1" }).error).toBe(
      "that power has already been used this turn",
    );

    const viaAlias = act(state, { type: "activatePower", instanceId: card.id, playerId: "p1" }).state;
    expect(hashState(viaAlias)).toBe(hashState(viaActivate));

    expect(actResult(state, activate("p1", card.id, { modes: ["x"] })).error).toBe("that power takes no mode choices");
    expect(actResult(state, activate("p1", card.id, { ability: "x" })).error).toBe("that card has no Activate ability");
  });

  it("R384 activatePower on a card with an Activate ability uses that ability", () => {
    const state = playing("alias-ability");
    const card = put(state, pinger.id, slot("p1", "backrow", 1));
    const after = act(state, { type: "activatePower", instanceId: card.id, targets: [{ pick: "hero", player: "p2" }], playerId: "p1" });
    expect(after.state.players.p2.hero.health).toBe(HERO_HEALTH - 1);
    expect(eventsOfType(after.events, "activated")).toHaveLength(1);
  });
});

describe("B3.2 and §10.8: what each player sees (R384, R97)", () => {
  it("R384 the controller's own view of a card on the field carries its abilities, and the other player's never does", () => {
    const state = playing("view");
    const once = put(state, pinger.id, slot("p1", "backrow", 1));
    put(state, endless.id, slot("p1", "backrow", 2));
    put(state, sentry.id, slot("p1", "units", 1));

    const mine = viewFor(state, "p1");
    expect(mine.you.backrow[0]).toMatchObject({ activations: [{ ability: "ping", label: "Deal 1 damage", usesLeft: 1, usable: true }] });
    expect(mine.you.backrow[1]).toMatchObject({ activations: [{ ability: "again", usesLeft: null, usable: true }] });
    expect(mine.you.units[0]).toMatchObject({ activations: [{ ability: "surge", usesLeft: 2, usable: true }] });
    // A card without abilities carries no key at all.
    expect(mine.opponent.backrow[LOG_LANE - 1]).not.toHaveProperty("activations");

    const theirs = viewFor(state, "p2");
    expect(theirs.opponent.backrow[0]).not.toHaveProperty("activations");
    expect(theirs.opponent.units[0]).not.toHaveProperty("activations");

    const used = act(state, activate("p1", once.id, { targets: [{ pick: "hero", player: "p2" }] })).state;
    expect(viewFor(used, "p1").you.backrow[0]).toMatchObject({
      activations: [{ ability: "ping", usesLeft: 0, usable: false, reason: "that ability has already been used this turn" }],
    });
  });

  it("R384 the activated event is public while its card is readable, and the sentinel once it is in a hand (R97)", () => {
    const state = playing("event");
    const card = put(state, pinger.id, slot("p1", "backrow", 1));
    const after = act(state, activate("p1", card.id, { targets: [{ pick: "hero", player: "p2" }] })).state;
    const seen = (viewer: PlayerId, at: GameState): GameEvent | undefined =>
      viewFor(at, viewer).events.find((event) => event.type === "activated");

    expect(seen("p2", after)).toMatchObject({ instanceId: card.id, defId: pinger.id });
    const moved = roundTrip(after);
    moveToZone(moved, moved.players.p1.backrow[0] as CardInstance, "hand");
    expect(seen("p2", moved)).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID, ability: "ping" });
    expect(seen("p1", moved)).toMatchObject({ instanceId: card.id });
  });
});
