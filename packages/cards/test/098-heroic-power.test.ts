// #98 Heroic Power, patch v0.2.1 — SPEC §8 #98, §6.2 ("Start of Game", Activate, Quickdraw), §10.6,
// §10.8, R43, R46, R103, R151, R352, R380, R384, R654–R661.
//
// BUILD M4-T4 row 98, as patch v0.2.1 reads it: in the opening hand (Quickdraw); a power rolled from
// the match rng, by a copy at the start of the game and by one that arrives later (R43, R151); the
// card costs (0) to play and its play activates nothing; its one power is its one Activate ability,
// "Activate: Spend (X)", once per turn through `activate`, and an old log's `activatePower` replays
// the same (R384); Indestructible; each of the thirteen powers on its base and its Radiant face;
// Discover and Stitching reach every set (R380); Stitching's two Discovers fused into a hand card at
// the fused cost, Radiant on the Radiant face (R352).
//
// WHAT IS COVERED, in two blocks:
//   base     the card (data, wiring, Indestructible), the (0) play that activates nothing, the roll
//            as a card arrives and the hand view that names it, the power as an Activate ability
//            (the X paid, once per turn, the refusals and their order, the old alias), then the
//            thirteen powers' base clauses: Expedition Map, Life Tap, Steady Shot, Ranching, Cat
//            Cafe, Ping, Witness Value, Stitching, Armor Up, Die Insect, KY Brainstorm, Pluck,
//            Terminus Tricks.
//   radiant  the same play and Activate on the Radiant face, then each power's Radiant clause:
//            Recruit made Radiant, Life Tap from both decks with no damage, Steady Shot's permanent
//            +2, the Radiant Rush Token, a random Felinor, Ping's Ghoul, Radiant Discovers and
//            Stitching, Tank Up's lasting Armor and its refresh, Die Insect's Lucky ordering, and
//            the Radiant KY card, Fruit and Trap.
//
// HOW A POWER IS PINNED. R103 makes the power names state, so a test that wants a named one writes
// that name into `memory.power` — exactly and only what `subsystems.ensurePower` writes, so the state
// is one the engine produces. Each power's X is read from `HERO_POWER_COST` (config, CLAUDE.md rule
// 9), and every assertion is about what the card did. A random pool (Cat Cafe's Radiant Felinor, KY
// Brainstorm, Pluck, Die Insect's pick) is asserted by membership, never by a named card.
//
// What the harness cannot reach: `scenario()` skips §2.1, so the start-of-game roll itself is
// `setup.finishSetup`'s, covered by the engine's own tests. The card's half — the `startOfGame` hook
// and the same roll run as a copy arrives (R151) — is covered here through a draw.

import { describe, expect, it } from "vitest";
import type { Action, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { HERO_POWER_COST, STEADY_SHOT_STEP_DAMAGE, effectiveCost, legalActions, reduce, subsystems } from "@jackioh/engine";
import type { CardInstance } from "@jackioh/engine";
import { cardDef } from "../src/catalog-data";
import { query } from "../src/query";
import { base as hpBase, def, radiant as hpRadiant } from "../src/scripts/098-heroic-power";
import { scenario, type Scenario, type SideSetup } from "./_harness";

const HEROIC = "core-098"; // Field Spell, (0), Mythic, Quickdraw, Indestructible
const RUSH_TOKEN = "core-t-rush";
const FELINOR_TOKEN = "core-t-felinor";
const GHOUL_TOKEN = "core-t-ghoul";

/** #53 Reno, a 3-cost Unit: the spare card that keeps §2.5's auto-end away from the assertions. */
const SPARE = "core-053";
/** #93.1 Combo-Fodder, a (0) Spell token: a spare that is still playable with no mana left. */
const FREE = "core-093-1";
/** #36 Magic Jammed, a (1) Spell that destroys a chosen backrow card (R46's test). */
const JAMMED = "core-036";
/** #19 Midrange Menace, a (3) 9/9 Unit (18/18 Radiant) — the permanent Recruit finds. */
const MENACE = "core-019";
/** #1 Big D-fender, 0/7: "Your Units in Defense Position have +2 Armor", itself included. */
const DFENDER = "core-001";
/** #5 Stockpile, a (1) Spell — what KY Brainstorm discounts. */
const STOCKPILE = "core-005";
/** #24 Efficiency Dividend, an (X) Spell — what no cost modifier reaches (R65). */
const DIVIDEND = "core-024";

type PowerName = keyof typeof HERO_POWER_COST;

/** The thirteen stored names, in the order the roll draws from (R103). */
const NAMES: readonly PowerName[] = [
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
];

const USED = /that ability has already been used this turn/;

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

function eventsOf<T extends GameEvent["type"]>(s: Scenario, type: T): Extract<GameEvent, { type: T }>[] {
  return s.events.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);
}

function unitsOf(s: Scenario, player: PlayerId): CardInstance[] {
  return [1, 2, 3, 4, 5].flatMap((lane) => {
    const found = s.unit(player, lane);
    return found === null ? [] : [found];
  });
}

function setPower(card: CardInstance, name: PowerName): CardInstance {
  card.memory[subsystems.POWER_KEY] = name;
  return card;
}

function pick(card: CardInstance): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

const ENEMY_HERO: Selection[] = [{ pick: "hero", player: "p2" }];

type Opts = { radiantFace?: boolean; seed?: string; p1?: SideSetup; p2?: SideSetup };

/** A #98 in p1's backrow lane 1 with a named power, 8 mana and a spare card in hand. */
function onField(name: PowerName, opts: Opts = {}): { s: Scenario; power: CardInstance } {
  const s = scenario({
    seed: opts.seed ?? `hp-${name}`,
    p1: {
      hand: [SPARE],
      mana: 8,
      ...(opts.p1 ?? {}),
      backrow: [{ def: HEROIC, radiant: opts.radiantFace === true }, ...(opts.p1?.backrow ?? [])],
    },
    ...(opts.p2 === undefined ? {} : { p2: opts.p2 }),
  });
  return { s, power: setPower(must(s.backrow("p1", 1), "the Heroic Power"), name) };
}

