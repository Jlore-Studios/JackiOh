// Casts from anywhere and random casts (docs/classic-sets.md B5 E12, E39; R70, R452, R453): the cast
// verbs of `src/effects/cast.ts` — a card out of a graveyard, a new card of a named definition, random
// catalog cards — how a cast's choices are made (its caster's, or the rng's, narrowed to enemies when
// it targets them), its X, a cast permanent's zone, a pause inside a cast surviving JSON, the chain cap,
// and what the other seat reads.

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { RANDOM_CAST_CHAIN_CAP } from "../src/config";
import { cast, castNew, castRandom, damage, heal } from "../src/effects";
import { legalActions } from "../src/reduce";
import { castModesOf, preferEnemies, preferFriends } from "../src/randomCast";
import { hashState } from "../src/replay";
import { applyEffects, castCard, makeContext } from "../src/resolve";
import type { CardScripts, Effect } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { newInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { activeUnitsOf } from "../src/zones";
import { eventsOfType, inHand, put, sinkFor, slot } from "./fixtures/harness";
import {
  DISCOVER_POOL,
  RANDOM_POOL,
  askTarget,
  castField,
  castTrap,
  discoverSpell,
  graveSpell,
  inGraveyard,
  joggBox,
  modeSpell,
  namedCaster,
  only,
  pbAct,
  pbPlaying,
  pbReduce,
  roundTrip,
  solarius,
  targetSpell,
  theirChoice,
  tyrant,
  xSpell,
  xTarget,
} from "./fixtures/playPipelineB";

// Inline cards for the chain cap and the nested cases, indexed clear of the shared fixtures.
function spell(name: string, index: number): CardDef {
  return {
    id: `pbc-${name}`,
    index: String(index),
    name: `${name} (casts)`,
    set: "Core",
    type: "Spell",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
  };
}
const pingA = spell("ping-a", 4601);
const pingB = spell("ping-b", 4602);
const echoTarget = spell("echo-target", 4603);
const castsNamed = spell("casts-named", 4604);
const castsDiscover = spell("casts-discover", 4605);
// R654: a helpful Spell — its target declaration aims "help", so a cast that targets enemies
// prefers friends for it.
const healFriend = spell("heal-friend", 4606);

function both(script: CardScripts["base"]): CardScripts {
  return { base: script, radiant: script };
}

const INLINE: Record<string, CardScripts> = {
  [pingA.id]: both({ cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 }), castRandom({ query: { defId: pingB.id }, count: 2 })] }),
  [pingB.id]: both({ cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 }), castRandom({ query: { defId: pingA.id }, count: 2 })] }),
  [echoTarget.id]: both({
    staticFlags: { echo: 1 },
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit", "hero"] } }],
    cry: () => [damage({ to: { of: "chosen" }, amount: 1 })],
  }),
  [castsNamed.id]: both({ cry: () => [castNew({ def: targetSpell.id })] }),
  [castsDiscover.id]: both({ cry: () => [castRandom({ query: { defId: discoverSpell.id }, count: 1 })] }),
  [healFriend.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit"] }, aim: "help" }],
    cry: () => [heal({ target: { of: "chosen" }, amount: 3 })],
  }),
};

function playing(seed: string): GameState {
  const state = pbPlaying(seed);
  registerCatalog({
    ...registeredCatalog(),
    [pingA.id]: pingA,
    [pingB.id]: pingB,
    [echoTarget.id]: echoTarget,
    [castsNamed.id]: castsNamed,
    [castsDiscover.id]: castsDiscover,
    [healFriend.id]: healFriend,
  });
  registerScripts({ ...registeredScripts(), ...INLINE });
  return state;
}

/** Apply effects as p1's own card would, then run the resolution loop, as a Cry's list is run. */
function run(state: GameState, effects: Effect[], selfId?: string): ReturnType<typeof sinkFor> {
  const sink = sinkFor(state);
  const self = selfId === undefined ? null : (state.players.p1.hand.find((card) => card.id === selfId) ?? null);
  applyEffects(effects, makeContext(sink, self, { controller: "p1" }));
  settle(sink);
  state.rngCursor = sink.rng.cursor;
  return sink;
}

