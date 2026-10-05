// Heroic Power (SPEC §8 #98, patch v0.2.1): the thirteen powers, the roll that picks one, and each
// power as an Activate ability of the card (`subsystems/heroPower.ts` through
// `subsystems/activate.ts`).
//
// Where the rules come from, by the row each test is named after (CLAUDE.md rule 3):
//   - R43: the rolled power lives on the instance (`memory.power`), and its uses are the Activate
//     count every ability keeps (`memory.activations`, R384). Every copy in a hand or library rolls
//     at the start of the game, from the match rng. #98 costs (0) to play and playing it activates
//     nothing; its one rolled power is an "Activate: Spend (X)" ability, once per turn, and
//     `activatePower` is kept as an alias of `activate` so old logs replay. "Recruit a permanent"
//     uses Recruit's permanent pool.
//   - R103: the surface — the stored names (state, so stable: a new power is added at the end), a
//     copy that has not rolled has no ability, the use is marked before the effect runs, Witness
//     Value's pick goes to hand (Radiant when the power is), Ping names its target in the action
//     (R81), and a token power whose token the catalog lacks fizzles in silence.
//   - R151: a copy rolls as it arrives anywhere a card can be looked at, not only at setup.
//   - R352: Stitching's two chained Discovers, fused per R77 at the fused cost.
//   - R384: the Activate machinery the powers run through — the refusal that is also the list, the
//     mana price, the declared target, once per turn, the `activated` event.
//   - R661–R669, patch v0.2.1's rows: Armor Up and Tank Up's Armor, Tank Up's refresh, Die Insect's
//     Lucky pick, Ping's Pierce and kill rider, Life Tap's damage and Radiant draw, Steady Shot's
//     permanent upgrade, Terminus Tricks' pool, and the KY Brainstorm and Pluck pools.
//
// The card under test is `./fixtures/scripts.ts`'s `heroicPower`, wired as #98 is: it costs (0), its
// `startOfGame` is `rollPower`, its `activations` are `heroPowerActivations(face)`, its `resume`
// table points `POWER_RESUME` at `heroPower`, and it declares Steady Shot's `shot` (2, Radiant 4,
// step 2) as the catalog does. The engine never imports `packages/cards` (CLAUDE.md, Architecture),
// so the tokens and pools the powers reach are this file's own test-only definitions below; the real
// card's test, `packages/cards/test/098-heroic-power.test.ts`, covers the same cases again.

import type { Action, ActionBody, ActionInput, CardDef, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { defByIndex, defOf, fusedIdParts, queryCost, registerCatalog, registeredCatalog } from "../src/catalog";
import {
  ARMOR_UP,
  DECK_SIZE,
  DIE_INSECT_DAMAGE,
  FUSE_COST_CAP,
  HERO_HEALTH,
  HERO_POWER_COST,
  KY_BRAINSTORM_DISCOUNT,
  LIFE_TAP_DAMAGE,
  LIFE_TAP_DRAW,
  MAX_MANA,
  PING_DAMAGE,
  PLUCK_COST,
} from "../src/config";
import { dealDamage, heroArmorOf } from "../src/damage";
import { addToHand, bounce, draw, summon } from "../src/effects";
import { unitView } from "../src/layers";
import { effectiveCost } from "../src/mana";
import { paramValue } from "../src/params";
import { isFaceDown } from "../src/preview";
import { beginGame, legalActions, reduce, seatToAct } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { applyEffects, makeContext } from "../src/resolve";
import { createRng } from "../src/rng";
import { registerScripts, registeredScripts } from "../src/scripts";
import { finishSetup } from "../src/setup";
import { createGame, findInstance, type CardInstance, type GameState, type PendingChoice } from "../src/state";
import {
  ACTIVATIONS_MEMORY_KEY,
  abilitiesOf,
  activateAbility,
  whyCannotActivateAbility,
} from "../src/subsystems/activate";
import {
  FELINOR_TOKEN_INDEX,
  GHOUL_TOKEN_INDEX,
  HERO_POWERS,
  HERO_POWER_NAMES,
  POWER_KEY,
  RUSH_TOKEN_INDEX,
  STEADY_SHOT_PARAM,
  STITCHING_MAX_COST,
  ensurePower,
  heroPowerActivations,
  powerByName,
  powerCostOf,
  powerOf,
  powerTitle,
  rollPower,
  usedThisTurn,
  type HeroPowerName,
} from "../src/subsystems/heroPower";
import { viewFor } from "../src/viewFor";
import { activeUnitsOf, moveToZone } from "../src/zones";
import { ACTIVATE_SCRIPTS, LOG_LANE, logCard, mourner, notes } from "./fixtures/activate";
import { vanillaDeck } from "./fixtures/catalog";
import { bigBody, plain } from "./fixtures/combat";
import { eventsOfType, inHand, newGame, put, setLibrary, setupCatalog, sinkFor, slot } from "./fixtures/harness";
import { heroicPower, stockpile, xBolt } from "./fixtures/scripts";

// ---------------------------------------------------------------------------
// Fixtures: the tokens and pools the powers reach (test-only, CLAUDE.md Architecture).
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

function unit(name: string, attack: number, health: number, extra: Partial<CardDef> = {}): CardDef {
  return def(name, "Unit", {
    base: { attack, health, keywords: [], text: `${attack}/${health}` },
    radiant: { attack: attack * 2, health: health * 2, keywords: [], text: `${attack * 2}/${health * 2}` },
    ...extra,
  });
}

/** §7: a token, which no random pool offers unless the pool names it (§5.1, R382). */
function token(base: CardDef, index: string): CardDef {
  return { ...base, index, tags: [...base.tags, "Token"], rarity: "Token", token: true };
}

/** §7's Felinor Token, which Cat Cafe summons by index. It is a Felinor, so a Felinor pool must leave it out. */
const felinorToken = token(unit("felinor-token", 1, 1, { tags: ["Felinor"] }), FELINOR_TOKEN_INDEX);
/** §7's Ghoul Token, whose X/X Radiant Ping sets from the Unit it killed (R664). */
const ghoulToken = token(unit("ghoul-token", 1, 1), GHOUL_TOKEN_INDEX);
/** Radiant Cat Cafe's "random Felinor": the one non-token Felinor Unit, in another set (R380). */
const felinor = unit("felinor", 2, 3, { tags: ["Felinor"], cost: 2, set: "Classic" });
/** KY Brainstorm's pool (R668): one KY card, a Spell, so the discount reaches it too. */
const KY_SPELL_COST = 3;
const kySpell = def("ky-spell", "Spell", { tags: ["KY"], cost: KY_SPELL_COST, set: "Classic" });
/** Pluck's pool (R668, R382): a Fruit card and a Grape, a Fruit token the pool holds too. */
const fruit = def("fruit", "Spell", { tags: ["Fruit"], cost: 3, set: "Classic+" });
const grape = token(def("grape", "Spell", { tags: ["Fruit"] }), "T-grape");
/** Terminus Tricks' pool (R667): a Trap and a Field Trap, of two sets. The fixture catalog has no other. */
const trap = def("trap", "Trap", { set: "Classic" });
const fieldTrap = def("field-trap", "Field Trap", { cost: 2, set: "Classic+" });
/** A Spell KY Brainstorm discounts. */
const CHEAP_SPELL_COST = 2;
const cheapSpell = def("cheap-spell", "Spell", { cost: CHEAP_SPELL_COST });
/** A 3/3 with Armor 2, which Ping's Pierce ignores (R664). */
const armored = def("armored", "Unit", {
  base: { attack: 3, health: 3, keywords: [{ kind: "Armor", n: 2 }], text: "Armor 2" },
  radiant: { attack: 6, health: 6, keywords: [{ kind: "Armor", n: 2 }], text: "Armor 2" },
});
/** A Field Spell to fill a backrow with. */
const filler = def("filler", "Field Spell");

const DEFS: CardDef[] = [felinorToken, ghoulToken, felinor, kySpell, fruit, grape, trap, fieldTrap, cheapSpell, armored, filler];

function register(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((entry) => [entry.id, entry])) });
}

/** The fixture catalog plus this file's definitions, with a new game on top. */
function game(seed: string, decks?: [string[], string[]]): GameState {
  const state = newGame(`hp-${seed}`, decks);
  register();
  return state;
}