/** A #98 first in p1's hand with a named power (none for `null`), and a spare card beside it. */
function inHand(name: PowerName | null, opts: Opts = {}): { s: Scenario; power: CardInstance } {
  const s = scenario({
    seed: opts.seed ?? `hp-hand-${String(name)}`,
    p1: {
      mana: 8,
      ...(opts.p1 ?? {}),
      hand: [{ def: HEROIC, radiant: opts.radiantFace === true }, SPARE, ...(opts.p1?.hand ?? [])],
    },
    ...(opts.p2 === undefined ? {} : { p2: opts.p2 }),
  });
  const card = must(s.hand("p1")[0], "the Heroic Power in hand");
  return { s, power: name === null ? card : setPower(card, name) };
}

/** The def ids a Discover prompt offers (§6.3), in the order offered. */
function offeredIds(s: Scenario): string[] {
  return must(s.state.pending, "a discover prompt").options.flatMap((option) =>
    option.selection.pick === "mode" ? [option.selection.option] : [],
  );
}

/** Activate Stitching and answer both Discovers with their first option; returns the two picks. */
function stitch(s: Scenario, power: CardInstance): [string, string] {
  s.activate(power);
  const first = must(offeredIds(s)[0], "a first Unit");
  s.answer(first);
  const second = must(offeredIds(s)[0], "a second Unit");
  s.answer(second);
  return [first, second];
}

/** The hand cards that arrived since `before` (by instance id). */
function newInHand(s: Scenario, before: readonly CardInstance[]): CardInstance[] {
  const had = new Set(before.map((card) => card.id));
  return s.hand("p1").filter((card) => !had.has(card.id));
}

/** The §5.1 pools the random powers draw from, read through the one catalog query (R380, R382). */
const felinorUnits = (): string[] => query({ type: "Unit", tags: ["Felinor"] }).map((card) => card.id);
const kyCards = (): string[] => query({ tags: ["KY"] }).map((card) => card.id);
const fruitCards = (): string[] => query({ tags: ["Fruit"] }).map((card) => card.id);

/** R656: what Die Insect hit in one game — the enemy hero, an enemy Unit, or something else. */
function insectHit(s: Scenario, enemyUnitIds: readonly string[]): "hero" | "unit" {
  const hits = eventsOf(s, "damage");
  expect(hits).toHaveLength(1);
  const hit = must(hits[0], "Die Insect's hit");
  if (hit.targetId === "hero-p2") return "hero";
  expect(enemyUnitIds).toContain(hit.targetId);
  return "unit";
}

// ===========================================================================
// Base
// ===========================================================================