describe("E12 casts from anywhere (R70, R453)", () => {
  it("R453 casts every Spell in the graveyard oldest first, each with its caster's choices, and exiles each once it resolves", () => {
    const state = playing("r453-tyrant");
    state.players.p1.mana.current = 4;
    const first = inGraveyard(state, graveSpell.id);
    const second = inGraveyard(state, modeSpell.id);
    const third = inGraveyard(state, targetSpell.id);
    const card = only(inHand(state, tyrant.id, "p1"));
    const before = state.players.p2.hero.health;

    // The Tyrant's Cry casts the first (no choices), then asks for the Mode Spell's mode (R70).
    const paused = pbAct(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 2 }, playerId: "p1" });
    expect(paused.pending?.playerId).toBe("p1");
    expect(paused.pending?.prompt).toMatch(/^Cast: /);
    expect(paused.pending?.options.map((option) => option.label)).toEqual(["a", "b"]);
    expect(paused.castsResolving).toBeUndefined();

    // JSON round trip mid-pause: both copies finish the same way.
    const round = roundTrip(paused);
    const pickB = only(legalActions(paused, "p1").filter((action) => action.type === "answer" && JSON.stringify(action.selection).includes('"b"')));
    const next = pbAct(paused, { ...pickB, playerId: "p1" });
    const nextRound = pbAct(round, { ...pickB, playerId: "p1" });
    expect(hashState(nextRound)).toBe(hashState(next));
    // Now the Target Spell asks for its target.
    expect(next.pending?.prompt).toMatch(/^Cast: /);
    const enemyHero = only(
      legalActions(next, "p1").filter((action) => action.type === "answer" && JSON.stringify(action.selection) === JSON.stringify([{ pick: "hero", player: "p2" }])),
    );
    const done = pbAct(next, { ...enemyHero, playerId: "p1" });
    expect(done.pending).toBeNull();
    // 1 + 2 + 3 damage to the enemy hero, each cast counted as a play paying 0 (R70).
    expect(done.players.p2.hero.health).toBe(before - 6);
    expect(done.players.p1.turnLog.playedIds.slice(-3)).toEqual([first.id, second.id, third.id]);
    expect(done.players.p1.turnLog.costsPaid?.slice(-3)).toEqual([0, 0, 0]);
    // "Then exile them": each resolved into exile, none back in the graveyard.
    expect(done.players.p1.exile.map((entry) => entry.id)).toEqual(expect.arrayContaining([first.id, second.id, third.id]));
    expect(done.players.p1.graveyard.map((entry) => entry.id)).not.toEqual(expect.arrayContaining([first.id]));
  });

  it("R453 a named definition is cast new, owned by the caster, asked of the caster, and lands in the caster's graveyard", () => {
    const state = playing("r453-named");
    const card = only(inHand(state, namedCaster.id, "p1"));
    const paused = pbAct(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 1 }, playerId: "p1" });
    expect(paused.pending?.kind).toBe("target");
    const cast = only(paused.players.p1.resolving);
    expect(cast.defId).toBe(targetSpell.id);
    expect(cast.owner).toBe("p1");
    const answer = only(
      legalActions(paused, "p1").filter((action) => action.type === "answer" && JSON.stringify(action.selection) === JSON.stringify([{ pick: "hero", player: "p2" }])),
    );
    const done = pbAct(paused, { ...answer, playerId: "p1" });
    expect(done.players.p1.graveyard.map((entry) => entry.id)).toContain(cast.id);
    expect(done.players.p2.hero.health).toBe(paused.players.p2.hero.health - 3);
  });

  it("R453 a cast X card asks its caster for X first — 1 up to their mana, at least 1 — then its target, and spends no mana", () => {
    const state = playing("r453-x-asked");
    state.players.p1.mana.current = 3;
    const sink = run(state, [castNew({ def: xTarget.id })]);
    expect(state.pending?.kind).toBe("number");
    expect(state.pending?.options.map((option) => option.selection)).toEqual(
      ["1", "2", "3"].map((option) => ({ pick: "mode", option })),
    );
    expect(eventsOfType(sink.events, "promptOpened")).toHaveLength(1);
    // The other seat sees only that a prompt is open.
    expect(viewFor(state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });

    const round = roundTrip(state);
    const two = only(legalActions(state, "p1").filter((action) => action.type === "answer" && JSON.stringify(action.selection).includes('"2"')));
    const asked = pbAct(state, { ...two, playerId: "p1" });
    const askedRound = pbAct(round, { ...two, playerId: "p1" });
    expect(hashState(askedRound)).toBe(hashState(asked));
    // Then its target, asked of the card as it will resolve (X is on it now).
    expect(asked.pending?.kind).toBe("target");
    expect(only(asked.players.p1.resolving).x).toBe(2);
    const hero = only(
      legalActions(asked, "p1").filter((action) => action.type === "answer" && JSON.stringify(action.selection) === JSON.stringify([{ pick: "hero", player: "p2" }])),
    );
    const done = pbAct(asked, { ...hero, playerId: "p1" });
    expect(done.players.p2.hero.health).toBe(state.players.p2.hero.health - 2);
    expect(done.players.p1.mana.current).toBe(3);

    // With no mana the one X there is is 1.
    const broke = playing("r453-x-broke");
    broke.players.p1.mana.current = 0;
    run(broke, [castNew({ def: xSpell.id })]);
    expect(broke.pending?.options.map((option) => option.label)).toEqual(["1"]);
  });

  it("R452 a random cast's X is its caster's current mana, and at least 1", () => {
    const rich = playing("r452-x-rich");
    rich.players.p1.mana.current = 3;
    const before = rich.players.p2.hero.health;
    run(rich, [castRandom({ query: { defId: xSpell.id }, count: 1 })]);
    expect(rich.pending).toBeNull();
    expect(rich.players.p2.hero.health).toBe(before - 3);
    expect(rich.players.p1.mana.current).toBe(3);

    const broke = playing("r452-x-broke");
    broke.players.p1.mana.current = 0;
    const was = broke.players.p2.hero.health;
    run(broke, [castRandom({ query: { defId: xSpell.id }, count: 1 })]);
    expect(broke.players.p2.hero.health).toBe(was - 1);
  });

  it("R453 a cast Field Spell or Trap is placed (a Trap face-down); with no zone it fizzles to the graveyard, still a play", () => {
    const state = playing("r453-zone");
    const before = state.players.p2.hero.health;
    run(state, [castNew({ def: castField.id })]);
    const field = state.players.p1.backrow.find((card) => card?.defId === castField.id);
    expect(field).toBeDefined();
    expect(state.players.p2.hero.health).toBe(before - 1);

    const sink = run(state, [castNew({ def: castTrap.id })]);
    const trap = state.players.p1.backrow.find((card) => card?.defId === castTrap.id);
    expect(trap).toBeDefined();
    // Face-down to p2: the zone shows a back (R33, R227).
    expect(viewFor(state, "p2").opponent.backrow.some((view) => view !== null && view.faceDown)).toBe(true);
    expect(eventsOfType(sink.events, "cardPlayed").some((event) => event.formerId !== undefined)).toBe(true);

    // Fill the backrow: the next cast Field Spell finds no zone and fizzles.
    const full = playing("r453-fizzle");
    for (const lane of [1, 2, 3, 4, 5]) put(full, castTrap.id, slot("p1", "backrow", lane));
    const health = full.players.p2.hero.health;
    const played = full.counters.played;
    const fizzle = run(full, [castNew({ def: castField.id })]);
    // Counted as played, no Cry, and step 7 puts it in the graveyard (R138's landing).
    expect(full.counters.played).toBe(played + 1);
    expect(full.players.p2.hero.health).toBe(health);
    const landed = only(full.players.p1.graveyard.filter((card) => card.defId === castField.id));
    expect(only(eventsOfType(fizzle.events, "cardResolved").filter((event) => event.instanceId === landed.id)).permanent).toBe(false);
  });

  it("R453 a card on the field is never cast; a graveyard card cast without a rider resolves and lands in the graveyard again", () => {
    const state = playing("r453-where");
    const onField = put(state, "fx-1", slot("p1", "units", 1));
    const played = state.counters.played;
    run(state, [cast({ target: { of: "instance", instanceId: onField.id } })]);
    expect(state.counters.played).toBe(played);
    expect(activeUnitsOf(state, "p1").map((unit) => unit.id)).toContain(onField.id);

    const grave = inGraveyard(state, graveSpell.id);
    const before = state.players.p2.hero.health;
    const sink = run(state, [cast({ target: { of: "instance", instanceId: grave.id } })]);
    expect(state.players.p2.hero.health).toBe(before - 1);
    expect(state.counters.played).toBe(played + 1);
    expect(only(eventsOfType(sink.events, "cardPlayed")).instanceId).toBe(grave.id);
    expect(state.players.p1.graveyard.map((card) => card.id)).toContain(grave.id);
    expect(state.players.p1.resolving).toEqual([]);
  });
});