/** A game that never began, set to p1's main phase at full mana: for checks that call the subsystem directly. */
function bare(seed: string): GameState {
  const state = game(seed);
  state.turn = 4;
  state.active = "p1";
  state.phase = "main";
  state.players.p1.mana = { current: MAX_MANA, max: MAX_MANA, nextTurnMod: 0, permMod: 0 };
  return state;
}

let nonce = 0;

function actResult(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `hp${nonce}` } as Action);
}

function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  const result = actResult(state, body);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

/** Past both mulligans: p1's main phase on turn 1 at full mana, with no turn ending behind a test's back (R345). */
function playing(seed: string): GameState {
  let state = beginGame(game(seed)).state;
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
  }
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  state.players.p1.mana.current = MAX_MANA;
  return state;
}

/** p1's turn ends, p2's turn ends, and p1 is back in their main phase at full mana. */
function nextOwnTurn(state: GameState): GameState {
  let next = act(state, { type: "endTurn", playerId: "p1" }).state;
  next = act(next, { type: "endTurn", playerId: "p2" }).state;
  expect(next.active).toBe("p1");
  next.players.p1.mana.current = MAX_MANA;
  return next;
}

/** A Heroic Power on p1's field that has rolled `power`, which is the state every activation needs. */
function powered(state: GameState, power: HeroPowerName, options: { radiant?: boolean; lane?: number } = {}): CardInstance {
  const card = put(state, heroicPower.id, slot("p1", "backrow", options.lane ?? 1), options.radiant === true ? { radiant: true } : {});
  card.memory[POWER_KEY] = power;
  return card;
}