describe("#98 Heroic Power — base", () => {
  // --- the card -------------------------------------------------------------

  it("R43 a Mythic Field Spell tagged Quickdraw that costs (0), Indestructible on both faces", () => {
    expect(def.id).toBe(HEROIC);
    expect(def.type).toBe("Field Spell");
    expect(def.cost).toBe(0);
    expect(def.rarity).toBe("Mythic");
    expect(def.tags).toContain("Quickdraw");
    expect(def.refs).toEqual([RUSH_TOKEN, FELINOR_TOKEN, GHOUL_TOKEN]);
    for (const face of [def.base, def.radiant]) {
      expect(face.keywords.map((keyword) => keyword.kind)).toEqual(["Indestructible"]);
    }
  });

  it("R43 both faces wire Quickdraw, the start-of-game roll, the thirteen Activate abilities and the Discovers' resume — no cost hook, no Cry", () => {
    for (const script of [hpBase, hpRadiant]) {
      expect(Object.keys(script).sort()).toEqual(["activations", "resume", "startOfGame", "staticFlags"]);
      // §6.2 Quickdraw: the flag `setup.ts` step 2 reads to put it in the opening hand.
      expect(script.staticFlags?.quickdraw).toBe(true);
      expect(Object.keys(script.resume ?? {})).toEqual([subsystems.POWER_RESUME]);
      const abilities = script.activations ?? [];
      expect(abilities.map((ability) => ability.id)).toEqual(NAMES);
      for (const ability of abilities) {
        expect(ability.uses).toBe(1);
        expect(ability.cost?.mana).toBe(HERO_POWER_COST[ability.id as PowerName]);
      }
    }
  });

  it("R103 the thirteen stored names, each priced at the (X) the card prints beside it", () => {
    expect(subsystems.HERO_POWER_NAMES).toEqual(NAMES);
    expect(Object.keys(HERO_POWER_COST).sort()).toEqual([...NAMES].sort());
    for (const power of subsystems.HERO_POWERS) {
      const x = HERO_POWER_COST[power.name];
      expect(power.x).toBe(x);
      expect(def.base.text).toContain(`(${x}) ${power.title}: `);
      expect(def.radiant.text).toContain(`(${x}) ${power.radiantTitle}: `);
    }
    // Armor Up's Radiant face is printed as Tank Up.
    expect(subsystems.powerByName("armor")?.radiantTitle).toBe("Tank Up");
  });

  it("R43 it costs (0) whatever power it rolled, and one that has not rolled yet costs (0) too", () => {
    const { s, power } = inHand(null);
    expect(effectiveCost(s.state, power)).toBe(0);
    for (const name of NAMES) {
      setPower(power, name);
      expect(effectiveCost(s.state, power)).toBe(0);
    }
  });

  it("R43 played from hand it pays (0), lands in the backrow, and the play activates nothing", () => {
    const { s, power } = inHand("burn", { p1: { mana: 4 } });
    s.play(power);

    s.expectInZone(power, "field");
    expect(s.backrow("p1", 1)?.id).toBe(power.id);
    s.expectMana("p1", 4);
    expect(eventsOf(s, "cardPlayed")[0]?.costPaid).toBe(0);
    // Nothing fired: no activation, no use counted, no shot.
    expect(eventsOf(s, "activated")).toEqual([]);
    expect(subsystems.usesThisTurn(s.state, s.card(power))).toBe(0);
    s.expectHealth("p2", 30);

    // The turn's use is still there, so the power can be used the turn it is played.
    s.activate(power);
    s.expectHealth("p2", 28);
    s.expectMana("p1", 3);
  });

  it("R46 Indestructible: Magic Jammed's destroy leaves it where it is", () => {
    const { s, power } = onField("burn", { p1: { hand: [JAMMED, SPARE] } });
    s.play(JAMMED, { targets: pick(power) });
    s.expectInZone(power, "field");
    expect(s.backrow("p1", 1)?.id).toBe(power.id);
    // The power is still its own, and still unused.
    expect(subsystems.powerOf(s.card(power))?.name).toBe("burn");
  });

  // --- the roll ---------------------------------------------------------------

  it("R151 a Heroic Power drawn from the library rolls its power as it arrives, and the hand view names it", () => {
    const s = scenario({ seed: "hp-arrive", p1: { hand: [SPARE], library: [HEROIC, SPARE] } });
    const card = must(s.pile("p1", "library")[0], "the Heroic Power in the library");
    expect(card.memory[subsystems.POWER_KEY]).toBeUndefined();

    s.startTurn();
    const drawn = s.card(card);
    expect(drawn.zone.z).toBe("hand");
    const name = drawn.memory[subsystems.POWER_KEY];
    expect(NAMES).toContain(name);

    // §10.8: the owner's hand view names the power; the card's (0) cost does not depend on it.
    const hand = s.view("p1").you.hand;
    const shown = Array.isArray(hand) ? hand.find((view) => view.instanceId === card.id) : undefined;
    expect(shown?.power).toBe(name);
    expect(shown?.cost).toBe(0);
    // The opponent sees a count, so not the power.
    expect(Array.isArray(s.view("p2").opponent.hand)).toBe(false);
  });

  it("R43 the roll is the match rng's: the same seed rolls the same power, and different seeds roll different ones", () => {
    const rolled = (seed: string): unknown => {
      const s = scenario({ seed, p1: { hand: [SPARE], library: [HEROIC, SPARE] } });
      const card = must(s.pile("p1", "library")[0], "the Heroic Power");
      s.startTurn();
      return s.card(card).memory[subsystems.POWER_KEY];
    };
    const first = rolled("hp-roll-a");
    expect(NAMES).toContain(first);
    expect(rolled("hp-roll-a")).toBe(first);

    const seen = new Set<unknown>();
    for (let n = 0; n < 24; n += 1) seen.add(rolled(`hp-roll-${n}`));
    expect(seen.size).toBeGreaterThan(1);
    for (const name of seen) expect(NAMES).toContain(name);
  });

  it("R151 a copy that already rolled keeps its power as it arrives again", () => {
    const s = scenario({ seed: "hp-keep", p1: { hand: [SPARE], library: [HEROIC, SPARE] } });
    const card = setPower(must(s.pile("p1", "library")[0], "the Heroic Power"), "pluck");
    s.startTurn();
    expect(s.card(card).memory[subsystems.POWER_KEY]).toBe("pluck");
  });

  // --- the power as an Activate ability ------------------------------------------

  it("R103 one that has not rolled declares no usable ability", () => {
    const s = scenario({ p1: { hand: [SPARE], mana: 8, backrow: [HEROIC] } });
    const card = must(s.backrow("p1", 1), "the Heroic Power");
    expect(card.memory[subsystems.POWER_KEY]).toBeUndefined();
    expect(subsystems.abilitiesOf(s.state, card)).toEqual([]);
    expect(() => s.activate(card)).toThrow(/no Activate ability/);
    s.expectMana("p1", 8);
  });

  it("R384 its rolled power is its one Activate ability, listed under the power's name and shown on the hero", () => {
    const { s, power } = onField("burn");
    expect(subsystems.abilitiesOf(s.state, power).map((ability) => ability.id)).toEqual(["burn"]);
    const listed = legalActions(s.state, "p1").filter(
      (action) => action.type === "activate" && action.instanceId === power.id,
    );
    expect(listed).toEqual([expect.objectContaining({ type: "activate", ability: "burn" })]);
    expect(s.view("p1").you.hero.powers).toEqual([
      expect.objectContaining({ instanceId: power.id, name: "burn", x: HERO_POWER_COST.burn, usedThisTurn: false }),
    ]);
    // Public once it is on the field: the opponent sees it as the other hero's.
    expect(s.view("p2").opponent.hero.powers.map((entry) => entry.name)).toEqual(["burn"]);
  });

  it("R384 once per turn, paying the power's X: a second use the same turn is refused and spends nothing", () => {
    const { s, power } = onField("burn");
    s.activate(power);
    s.expectMana("p1", 8 - HERO_POWER_COST.burn);
    s.expectHealth("p2", 28);
    expect(s.lastEvents).toContainEqual(
      expect.objectContaining({ type: "activated", player: "p1", instanceId: power.id, ability: "burn" }),
    );
    expect(s.card(power).memory[subsystems.ACTIVATIONS_MEMORY_KEY]).toEqual({ turn: s.state.turn, count: 1 });

    expect(() => s.activate(power)).toThrow(USED);
    s.expectMana("p1", 8 - HERO_POWER_COST.burn);
    s.expectHealth("p2", 28);
    expect(legalActions(s.state, "p1").some((a) => a.type === "activate" && a.instanceId === power.id)).toBe(false);
    expect(s.view("p1").you.hero.powers[0]?.usedThisTurn).toBe(true);
  });

  it("R384 the next turn is a fresh use", () => {
    const { s, power } = onField("burn", { p1: { library: [SPARE, SPARE] } });
    s.activate(power);
    s.expectHealth("p2", 28);

    s.startTurn();
    expect(subsystems.usedThisTurn(s.state, s.card(power))).toBe(false);
    s.activate(power);
    s.expectHealth("p2", 26);
  });

  it("R384 a power that costs more than the mana left is refused and spends nothing", () => {
    const { s, power } = onField("recruit", { p1: { hand: [SPARE], mana: HERO_POWER_COST.recruit - 1, library: [MENACE] } });
    expect(() => s.activate(power)).toThrow(`that ability costs ${HERO_POWER_COST.recruit}, more than your mana`);
    s.expectMana("p1", HERO_POWER_COST.recruit - 1);
    expect(unitsOf(s, "p1")).toEqual([]);
    expect(subsystems.usesThisTurn(s.state, s.card(power))).toBe(0);
  });

  it("R103 the turn's use is checked before the mana, so a spent power with no mana left says it is spent", () => {
    const { s, power } = onField("burn", { p1: { hand: [FREE], mana: HERO_POWER_COST.burn } });
    s.activate(power);
    s.expectMana("p1", 0);
    expect(subsystems.whyCannotActivateAbility(s.state, "p1", power.id)).toBe(
      "that ability has already been used this turn",
    );
    expect(() => s.activate(power)).toThrow(USED);
  });

  it("R103 the use is marked before the effect runs, so a prompted power cannot be spent twice", () => {
    const { s, power } = onField("discover");
    s.activate(power);
    expect(s.state.pending?.kind).toBe("discover");
    expect(subsystems.usesThisTurn(s.state, s.card(power))).toBe(1);

    s.answer(must(offeredIds(s)[0], "an offered Unit"));
    expect(s.state.pending).toBeNull();
    expect(() => s.activate(power)).toThrow(USED);
    expect(eventsOf(s, "addedToHand")).toHaveLength(1);
  });

  it("R384 an old log's activatePower runs the same ability (R43)", () => {
    const { s, power } = onField("burn");
    const action: Action = { type: "activatePower", playerId: "p1", instanceId: power.id, nonce: "hp-alias" };
    const result = reduce(s.state, action);
    expect(result.error).toBeUndefined();
    expect(result.state.players.p2.hero.health).toBe(28);
    expect(result.state.players.p1.mana.current).toBe(8 - HERO_POWER_COST.burn);
    expect(result.events).toContainEqual(expect.objectContaining({ type: "activated", ability: "burn" }));
  });

  // --- the thirteen powers, base face ------------------------------------------------

  it("R43 Expedition Map (3): recruits the library's first permanent top down, skipping a Spell, on its base face", () => {
    const { s, power } = onField("recruit", { p1: { library: [JAMMED, MENACE] } });
    s.activate(power);

    const recruited = unitsOf(s, "p1");
    expect(recruited.map((unit) => unit.defId)).toEqual([MENACE]);
    expect(recruited[0]?.radiant).toBe(false);
    s.expectStats(must(recruited[0], "the recruited #19"), { attack: 9, health: 9 });
    expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([JAMMED]);
    s.expectMana("p1", 8 - HERO_POWER_COST.recruit);
  });

  it("R658 Life Tap (1): draws 1, then deals 2 ordinary damage to its own hero", () => {
    const { s, power } = onField("draw", { p1: { library: [MENACE, JAMMED] } });
    s.activate(power);

    expect(s.hand("p1").map((card) => card.defId)).toEqual([SPARE, MENACE]);
    s.expectHealth("p1", 28);
    expect(eventsOf(s, "damage")).toEqual([expect.objectContaining({ targetId: "hero-p1", combat: false })]);
    expect(eventsOf(s, "healthLost")).toEqual([]);
    s.expectMana("p1", 8 - HERO_POWER_COST.draw);
  });

  it("R658 Life Tap's damage is ordinary damage, so its hero's Armor reduces it", () => {
    const { s, power } = onField("draw", { p1: { library: [MENACE], armor: 1 } });
    s.activate(power);
    s.expectHealth("p1", 29);
  });

  it("R659 Steady Shot (1): 2 damage to the enemy hero only, and 2 again next turn", () => {
    const { s, power } = onField("burn", { p1: { library: [SPARE, SPARE] } });
    s.activate(power);
    s.expectHealth("p2", 28);
    s.expectHealth("p1", 30);
    expect(subsystems.STEADY_SHOT_PARAM).toBe("shot");

    s.startTurn();
    s.activate(power);
    s.expectHealth("p2", 26);
  });

  it("R43 Ranching (2): a Rush Token, §7's 3/3 Rush, on its base face", () => {
    const { s, power } = onField("rush");
    s.activate(power);
    const token = must(unitsOf(s, "p1")[0], "the Rush Token");
    expect(unitsOf(s, "p1").map((unit) => unit.defId)).toEqual([RUSH_TOKEN]);
    expect(token.radiant).toBe(false);
    s.expectStats(token, { attack: 3, health: 3 });
    expect(s.stats(token).keywords.map((keyword) => keyword.kind)).toEqual(["Rush"]);
    s.expectMana("p1", 8 - HERO_POWER_COST.rush);
  });

  it("R43 Cat Cafe (1): a Felinor Token, §7's 1/1 Felinor", () => {
    const { s, power } = onField("felinor");
    s.activate(power);
    const token = must(unitsOf(s, "p1")[0], "the Felinor Token");
    expect(unitsOf(s, "p1").map((unit) => unit.defId)).toEqual([FELINOR_TOKEN]);
    s.expectStats(token, { attack: 1, health: 1 });
    expect(cardDef(FELINOR_TOKEN).tags).toContain("Felinor");
    s.expectMana("p1", 8 - HERO_POWER_COST.felinor);
  });

  it("R657 Ping (1): Pierce — 1 damage to a Unit in Defense Position through its +2 Armor", () => {
    const { s, power } = onField("ping", { p2: { field: [{ def: DFENDER, position: "DEF" }] } });
    const wall = must(s.unit("p2", 1), "the enemy Big D-fender");
    expect(s.stats(wall).armor).toBeGreaterThanOrEqual(2);

    s.activate(power, { targets: pick(wall) });
    s.expectStats(wall, { health: 6 });
    expect(s.state.pending).toBeNull();
    s.expectMana("p1", 8 - HERO_POWER_COST.ping);
  });

  it("R657 Ping reaches a hero too, ignoring the hero's Armor, and a friendly unit as well", () => {
    const hero = onField("ping", { p2: { armor: 3 } });
    hero.s.activate(hero.power, { targets: ENEMY_HERO });
    hero.s.expectHealth("p2", 29);

    const own = onField("ping", { p1: { field: [MENACE] } });
    const ally = must(own.s.unit("p1", 1), "the friendly #19");
    own.s.activate(own.power, { targets: pick(ally) });
    own.s.expectStats(ally, { health: 8 });
  });

  it("R103 Ping must name its target in the activation: without one it is refused and spends nothing", () => {
    const { s, power } = onField("ping");
    expect(() => s.activate(power)).toThrow(/needs 1 target/);
    s.expectMana("p1", 8);
    expect(subsystems.usesThisTurn(s.state, s.card(power))).toBe(0);
    // `legalActions` offers the ping once per target it can reach, both heroes among them.
    const targets = legalActions(s.state, "p1").flatMap((action) =>
      action.type === "activate" && action.instanceId === power.id ? (action.targets ?? []) : [],
    );
    expect(targets).toContainEqual({ pick: "hero", player: "p1" });
    expect(targets).toContainEqual({ pick: "hero", player: "p2" });
  });

  it("R657 base Ping that kills a Unit summons nothing", () => {
    const { s, power } = onField("ping", { p2: { field: [{ def: MENACE, damage: 8 }] } });
    const victim = must(s.unit("p2", 1), "the enemy #19 at 1 health");
    s.activate(power, { targets: pick(victim) });
    s.expectInZone(victim, "graveyard");
    expect(unitsOf(s, "p1")).toEqual([]);
  });

  it("R103 Witness Value (2): Discover a Unit — three different Units, the pick to hand on its base face", () => {
    const { s, power } = onField("discover");
    s.activate(power);

    const pending = must(s.state.pending, "a discover prompt");
    expect(pending.kind).toBe("discover");
    expect(pending.playerId).toBe("p1");
    const offered = offeredIds(s);
    expect(offered).toHaveLength(3);
    expect(new Set(offered).size).toBe(3);
    const units = query({ type: "Unit" }).map((card) => card.id);
    for (const id of offered) expect(units).toContain(id);

    const picked = must(offered[0], "an offered Unit");
    s.answer(picked);
    const added = s.hand("p1").filter((card) => card.defId === picked);
    expect(added).toHaveLength(1);
    expect(added[0]?.radiant).toBe(false);
    s.expectMana("p1", 8 - HERO_POWER_COST.discover);
  });

  it("R380 Witness Value's and Stitching's Discovers draw from every set", () => {
    const sets = new Set<string>();
    for (let n = 0; n < 6; n += 1) {
      for (const name of ["discover", "stitching"] as const) {
        const { s, power } = onField(name, { seed: `hp-sets-${name}-${n}` });
        s.activate(power);
        for (const id of offeredIds(s)) sets.add(cardDef(id).set);
      }
    }
    expect(sets.has("Core")).toBe(true);
    expect([...sets].some((set) => set !== "Core")).toBe(true);
  });

  it("R352 Stitching (2): each of its two Discovers offers three Units costing (2) or less", () => {
    const { s, power } = onField("stitching");
    s.activate(power);
    for (const step of [1, 2]) {
      const pending = must(s.state.pending, `Discover ${step}`);
      expect(pending.kind).toBe("discover");
      const offered = offeredIds(s);
      expect(offered).toHaveLength(3);
      expect(new Set(offered).size).toBe(3);
      for (const id of offered) {
        const offeredDef = cardDef(id);
        expect(offeredDef.type).toBe("Unit");
        expect(offeredDef.token).toBe(false);
        expect(typeof offeredDef.cost === "number" && offeredDef.cost <= subsystems.STITCHING_MAX_COST).toBe(true);
      }
      s.answer(must(offered[0], "an offered Unit"));
    }
    expect(s.state.pending).toBeNull();
    s.expectMana("p1", 8 - HERO_POWER_COST.stitching);
  });

  it("R352 Stitching fuses the two picks into one hand card at R77's fused cost, on its base face", () => {
    const { s, power } = onField("stitching");
    const before = s.hand("p1");
    const [first, second] = stitch(s, power);

    const added = newInHand(s, before);
    expect(added).toHaveLength(1);
    const result = must(added[0], "the fused card");
    const fusedDef = must(s.state.transientDefs[result.defId], "a fused definition");
    expect(fusedDef.type).toBe("Unit");
    const a = cardDef(first);
    const b = cardDef(second);
    expect(fusedDef.base.attack).toBe((a.base.attack ?? 0) + (b.base.attack ?? 0));
    expect(fusedDef.base.health).toBe((a.base.health ?? 0) + (b.base.health ?? 0));
    expect(result.costOverride).toBeUndefined();
    expect(effectiveCost(s.state, result)).toBe(Math.min((a.cost as number) + (b.cost as number), 4));
    expect(result.radiant).toBe(false);
    expect(eventsOf(s, "fused")).toHaveLength(1);
    expect(() => s.activate(power)).toThrow(USED);
  });

  it("R654 Armor Up (1): +2 Armor on its hero that still cuts a hit in the opponent's turn, gone on its next turn", () => {
    const { s, power } = onField("armor", {
      p1: { library: [SPARE, SPARE] },
      p2: { hand: [SPARE], library: [SPARE, SPARE], field: [MENACE] },
    });
    s.activate(power);
    expect(s.view("p1").you.hero.armor).toBe(2);
    expect(s.state.players.p1.hero.armor).toBe(0); // a modifier, not the hero's own Armor
    s.expectMana("p1", 8 - HERO_POWER_COST.armor);

    s.endTurn();
    expect(s.state.active).toBe("p2");
    expect(s.view("p1").you.hero.armor).toBe(2);
    s.attack(must(s.unit("p2", 1), "the enemy #19"), "hero");
    s.expectHealth("p1", 30 - (9 - 2));

    s.endTurn();
    expect(s.state.active).toBe("p1");
    expect(s.view("p1").you.hero.armor).toBe(0);
  });

  it("R656 Die Insect (2): 8 damage to the enemy hero when it has no Units", () => {
    const { s, power } = onField("insect", { p1: { field: [MENACE] } });
    s.activate(power);
    s.expectHealth("p2", 22);
    s.expectHealth("p1", 30);
    s.expectStats(must(s.unit("p1", 1), "the friendly #19"), { health: 9 });
    s.expectMana("p1", 8 - HERO_POWER_COST.insect);
  });

  it("R656 Die Insect's one pick is uniform over the enemy hero and the enemy Units, never a friendly one", () => {
    const seen = new Set<string>();
    for (let n = 0; n < 12; n += 1) {
      const { s, power } = onField("insect", {
        seed: `hp-insect-${n}`,
        p1: { field: [MENACE] },
        p2: { field: [MENACE, MENACE] },
      });
      const enemies = unitsOf(s, "p2").map((unit) => unit.id);
      s.activate(power);
      seen.add(insectHit(s, enemies));
      s.expectHealth("p1", 30);
      s.expectStats(must(s.unit("p1", 1), "the friendly #19"), { health: 9 });
    }
    expect(seen).toEqual(new Set(["hero", "unit"]));
  });

  it("R661 KY Brainstorm (2): a random KY card to hand, then every non-X Spell in hand costs 1 less", () => {
    const { s, power } = onField("brainstorm", { p1: { hand: [SPARE, STOCKPILE, DIVIDEND] } });
    const before = s.hand("p1");
    s.activate(power);

    const added = newInHand(s, before);
    expect(added).toHaveLength(1);
    const ky = must(added[0], "the KY card");
    expect(kyCards()).toContain(ky.defId);
    expect(cardDef(ky.defId).tags).toContain("KY");
    expect(ky.radiant).toBe(false);

    const costOf = (defId: string): number => effectiveCost(s.state, must(s.hand("p1").find((c) => c.defId === defId), defId));
    expect(costOf(STOCKPILE)).toBe(0); // (1) − 1
    expect(costOf(SPARE)).toBe(3); // a Unit, untouched
    expect(s.card(must(s.hand("p1").find((c) => c.defId === DIVIDEND), "the X Spell")).costMod ?? 0).toBe(0);
    if (cardDef(ky.defId).type === "Spell") expect(s.card(ky).costMod).toBe(-1);
    s.expectMana("p1", 8 - HERO_POWER_COST.brainstorm);
  });

  it("R661 Pluck (2): a random Fruit to hand that costs (0)", () => {
    const { s, power } = onField("pluck");
    const before = s.hand("p1");
    s.activate(power);

    const added = newInHand(s, before);
    expect(added).toHaveLength(1);
    const fruit = must(added[0], "the Fruit");
    expect(fruitCards()).toContain(fruit.defId);
    expect(cardDef(fruit.defId).tags).toContain("Fruit");
    expect(fruit.radiant).toBe(false);
    expect(effectiveCost(s.state, fruit)).toBe(0);
    s.expectMana("p1", 8 - HERO_POWER_COST.pluck);
  });

  it("R660 Terminus Tricks (3): Discover a Trap or Field Trap, summoned face-down into its backrow", () => {
    const { s, power } = onField("terminus");
    s.activate(power);

    const pending = must(s.state.pending, "a discover prompt");
    expect(pending.kind).toBe("discover");
    const offered = offeredIds(s);
    expect(offered).toHaveLength(3);
    for (const id of offered) expect(["Trap", "Field Trap"]).toContain(cardDef(id).type);

    const picked = must(offered[0], "an offered Trap");
    s.answer(picked);
    const trap = must(s.backrow("p1", 2), "the summoned Trap");
    expect(trap.defId).toBe(picked);
    expect(trap.controller).toBe("p1");
    expect(trap.faceUp === true).toBe(false);
    expect(trap.radiant).toBe(false);
    expect(s.hand("p1").map((card) => card.defId)).toEqual([SPARE]);
    s.expectMana("p1", 8 - HERO_POWER_COST.terminus);
  });
});

