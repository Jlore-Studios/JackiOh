// Heroic Power (SPEC §8 #98; R43, R103, R752–R758; BUILD M3-T7's `heroPower.ts` row).
//
// R752: "Heroic Power costs (0) and playing it uses nothing. Each of its thirteen powers is an
// 'Activate: Spend (X)' ability (R384): once per turn, its X paid in mana as it is activated, and the
// card has only the one it rolled." This file is the engine's half: the table, the abilities, the
// roll, and the powers whose behaviour needs nothing from the real catalog. `packages/cards`'
// `098-heroic-power.test.ts` covers the real card again, with the powers that draw from the
// catalog's pools (Witness Value, Stitching, KY Brainstorm, Pluck, Terminus Tricks, Cat Cafe).
//
// `hp-heroic` below is this file's stand-in for #98, wired as the real card is: `startOfGame` rolls
// with `rollPower`, `activations` are `powerAbilities`, and `resume` points `POWER_RESUME` at
// `heroPower`. Its catalog entry declares the `shot` number Steady Shot reads (R754).

import type { Action, ActionInput, CardDef, Selection, Tag } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { DECK_SIZE, HERO_HEALTH } from "../src/config";
import { heroArmorOf } from "../src/damage";
import { bounce } from "../src/effects";
import { effectiveCost } from "../src/mana";
import { paramValue } from "../src/params";
import { beginGame, legalActions, reduce } from "../src/reduce";
import { applyEffects, makeContext } from "../src/resolve";
import type { CardScripts, Script } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { finishSetup } from "../src/setup";
import { createGame, findInstance, type CardInstance, type GameState } from "../src/state";
import { usesThisTurn } from "../src/subsystems/activate";
import {
  ARMOR_UP_ARMOR,
  DIE_INSECT_DAMAGE,
  HERO_POWERS,
  HERO_POWER_NAMES,
  LIFE_TAP_DAMAGE,
  POWER_KEY,
  POWER_RESUME,
  STEADY_SHOT_PARAM,
  STEADY_SHOT_RAISE,
  TANK_UP_ARMOR,
  ensurePower,
  heroPower,
  powerAbilities,
  powerAbilityOf,
  powerByName,
  powerOf,
  rollPower,
  usedThisTurn,
} from "../src/subsystems/heroPower";
import { settle } from "../src/triggers";
import { activeUnitsOf } from "../src/zones";
import { vanillaDeck } from "./fixtures/catalog";
import { plain } from "./fixtures/combat";
import { HERO_POWERS as FIXTURE_POWER_NAMES, heroicPower } from "./fixtures/scripts";
import { eventsOfType, inHand, newGame, put, setLibrary, sinkFor, slot } from "./fixtures/harness";

// ---------------------------------------------------------------------------
// Fixtures: this file's stand-in for #98 and the three tokens it summons by index.
// ---------------------------------------------------------------------------

let nextIndex = 1600;