/** The `activate` of the card's rolled power. */
function use(card: CardInstance, extra: { targets?: Selection[] } = {}): ActionInput {
  const ability = must(powerOf(card), "a rolled power").name;
  return { type: "activate", instanceId: card.id, ability, playerId: "p1", ...extra };
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

function cardIn(state: GameState, instanceId: string): CardInstance {
  return must(findInstance(state, instanceId), `card ${instanceId}`);
}

/** The catalog id a Discover option stands for (§6.3: the option is a mode selection carrying it). */
function offered(option: PendingChoice["options"][number]): string {
  if (option.selection.pick !== "mode") throw new Error("a Discover option is a mode selection");
  return option.selection.option;
}

/** Answer the open prompt with the option `which` picks (the first by default), whoever holds it. */
function answer(
  state: GameState,
  which: (option: PendingChoice["options"][number]) => boolean = () => true,
): { state: GameState; events: GameEvent[] } {
  const pending = must(state.pending, "an open prompt");
  const option = must(pending.options.find(which), "an option to pick");
  return act(state, { type: "answer", choiceId: pending.id, selection: [option.selection], playerId: pending.playerId });
}

const roundTrip = (state: GameState): GameState => JSON.parse(JSON.stringify(state)) as GameState;

const atHero = (player: PlayerId): Selection => ({ pick: "hero", player });
const atCard = (card: CardInstance): Selection => ({ pick: "instance", instanceId: card.id });

/** The targets this card's hits landed on, in order (`damage` events, R42's source). */
function hitsBy(events: readonly GameEvent[], card: CardInstance): string[] {
  return eventsOfType(events, "damage")
    .filter((event) => event.sourceId === card.id)
    .map((event) => event.targetId);
}

// ---------------------------------------------------------------------------
// The thirteen powers (R43, R103).
// ---------------------------------------------------------------------------

describe("Heroic Power: the thirteen powers and their X (R43, R103, §8 #98)", () => {
  it("R103 stores the thirteen names in roll order, the eight before patch v0.2.1 first, each with its X", () => {
    // R103: the names are state, so a new power is added at the end and none moves.
    expect([...HERO_POWER_NAMES]).toEqual([
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
      "terminus",
    ]);
    expect(new Set(HERO_POWER_NAMES).size).toBe(HERO_POWERS.length);
    // Every power has its X in the config table, and the table names no other power (CLAUDE.md rule 9).
    expect(Object.keys(HERO_POWER_COST).sort()).toEqual([...HERO_POWER_NAMES].sort());
    for (const power of HERO_POWERS) {
      expect(power.x).toBe(HERO_POWER_COST[power.name]);
      expect(powerByName(power.name)).toBe(power);
      expect(power.label.length).toBeGreaterThan(0);
      expect(power.radiantLabel.length).toBeGreaterThan(0);
    }
    expect(powerByName("fireball")).toBeNull();

    // §8 #98 prints each power under its name; Armor Up's Radiant face is Tank Up.
    expect(HERO_POWERS.map((power) => powerTitle(power, false))).toEqual([
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
    const armor = must(powerByName("armor"), "Armor Up");
    expect(powerTitle(armor, true)).toBe("Tank Up");
  });

  it("R43 declares one Activate ability per power, once per turn for its X, and a copy has only the one it rolled", () => {
    for (const radiant of [false, true]) {
      const decls = heroPowerActivations(radiant);
      expect(decls.map((decl) => decl.id)).toEqual([...HERO_POWER_NAMES]);
      for (const decl of decls) {
        const power = must(powerByName(decl.id), decl.id);
        expect(decl.uses).toBe(1);
        expect(decl.cost).toEqual({ mana: HERO_POWER_COST[power.name] });
        expect(decl.label.startsWith(`${powerTitle(power, radiant)}: `)).toBe(true);
        // R103, R81: only Ping declares a target, one unit or hero on either side.
        if (decl.id === "ping") {
          expect(decl.targets).toEqual([{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }]);
        } else {
          expect(decl.targets).toBeUndefined();
        }
      }
    }

    const state = bare("abilities");
    const card = put(state, heroicPower.id, slot("p1", "backrow", 1));
    // R103: a copy that has not rolled declares no usable ability.
    expect(abilitiesOf(state, card)).toEqual([]);
    for (const name of HERO_POWER_NAMES) {
      card.memory[POWER_KEY] = name;
      expect(abilitiesOf(state, card).map((decl) => decl.id)).toEqual([name]);
      expect(powerCostOf(card)).toBe(HERO_POWER_COST[name]);
    }
  });
});

// ---------------------------------------------------------------------------
// The roll (R43, R151).
// ---------------------------------------------------------------------------

describe("Heroic Power: rolling the power (R43, R151, R78)", () => {
  it("R43 stores the power in memory.power, and ensurePower rolls only a copy that has none", () => {
    const state = bare("ensure");
    const sink = sinkFor(state);
    const [card] = inHand(state, heroicPower.id, "p1");
    const held = must(card, "a Heroic Power in hand");
    expect(powerOf(held)).toBeNull();
    expect(powerCostOf(held)).toBe(0);

    const rolled = must(ensurePower(sink, held), "a rolled power");
    expect(held.memory[POWER_KEY]).toBe(rolled.name);
    expect(powerOf(held)).toBe(rolled);
    // Idempotent: a copy that rolled keeps its power however often it is asked, and draws no rng.
    const cursor = sink.rng.cursor;
    expect(ensurePower(sink, held)).toBe(rolled);
    applyEffects([rollPower({ instanceId: held.id })], makeContext(sink, null, { controller: "p1" }));
    expect(held.memory[POWER_KEY]).toBe(rolled.name);
    expect(sink.rng.cursor).toBe(cursor);

    // Whatever it holds is what it keeps.
    held.memory[POWER_KEY] = "terminus";
    expect(ensurePower(sink, held)?.name).toBe("terminus");
  });

  it("R43 rolls at start of game for every copy in either player's hand or library", () => {
    const state = game("roll-setup");
    const [p1Hand] = inHand(state, heroicPower.id, "p1");
    const [p2Hand] = inHand(state, heroicPower.id, "p2");
    const [p1Library] = setLibrary(state, "p1", [heroicPower.id, plain.id]);
    const [, p2Library] = setLibrary(state, "p2", [plain.id, heroicPower.id]);
    const copies = [p1Hand, p2Hand, p1Library, p2Library].map((card, at) => must(card, `copy ${at}`));
    for (const card of copies) expect(card.memory[POWER_KEY]).toBeUndefined();

    // §2.1 step 4: the start-of-game clauses resolve for every card in a hand or library.
    finishSetup(sinkFor(state));

    for (const card of copies) {
      expect(HERO_POWER_NAMES).toContain(card.memory[POWER_KEY]);
      expect(powerCostOf(card)).toBe(must(powerOf(card), "a power").x);
    }
  });

  it("R43 the roll comes from the match rng: the same seed rolls the same power, and copies roll apart", () => {
    const rolled = (seed: string): unknown => {
      const state = game(seed);
      const [card] = inHand(state, heroicPower.id, "p1");
      finishSetup(sinkFor(state));
      return must(card, "a copy").memory[POWER_KEY];
    };
    const first = rolled("seeded");
    expect(HERO_POWER_NAMES).toContain(first);
    expect(rolled("seeded")).toBe(first);

    // Six copies in one game roll one at a time, so a game holds more than one power.
    const state = game("six-copies");
    const cards = inHand(state, heroicPower.id, "p1", 6);
    finishSetup(sinkFor(state));
    expect(new Set(cards.map((card) => card.memory[POWER_KEY])).size).toBeGreaterThan(1);
  });

  it("R43 a game begun with a Heroic Power in the deck holds it with a power once play starts (§2.1)", () => {
    // The decks are built against this file's catalog, so the game is created after registering it.
    setupCatalog();
    register();
    const deck = [heroicPower.id, ...vanillaDeck(DECK_SIZE - 1, 1)];
    let state = beginGame(createGame({ seed: "hp-begin", decks: [deck, vanillaDeck(DECK_SIZE, 21)] })).state;
    for (const player of ["p1", "p2"] as const) {
      state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
    }
    // Quickdraw put it in the opening hand (§6.2), and start of game rolled its power (R43).
    const card = must(state.players.p1.hand.find((entry) => entry.defId === heroicPower.id), "the Heroic Power in hand");
    const power = must(powerOf(card), "its power");
    expect(HERO_POWER_NAMES).toContain(power.name);
    // It costs (0) to play whatever it rolled; its X is the price of the power.
    expect(effectiveCost(state, card)).toBe(0);
    expect(powerCostOf(card)).toBe(HERO_POWER_COST[power.name]);
  });

  it("R151 a copy that arrives with no power rolls as it arrives: drawn, added to a hand, or summoned", () => {
    const state = bare("arrival");
    const sink = sinkFor(state);
    const ctx = makeContext(sink, null, { controller: "p1" });

    // A draw.
    const [top] = setLibrary(state, "p1", [heroicPower.id]);
    const drawn = must(top, "the library copy");
    expect(drawn.memory[POWER_KEY]).toBeUndefined();
    applyEffects([draw({ count: 1 })], ctx);
    expect(drawn.zone).toEqual({ z: "hand", player: "p1" });
    expect(HERO_POWER_NAMES).toContain(drawn.memory[POWER_KEY]);

    // A copy an effect creates in a hand.
    applyEffects([addToHand({ defId: heroicPower.id, player: "self" })], ctx);
    const created = must(state.players.p1.hand.at(-1), "the created copy");
    expect(created.defId).toBe(heroicPower.id);
    expect(created.id).not.toBe(drawn.id);
    expect(HERO_POWER_NAMES).toContain(created.memory[POWER_KEY]);

    // A copy summoned straight onto the field, which reaches neither a hand nor a library.
    applyEffects([summon({ defId: heroicPower.id })], ctx);
    const summoned = must(
      state.players.p1.backrow.find((card) => card?.defId === heroicPower.id),
      "the summoned copy",
    );
    const power = must(powerOf(summoned), "its power");
    expect(abilitiesOf(state, summoned).map((decl) => decl.id)).toEqual([power.name]);
  });

  it("R151 a bounced copy loses its power and its use with its memory (R78) and rolls again as it reaches the hand", () => {
    const state = bare("bounced");
    const sink = sinkFor(state);
    const card = powered(state, "burn");
    expect(activateAbility(sink, "p1", { type: "activate", instanceId: card.id })).toBeNull();
    expect(usedThisTurn(state, card)).toBe(true);

    applyEffects(
      [bounce({ target: { of: "chosen" } })],
      makeContext(sink, card, { controller: "p1", targets: [atCard(card)] }),
    );
    expect(card.zone).toEqual({ z: "hand", player: "p1" });
    expect(card.memory[ACTIVATIONS_MEMORY_KEY]).toBeUndefined();
    // R43: "one that ends up in a hand … with no power (a bounced or reset instance, R78) rolls as it arrives".
    expect(HERO_POWER_NAMES).toContain(card.memory[POWER_KEY]);
    expect(effectiveCost(state, card)).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Playing it, and activating the power (R43, R103, R384).
// ---------------------------------------------------------------------------

describe("Heroic Power: playing it and activating its power (R43, R103, R384)", () => {
  it("R43 costs (0) to play whatever it rolled, and playing it activates nothing", () => {
    const state = playing("play");
    const card = must(inHand(state, heroicPower.id, "p1")[0], "a Heroic Power in hand");
    expect(defOf(state, heroicPower.id).cost).toBe(0);
    for (const name of HERO_POWER_NAMES) {
      card.memory[POWER_KEY] = name;
      expect(effectiveCost(state, card)).toBe(0);
    }
    // Expedition Map, X 3: had the play activated it, a Unit would come out of the library.
    card.memory[POWER_KEY] = "recruit";
    setLibrary(state, "p1", [plain.id]);
    const mana = state.players.p1.mana.current;

    const { state: after, events } = act(state, { type: "play", instanceId: card.id, playerId: "p1" });
    expect(after.players.p1.mana.current).toBe(mana);
    expect(eventsOfType(events, "cardPlayed").map((event) => event.costPaid)).toEqual([0]);
    expect(eventsOfType(events, "activated")).toEqual([]);
    expect(activeUnitsOf(after, "p1")).toEqual([]);

    // On the field with the power it rolled and its use untouched, so it can be used this turn.
    const onField = cardIn(after, card.id);
    expect(onField.zone.z).toBe("field");
    expect(powerOf(onField)?.name).toBe("recruit");
    expect(onField.memory[ACTIVATIONS_MEMORY_KEY]).toBeUndefined();
    expect(whyCannotActivateAbility(after, "p1", card.id)).toBeNull();
    const used = act(after, use(onField)).state;
    expect(activeUnitsOf(used, "p1").map((entry) => entry.defId)).toEqual([plain.id]);
    expect(used.players.p1.mana.current).toBe(mana - HERO_POWER_COST.recruit);
  });

  it("R43 the activate action spends the power's X and counts one use, and activatePower is the same activation", () => {
    const state = playing("activate");
    const card = powered(state, "burn");
    expect(viewFor(state, "p1").you.hero.powers).toEqual([
      {
        instanceId: card.id,
        defId: heroicPower.id,
        name: "burn",
        ability: "burn",
        radiant: false,
        x: HERO_POWER_COST.burn,
        usedThisTurn: false,
        params: { [STEADY_SHOT_PARAM]: paramValue(state, card, STEADY_SHOT_PARAM) },
      },
    ]);

    const { state: after, events } = act(state, use(card));
    expect(after.players.p2.hero.health).toBe(HERO_HEALTH - paramValue(state, card, STEADY_SHOT_PARAM));
    expect(after.players.p1.mana.current).toBe(MAX_MANA - HERO_POWER_COST.burn);
    expect(eventsOfType(events, "activated")).toEqual([
      { type: "activated", player: "p1", instanceId: card.id, defId: heroicPower.id, ability: "burn" },
    ]);
    // R384: activating is not a play.
    expect(eventsOfType(events, "cardPlayed")).toEqual([]);
    expect(after.counters.played).toBe(state.counters.played);
    expect(cardIn(after, card.id).memory[ACTIVATIONS_MEMORY_KEY]).toEqual({ turn: state.turn, count: 1 });
    expect(viewFor(after, "p1").you.hero.powers).toEqual([
      {
        instanceId: card.id,
        defId: heroicPower.id,
        name: "burn",
        ability: "burn",
        radiant: false,
        x: HERO_POWER_COST.burn,
        usedThisTurn: true,
        params: { [STEADY_SHOT_PARAM]: paramValue(state, card, STEADY_SHOT_PARAM) },
      },
    ]);

    // The alias every old log carries is the same activation (reduce.ts routes it to `activateAbility`).
    const alias = act(state, { type: "activatePower", instanceId: card.id, playerId: "p1" });
    expect(hashState(alias.state)).toBe(hashState(after));

    // And it carries a declared target as `activate` does (R81).
    const pinging = playing("alias-ping");
    const pinger = powered(pinging, "ping");
    const pinged = act(pinging, { type: "activatePower", instanceId: pinger.id, targets: [atHero("p2")], playerId: "p1" }).state;
    expect(pinged.players.p2.hero.health).toBe(HERO_HEALTH - PING_DAMAGE);
  });

  it("R384 a power is used once per turn, and the controller's next turn is a fresh use", () => {
    const state = playing("once");
    const card = powered(state, "burn");
    const listedFor = (at: GameState): ActionBody[] =>
      legalActions(at, "p1").filter((body) => body.type === "activate" && body.instanceId === card.id);
    expect(listedFor(state)).toEqual([{ type: "activate", instanceId: card.id, ability: "burn" }]);

    const after = act(state, use(card)).state;
    expect(actResult(after, use(card)).error).toBe("that ability has already been used this turn");
    expect(actResult(after, { type: "activatePower", instanceId: card.id, playerId: "p1" }).error).toBe(
      "that ability has already been used this turn",
    );
    expect(listedFor(after)).toEqual([]);

    // The count is kept with the turn it was made on, so the next turn of p1's starts afresh.
    const later = nextOwnTurn(after);
    expect(usedThisTurn(later, cardIn(later, card.id))).toBe(false);
    expect(whyCannotActivateAbility(later, "p1", card.id)).toBeNull();
    const again = act(later, use(card)).state;
    expect(cardIn(again, card.id).memory[ACTIVATIONS_MEMORY_KEY]).toEqual({ turn: later.turn, count: 1 });
  });

  it("R384 refuses in the Activate order: game over, prompt, turn, phase, used, then mana", () => {
    const state = playing("order");
    const card = powered(state, "recruit");
    const x = HERO_POWER_COST.recruit;

    // Everything is wrong at once; each fix surfaces the next reason, in the order the player hears them.
    const all = roundTrip(state);
    const copy = cardIn(all, card.id);
    all.result = { winner: "p2", reason: "concede" };
    all.pending = {
      id: "q-test",
      playerId: "p1",
      kind: "target",
      prompt: "a question",
      options: [{ key: "none", label: "none", selection: { pick: "none" } }],
      min: 1,
      max: 1,
      resume: { defId: "", hook: "resume", step: "x", radiant: false, data: {} },
    };
    all.active = "p2";
    all.phase = "start";
    copy.memory[ACTIVATIONS_MEMORY_KEY] = { turn: all.turn, count: 1 };
    all.players.p1.mana.current = x - 1;
    const why = (): string | null => whyCannotActivateAbility(all, "p1", card.id);

    expect(why()).toBe("the game is over");
    all.result = null;
    expect(why()).toBe("answer the open prompt first");
    all.pending = null;
    expect(why()).toBe("it is not your turn");
    all.active = "p1";
    expect(why()).toBe("an ability is activated in the main phase");
    all.phase = "main";
    expect(why()).toBe("that ability has already been used this turn");
    delete copy.memory[ACTIVATIONS_MEMORY_KEY];
    expect(why()).toBe(`that ability costs ${x}, more than your mana`);
    all.players.p1.mana.current = x;
    expect(why()).toBeNull();

    // `reduce` refuses with the same reason and spends nothing.
    state.players.p1.mana.current = x - 1;
    const refused = actResult(state, use(card));
    expect(refused.error).toBe(`that ability costs ${x}, more than your mana`);
    expect(refused.state.players.p1.mana.current).toBe(x - 1);
  });

  it("R103 a copy with no rolled power, one not on its controller's field, or an ability it did not roll has nothing to activate", () => {
    const state = playing("nothing");
    const card = put(state, heroicPower.id, slot("p1", "backrow", 1));
    expect(whyCannotActivateAbility(state, "p1", card.id)).toBe("that card has no Activate ability");
    expect(legalActions(state, "p1").some((body) => body.type === "activate" && body.instanceId === card.id)).toBe(false);
    expect(viewFor(state, "p1").you.hero.powers).toEqual([]);
    expect(viewFor(state, "p1").you.backrow[0]).not.toHaveProperty("activations");

    card.memory[POWER_KEY] = "burn";
    expect(whyCannotActivateAbility(state, "p1", card.id, "ping")).toBe('that card has no ability "ping"');
    expect(actResult(state, { type: "activate", instanceId: card.id, ability: "ping", playerId: "p1" }).error).toBe(
      'that card has no ability "ping"',
    );
    expect(whyCannotActivateAbility(state, "p2", card.id)).toBe("that card is not yours");

    const held = must(inHand(state, heroicPower.id, "p1")[0], "a copy in hand");
    held.memory[POWER_KEY] = "burn";
    expect(whyCannotActivateAbility(state, "p1", held.id)).toBe("that card is not on the field");
  });

  it("R384 legalActions lists the rolled power's activate, and Ping once per unit or hero it may target", () => {
    const state = playing("listing");
    const card = powered(state, "ping");
    const mine = put(state, plain.id, slot("p1", "units", 1));
    const theirs = put(state, plain.id, slot("p2", "units", 1));

    const listed = legalActions(state, "p1").filter((body) => body.type === "activate" && body.instanceId === card.id);
    const targets = [atCard(mine), atCard(theirs), atHero("p1"), atHero("p2")];
    expect(listed).toHaveLength(targets.length);
    for (const target of targets) {
      expect(listed).toContainEqual({ type: "activate", instanceId: card.id, ability: "ping", targets: [target] });
    }
    // Every listed action is one the reducer accepts (§10.2: the list is the refusal).
    for (const body of listed) expect(actResult(state, { ...body, playerId: "p1" }).error).toBeUndefined();
    // R43: the alias is for old logs; the list offers `activate`.
    expect(legalActions(state, "p1").some((body) => body.type === "activatePower")).toBe(false);

    // Unaffordable, it is not listed.
    state.players.p1.mana.current = HERO_POWER_COST.ping - 1;
    expect(legalActions(state, "p1").some((body) => body.type === "activate" && body.instanceId === card.id)).toBe(false);
  });

  it("R103 marks the use before the effect runs, and a paused Witness Value survives a JSON round trip (§9.3)", () => {
    const state = playing("round-trip");
    const card = powered(state, "discover", { radiant: true });
    const paused = act(state, use(card)).state;

    expect(must(paused.pending, "the Discover").kind).toBe("discover");
    // The use and the mana are spent as the power is activated, so the answer cannot buy another.
    expect(usedThisTurn(paused, cardIn(paused, card.id))).toBe(true);
    expect(paused.players.p1.mana.current).toBe(MAX_MANA - HERO_POWER_COST.discover);

    // §9.3: the pause is plain data, so a copy through JSON finishes exactly as the live state does.
    const copy = roundTrip(paused);
    expect(copy).toEqual(paused);
    const live = answer(paused).state;
    const restored = answer(copy).state;
    expect(hashState(restored)).toBe(hashState(live));
    const pick = offered(must(paused.pending?.options[0], "an option"));
    expect(live.players.p1.hand.at(-1)?.defId).toBe(pick);
    expect(restored.players.p1.hand.at(-1)?.defId).toBe(pick);
    expect(whyCannotActivateAbility(live, "p1", card.id)).toBe("that ability has already been used this turn");
  });

  it("R43 a game that plays and activates a Heroic Power replays from its log to the same state (§9.3)", () => {
    const seed = "hp-replay";
    const decks: [string[], string[]] = [[heroicPower.id, ...vanillaDeck(DECK_SIZE - 1, 1)], vanillaDeck(DECK_SIZE, 21)];
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
    const held = must(state.players.p1.hand.find((card) => card.defId === heroicPower.id), "Quickdraw's opening-hand copy");
    step({ type: "play", instanceId: held.id, playerId: "p1" });

    // Turns pass until p1 has the mana for whatever it rolled (no X is above MAX_MANA).
    const offeredNow = (): ActionInput | undefined => {
      if (state.active !== "p1" || state.pending !== null) return undefined;
      const body = legalActions(state, "p1").find((entry) => entry.type === "activate" && entry.instanceId === held.id);
      return body === undefined ? undefined : { ...body, playerId: "p1" };
    };
    for (let guard = 0; offeredNow() === undefined; guard += 1) {
      if (guard > 2 * MAX_MANA) throw new Error("the power was never offered");
      step({ type: "endTurn", playerId: state.active });
    }
    step(must(offeredNow(), "the power's activate"));
    for (let guard = 0; state.pending !== null; guard += 1) {
      if (guard > 2) throw new Error("the power kept asking");
      const option = must(state.pending.options[0], "an option");
      step({ type: "answer", choiceId: state.pending.id, selection: [option.selection], playerId: seatToAct(state) });
    }
    expect(usedThisTurn(state, cardIn(state, held.id))).toBe(true);

    const replayed = fold({ seed, decks, log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
  });
});

// ---------------------------------------------------------------------------
// The powers, base and Radiant.
// ---------------------------------------------------------------------------

describe("Heroic Power: Expedition Map, Life Tap and Ping (R43, R664, R665)", () => {
  it("R43 Expedition Map recruits a permanent, and the Radiant face makes it Radiant", () => {
    for (const radiant of [false, true]) {
      const state = playing(`recruit-${radiant}`);
      const card = powered(state, "recruit", { radiant });
      setLibrary(state, "p1", [stockpile.id, plain.id]); // a Spell on top, then a Unit

      const { state: after, events } = act(state, use(card));
      // The Spell is passed over and stays; the permanent is summoned (§6.3 Recruit).
      expect(activeUnitsOf(after, "p1").map((entry) => entry.defId)).toEqual([plain.id]);
      expect(after.players.p1.library.map((entry) => entry.defId)).toEqual([stockpile.id]);
      expect(activeUnitsOf(after, "p1")[0]?.radiant).toBe(radiant);
      expect(eventsOfType(events, "radiantSet")).toHaveLength(radiant ? 1 : 0);
    }
  });

  it("R665 Life Tap draws 1, then deals 2 ordinary damage to its own hero", () => {
    const state = playing("life-tap");
    const card = powered(state, "draw");
    setLibrary(state, "p1", [plain.id, stockpile.id]);
    const hand = state.players.p1.hand.length;

    const { state: after, events } = act(state, use(card));
    expect(after.players.p1.hand).toHaveLength(hand + LIFE_TAP_DRAW);
    expect(after.players.p1.hand.at(-1)?.defId).toBe(plain.id);
    expect(after.players.p1.hero.health).toBe(HERO_HEALTH - LIFE_TAP_DAMAGE);
    // Damage through §4.4 from the card, not a loss of health (R18).
    expect(eventsOfType(events, "damage")).toEqual([
      { type: "damage", sourceId: card.id, targetId: "hero-p1", amount: LIFE_TAP_DAMAGE, combat: false },
    ]);
    expect(eventsOfType(events, "healthLost")).toEqual([]);
    const types = events.map((event) => event.type);
    expect(types.indexOf("drawn")).toBeLessThan(types.indexOf("damage"));

    // Ordinary damage, so Armor takes its share (§4.4 step 2).
    const armoredState = playing("life-tap-armor");
    const tapper = powered(armoredState, "draw");
    armoredState.players.p1.hero.armor = 1;
    const tapped = act(armoredState, use(tapper)).state;
    expect(tapped.players.p1.hero.health).toBe(HERO_HEALTH - (LIFE_TAP_DAMAGE - 1));
  });

  it("R665 Radiant Life Tap draws its own top card and the opponent's, which becomes its own, and deals no damage", () => {
    const state = playing("life-tap-radiant");
    const card = powered(state, "draw", { radiant: true });
    const [ownTop] = setLibrary(state, "p1", [plain.id, bigBody.id]);
    const [theirTop, theirNext] = setLibrary(state, "p2", [stockpile.id, bigBody.id]);
    const hand = state.players.p1.hand.length;

    const { state: after, events } = act(state, use(card));
    expect(after.players.p1.hand).toHaveLength(hand + 2 * LIFE_TAP_DRAW);
    expect(after.players.p1.hand.slice(-2).map((entry) => entry.id)).toEqual([ownTop?.id, theirTop?.id]);
    // R12: the card taken from their deck is the drawer's now.
    const taken = cardIn(after, must(theirTop, "their top card").id);
    expect(taken.owner).toBe("p1");
    expect(taken.controller).toBe("p1");
    expect(after.players.p2.library.map((entry) => entry.id)).toEqual([theirNext?.id]);
    expect(after.players.p1.hero.health).toBe(HERO_HEALTH);
    expect(eventsOfType(events, "damage")).toEqual([]);
  });

  it("R664 Ping deals 1 Pierce damage to the declared target, a unit or a hero on either side, and never asks", () => {
    const state = playing("ping");
    const card = powered(state, "ping");
    const target = put(state, armored.id, slot("p2", "units", 1));
    expect(unitView(state, target).armor).toBeGreaterThan(0);

    // Pierce: the unit's Armor takes nothing off (R346).
    const hitUnit = act(state, use(card, { targets: [atCard(target)] }));
    expect(cardIn(hitUnit.state, target.id).damage).toBe(PING_DAMAGE);
    expect(hitUnit.state.pending).toBeNull();
    expect(hitsBy(hitUnit.events, card)).toEqual([target.id]);

    // Nor does the hero's.
    state.players.p2.hero.armor = 5;
    const hitHero = act(state, use(card, { targets: [atHero("p2")] })).state;
    expect(hitHero.players.p2.hero.health).toBe(HERO_HEALTH - PING_DAMAGE);

    // Its own hero is a target too (§8 #98: any unit or hero).
    const hitOwn = act(state, use(card, { targets: [atHero("p1")] })).state;
    expect(hitOwn.players.p1.hero.health).toBe(HERO_HEALTH - PING_DAMAGE);

    // R103, R81: the target travels in the action; without one the activation is refused, not asked.
    const refused = actResult(state, use(card));
    expect(refused.error).toMatch(/target/);
    expect(refused.state.pending).toBeNull();
  });

  it("R664 Radiant Ping that kills a Unit summons a Ghoul Token for its player with the Unit's attack and max health", () => {
    const state = playing("ping-kill");
    const card = powered(state, "ping", { radiant: true });
    const victim = put(state, plain.id, slot("p2", "units", 2));
    // A buffed 3/3, so the Ghoul's stats are the destroyed event's (R89), not the printed ones.
    victim.buffs = { attack: 2, health: 1 };
    victim.damage = 3;
    const view = unitView(state, victim);
    expect(view.health).toBe(PING_DAMAGE);

    const { state: after, events } = act(state, use(card, { targets: [atCard(victim)] }));
    const destroyed = eventsOfType(events, "destroyed");
    expect(destroyed).toHaveLength(1);
    expect(destroyed[0]).toMatchObject({ instanceId: victim.id, killerId: card.id, attack: view.attack, maxHealth: view.maxHealth });

    const ghouls = activeUnitsOf(after, "p1");
    expect(ghouls.map((entry) => entry.defId)).toEqual([ghoulToken.id]);
    const ghoul = must(ghouls[0], "the Ghoul");
    expect(ghoul.controller).toBe("p1");
    expect(unitView(after, ghoul)).toMatchObject({ attack: view.attack, health: view.maxHealth, maxHealth: view.maxHealth });
    // The Ghoul follows the death (R664: the rider reads the hit's state check).
    const types = events.map((event) => event.type);
    expect(types.indexOf("destroyed")).toBeLessThan(types.lastIndexOf("summoned"));
  });

  it("R664 no Ghoul without a Ping kill: a Unit that survives, a hero, or the base face's kill", () => {
    const survives = playing("ping-survives");
    const radiantPing = powered(survives, "ping", { radiant: true });
    const sturdy = put(survives, plain.id, slot("p2", "units", 1));
    const hit = act(survives, use(radiantPing, { targets: [atCard(sturdy)] }));
    expect(cardIn(hit.state, sturdy.id).damage).toBe(PING_DAMAGE);
    expect(eventsOfType(hit.events, "summoned")).toEqual([]);

    const hero = act(survives, use(radiantPing, { targets: [atHero("p2")] }));
    expect(eventsOfType(hero.events, "summoned")).toEqual([]);

    const base = playing("ping-base-kill");
    const basePing = powered(base, "ping");
    const frail = put(base, plain.id, slot("p2", "units", 1));
    frail.damage = 2;
    const killed = act(base, use(basePing, { targets: [atCard(frail)] }));
    expect(eventsOfType(killed.events, "destroyed").map((event) => event.instanceId)).toEqual([frail.id]);
    expect(eventsOfType(killed.events, "summoned")).toEqual([]);
    expect(activeUnitsOf(killed.state, "p1")).toEqual([]);
  });

  it("R664 a Death prompt the Ping kill opens is answered before the Ghoul is made", () => {
    const state = playing("ping-death");
    registerCatalog({ ...registeredCatalog(), [mourner.id]: mourner, [logCard.id]: logCard });
    registerScripts({
      ...registeredScripts(),
      [mourner.id]: must(ACTIVATE_SCRIPTS[mourner.id], "the mourner's script"),
      [logCard.id]: must(ACTIVATE_SCRIPTS[logCard.id], "the log's script"),
    });
    put(state, logCard.id, slot("p2", "backrow", LOG_LANE));
    const card = powered(state, "ping", { radiant: true });
    // A 1/1 whose Death asks its controller something.
    const victim = put(state, mourner.id, slot("p2", "units", 1));

    const paused = act(state, use(card, { targets: [atCard(victim)] })).state;
    expect(notes(paused)).toEqual(["death"]);
    expect(paused.pending?.playerId).toBe("p2");
    expect(activeUnitsOf(paused, "p1")).toEqual([]);

    const done = answer(paused).state;
    expect(notes(done)).toEqual(["death", "mourned", "death:tail"]);
    expect(activeUnitsOf(done, "p1").map((entry) => entry.defId)).toEqual([ghoulToken.id]);
    expect(unitView(done, must(activeUnitsOf(done, "p1")[0], "the Ghoul"))).toMatchObject({ attack: 1, maxHealth: 1 });
  });
});

describe("Heroic Power: Steady Shot, Ranching, Cat Cafe (R103, R666)", () => {
  it("R666 Steady Shot deals its shot, 2 on the base face, to the enemy hero, the same each turn", () => {
    let state = playing("steady");
    const card = powered(state, "burn");
    const shot = paramValue(state, card, STEADY_SHOT_PARAM);
    expect(shot).toBe(2);

    const first = act(state, use(card));
    expect(hitsBy(first.events, card)).toEqual(["hero-p2"]);
    expect(eventsOfType(first.events, "upgraded")).toEqual([]);
    state = nextOwnTurn(first.state);
    state = act(state, use(card)).state;
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 2 * shot);
    expect(cardIn(state, card.id).tuning).toBeUndefined();
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH);
  });

  it("R666 Radiant Steady Shot upgrades its shot by 2 permanently after each hit: 4, then 6, then 8", () => {
    let state = playing("steady-radiant");
    const card = powered(state, "burn", { radiant: true });
    // SPEC R666: 4 on the Radiant face, and each activation adds 2 after it hits.
    const shots = [4, 6, 8];
    let health = HERO_HEALTH;
    for (const [turn, shot] of shots.entries()) {
      if (turn > 0) state = nextOwnTurn(state);
      expect(paramValue(state, cardIn(state, card.id), STEADY_SHOT_PARAM)).toBe(shot);
      const { state: after, events } = act(state, use(card));
      health -= shot;
      expect(after.players.p2.hero.health).toBe(health);
      expect(eventsOfType(events, "upgraded")).toEqual([
        {
          type: "upgraded",
          instanceId: card.id,
          defId: heroicPower.id,
          change: { kind: "number", key: STEADY_SHOT_PARAM, delta: 2 },
        },
      ]);
      // The hit reads the number before the Upgrade moves it.
      const types = events.map((event) => event.type);
      expect(types.indexOf("damage")).toBeLessThan(types.indexOf("upgraded"));
      state = after;
    }
    // Permanent: kept in the card's tuning, so it holds in a hand too (R386).
    const held = cardIn(state, card.id);
    moveToZone(state, held, "hand");
    expect(paramValue(state, held, STEADY_SHOT_PARAM)).toBe(10);
  });

  it("R103 Ranching summons a Rush Token, and a Radiant one on the Radiant face (§7)", () => {
    const rushToken = must(defByIndex("Core", RUSH_TOKEN_INDEX), "the fixture Rush Token");
    for (const radiant of [false, true]) {
      const state = playing(`ranching-${radiant}`);
      const card = powered(state, "rush", { radiant });
      const after = act(state, use(card)).state;
      const units = activeUnitsOf(after, "p1");
      expect(units.map((entry) => entry.defId)).toEqual([rushToken.id]);
      const summoned = must(units[0], "the token");
      expect(summoned.radiant).toBe(radiant);
      const face = radiant ? rushToken.radiant : rushToken.base;
      expect(unitView(after, summoned).attack).toBe(face.attack);
      expect(unitView(after, summoned).keywords.map((keyword) => keyword.kind)).toContain("Rush");
    }
  });

  it("R103 Cat Cafe summons a Felinor Token; its Radiant face a random non-token Felinor Unit of any set", () => {
    const state = playing("cat-cafe");
    const card = powered(state, "felinor");
    const after = act(state, use(card)).state;
    expect(activeUnitsOf(after, "p1").map((entry) => entry.defId)).toEqual([felinorToken.id]);

    // The pool is every set's Felinor Units (R380) less the tokens (§5.1): here, the one.
    const radiantState = playing("cat-cafe-radiant");
    const radiantCard = powered(radiantState, "felinor", { radiant: true });
    const radiantAfter = act(radiantState, use(radiantCard)).state;
    const units = activeUnitsOf(radiantAfter, "p1");
    expect(units.map((entry) => entry.defId)).toEqual([felinor.id]);
    // The text does not say Radiant, so the Felinor comes on its base face.
    expect(units[0]?.radiant).toBe(false);
  });

  it("R103 a token power whose token the catalog lacks fizzles in silence: Cat Cafe, and Radiant Ping's Ghoul", () => {
    const state = playing("missing-tokens");
    registerCatalog(
      Object.fromEntries(
        Object.entries(registeredCatalog()).filter(([, entry]) => entry.index !== FELINOR_TOKEN_INDEX && entry.index !== GHOUL_TOKEN_INDEX),
      ),
    );
    const cafe = powered(state, "felinor");
    const ping = powered(state, "ping", { radiant: true, lane: 2 });
    const victim = put(state, plain.id, slot("p2", "units", 1));
    victim.damage = 2;

    const cats = act(state, use(cafe));
    expect(eventsOfType(cats.events, "summoned")).toEqual([]);
    expect(cats.state.pending).toBeNull();

    const killed = act(cats.state, use(ping, { targets: [atCard(victim)] }));
    expect(eventsOfType(killed.events, "destroyed").map((event) => event.instanceId)).toEqual([victim.id]);
    expect(eventsOfType(killed.events, "summoned")).toEqual([]);
    expect(activeUnitsOf(killed.state, "p1")).toEqual([]);
  });
});

describe("Heroic Power: Witness Value and Stitching (R103, R352)", () => {
  it("R103 Witness Value Discovers a Unit to hand, Radiant when the power is", () => {
    for (const radiant of [false, true]) {
      const state = playing(`witness-${radiant}`);
      const card = powered(state, "discover", { radiant });
      const hand = state.players.p1.hand.length;

      const paused = act(state, use(card)).state;
      const pending = must(paused.pending, "the Discover");
      expect(pending.kind).toBe("discover");
      expect(pending.playerId).toBe("p1");
      expect(pending.options).toHaveLength(3);
      for (const option of pending.options) {
        const picked = defOf(paused, offered(option));
        expect(picked.type).toBe("Unit");
        expect(picked.token).toBe(false);
      }
      const pick = offered(must(pending.options[0], "an option"));

      const after = answer(paused).state;
      expect(after.players.p1.hand).toHaveLength(hand + 1);
      const added = must(after.players.p1.hand.at(-1), "the discovered card");
      expect(added.defId).toBe(pick);
      expect(added.radiant).toBe(radiant);
      expect(after.pending).toBeNull();
    }
  });

  it("R352 Stitching Discovers two Units that cost (2) or less and fuses them to hand at R77's cost, Radiant on the Radiant face", () => {
    for (const radiant of [false, true]) {
      const state = playing(`stitching-${radiant}`);
      const card = powered(state, "stitching", { radiant });
      const hand = state.players.p1.hand.length;

      let step = act(state, use(card));
      const picks: string[] = [];
      const prompts: string[] = [];
      for (let at = 0; at < 2; at += 1) {
        const pending = must(step.state.pending, `Discover ${at + 1}`);
        expect(pending.kind).toBe("discover");
        expect(pending.playerId).toBe("p1");
        prompts.push(pending.id);
        for (const option of pending.options) {
          const picked = defOf(step.state, offered(option));
          expect(picked.type).toBe("Unit");
          expect(picked.token).toBe(false);
          expect(queryCost(picked)).toBeLessThanOrEqual(STITCHING_MAX_COST);
        }
        // Nothing reaches the hand until both are picked.
        expect(step.state.players.p1.hand).toHaveLength(hand);
        picks.push(offered(must(pending.options[0], "an option")));
        step = answer(step.state);
      }
      expect(new Set(prompts).size).toBe(2);

      const after = step.state;
      expect(after.pending).toBeNull();
      expect(after.players.p1.hand).toHaveLength(hand + 1);
      const fused = must(after.players.p1.hand.at(-1), "the fused card");
      expect(fusedIdParts(fused.defId)).toEqual(picks);
      // R77's fused cost, min(sum, 4): the power names no price of its own.
      const sum = picks.reduce((total, defId) => total + queryCost(defOf(after, defId)), 0);
      expect(effectiveCost(after, fused)).toBe(Math.min(sum, FUSE_COST_CAP));
      expect(fused.radiant).toBe(radiant);
      // One activation: one use, one price.
      expect(after.players.p1.mana.current).toBe(MAX_MANA - HERO_POWER_COST.stitching);
      expect(cardIn(after, card.id).memory[ACTIVATIONS_MEMORY_KEY]).toEqual({ turn: state.turn, count: 1 });
    }
  });
});

describe("Heroic Power: Armor Up and Tank Up (R661, R662)", () => {
  it("R661 Armor Up gives its hero 2 Armor until the start of its controller's next turn, held through the opponent's", () => {
    let state = playing("armor-up");
    const card = powered(state, "armor");
    state = act(state, use(card)).state;
    expect(heroArmorOf(state, "p1")).toBe(ARMOR_UP.base);
    // A modifier on the player, never the Armor written on the hero.
    expect(state.players.p1.hero.armor).toBe(0);
    const mod = must(state.players.p1.mods.find((entry) => entry.kind === "heroArmor"), "the Armor Up modifier");
    expect(viewFor(state, "p1").you.hero.armor).toBe(ARMOR_UP.base);

    // The opponent's turn: still held, and it applies to ordinary damage.
    state = act(state, { type: "endTurn", playerId: "p1" }).state;
    expect(state.active).toBe("p2");
    expect(heroArmorOf(state, "p1")).toBe(ARMOR_UP.base);
    const hit = ARMOR_UP.base + 1;
    dealDamage(sinkFor(state), { source: null, target: { kind: "hero", player: "p1" }, amount: hit });
    expect(state.players.p1.hero.health).toBe(HERO_HEALTH - (hit - ARMOR_UP.base));

    // The start of p1's next turn ends it, before that turn's mana refresh.
    const { state: next, events } = act(state, { type: "endTurn", playerId: "p2" });
    expect(next.active).toBe("p1");
    expect(heroArmorOf(next, "p1")).toBe(0);
    expect(next.players.p1.mods.some((entry) => entry.kind === "heroArmor")).toBe(false);
    const started = events.findIndex((event) => event.type === "turnStarted" && event.player === "p1");
    const removed = events.findIndex(
      (event) => event.type === "modifierChanged" && event.player === "p1" && event.modifierId === mod.id && !event.added,
    );
    const refreshed = events.findIndex((event, at) => at > started && event.type === "manaChanged" && event.player === "p1");
    expect(started).toBeGreaterThanOrEqual(0);
    expect(removed).toBeGreaterThan(started);
    expect(removed).toBeLessThan(refreshed);
  });

  it("R661 Tank Up's 4 Armor is the hero's for the rest of the game, past the start of its next turn", () => {
    let state = playing("tank-up-armor");
    const card = powered(state, "armor", { radiant: true });
    state = act(state, use(card)).state;
    expect(state.players.p1.hero.armor).toBe(ARMOR_UP.radiant);
    expect(state.players.p1.mods.some((entry) => entry.kind === "heroArmor")).toBe(false);
    state = nextOwnTurn(state);
    expect(heroArmorOf(state, "p1")).toBe(ARMOR_UP.radiant);
    state = nextOwnTurn(state);
    expect(heroArmorOf(state, "p1")).toBe(ARMOR_UP.radiant);
  });

  it("R662 Tank Up refreshes the power into one of the other twelve, uniformly from the match rng", () => {
    const others = HERO_POWER_NAMES.filter((name) => name !== "armor");
    const seen = new Set<string>();
    for (let seed = 0; seed < 8; seed += 1) {
      const state = bare(`tank-up-${seed}`);
      const card = powered(state, "armor", { radiant: true });
      // The Armor draws nothing, so the refresh is the activation's first draw.
      const predicted = createRng(state.seed, state.rngCursor).pick(others);
      expect(activateAbility(sinkFor(state), "p1", { type: "activate", instanceId: card.id })).toBeNull();
      expect(card.memory[POWER_KEY]).toBe(predicted);
      expect(card.memory[POWER_KEY]).not.toBe("armor");
      seen.add(String(card.memory[POWER_KEY]));
    }
    expect(seen.size).toBeGreaterThan(1);
  });

  it("R662 after a refresh the use stays spent, and the new power is the card's to use next turn", () => {
    const state = playing("tank-up-refresh");
    const card = powered(state, "armor", { radiant: true });
    const after = act(state, use(card)).state;
    const refreshed = cardIn(after, card.id);
    const name = must(powerOf(refreshed), "the new power").name;
    expect(name).not.toBe("armor");
    expect(abilitiesOf(after, refreshed).map((decl) => decl.id)).toEqual([name]);

    // The Activate count is the card's, whichever ability spent it (R384).
    expect(usedThisTurn(after, refreshed)).toBe(true);
    expect(whyCannotActivateAbility(after, "p1", card.id, name)).toBe("that ability has already been used this turn");
    expect(legalActions(after, "p1").some((body) => body.type === "activate" && body.instanceId === card.id)).toBe(false);

    const next = nextOwnTurn(after);
    expect(powerOf(cardIn(next, card.id))?.name).toBe(name);
    expect(whyCannotActivateAbility(next, "p1", card.id, name)).toBeNull();
    expect(
      legalActions(next, "p1").some((body) => body.type === "activate" && body.instanceId === card.id && body.ability === name),
    ).toBe(true);
  });
});

describe("Heroic Power: Die Insect (R663)", () => {
  type InsectPick = { kind: "hero" } | { kind: "unit"; unit: CardInstance };

  /** R663's pool: the enemy Units acting on the field, in board order, then the enemy hero. */
  function insectPool(state: GameState): InsectPick[] {
    return [...activeUnitsOf(state, "p2").map((entry): InsectPick => ({ kind: "unit", unit: entry })), { kind: "hero" }];
  }

  function targetOf(pick: InsectPick): string {
    return pick.kind === "hero" ? "hero-p2" : pick.unit.id;
  }

  /** R663's order, written from the row: lethal hero, a Unit the hit destroys, a non-lethal hero, a surviving Unit. */
  function rank(state: GameState, pick: InsectPick): number {
    if (pick.kind === "hero") return DIE_INSECT_DAMAGE >= state.players.p2.hero.health ? 3 : 1;
    return DIE_INSECT_DAMAGE >= unitView(state, pick.unit).health ? 2 : 0;
  }

  /** R663: the better of two picks; two Units of one rank by attack plus health (the scenarios need no deeper tie-break). */
  function better(state: GameState, a: InsectPick, b: InsectPick): InsectPick {
    if (rank(state, a) !== rank(state, b)) return rank(state, a) > rank(state, b) ? a : b;
    if (a.kind !== "unit" || b.kind !== "unit") return a;
    const worth = (pick: CardInstance): number => unitView(state, pick).attack + unitView(state, pick).health;
    return worth(b.unit) > worth(a.unit) ? b : a;
  }

  type Scenario = { name: string; heroHealth: number; units: string[] };
  const SCENARIOS: Scenario[] = [
    { name: "lethal hero over a Unit it destroys", heroHealth: DIE_INSECT_DAMAGE, units: [plain.id] },
    { name: "a destroyed Unit over the hero, the bigger of two", heroHealth: HERO_HEALTH, units: [plain.id, "fx-1"] },
    { name: "the hero over a Unit that survives", heroHealth: HERO_HEALTH, units: [bigBody.id] },
  ];

  function insectGame(seed: string, scenario: Scenario, radiant: boolean): { state: GameState; card: CardInstance } {
    const state = bare(seed);
    const card = powered(state, "insect", { radiant });
    state.players.p2.hero.health = scenario.heroHealth;
    scenario.units.forEach((defId, at) => put(state, defId, slot("p2", "units", at + 1)));
    return { state, card };
  }

  it("R663 Die Insect deals 8 damage to one pick, uniform over the enemy hero and the enemy Units", () => {
    const scenario = SCENARIOS[1] as Scenario;
    const reached = new Set<string>();
    for (let seed = 0; seed < 12; seed += 1) {
      const { state, card } = insectGame(`insect-${seed}`, scenario, false);
      const pool = insectPool(state);
      const predicted = must(createRng(state.seed, state.rngCursor).pick(pool), "a pick");
      const events: GameEvent[] = [];
      expect(activateAbility(sinkFor(state, events), "p1", { type: "activate", instanceId: card.id })).toBeNull();
      expect(hitsBy(events, card)).toEqual([targetOf(predicted)]);
      expect(eventsOfType(events, "damage").find((event) => event.sourceId === card.id)?.amount).toBeGreaterThan(0);
      reached.add(targetOf(predicted));
    }
    // Every member of the pool is reachable.
    expect(reached.size).toBe(scenario.units.length + 1);

    // With no enemy Unit, the hero takes all 8.
    const alone = bare("insect-alone");
    const card = powered(alone, "insect");
    expect(activateAbility(sinkFor(alone), "p1", { type: "activate", instanceId: card.id })).toBeNull();
    expect(alone.players.p2.hero.health).toBe(HERO_HEALTH - DIE_INSECT_DAMAGE);
  });

  it("R663 Radiant Die Insect's Lucky 1 draws a second pick and keeps the better: lethal hero, destroyed Unit, hero, surviving Unit", () => {
    for (const scenario of SCENARIOS) {
      let luckyChanged = 0;
      for (let seed = 0; seed < 24; seed += 1) {
        const { state, card } = insectGame(`lucky-${seed}`, scenario, true);
        const rng = createRng(state.seed, state.rngCursor);
        const pool = insectPool(state);
        const first = must(rng.pick(pool), "the first pick");
        const second = must(rng.pick(pool), "the second pick");
        const kept = better(state, first, second);
        if (kept !== first) luckyChanged += 1;

        const events: GameEvent[] = [];
        expect(activateAbility(sinkFor(state, events), "p1", { type: "activate", instanceId: card.id })).toBeNull();
        expect(hitsBy(events, card), `${scenario.name}, seed ${seed}`).toEqual([targetOf(kept)]);
        if (kept.kind === "hero" && scenario.heroHealth <= DIE_INSECT_DAMAGE) {
          expect(state.result?.winner).toBe("p1");
        }
      }
      // Each scenario meets a seed where the second pick beats the first, so Lucky is seen to choose.
      expect(luckyChanged, scenario.name).toBeGreaterThan(0);
    }
  });
});

describe("Heroic Power: KY Brainstorm, Pluck and Terminus Tricks (R667, R668)", () => {
  it("R668 KY Brainstorm adds a random KY card, then discounts every non-X Spell in hand by 1, the new one included", () => {
    for (const radiant of [false, true]) {
      const state = playing(`brainstorm-${radiant}`);
      const card = powered(state, "brainstorm", { radiant });
      state.players.p1.hand = [];
      const [spell] = inHand(state, cheapSpell.id, "p1");
      const [xSpell] = inHand(state, xBolt.id, "p1");
      const [body] = inHand(state, plain.id, "p1");

      const after = act(state, use(card)).state;
      const hand = after.players.p1.hand;
      expect(hand.map((entry) => entry.defId)).toEqual([cheapSpell.id, xBolt.id, plain.id, kySpell.id]);
      const ky = must(hand[3], "the KY card");
      expect(ky.radiant).toBe(radiant);
      expect(effectiveCost(after, ky)).toBe(KY_SPELL_COST - KY_BRAINSTORM_DISCOUNT);
      expect(effectiveCost(after, cardIn(after, must(spell, "the Spell").id))).toBe(CHEAP_SPELL_COST - KY_BRAINSTORM_DISCOUNT);
      // An X-cost Spell is out of a modifier's reach (R65), and a Unit is no Spell.
      expect(cardIn(after, must(xSpell, "the X Spell").id).costMod).toBe(0);
      expect(cardIn(after, must(body, "the Unit").id).costMod).toBe(0);
    }
  });

  it("R668 Pluck adds a random Fruit, the Grape tokens included (R382), which costs (0), Radiant on the Radiant face", () => {
    const seen = new Set<string>();
    for (let seed = 0; seed < 12; seed += 1) {
      const radiant = seed % 2 === 1;
      const state = bare(`pluck-${seed}`);
      const card = powered(state, "pluck", { radiant });
      const hand = state.players.p1.hand.length;
      expect(activateAbility(sinkFor(state), "p1", { type: "activate", instanceId: card.id })).toBeNull();
      expect(state.players.p1.hand).toHaveLength(hand + 1);
      const added = must(state.players.p1.hand.at(-1), "the Fruit");
      expect(defOf(state, added.defId).tags).toContain("Fruit");
      expect(added.radiant).toBe(radiant);
      expect(effectiveCost(state, added)).toBe(PLUCK_COST);
      seen.add(added.defId);
    }
    expect([...seen].sort()).toEqual([fruit.id, grape.id].sort());
  });

  it("R667 Terminus Tricks Discovers a Trap or Field Trap of any set and summons the pick face-down into its backrow", () => {
    for (const [radiant, pickId] of [
      [false, trap.id],
      [true, fieldTrap.id],
    ] as const) {
      const state = playing(`terminus-${radiant}`);
      const card = powered(state, "terminus", { radiant });
      const paused = act(state, use(card)).state;
      const pending = must(paused.pending, "the Discover");
      expect(pending.kind).toBe("discover");
      expect(pending.playerId).toBe("p1");
      // The fixture catalog's only Traps: one of Classic, one Field Trap of Classic+ (R380).
      expect(pending.options.map(offered).sort()).toEqual([fieldTrap.id, trap.id].sort());

      const { state: after, events } = answer(paused, (option) => offered(option) === pickId);
      // R64's leftmost open zone: the Heroic Power holds lane 1.
      const placed = must(after.players.p1.backrow[1], "the summoned Trap");
      expect(placed.defId).toBe(pickId);
      expect(isFaceDown(after, placed)).toBe(true);
      expect(placed.radiant).toBe(radiant);
      expect(eventsOfType(events, "summoned")).toMatchObject([{ player: "p1", instanceId: placed.id, row: "backrow", lane: 2 }]);
      expect(after.players.p1.hand.some((entry) => entry.defId === pickId)).toBe(false);
    }
  });

  it("R667 Terminus Tricks with no open backrow zone summons nothing and the activation still resolves", () => {
    const state = playing("terminus-full");
    const card = powered(state, "terminus");
    for (let lane = 2; lane <= 5; lane += 1) put(state, filler.id, slot("p1", "backrow", lane));
    const before = state.players.p1.backrow.map((entry) => entry?.id);

    const paused = act(state, use(card)).state;
    const { state: after, events } = answer(paused);
    expect(after.pending).toBeNull();
    expect(eventsOfType(events, "summoned")).toEqual([]);
    expect(after.players.p1.backrow.map((entry) => entry?.id)).toEqual(before);
    expect(usedThisTurn(after, cardIn(after, card.id))).toBe(true);
  });
});