describe("E12 random casts (R452)", () => {
  it("R452 a random cast makes every choice at random — declared targets and modes, Discover picks, a text's prompts — and nothing pauses", () => {
    const state = playing("r452-random");
    const jogg = only(inHand(state, joggBox.id, "p1"));
    const handBefore = state.players.p1.hand.length;
    const result = pbReduce(state, { type: "play", instanceId: jogg.id, playerId: "p1" });
    expect(result.error).toBeUndefined();
    const after = result.state;
    expect(after.pending).toBeNull();
    expect(after.castsResolving).toBeUndefined();
    // Three casts, each a play for 0 (R70), none of them Jogg's Box itself (B4.1, R387).
    const casts = eventsOfType(result.events, "cardPlayed").filter((event) => event.instanceId !== jogg.id);
    expect(casts).toHaveLength(3);
    for (const cast of casts) {
      expect(cast.costPaid).toBe(0);
      expect(RANDOM_POOL).toContain(cast.defId);
    }
    // Nothing was asked of anyone.
    expect(eventsOfType(result.events, "promptOpened")).toEqual([]);
    // A random Discover put one of its options in the caster's hand.
    const discovers = casts.filter((cast) => cast.defId === discoverSpell.id).length;
    const added = eventsOfType(result.events, "addedToHand").filter((event) => DISCOVER_POOL.includes(event.defId));
    expect(added).toHaveLength(discovers);
    expect(after.players.p1.hand.length).toBe(handBefore - 1 + discovers);
  });

  it("R452 random casts are the rng's: the same state casts the same way, live and after a JSON round trip", () => {
    const state = playing("r452-determinism");
    const jogg = only(inHand(state, joggBox.id, "p1"));
    const body = { type: "play" as const, instanceId: jogg.id, playerId: "p1" as const };
    const live = pbAct(state, body);
    const again = pbAct(roundTrip(state), body);
    expect(hashState(again)).toBe(hashState(live));
  });

  it("R452 a random cast that targets enemies picks an enemy whenever one is legal", () => {
    for (const seed of ["r452-enemy-1", "r452-enemy-2", "r452-enemy-3", "r452-enemy-4", "r452-enemy-5"]) {
      const state = playing(seed);
      const mine = put(state, "fx-2", slot("p1", "units", 2));
      put(state, "fx-3", slot("p2", "units", 3));
      const card = only(inHand(state, solarius.id, "p1"));
      const result = pbReduce(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 1 }, playerId: "p1" });
      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      const hits = eventsOfType(result.events, "damage");
      expect(hits.length).toBeGreaterThan(0);
      for (const hit of hits) expect([mine.id, "hero-p1"]).not.toContain(hit.targetId);
      expect(result.state.players.p1.hero.health).toBe(state.players.p1.hero.health);
    }
  });

  it("R452 the other player's prompts are theirs: a random cast pauses on one, and the rest of the casts follow the answer", () => {
    const state = playing("r452-theirs");
    const before = state.players.p2.hero.health;
    const sink = run(state, [castRandom({ query: { defId: theirChoice.id }, count: 2 })]);
    expect(state.pending?.playerId).toBe("p2");
    expect(state.pending?.kind).toBe("mode");
    // At rest: no cast mode left on the state (the stack is transient).
    expect(castModesOf(state)).toEqual([]);
    expect(eventsOfType(sink.events, "promptOpened")).toHaveLength(1);

    const round = roundTrip(state);
    const answer = only(legalActions(state, "p2").filter((action) => action.type === "answer").slice(0, 1));
    const next = pbAct(state, { ...answer, playerId: "p2" });
    const nextRound = pbAct(round, { ...answer, playerId: "p2" });
    expect(hashState(nextRound)).toBe(hashState(next));
    // The second random cast asks p2 again.
    expect(next.pending?.playerId).toBe("p2");
    const last = pbAct(next, { ...only(legalActions(next, "p2").filter((action) => action.type === "answer").slice(0, 1)), playerId: "p2" });
    expect(last.pending).toBeNull();
    // Each cast's answer hurt the one who answered (1) and its tail the enemy hero (1): p2 twice over.
    expect(last.players.p2.hero.health).toBe(before - 4);
    expect(last.players.p1.hero.health).toBe(state.players.p1.hero.health);
  });

  it("R452 a cast made while a random cast resolves is random too, and its Echo repeats pick at random", () => {
    const state = playing("r452-nested");
    run(state, [castRandom({ query: { defId: castsNamed.id }, count: 1 })]);
    expect(state.pending).toBeNull();
    expect(state.players.p1.graveyard.map((card) => card.defId)).toEqual(expect.arrayContaining([castsNamed.id, targetSpell.id]));

    const echoing = playing("r452-echo");
    const sink = run(echoing, [castRandom({ query: { defId: echoTarget.id }, count: 1 })]);
    expect(echoing.pending).toBeNull();
    // The cast and its one repeat each dealt 1 to a target the rng picked.
    expect(eventsOfType(sink.events, "damage")).toHaveLength(2);
  });

  it(`R452 a random cast chain stops at RANDOM_CAST_CHAIN_CAP casts (${RANDOM_CAST_CHAIN_CAP})`, () => {
    const state = playing("r452-cap");
    state.players.p2.hero.health = 1000;
    const card = only(inHand(state, pingA.id, "p1"));
    const result = pbReduce(state, { type: "play", instanceId: card.id, playerId: "p1" });
    expect(result.error).toBeUndefined();
    // The played Ping A's two random casts each start a chain of at most the cap.
    const plays = eventsOfType(result.events, "cardPlayed");
    expect(plays).toHaveLength(1 + 2 * RANDOM_CAST_CHAIN_CAP);
    expect(result.state.players.p2.hero.health).toBe(1000 - plays.length);
    expect(result.state.castsResolving).toBeUndefined();
  });

  it("R452 a random answer shows the other seat nothing it may not read: no prompt, the Discovered card hidden", () => {
    const state = playing("r452-hidden");
    const card = only(inHand(state, castsDiscover.id, "p1"));
    const result = pbReduce(state, { type: "play", instanceId: card.id, playerId: "p1" });
    const after = result.state;
    expect(after.pending).toBeNull();
    const count = result.events.length;
    const theirs = { events: viewFor(after, "p2").events.slice(-count) };
    expect(eventsOfType(theirs.events, "promptOpened")).toEqual([]);
    const added = only(eventsOfType(theirs.events, "addedToHand"));
    expect(added.defId).toBe(HIDDEN_ID);
    const mine = only(eventsOfType(viewFor(after, "p1").events.slice(-count), "addedToHand"));
    expect(DISCOVER_POOL).toContain(mine.defId);
  });
});