function def(name: string, type: CardDef["type"], extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `hp-${name}`,
    index: String(nextIndex),
    name: `${name} (heroPower)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
    ...extra,
  };
}

/** §8 #98's shape since R752: a Quickdraw Field Spell that costs (0) and declares Steady Shot's number. */
const heroic = def("heroic", "Field Spell", {
  cost: 0,
  tags: ["Quickdraw"],
  rarity: "Mythic",
  params: [{ key: STEADY_SHOT_PARAM, base: 2, radiant: 4, better: "up", step: STEADY_SHOT_RAISE, min: 1 }],
});

function token(index: string, name: string, attack: number, health: number, tags: Tag[] = []): CardDef {
  return {
    id: `hp-token-${name}`,
    index,
    name: `${name} Token (heroPower)`,
    set: "Core",
    type: "Unit",
    tags: [...tags, "Token"],
    rarity: "Token",
    token: true,
    cost: 1,
    base: { attack, health, keywords: [], text: `${attack}/${health}` },
    radiant: { attack: attack * 2, health: health * 2, keywords: [], text: `${attack * 2}/${health * 2}` },
  };
}

const rushToken = token("T-rush", "rush", 3, 3);
const felinorToken = token("T-felinor", "felinor", 1, 1, ["Felinor"]);
const ghoulToken = token("T-ghoul", "ghoul", 0, 0);

/** A 1-health unit with Armor 5, for Ping's Pierce and its Radiant Ghoul. */
const armored = def("armored", "Unit", {
  base: { attack: 4, health: 1, keywords: [{ kind: "Armor", n: 5 }], text: "Armor 5" },
  radiant: { attack: 8, health: 2, keywords: [{ kind: "Armor", n: 5 }], text: "Armor 5" },
});

const DEFS: CardDef[] = [heroic, rushToken, felinorToken, ghoulToken, armored];

function heroicScript(radiant: boolean): Script {
  return {
    staticFlags: { quickdraw: true },
    startOfGame: () => [rollPower()],
    activations: powerAbilities(radiant),
    resume: { [POWER_RESUME]: heroPower },
  };
}

const SCRIPTS: Record<string, CardScripts> = {
  [heroic.id]: { base: heroicScript(false), radiant: heroicScript(true) },
};

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((entry) => [entry.id, entry])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  state.turn = 4;
  state.active = "p1";
  state.phase = "main";
  state.players.p1.mana = { current: 4, max: 4, nextTurnMod: 0, permMod: 0 };
  return state;
}

/** A Heroic Power on the field with a chosen power, which is the state every activation needs. */
function powered(state: GameState, name: string, options: { radiant?: boolean; lane?: number } = {}): CardInstance {
  const card = put(state, heroic.id, slot("p1", "backrow", options.lane ?? 1), options);
  card.memory[POWER_KEY] = name;
  return card;
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

let nonce = 0;
function act(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `hp${nonce}` } as Action);
}

/** One `activate` of the card's power, as the client sends it (R752). */
function use(state: GameState, card: CardInstance, targets?: Selection[]): ReturnType<typeof reduce> {
  return act(state, { type: "activate", instanceId: card.id, playerId: "p1", ...(targets === undefined ? {} : { targets }) });
}

/** Keep the turn from auto-ending (§2.5) under the assertions: a second card p1 could still play. */
function keepTurn(state: GameState): void {
  inHand(state, plain.id, "p1");
}

function onFieldNow(state: GameState, id: string): CardInstance {
  return must(findInstance(state, id), `card ${id}`);
}

// ---------------------------------------------------------------------------
// The table and the abilities (R103, R752).
// ---------------------------------------------------------------------------

describe("Heroic Power: the thirteen powers and their abilities (R103, R752)", () => {
  it("R103 keeps the eight stored names and adds the patch's five at the end, each with its X", () => {
    expect(HERO_POWER_NAMES).toEqual([
      "recruit",
      "draw",
      "ping",
      "burn",
      "rush",
      "felinor",
      "discover",
      "stitching",
      "armor",
      "insect",
      "brainstorm",
      "pluck",
      "tricks",
    ]);
    expect(HERO_POWERS.map((power) => power.x)).toEqual([3, 1, 1, 1, 2, 1, 2, 2, 1, 2, 2, 2, 3]);
    expect(HERO_POWERS.map((power) => power.title)).toEqual([
      "Expedition Map",
      "Life Tap",
      "Ping",
      "Steady Shot",
      "Ranching",
      "Cat Cafe",
      "Witness Value",
      "Stitching",
      "Armor Up",
      "Die Insect",
      "KY Brainstorm",
      "Pluck",
      "Terminus Tricks",
    ]);
    // R757: Armor Up is the one power named otherwise on the Radiant face.
    expect(HERO_POWERS.filter((power) => power.radiantTitle !== power.title).map((power) => power.radiantTitle)).toEqual(["Tank Up"]);
    expect(powerByName("nonsense")).toBeNull();
  });

  it("R752 each power is a once-per-turn Activate ability paying its X, the card's only while it rolled it", () => {
    const state = game("r752-abilities");
    const card = powered(state, "ping");
    for (const radiant of [false, true]) {
      const abilities = powerAbilities(radiant);
      expect(abilities.map((decl) => decl.id)).toEqual(HERO_POWER_NAMES);
      for (const [index, decl] of abilities.entries()) {
        const power = must(HERO_POWERS[index], "a power");
        expect(decl).toMatchObject({ uses: 1, cost: { mana: power.x } });
        expect(decl.label.startsWith(`${radiant ? power.radiantTitle : power.title}: `)).toBe(true);
        expect(decl.has?.({ state, self: card, radiant })).toBe(power.name === "ping");
      }
    }
    // R81: only Ping declares a target, any unit or hero.
    expect(powerAbilities(false).filter((decl) => decl.targets !== undefined).map((decl) => decl.id)).toEqual(["ping"]);
    expect(powerAbilityOf(state, card)?.id).toBe("ping");
    delete card.memory[POWER_KEY];
    expect(powerAbilityOf(state, card)).toBeNull();
  });

  it("R752 the card costs (0) and playing it uses nothing: its power is then one activation a turn", () => {
    const state = game("r752-play");
    const card = must(inHand(state, heroic.id, "p1")[0], "a Heroic Power in hand");
    card.memory[POWER_KEY] = "burn";
    keepTurn(state);
    expect(effectiveCost(state, card)).toBe(0);

    const played = act(state, { type: "play", instanceId: card.id, playerId: "p1" });
    expect(played.error).toBeUndefined();
    expect(eventsOfType(played.events, "cardPlayed")[0]?.costPaid).toBe(0);
    expect(played.state.players.p1.mana.current).toBe(4);
    expect(played.state.players.p2.hero.health).toBe(HERO_HEALTH);
    expect(eventsOfType(played.events, "activated")).toEqual([]);

    const onField = onFieldNow(played.state, card.id);
    expect(usedThisTurn(played.state, onField)).toBe(false);
    const shot = use(played.state, onField);
    expect(shot.error).toBeUndefined();
    expect(shot.state.players.p2.hero.health).toBe(HERO_HEALTH - 2);
    expect(shot.state.players.p1.mana.current).toBe(3);
  });

  it("R752 a use pays X once per turn, is refused unaffordable, and the next turn is a new use", () => {
    const state = game("r752-once");
    const card = powered(state, "recruit"); // X 3
    setLibrary(state, "p1", [plain.id]);
    keepTurn(state);
    state.players.p1.mana.current = 2;
    expect(act(state, { type: "activatePower", instanceId: card.id, playerId: "p1" }).error).toBe(
      "that ability costs 3, more than your mana",
    );
    expect(legalActions(state, "p1").some((body) => body.type === "activate" && body.instanceId === card.id)).toBe(false);

    state.players.p1.mana.current = 4;
    expect(legalActions(state, "p1")).toContainEqual({ type: "activate", instanceId: card.id, ability: "recruit" });
    const used = act(state, { type: "activatePower", instanceId: card.id, playerId: "p1" });
    expect(used.error).toBeUndefined();
    expect(used.state.players.p1.mana.current).toBe(1);
    expect(activeUnitsOf(used.state, "p1").map((unit) => unit.defId)).toEqual([plain.id]);
    expect(usedThisTurn(used.state, onFieldNow(used.state, card.id))).toBe(true);
    expect(use(used.state, card).error).toBe("that ability has already been used this turn");

    const later = structuredClone(used.state);
    later.turn += 2;
    later.players.p1.mana.current = 4;
    expect(usedThisTurn(later, onFieldNow(later, card.id))).toBe(false);
    expect(use(later, card).error).toBeUndefined();
  });

  it("R752 the power is not the opponent's, works only from the field, and the alias names it (R384)", () => {
    const state = game("r752-who");
    const card = powered(state, "burn");
    expect(act(state, { type: "activate", instanceId: card.id, playerId: "p2" }).error).toBe("it is not your turn");
    const held = must(inHand(state, heroic.id, "p1")[0], "a Heroic Power in hand");
    held.memory[POWER_KEY] = "burn";
    expect(act(state, { type: "activatePower", instanceId: held.id, playerId: "p1" }).error).toBe("that card is not on the field");
    keepTurn(state);
    const viaAlias = act(state, { type: "activatePower", instanceId: card.id, playerId: "p1" });
    const viaActivate = act(state, { type: "activate", instanceId: card.id, ability: "burn", playerId: "p1" });
    expect(viaAlias.error).toBeUndefined();
    expect(viaAlias.state.players.p2.hero.health).toBe(viaActivate.state.players.p2.hero.health);
  });
});

// ---------------------------------------------------------------------------
// The powers that need nothing from the catalog (R753–R758).
// ---------------------------------------------------------------------------

describe("Heroic Power: the powers (R753–R758)", () => {
  it("R753 Life Tap draws 1 and deals 2 to your own hero; Radiant draws the top card of each deck", () => {
    const state = game("r753-life-tap");
    const card = powered(state, "draw");
    setLibrary(state, "p1", [plain.id, plain.id]);
    setLibrary(state, "p2", [plain.id, plain.id]);
    keepTurn(state);
    const hand = state.players.p1.hand.length;
    const after = use(state, card).state;
    expect(after.players.p1.hand.length).toBe(hand + 1);
    expect(after.players.p1.hero.health).toBe(HERO_HEALTH - LIFE_TAP_DAMAGE);
    expect(eventsOfType(use(state, card).events, "damage")).toHaveLength(1);

    const shining = game("r753-life-tap-radiant");
    const radiant = powered(shining, "draw", { radiant: true });
    setLibrary(shining, "p1", [plain.id, plain.id]);
    const theirs = setLibrary(shining, "p2", [plain.id, plain.id]);
    keepTurn(shining);
    const before = shining.players.p1.hand.length;
    const drawn = use(shining, radiant).state;
    expect(drawn.players.p1.hand.length).toBe(before + 2);
    expect(drawn.players.p1.hand.map((entry) => entry.id)).toContain(must(theirs[0], "their top card").id);
    expect(drawn.players.p2.library).toHaveLength(1);
    expect(drawn.players.p1.hero.health).toBe(HERO_HEALTH);
  });

  it("R754 Steady Shot deals {shot} to the enemy hero; on the Radiant face it then deals 2 more each use", () => {
    const state = game("r754-steady");
    const card = powered(state, "burn");
    keepTurn(state);
    const once = use(state, card).state;
    expect(once.players.p2.hero.health).toBe(HERO_HEALTH - 2);
    expect(paramValue(once, onFieldNow(once, card.id), STEADY_SHOT_PARAM)).toBe(2);

    let shining = game("r754-steady-radiant");
    const radiant = powered(shining, "burn", { radiant: true });
    keepTurn(shining);
    let lost = 0;
    for (const expected of [4, 6, 8]) {
      const result = use(shining, radiant);
      expect(result.error).toBeUndefined();
      lost += expected;
      expect(result.state.players.p2.hero.health).toBe(HERO_HEALTH - lost);
      expect(eventsOfType(result.events, "numberChanged")[0]).toMatchObject({ key: STEADY_SHOT_PARAM, value: expected + STEADY_SHOT_RAISE });
      shining = structuredClone(result.state);
      shining.turn += 2;
      shining.players.p1.mana.current = 4;
    }
  });

  it("R756 Ping pierces Armor; on the Radiant face a Unit it kills leaves a Ghoul Token with its stats", () => {
    const state = game("r756-ping");
    const card = powered(state, "ping");
    const victim = put(state, armored.id, slot("p2", "units", 1));
    keepTurn(state);
    // R81: the target is declared with the activation; legalActions lists one per target.
    const listed = legalActions(state, "p1").filter((body) => body.type === "activate" && body.instanceId === card.id);
    expect(listed.length).toBeGreaterThan(2);
    expect(use(state, card).error).not.toBeUndefined();
    const hit = use(state, card, [{ pick: "instance", instanceId: victim.id }]);
    expect(hit.error).toBeUndefined();
    expect(eventsOfType(hit.events, "damage")[0]?.amount).toBe(1);
    expect(hit.state.players.p2.units[0]?.[0]).toBeUndefined();
    expect(activeUnitsOf(hit.state, "p1")).toEqual([]);

    const shining = game("r756-ping-radiant");
    const radiant = powered(shining, "ping", { radiant: true });
    const prey = put(shining, armored.id, slot("p2", "units", 1));
    keepTurn(shining);
    const killed = use(shining, radiant, [{ pick: "instance", instanceId: prey.id }]);
    expect(killed.error).toBeUndefined();
    const ghouls = activeUnitsOf(killed.state, "p1");
    expect(ghouls.map((unit) => unit.defId)).toEqual([ghoulToken.id]);
    expect(ghouls[0]?.statsOverride).toEqual({ attack: 4, health: 1 });

    // A hero hit, or a Unit the hit leaves standing, summons nothing.
    const face = use(shining, radiant, [{ pick: "hero", player: "p2" }]);
    expect(activeUnitsOf(face.state, "p1")).toEqual([]);
    const sturdy = put(shining, plain.id, slot("p2", "units", 2));
    const survived = use(shining, radiant, [{ pick: "instance", instanceId: sturdy.id }]);
    expect(activeUnitsOf(survived.state, "p1")).toEqual([]);
  });

  it("R757 Armor Up's 2 Armor holds through the opponent's turn and is gone when yours begins", () => {
    const state = game("r757-armor");
    const card = powered(state, "armor");
    keepTurn(state);
    const armored = use(state, card).state;
    expect(heroArmorOf(armored, "p1")).toBe(ARMOR_UP_ARMOR);
    const theirTurn = act(armored, { type: "endTurn", playerId: "p1" }).state;
    expect(theirTurn.active).toBe("p2");
    expect(heroArmorOf(theirTurn, "p1")).toBe(ARMOR_UP_ARMOR);
    const mine = act(theirTurn, { type: "endTurn", playerId: "p2" }).state;
    expect(mine.active).toBe("p1");
    expect(heroArmorOf(mine, "p1")).toBe(0);
  });

  it("R757 Tank Up keeps 4 Armor, then refreshes into a different power that may be used this turn", () => {
    const state = game("r757-tank-up");
    const card = powered(state, "armor", { radiant: true });
    keepTurn(state);
    const tanked = use(state, card);
    expect(tanked.error).toBeUndefined();
    const after = tanked.state;
    expect(after.players.p1.hero.armor).toBe(TANK_UP_ARMOR);
    const changed = onFieldNow(after, card.id);
    const next = must(powerOf(changed), "a new power");
    expect(next.name).not.toBe("armor");
    expect(usesThisTurn(after, changed)).toBe(0);
    expect(usedThisTurn(after, changed)).toBe(false);
    // The armor stays into the next turn: Tank Up's is the hero's own (R757).
    const later = act(act(after, { type: "endTurn", playerId: "p1" }).state, { type: "endTurn", playerId: "p2" }).state;
    expect(heroArmorOf(later, "p1")).toBeGreaterThanOrEqual(TANK_UP_ARMOR);
  });

  it("R758 Die Insect deals 8 to a random enemy; the Radiant face's Lucky 1 hits a Unit more often", () => {
    const hits = (radiant: boolean): { hero: number; unit: number } => {
      const count = { hero: 0, unit: 0 };
      for (let n = 0; n < 40; n += 1) {
        const state = game(`r758-insect-${radiant ? "r" : "b"}-${n}`);
        const card = powered(state, "insect", { radiant });
        put(state, plain.id, slot("p2", "units", 1));
        keepTurn(state);
        const result = use(state, card);
        expect(result.error).toBeUndefined();
        const damage = eventsOfType(result.events, "damage");
        expect(damage).toHaveLength(1);
        expect(damage[0]?.amount).toBe(DIE_INSECT_DAMAGE);
        if (damage[0]?.targetId === "hero-p2") count.hero += 1;
        else count.unit += 1;
      }
      return count;
    };
    const base = hits(false);
    const radiant = hits(true);
    expect(base.hero).toBeGreaterThan(0);
    expect(base.unit).toBeGreaterThan(0);
    expect(radiant.unit).toBeGreaterThan(base.unit);
  });
});

// ---------------------------------------------------------------------------
// The roll (R43, R78, R151).
// ---------------------------------------------------------------------------

describe("Heroic Power: rolling the power (R43, R78)", () => {
  it("R43 rolls a power at start of game for every copy in either player's hand or library", () => {
    const state = game("r43-roll-setup");
    const p1Hand = must(inHand(state, heroic.id, "p1")[0], "p1's hand copy");
    const p2Hand = must(inHand(state, heroic.id, "p2")[0], "p2's hand copy");
    setLibrary(state, "p1", [heroic.id, plain.id]);
    setLibrary(state, "p2", [plain.id, heroic.id]);
    const p1Library = must(state.players.p1.library.find((c) => c.defId === heroic.id), "p1's library copy");
    const p2Library = must(state.players.p2.library.find((c) => c.defId === heroic.id), "p2's library copy");

    for (const card of [p1Hand, p2Hand, p1Library, p2Library]) expect(card.memory[POWER_KEY]).toBeUndefined();
    finishSetup(sinkFor(state));
    for (const card of [p1Hand, p2Hand, p1Library, p2Library]) {
      expect(HERO_POWER_NAMES).toContain(card.memory[POWER_KEY]);
      expect(powerOf(card)).not.toBeNull();
    }
  });

  it("R43 the roll comes from the match rng, so the same seed rolls the same power", () => {
    const rolled = (seed: string): unknown => {
      const state = game(seed);
      const card = must(inHand(state, heroic.id, "p1")[0], "a Heroic Power in hand");
      finishSetup(sinkFor(state));
      return card.memory[POWER_KEY];
    };
    const first = rolled("r43-seeded");
    expect(HERO_POWER_NAMES).toContain(first);
    expect(rolled("r43-seeded")).toBe(first);

    const state = game("r43-two-copies");
    const cards = inHand(state, heroic.id, "p1", 6);
    finishSetup(sinkFor(state));
    expect(new Set(cards.map((card) => card.memory[POWER_KEY])).size).toBeGreaterThan(1);
  });

  it("R43 the M1-M3 fixture Heroic Power also rolls at start of game (setup §2.1)", () => {
    const state = newGame("r43-fixture-roll");
    const card = must(inHand(state, heroicPower.id, "p2")[0], "the fixture Heroic Power");
    expect(card.memory.power).toBeUndefined();
    finishSetup(sinkFor(state));
    expect(FIXTURE_POWER_NAMES).toContain(card.memory.power);
  });

  it("R43 ensurePower rolls for an instance with no power and keeps the one it has", () => {
    const state = game("r43-ensure");
    const sink = sinkFor(state);
    const card = must(inHand(state, heroic.id, "p1")[0], "a Heroic Power in hand");
    const rolled = must(ensurePower(sink, card), "a rolled power");
    expect(card.memory[POWER_KEY]).toBe(rolled.name);
    expect(ensurePower(sink, card)?.name).toBe(rolled.name);
    card.memory[POWER_KEY] = "recruit";
    expect(ensurePower(sink, card)?.name).toBe("recruit");
  });

  it("R43 a bounced Heroic Power arrives in hand with a power again (R78, R151)", () => {
    const state = game("r43-bounced");
    const sink = sinkFor(state);
    const card = powered(state, "recruit");
    applyEffects(
      [bounce({ target: { of: "chosen" } })],
      makeContext(sink, card, { controller: "p1", targets: [{ pick: "instance", instanceId: card.id }] }),
    );
    expect(card.zone).toEqual({ z: "hand", player: "p1" });
    settle(sink);
    expect(HERO_POWER_NAMES).toContain(card.memory[POWER_KEY]);
    expect(effectiveCost(state, card)).toBe(0);
  });

  it("R43 a game begun with a Heroic Power in the deck has one with a power in play (§2.1)", () => {
    game("r43-begin");
    const deck = [heroic.id, ...vanillaDeck(DECK_SIZE - 1, 1)];
    const begun = beginGame(createGame({ seed: "r43-begin", decks: [deck, vanillaDeck(DECK_SIZE, 21)] })).state;
    let playing = act(begun, { type: "mulligan", keep: begun.players.p1.hand.map((card) => card.id), playerId: "p1" }).state;
    playing = act(playing, { type: "mulligan", keep: playing.players.p2.hand.map((card) => card.id), playerId: "p2" }).state;
    const card = must(
      [...playing.players.p1.hand, ...playing.players.p1.library].find((entry) => entry.defId === heroic.id),
      "the Heroic Power",
    );
    expect(HERO_POWER_NAMES).toContain(card.memory[POWER_KEY]);
    expect(effectiveCost(playing, card)).toBe(0);
  });
});