// ===========================================================================
// Radiant
// ===========================================================================

describe("#98 Heroic Power — radiant", () => {
  it("R43 the Radiant face costs (0) as well, its play activates nothing, and its power is once per turn", () => {
    const { s, power } = inHand("burn", { radiantFace: true, p1: { mana: 4 } });
    expect(effectiveCost(s.state, power)).toBe(0);
    s.play(power);
    s.expectMana("p1", 4);
    s.expectHealth("p2", 30);
    expect(eventsOf(s, "activated")).toEqual([]);

    s.activate(power);
    s.expectMana("p1", 4 - HERO_POWER_COST.burn);
    s.expectHealth("p2", 26);
    expect(() => s.activate(power)).toThrow(USED);
  });

  it("R46 the Radiant face is Indestructible too", () => {
    const { s, power } = onField("burn", { radiantFace: true, p1: { hand: [JAMMED, SPARE] } });
    s.play(JAMMED, { targets: pick(power) });
    expect(s.backrow("p1", 1)?.id).toBe(power.id);
  });

  it("R384 the Radiant face's abilities are the Radiant clauses at the same X", () => {
    for (const ability of hpRadiant.activations ?? []) {
      const power = must(subsystems.powerByName(ability.id), ability.id);
      expect(ability.label).toBe(`${power.radiantTitle}: ${power.radiantLabel}`);
      expect(ability.cost?.mana).toBe(HERO_POWER_COST[ability.id as PowerName]);
    }
  });

  it("R43 Expedition Map: the recruited permanent is made Radiant, and says so", () => {
    const { s, power } = onField("recruit", { radiantFace: true, p1: { library: [JAMMED, MENACE] } });
    const card = must(s.pile("p1", "library")[1], "the #19 in the library");
    s.activate(power);

    const recruited = must(unitsOf(s, "p1")[0], "the recruited #19");
    expect(recruited.id).toBe(card.id);
    expect(recruited.radiant).toBe(true);
    s.expectStats(recruited, { attack: 18, health: 18 });
    expect(s.lastEvents).toContainEqual(expect.objectContaining({ type: "radiantSet", instanceId: card.id }));
    s.expectMana("p1", 8 - HERO_POWER_COST.recruit);
  });

  it("R658 Life Tap: draws its own top card and the opponent's top card into its hand, and takes no damage", () => {
    const { s, power } = onField("draw", {
      radiantFace: true,
      p1: { library: [MENACE, SPARE] },
      p2: { library: [JAMMED, STOCKPILE] },
    });
    s.activate(power);

    const hand = s.hand("p1");
    expect(hand.map((card) => card.defId)).toEqual([SPARE, MENACE, JAMMED]);
    const taken = must(hand[2], "the card from p2's deck");
    expect(taken.controller).toBe("p1");
    expect(s.pile("p2", "library").map((card) => card.defId)).toEqual([STOCKPILE]);
    expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([SPARE]);
    s.expectHealth("p1", 30);
    expect(eventsOf(s, "damage")).toEqual([]);
    s.expectMana("p1", 8 - HERO_POWER_COST.draw);
  });

  it("R659 Steady Shot: 4 damage, then +2 permanently — the next turn's shot is 6", () => {
    const { s, power } = onField("burn", { radiantFace: true, p1: { library: [SPARE, SPARE] } });
    s.activate(power);
    s.expectHealth("p2", 26);
    s.expectHealth("p1", 30);

    s.startTurn();
    s.activate(power);
    s.expectHealth("p2", 20);
  });

  it("R659 the Radiant clause's \"+2 damage\" is one step of the card's declared `shot`", () => {
    const shot = must(
      def.params?.find((entry) => entry.key === subsystems.STEADY_SHOT_PARAM),
      "#98 declares `shot`",
    );
    expect(shot.step).toBe(STEADY_SHOT_STEP_DAMAGE);
    expect(must(subsystems.powerByName("burn"), "burn").radiantLabel).toContain(`+${String(STEADY_SHOT_STEP_DAMAGE)} damage`);
  });

  it("R43 Ranching: a Radiant Rush Token, §7's 6/6 Rush Cleave", () => {
    const { s, power } = onField("rush", { radiantFace: true });
    s.activate(power);
    const token = must(unitsOf(s, "p1")[0], "the Rush Token");
    expect(unitsOf(s, "p1").map((unit) => unit.defId)).toEqual([RUSH_TOKEN]);
    expect(token.radiant).toBe(true);
    s.expectStats(token, { attack: 6, health: 6 });
    expect(s.stats(token).keywords.map((keyword) => keyword.kind).sort()).toEqual(["Cleave", "Rush"]);
  });

  it("R43 Cat Cafe: a random non-token Felinor Unit, on its base face", () => {
    const pool = felinorUnits();
    expect(pool.length).toBeGreaterThan(1);
    for (let n = 0; n < 4; n += 1) {
      const { s, power } = onField("felinor", { radiantFace: true, seed: `hp-cat-${n}` });
      s.activate(power);
      const units = unitsOf(s, "p1");
      expect(units).toHaveLength(1);
      const cat = must(units[0], "the Felinor");
      expect(pool).toContain(cat.defId);
      expect(cardDef(cat.defId).token).toBe(false);
      expect(cardDef(cat.defId).tags).toContain("Felinor");
      expect(cat.defId).not.toBe(FELINOR_TOKEN);
      expect(cat.radiant).toBe(false);
    }
  });

  it("R657 Ping: a kill summons a Ghoul Token for its controller with the Unit's attack and max health", () => {
    const { s, power } = onField("ping", { radiantFace: true, p2: { field: [{ def: MENACE, damage: 8 }] } });
    const victim = must(s.unit("p2", 1), "the enemy #19 at 1 health");
    s.activate(power, { targets: pick(victim) });

    s.expectInZone(victim, "graveyard");
    const ghouls = unitsOf(s, "p1");
    expect(ghouls.map((unit) => unit.defId)).toEqual([GHOUL_TOKEN]);
    const ghoul = must(ghouls[0], "the Ghoul Token");
    expect(ghoul.controller).toBe("p1");
    // The destroyed event's numbers: attack 9, max health 9 — not the 1 health it died on.
    s.expectStats(ghoul, { attack: 9, health: 9, maxHealth: 9 });
    expect(s.stats(ghoul).keywords.map((keyword) => keyword.kind)).toContain("Pierce");
    expect(unitsOf(s, "p2")).toEqual([]);
  });

  it("R657 Ping: a hit that kills nothing summons no Ghoul", () => {
    const unit = onField("ping", { radiantFace: true, p2: { field: [MENACE] } });
    const victim = must(unit.s.unit("p2", 1), "the enemy #19");
    unit.s.activate(unit.power, { targets: pick(victim) });
    unit.s.expectStats(victim, { health: 8 });
    expect(unitsOf(unit.s, "p1")).toEqual([]);

    const hero = onField("ping", { radiantFace: true });
    hero.s.activate(hero.power, { targets: ENEMY_HERO });
    hero.s.expectHealth("p2", 29);
    expect(unitsOf(hero.s, "p1")).toEqual([]);
  });

  it("R103 Witness Value: the Discovered Unit arrives Radiant", () => {
    const { s, power } = onField("discover", { radiantFace: true });
    s.activate(power);
    expect(s.state.pending?.prompt).toBe("Discover a Radiant Unit");
    const picked = must(offeredIds(s)[0], "an offered Unit");
    expect(cardDef(picked).type).toBe("Unit");
    s.answer(picked);
    const added = must(s.hand("p1").find((card) => card.defId === picked), "the discovered Unit");
    expect(added.radiant).toBe(true);
  });

  it("R352 Stitching: two Radiant Discovers, and the fused card is Radiant, wearing the sum of the Radiant faces", () => {
    const { s, power } = onField("stitching", { radiantFace: true });
    const before = s.hand("p1");
    s.activate(power);
    expect(s.state.pending?.prompt).toBe("Discover a Radiant Unit that costs (2) or less");
    const first = must(offeredIds(s)[0], "a first Unit");
    s.answer(first);
    const second = must(offeredIds(s)[0], "a second Unit");
    s.answer(second);

    const result = must(newInHand(s, before)[0], "the fused card");
    expect(result.radiant).toBe(true);
    const fusedDef = must(s.state.transientDefs[result.defId], "a fused definition");
    expect(fusedDef.radiant.attack).toBe((cardDef(first).radiant.attack ?? 0) + (cardDef(second).radiant.attack ?? 0));
    const hand = s.view("p1").you.hand;
    const shown = Array.isArray(hand) ? hand.find((card) => card.instanceId === result.id) : undefined;
    expect(shown?.attack).toBe(fusedDef.radiant.attack);
    expect(result.costOverride).toBeUndefined();
  });

  it("R654 Tank Up: +4 Armor on its hero for the rest of the game, through the opponent's turn and past its own next one", () => {
    const { s, power } = onField("armor", {
      radiantFace: true,
      p1: { library: [SPARE, SPARE] },
      p2: { hand: [SPARE], library: [SPARE, SPARE], field: [MENACE] },
    });
    s.activate(power);
    expect(s.state.players.p1.hero.armor).toBe(4);
    expect(s.view("p1").you.hero.armor).toBe(4);

    s.endTurn();
    s.attack(must(s.unit("p2", 1), "the enemy #19"), "hero");
    s.expectHealth("p1", 30 - (9 - 4));

    s.endTurn();
    expect(s.state.active).toBe("p1");
    expect(s.view("p1").you.hero.armor).toBe(4);
  });

  it("R655 Tank Up then refreshes into a different power, which waits for the next turn", () => {
    const refreshed = new Set<unknown>();
    for (let n = 0; n < 6; n += 1) {
      const { s, power } = onField("armor", { radiantFace: true, seed: `hp-tank-${n}`, p1: { library: [SPARE, SPARE] } });
      s.activate(power);

      const now = s.card(power).memory[subsystems.POWER_KEY];
      expect(now).not.toBe("armor");
      expect(NAMES).toContain(now);
      refreshed.add(now);
      expect(subsystems.abilitiesOf(s.state, s.card(power)).map((ability) => ability.id)).toEqual([now]);
      // The card is still Radiant, so the new power runs its Radiant clause.
      const after = must(subsystems.powerByName(String(now)), String(now));
      expect(subsystems.abilitiesOf(s.state, s.card(power))[0]?.label).toBe(`${after.radiantTitle}: ${after.radiantLabel}`);
      expect(() => s.activate(power, { targets: ENEMY_HERO })).toThrow(USED);
      expect(s.view("p1").you.hero.powers[0]?.name).toBe(now);
      expect(s.view("p1").you.hero.powers[0]?.radiant).toBe(true);

      s.startTurn();
      expect(subsystems.whyCannotActivateAbility(s.state, "p1", power.id)).toBeNull();
    }
    expect(refreshed.size).toBeGreaterThan(1);
  });

  it("R655 the power Tank Up refreshes into runs its Radiant face: a Steady Shot hits for 4", () => {
    // Seeds are tried in order until one refreshes into Steady Shot (1 in 12 each), so the test is fixed.
    for (let n = 0; n < 60; n += 1) {
      const { s, power } = onField("armor", { radiantFace: true, seed: `hp-tank-burn-${n}`, p1: { library: [SPARE, SPARE] } });
      s.activate(power);
      if (s.card(power).memory[subsystems.POWER_KEY] !== "burn") continue;
      s.startTurn();
      s.activate(power);
      s.expectHealth("p2", 26);
      return;
    }
    throw new Error("no seed refreshed Tank Up into Steady Shot");
  });

  it("R656 Die Insect Lucky 1: a lethal hit on the enemy hero beats hitting a Unit", () => {
    let baseLethal = 0;
    let radiantLethal = 0;
    for (let n = 0; n < 16; n += 1) {
      const lethal = (radiantFace: boolean): boolean => {
        const { s, power } = onField("insect", {
          radiantFace,
          seed: `hp-lucky-hero-${n}`,
          p2: { health: 8, field: [MENACE] },
        });
        const enemies = unitsOf(s, "p2").map((unit) => unit.id);
        s.activate(power);
        const hit = insectHit(s, enemies);
        if (hit === "unit") s.expectStats(must(s.unit("p2", 1), "the enemy #19"), { health: 1 });
        return hit === "hero";
      };
      const fromBase = lethal(false);
      const fromRadiant = lethal(true);
      // The Radiant face's first pick is the base face's, so it is never worse.
      if (fromBase) expect(fromRadiant).toBe(true);
      baseLethal += fromBase ? 1 : 0;
      radiantLethal += fromRadiant ? 1 : 0;
    }
    expect(radiantLethal).toBeGreaterThan(baseLethal);
  });

  it("R656 Die Insect Lucky 1: a Unit it destroys beats a hero hit that is not lethal", () => {
    let baseKills = 0;
    let radiantKills = 0;
    for (let n = 0; n < 16; n += 1) {
      const kills = (radiantFace: boolean): boolean => {
        const { s, power } = onField("insect", {
          radiantFace,
          seed: `hp-lucky-unit-${n}`,
          p2: { field: [{ def: MENACE, damage: 1 }] },
        });
        const victim = must(s.unit("p2", 1), "the enemy #19 at 8 health");
        s.activate(power);
        const hit = insectHit(s, [victim.id]);
        if (hit === "unit") s.expectInZone(victim, "graveyard");
        else s.expectHealth("p2", 22);
        return hit === "unit";
      };
      const fromBase = kills(false);
      const fromRadiant = kills(true);
      if (fromBase) expect(fromRadiant).toBe(true);
      baseKills += fromBase ? 1 : 0;
      radiantKills += fromRadiant ? 1 : 0;
    }
    expect(radiantKills).toBeGreaterThan(baseKills);
  });

  it("R661 KY Brainstorm: the KY card is Radiant, and the Spells in hand still cost 1 less", () => {
    const { s, power } = onField("brainstorm", { radiantFace: true, p1: { hand: [SPARE, STOCKPILE] } });
    const before = s.hand("p1");
    s.activate(power);

    const ky = must(newInHand(s, before)[0], "the KY card");
    expect(kyCards()).toContain(ky.defId);
    expect(ky.radiant).toBe(true);
    const stockpile = must(s.hand("p1").find((card) => card.defId === STOCKPILE), "#5");
    expect(effectiveCost(s.state, stockpile)).toBe(0);
  });

  it("R661 Pluck: the Fruit is Radiant and costs (0)", () => {
    const { s, power } = onField("pluck", { radiantFace: true });
    const before = s.hand("p1");
    s.activate(power);

    const fruit = must(newInHand(s, before)[0], "the Fruit");
    expect(fruitCards()).toContain(fruit.defId);
    expect(fruit.radiant).toBe(true);
    expect(effectiveCost(s.state, fruit)).toBe(0);
  });

  it("R660 Terminus Tricks: the summoned Trap is Radiant and face-down", () => {
    const { s, power } = onField("terminus", { radiantFace: true });
    s.activate(power);
    expect(s.state.pending?.prompt).toBe("Discover a Radiant Trap to summon");
    const picked = must(offeredIds(s)[0], "an offered Trap");
    expect(["Trap", "Field Trap"]).toContain(cardDef(picked).type);
    s.answer(picked);

    const trap = must(s.backrow("p1", 2), "the summoned Trap");
    expect(trap.defId).toBe(picked);
    expect(trap.radiant).toBe(true);
    expect(trap.faceUp === true).toBe(false);
  });
});