describe("E39 target enemies on a cast its caster makes (R452)", () => {
  it("R452 a card carrying the targetEnemies enchantment, cast, offers its caster only enemies when there is one", () => {
    const state = playing("r452-enchanted");
    put(state, "fx-2", slot("p1", "units", 1));
    const enemy = put(state, "fx-3", slot("p2", "units", 1));
    const card = newInstance(state, targetSpell.id, "p1", { z: "hand", player: "p1" });
    card.enchantments = [{ kind: "targetEnemies" }];
    state.players.p1.hand.push(card);
    const sink = sinkFor(state);
    castCard(sink, card);
    expect(state.pending?.options.map((option) => option.selection)).toEqual([
      { pick: "instance", instanceId: enemy.id },
      { pick: "hero", player: "p2" },
    ]);
  });

  it("R452 the prompts a targeting-enemies cast's own text opens are narrowed the same way", () => {
    const state = playing("r452-own-prompt");
    const card = newInstance(state, askTarget.id, "p1", { z: "hand", player: "p1" });
    state.players.p1.hand.push(card);
    castCard(sinkFor(state), card, { targetEnemies: true });
    expect(state.pending?.options.map((option) => option.selection)).toEqual([{ pick: "hero", player: "p2" }]);
  });

  it("R452 with no enemy to pick, or too few, every option stays", () => {
    const state = playing("r452-no-enemy");
    const own = put(state, "fx-2", slot("p1", "units", 1));
    const options = [{ pick: "instance" as const, instanceId: own.id }, { pick: "hero" as const, player: "p1" as const }];
    expect(preferEnemies(state, "p1", options, (selection) => selection, 1)).toEqual(options);
    const mixed = [...options, { pick: "hero" as const, player: "p2" as const }];
    expect(preferEnemies(state, "p1", mixed, (selection) => selection, 1)).toEqual([{ pick: "hero", player: "p2" }]);
    expect(preferEnemies(state, "p1", mixed, (selection) => selection, 2)).toEqual(mixed);
  });
});

