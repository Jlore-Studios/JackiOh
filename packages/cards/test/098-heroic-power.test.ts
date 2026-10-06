// #98 Heroic Power — SPEC §8.5, §6.2 ("Start of Game", Quickdraw, Activate), §10.2, §10.6, §10.8;
// R43, R46, R81, R103, R352, R752–R761 (the Heroic Power patch, issue #37).
//
// BUILD M4-T4 row 98: "In opening hand; costs (0) and playing it uses nothing; power chosen at start
// of game from the seed; each power an Activate paying its X once per turn; Indestructible; each power
// on both faces".
//
// HOW A POWER IS PINNED. R103 makes the power names state, so a test that wants a named one writes
// that name into `memory.power` — which is exactly and only what `subsystems.ensurePower` writes, so
// the state is one the engine produces. Every assertion below is then about what the card DID.
//
// What the harness cannot reach: `scenario()` skips §2.1, so `startOfGame` never runs and the
// start-of-game roll has no card-level path; `packages/engine/test/heroPower.test.ts` covers the roll
// by calling `finishSetup` directly, and the arrival roll (R151) with a bounce.

import { describe, expect, it } from "vitest";
import type { ActionBody, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { effectiveCost, legalActions, subsystems } from "@jackioh/engine";
import type { CardInstance } from "@jackioh/engine";
import { cardDef } from "../src/catalog-data";
import { query } from "../src/query";
import { base as hpBase, radiant as hpRadiant } from "../src/scripts/098-heroic-power";
import { scenario, type Scenario, type SideSetup } from "./_harness";

const HEROIC = "core-098"; // Field Spell, cost (0), Mythic, tag Quickdraw, Indestructible
const RUSH_TOKEN = "core-t-rush";
const FELINOR_TOKEN = "core-t-felinor";
const GHOUL_TOKEN = "core-t-ghoul";

/** #53 Reno, a 3-cost Unit: the spare card that keeps §2.5's auto-end away from the assertions. */
const SPARE = "core-053";
/** #36 Magic Jammed, a 1-cost Spell that destroys a chosen backrow card (R46's test). */
const JAMMED = "core-036";
/** #19 Midrange Menace, a 3-cost 9/9 Unit — the permanent Expedition Map finds in a library. */
const MENACE = "core-019";
/** #93.1 Combo-Fodder, a 0-cost Spell token: a card p2 can always play, so its turn never auto-ends. */
const FREE = "core-093-1";
/** Both sides' turns pass with cards to draw and, for p2, a card to play (§2.5's auto-end aside). */
const TURNS: { p1: SideSetup; p2: SideSetup } = {
  p1: { hand: [SPARE], mana: 8, library: [JAMMED, JAMMED, JAMMED] },
  p2: { hand: [FREE], library: [JAMMED, JAMMED, JAMMED] },
};

type PowerName = (typeof subsystems.HERO_POWERS)[number]["name"];

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

function eventsOf<T extends GameEvent["type"]>(s: Scenario, type: T): Extract<GameEvent, { type: T }>[] {
  return s.lastEvents.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);
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

/** A #98 on p1's backrow with a named power, mana to spend and a spare card in hand. */
function onField(
  name: PowerName,
  opts: { radiantFace?: boolean; p1?: SideSetup; p2?: SideSetup; seed?: string } = {},
): { s: Scenario; power: CardInstance } {
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

function unitPick(card: CardInstance): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

/** The def ids a Discover prompt offers (§6.3), in the order offered. */
function offeredIds(s: Scenario): string[] {
  return must(s.state.pending, "a discover prompt").options.flatMap((option) =>
    option.selection.pick === "mode" ? [option.selection.option] : [],
  );
}

/** The power's activations `legalActions` lists for p1 now. */
function listed(s: Scenario, card: CardInstance): ActionBody[] {
  return legalActions(s.state, "p1").filter((body) => body.type === "activate" && body.instanceId === card.id);
}

// ---------------------------------------------------------------------------
// The card: its data, its wiring and its keyword.
// ---------------------------------------------------------------------------

describe("#98 Heroic Power — the card (R752)", () => {
  it("R752 is a Mythic Field Spell costing (0), tagged Quickdraw, Indestructible on both faces", () => {
    const def = cardDef(HEROIC);
    expect(def.type).toBe("Field Spell");
    expect(def.cost).toBe(0);
    expect(def.rarity).toBe("Mythic");
    expect(def.tags).toContain("Quickdraw");
    for (const face of [def.base, def.radiant]) {
      expect(face.keywords.map((keyword) => keyword.kind)).toContain("Indestructible");
      expect(face.text).toContain("Gain one of 13 random powers. Each is an Activate that spends its (X).");
      expect(face.text).not.toMatch(/Playing it|Once per turn, spend/);
    }
    expect(def.params).toEqual([{ key: "shot", base: 2, radiant: 4, better: "up", step: 2, min: 1 }]);
    expect(def.refs).toEqual([RUSH_TOKEN, FELINOR_TOKEN, GHOUL_TOKEN]);
  });

  it("R752 each power's line on each face is its ability's words, after its X and its name", () => {
    const def = cardDef(HEROIC);
    for (const entry of subsystems.HERO_POWERS) {
      expect(def.base.text).toContain(`(${entry.x}) ${entry.title}: ${entry.label}`);
      expect(def.radiant.text).toContain(`(${entry.x}) ${entry.radiantTitle}: ${entry.radiantLabel}`);
    }
    expect(def.radiant.text).toContain("(1) Tank Up: Your hero gains 4 Armor, then this power refreshes.");
  });

  it("§6.2 Quickdraw: both faces carry the flag `setup.ts` reads for the opening hand", () => {
    expect(hpBase.staticFlags?.quickdraw).toBe(true);
    expect(hpRadiant.staticFlags?.quickdraw).toBe(true);
  });

  it("R752 both faces wire the roll, the thirteen abilities and the Discover step — no cost hook, no Cry", () => {
    const members = ["staticFlags", "startOfGame", "activations", "resume"];
    expect(Object.keys(hpBase).sort()).toEqual([...members].sort());
    expect(Object.keys(hpRadiant).sort()).toEqual([...members].sort());
    expect(hpBase.activations?.map((decl) => decl.id)).toEqual(subsystems.HERO_POWER_NAMES);
    expect(hpRadiant.activations?.map((decl) => decl.cost?.mana)).toEqual(subsystems.HERO_POWERS.map((entry) => entry.x));
    expect(Object.keys(hpBase.resume ?? {})).toEqual([subsystems.POWER_RESUME]);
  });

  it("R46 Indestructible: a Field Spell that is destroyed simply stays", () => {
    const { s, power: card } = onField("burn", { p1: { hand: [JAMMED], mana: 8 } });
    s.play(JAMMED, { targets: unitPick(card) });
    s.expectInZone(card, "field");
    expect(s.backrow("p1", 1)?.id).toBe(card.id);
  });
});

// ---------------------------------------------------------------------------
// Playing it, and using a power (R752).
// ---------------------------------------------------------------------------

describe("#98 Heroic Power — costs (0); each power an Activate (R752)", () => {
  it("R752 playing it costs (0) and uses nothing; the power is then one activation a turn for its X", () => {
    const s = scenario({ seed: "hp-play", p1: { hand: [HEROIC, SPARE], mana: 8 } });
    const card = setPower(must(s.hand("p1")[0], "the Heroic Power"), "burn");
    expect(effectiveCost(s.state, card)).toBe(0);
    s.play(card, { zone: 1 });
    s.expectMana("p1", 8).expectHealth("p2", 30);
    expect(eventsOf(s, "activated")).toEqual([]);

    s.activate(card);
    s.expectMana("p1", 7).expectHealth("p2", 28);
    expect(eventsOf(s, "activated")).toHaveLength(1);
    expect(listed(s, card)).toEqual([]);
    expect(() => s.activate(card)).toThrow(/already been used this turn/);
  });

  it("R752 a power its controller cannot pay for is neither listed nor accepted", () => {
    const { s, power: card } = onField("tricks", { p1: { hand: [SPARE], mana: 2 } }); // X 3
    expect(listed(s, card)).toEqual([]);
    expect(() => s.activate(card)).toThrow(/costs 3, more than your mana/);
  });

  it("R752 R81 Ping is listed once per unit or hero it may hit, so the client can drag it onto one", () => {
    const { s, power: card } = onField("ping", { p2: { field: [FELINOR_TOKEN] } });
    const aims = listed(s, card).map((body) => (body.type === "activate" ? body.targets : undefined));
    expect(aims).toContainEqual([{ pick: "hero", player: "p2" }]);
    expect(aims).toContainEqual([{ pick: "hero", player: "p1" }]);
    expect(aims.length).toBe(3);
  });

  it("R752 the hero panel names the power by its name on the card, with its X and its use", () => {
    const { s, power: card } = onField("armor", { radiantFace: true });
    expect(s.view("p1").you.hero.power).toMatchObject({ instanceId: card.id, name: "armor", title: "Tank Up", x: 1, usedThisTurn: false });
    expect(s.view("p2").opponent.hero.power).toMatchObject({ name: "armor", title: "Tank Up" });
  });
});

// ---------------------------------------------------------------------------
// The thirteen powers, base face.
// ---------------------------------------------------------------------------

describe("#98 Heroic Power — the powers (R752–R761)", () => {
  it("R43 (3) Expedition Map recruits a permanent", () => {
    const { s, power: card } = onField("recruit", { p1: { library: [MENACE], hand: [SPARE], mana: 8 } });
    s.activate(card);
    expect(unitsOf(s, "p1").map((unit) => unit.defId)).toEqual([MENACE]);
    expect(unitsOf(s, "p1")[0]?.radiant).toBe(false);
    s.expectMana("p1", 5);
  });

  it("R753 (1) Life Tap draws 1, then deals 2 damage to your own hero", () => {
    const { s, power: card } = onField("draw", { p1: { library: [MENACE, JAMMED], hand: [SPARE], mana: 8 } });
    s.activate(card);
    expect(s.hand("p1").map((entry) => entry.defId)).toEqual([SPARE, MENACE]);
    s.expectHealth("p1", 28).expectMana("p1", 7);
    expect(eventsOf(s, "damage").map((event) => event.targetId)).toEqual(["hero-p1"]);
  });

  it("R754 (1) Steady Shot deals {shot}, 2, to the enemy hero", () => {
    const { s, power: card } = onField("burn");
    s.activate(card);
    s.expectHealth("p2", 28);
  });

  it("R752 (2) Ranching summons a Rush Token", () => {
    const { s, power: card } = onField("rush");
    s.activate(card);
    expect(unitsOf(s, "p1").map((unit) => [unit.defId, unit.radiant])).toEqual([[RUSH_TOKEN, false]]);
  });

  it("R755 (1) Cat Cafe summons a Felinor Token", () => {
    const { s, power: card } = onField("felinor");
    s.activate(card);
    expect(unitsOf(s, "p1").map((unit) => unit.defId)).toEqual([FELINOR_TOKEN]);
  });

  it("R756 (1) Ping deals 1 Pierce damage to the unit or hero declared with it", () => {
    const { s, power: card } = onField("ping");
    s.activate(card, { targets: [{ pick: "hero", player: "p2" }] });
    s.expectHealth("p2", 29);
    expect(s.state.pending).toBeNull();
    // R81: the target travels with the activation; a Ping that names none is refused.
    const bare = onField("ping", { seed: "hp-ping-bare" });
    expect(() => bare.s.activate(bare.power)).toThrow();
  });

  it("R103 (2) Witness Value Discovers a Unit, which goes to hand", () => {
    const { s, power: card } = onField("discover");
    s.activate(card);
    const offered = offeredIds(s);
    expect(offered).toHaveLength(3);
    for (const id of offered) expect(cardDef(id).type).toBe("Unit");
    s.answer(must(offered[0], "an offered Unit"));
    expect(s.hand("p1").filter((entry) => entry.defId === offered[0]).map((entry) => entry.radiant)).toEqual([false]);
    s.expectMana("p1", 6);
  });

  it("R352 (2) Stitching Discovers two Units that cost (2) or less and fuses them into your hand", () => {
    const { s, power: card } = onField("stitching");
    const before = s.hand("p1").length;
    s.activate(card);
    for (const step of [1, 2]) {
      const offered = offeredIds(s);
      expect(offered, `Discover ${step}`).toHaveLength(3);
      for (const id of offered) {
        const offeredDef = cardDef(id);
        expect(typeof offeredDef.cost === "number" && offeredDef.cost <= subsystems.STITCHING_MAX_COST).toBe(true);
      }
      s.answer(must(offered[0], "an offered Unit"));
    }
    expect(s.hand("p1")).toHaveLength(before + 1);
    expect(eventsOf(s, "fused")).toHaveLength(1);
    expect(() => s.activate(card)).toThrow(/already been used this turn/);
  });

  it("R757 (1) Armor Up gives your hero 2 Armor until your next turn", () => {
    const { s, power: card } = onField("armor", TURNS);
    s.activate(card);
    expect(s.view("p1").you.hero.armor).toBe(2);
    s.endTurn();
    expect(s.state.active).toBe("p2");
    expect(s.view("p1").you.hero.armor).toBe(2);
    s.endTurn();
    expect(s.state.active).toBe("p1");
    expect(s.view("p1").you.hero.armor).toBe(0);
  });

  it("R758 (2) Die Insect deals 8 damage to a random enemy", () => {
    const { s, power: card } = onField("insect", { p2: { field: [MENACE] } });
    s.activate(card);
    const hits = eventsOf(s, "damage");
    expect(hits).toHaveLength(1);
    expect(hits[0]?.amount).toBe(8);
    const menace = must(s.unit("p2", 1), "the enemy Menace");
    expect([`hero-p2`, menace.id]).toContain(hits[0]?.targetId);
  });

  it("R759 (2) KY Brainstorm adds a random KY card, then every Spell in your hand costs (1) less", () => {
    const { s, power: card } = onField("brainstorm", { p1: { hand: [SPARE, JAMMED], mana: 8 } });
    s.activate(card);
    const hand = s.hand("p1");
    const added = must(hand[hand.length - 1], "the KY card");
    expect(cardDef(added.defId).tags).toContain("KY");
    expect(cardDef(added.defId).token).toBe(false);
    expect(added.radiant).toBe(false);
    expect(effectiveCost(s.state, added)).toBe(Math.max(0, (cardDef(added.defId).cost as number) - 1));
    expect(effectiveCost(s.state, must(hand.find((entry) => entry.defId === JAMMED), "Magic Jammed"))).toBe(0);
    expect(effectiveCost(s.state, must(hand.find((entry) => entry.defId === SPARE), "Reno"))).toBe(3);
  });

  it("R760 (2) Pluck adds a random Fruit to your hand, costing (0)", () => {
    const { s, power: card } = onField("pluck");
    s.activate(card);
    const hand = s.hand("p1");
    const fruit = must(hand[hand.length - 1], "the Fruit");
    expect(cardDef(fruit.defId).tags).toContain("Fruit");
    expect(effectiveCost(s.state, fruit)).toBe(0);
    expect(fruit.radiant).toBe(false);
  });

  it("R761 (3) Terminus Tricks Discovers a Trap and summons it face-down into your backrow", () => {
    const { s, power: card } = onField("tricks");
    s.activate(card);
    const offered = offeredIds(s);
    expect(offered).toHaveLength(3);
    const traps = query({ type: ["Trap", "Field Trap"] }).map((def) => def.id);
    for (const id of offered) expect(traps).toContain(id);
    s.answer(must(offered[0], "an offered Trap"));
    const set = must(s.backrow("p1", 2), "the summoned Trap");
    expect(set.defId).toBe(offered[0]);
    expect(set.radiant).toBe(false);
    expect(s.view("p2").opponent.backrow[1]).toMatchObject({ faceDown: true });
    s.expectMana("p1", 5);
  });
});

// ---------------------------------------------------------------------------
// The thirteen powers, Radiant face.
// ---------------------------------------------------------------------------

describe("#98 Heroic Power — the powers, Radiant (R752–R761)", () => {
  it("R43 Expedition Map makes the permanent Radiant", () => {
    const { s, power: card } = onField("recruit", { radiantFace: true, p1: { library: [MENACE], hand: [SPARE], mana: 8 } });
    s.activate(card);
    expect(must(unitsOf(s, "p1")[0], "the recruited #19").radiant).toBe(true);
  });

  it("R753 Life Tap draws the top card of each player's deck, and deals no damage", () => {
    const { s, power: card } = onField("draw", {
      radiantFace: true,
      p1: { library: [MENACE], hand: [SPARE], mana: 8 },
      p2: { library: [JAMMED] },
    });
    s.activate(card);
    expect(s.hand("p1").map((entry) => entry.defId)).toEqual([SPARE, MENACE, JAMMED]);
    s.expectHealth("p1", 30);
  });

  it("R754 Steady Shot deals {shot}, 4, then upgrades itself by +2 damage for good", () => {
    const { s, power: card } = onField("burn", { radiantFace: true, ...TURNS });
    s.activate(card);
    s.expectHealth("p2", 26);
    expect(eventsOf(s, "numberChanged")).toMatchObject([{ key: "shot", value: 6 }]);
    s.endTurn().endTurn();
    s.activate(card);
    expect(eventsOf(s, "damage").map((event) => [event.targetId, event.amount])).toEqual([["hero-p2", 6]]);
    expect(eventsOf(s, "numberChanged")).toMatchObject([{ key: "shot", value: 8 }]);
  });

  it("R752 Ranching summons a Radiant Rush Token", () => {
    const { s, power: card } = onField("rush", { radiantFace: true });
    s.activate(card);
    expect(unitsOf(s, "p1").map((unit) => [unit.defId, unit.radiant])).toEqual([[RUSH_TOKEN, true]]);
  });

  it("R755 Cat Cafe summons a random non-token Felinor", () => {
    const { s, power: card } = onField("felinor", { radiantFace: true });
    s.activate(card);
    const [summoned] = unitsOf(s, "p1");
    const def = cardDef(must(summoned, "a Felinor").defId);
    expect(def.tags).toContain("Felinor");
    expect(def.token).toBe(false);
  });

  it("R756 Ping, killing a Unit, summons a Ghoul Token with its stats", () => {
    const { s, power: card } = onField("ping", { radiantFace: true, p2: { field: [FELINOR_TOKEN] } });
    const kitten = must(s.unit("p2", 1), "the enemy Felinor Token");
    s.activate(card, { targets: unitPick(kitten) });
    expect(s.unit("p2", 1)).toBeNull();
    const [ghoul] = unitsOf(s, "p1");
    expect(must(ghoul, "a Ghoul Token").defId).toBe(GHOUL_TOKEN);
    s.expectStats(must(ghoul, "a Ghoul Token"), { attack: 1, health: 1 });
  });

  it("R103 Witness Value Discovers a Radiant Unit", () => {
    const { s, power: card } = onField("discover", { radiantFace: true });
    s.activate(card);
    const picked = must(offeredIds(s)[0], "an offered Unit");
    s.answer(picked);
    expect(s.hand("p1").filter((entry) => entry.defId === picked).map((entry) => entry.radiant)).toEqual([true]);
  });

  it("R352 Stitching fuses two Radiant Units into a Radiant card", () => {
    const { s, power: card } = onField("stitching", { radiantFace: true });
    s.activate(card);
    s.answer(must(offeredIds(s)[0], "a first Unit"));
    s.answer(must(offeredIds(s)[0], "a second Unit"));
    const hand = s.hand("p1");
    expect(must(hand[hand.length - 1], "the fused card").radiant).toBe(true);
  });

  it("R757 Tank Up keeps 4 Armor, then refreshes into a different power you may use this turn", () => {
    const { s, power: card } = onField("armor", { radiantFace: true, ...TURNS });
    s.activate(card);
    expect(s.state.players.p1.hero.armor).toBe(4);
    const now = must(s.backrow("p1", 1), "the Heroic Power");
    const next = must(subsystems.powerOf(now), "the new power");
    expect(next.name).not.toBe("armor");
    const panel = must(s.view("p1").you.hero.power, "the hero panel's power");
    expect(panel).toMatchObject({ name: next.name, usedThisTurn: false });
    s.endTurn().endTurn();
    expect(s.view("p1").you.hero.armor).toBeGreaterThanOrEqual(4);
  });

  it("R758 Die Insect is Lucky 1, and still deals 8", () => {
    const { s, power: card } = onField("insect", { radiantFace: true, p2: { field: [MENACE] } });
    s.activate(card);
    expect(eventsOf(s, "damage").map((event) => event.amount)).toEqual([8]);
  });

  it("R759 KY Brainstorm's KY card is Radiant", () => {
    const { s, power: card } = onField("brainstorm", { radiantFace: true });
    s.activate(card);
    const hand = s.hand("p1");
    const added = must(hand[hand.length - 1], "the KY card");
    expect(cardDef(added.defId).tags).toContain("KY");
    expect(added.radiant).toBe(true);
  });

  it("R760 Pluck's Fruit is Radiant and costs (0)", () => {
    const { s, power: card } = onField("pluck", { radiantFace: true });
    s.activate(card);
    const hand = s.hand("p1");
    const fruit = must(hand[hand.length - 1], "the Fruit");
    expect(fruit.radiant).toBe(true);
    expect(effectiveCost(s.state, fruit)).toBe(0);
  });

  it("R761 Terminus Tricks summons a Radiant Trap", () => {
    const { s, power: card } = onField("tricks", { radiantFace: true });
    s.activate(card);
    s.answer(must(offeredIds(s)[0], "an offered Trap"));
    expect(must(s.backrow("p1", 2), "the summoned Trap").radiant).toBe(true);
  });
});