describe("R654 aimed random targets", () => {
  it("R654 a helpful pick under targetEnemies narrows to friends when one is legal", () => {
    const state = playing("r651-enchanted");
    const own = put(state, "fx-2", slot("p1", "units", 1));
    put(state, "fx-3", slot("p2", "units", 1));
    const card = newInstance(state, healFriend.id, "p1", { z: "hand", player: "p1" });
    card.enchantments = [{ kind: "targetEnemies" }];
    state.players.p1.hand.push(card);
    const sink = sinkFor(state);
    castCard(sink, card, { targetEnemies: true });
    expect(state.pending?.options.map((option) => option.selection)).toEqual([
      { pick: "instance", instanceId: own.id },
    ]);
  });

  it("R654 with no friend to pick, or too few, every option stays — and a mode pick is kept either way", () => {
    const state = playing("r651-no-friend");
    const foe = put(state, "fx-3", slot("p2", "units", 1));
    const options = [
      { pick: "instance" as const, instanceId: foe.id },
      { pick: "hero" as const, player: "p2" as const },
    ];
    expect(preferFriends(state, "p1", options, (selection) => selection, 1)).toEqual(options);
    const own = put(state, "fx-2", slot("p1", "units", 1));
    const mode = { pick: "mode" as const, option: "a" };
    const mixed = [...options, mode, { pick: "instance" as const, instanceId: own.id }, { pick: "hero" as const, player: "p1" as const }];
    expect(preferFriends(state, "p1", mixed, (selection) => selection, 1)).toEqual([
      mode,
      { pick: "instance", instanceId: own.id },
      { pick: "hero", player: "p1" },
    ]);
    expect(preferFriends(state, "p1", mixed, (selection) => selection, 4)).toEqual(mixed);
  });

  it("R654 a random cast that targets enemies aims each pick: harm at enemies, help at friends", () => {
    let sawDamage = false;
    let sawHeal = false;
    for (const seed of ["r651-aim-1", "r651-aim-2", "r651-aim-3", "r651-aim-4", "r651-aim-5"]) {
      const state = playing(seed);
      const mine = put(state, "fx-2", slot("p1", "units", 1));
      const foe = put(state, "fx-3", slot("p2", "units", 1));
      mine.damage = 1;
      foe.damage = 1;
      const sink = run(state, [castRandom({ query: { defId: [targetSpell.id, healFriend.id] }, count: 4, targetEnemies: true })]);
      expect(state.pending).toBeNull();
      for (const hit of eventsOfType(sink.events, "damage")) {
        sawDamage = true;
        expect([foe.id, "hero-p2"]).toContain(hit.targetId);
      }
      for (const cured of eventsOfType(sink.events, "healed")) {
        sawHeal = true;
        expect(cured.targetId).toBe(mine.id);
      }
    }
    expect(sawDamage).toBe(true);
    expect(sawHeal).toBe(true);
  });

  it("R654 Jogg's Box stays fully random: with no targetEnemies even a helpful cast may land on enemies", () => {
    const seen = new Set<string>();
    for (const seed of ["r651-box-1", "r651-box-2", "r651-box-3", "r651-box-4", "r651-box-5", "r651-box-6"]) {
      const state = playing(seed);
      const mine = put(state, "fx-2", slot("p1", "units", 1));
      const foe = put(state, "fx-3", slot("p2", "units", 1));
      mine.damage = 1;
      foe.damage = 1;
      const sink = run(state, [castRandom({ query: { defId: [healFriend.id] }, count: 2 })]);
      expect(state.pending).toBeNull();
      for (const cured of eventsOfType(sink.events, "healed")) {
        seen.add(cured.targetId === mine.id ? "friend" : "enemy");
      }
    }
    expect(seen).toEqual(new Set(["friend", "enemy"]));
  });
});
