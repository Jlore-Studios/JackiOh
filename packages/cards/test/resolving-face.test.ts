// #64 Gifted Program's first cheap card, and the face a play's choices answer (SPEC §8 #64, §10.5
// steps 1 and 3, R56, R81, R90, R213, R214). Found by the polish-4 edge-case hunt, round 3
// (docs/polish/4-edge-cases.md, lenses L1, L2 and L9); every case here failed before its fix, except
// the third R213 case, which pins the reading the fix adopts.
//
//  - R213: "the first card costing 1 or less you play each turn" counts the player's plays that turn,
//    so a Gifted Program that changes hands, or leaves and comes back, neither uses up another
//    player's first cheap card nor gives its own player a second.
//  - R214: step 3 can make the played card Radiant after step 1 has read its choices, so step 1 and
//    `legalActions` read the choices of the face step 5 will resolve.

import type { ActionBody, Selection } from "@jackioh/shared";
import { createRng, legalActions, subsystems, type CardInstance, type EngineSink } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "./_harness";

const BIGOT = "core-002";
const STOCKPILE = "core-005";
const VANILLA = "core-008";
const MOTHS = "core-009";
const TIMMY = "core-011";
const HINDER = "core-021";
const PANTHER = "core-032";
const PEK_CONTROLLER = "core-048";
const MIND_CONTROL = "core-049";
const SILAS = "core-052";
const RENO = "core-053";
const FRIEND = "core-062";
const GIFTED = "core-064";
const TWISTED_SORCERER = "core-068";
const POCKET_CHAOS = "core-087";
const CRAFT_A_CARD = "core-099";
const LIBRARY = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

const at = (card: CardInstance): Selection[] => [{ pick: "instance", instanceId: card.id }];

function unitAt(g: Scenario, player: "p1" | "p2", lane: number): CardInstance {
  const card = g.unit(player, lane);
  if (card === null) throw new Error(`setup: ${player} should hold a unit in lane ${lane}`);
  return card;
}

function backrowAt(g: Scenario, player: "p1" | "p2", lane: number): CardInstance {
  const card = g.backrow(player, lane);
  if (card === null) throw new Error(`setup: ${player} should hold a backrow card in lane ${lane}`);
  return card;
}

function offered(g: Scenario, card: CardInstance): Extract<ActionBody, { type: "play" }>[] {
  return legalActions(g.state, "p1").filter(
    (action): action is Extract<ActionBody, { type: "play" }> => action.type === "play" && action.instanceId === card.id,
  );
}

function resolvedFace(g: Scenario, card: CardInstance): boolean | null {
  const resolved = g.events.find((event) => event.type === "cardResolved" && event.instanceId === card.id);
  return resolved?.type === "cardResolved" ? (resolved.radiant ?? null) : null;
}

describe("R213: Gifted Program's first cheap card is its controller's first of the turn", () => {
  it("R213 a Gifted Program stolen after it fired for its owner this turn still makes the thief's first cheap card Radiant (§8 Conventions, R171)", () => {
    // p1's turn begins. p1 holds #9 Moths to the Flame, worn down to 4 health; p2 holds Gifted
    // Program, a Prem Panther, and a Hinder on top of the library.
    const g = scenario({
      // Snom Bunny Mind Control (4 since patch v0.2.0, issue #40) and Stockpile (1).
      p1: {
        hand: [MIND_CONTROL, STOCKPILE],
        field: [{ def: MOTHS, lane: 1, damage: 10 }],
        library: [...LIBRARY],
      },
      p2: {
        hand: [VANILLA],
        field: [{ def: PANTHER, lane: 1 }],
        backrow: [{ def: GIFTED, lane: 1, faceUp: true }],
        library: [HINDER, VANILLA, VANILLA, VANILLA],
      },
    });
    const gifted = backrowAt(g, "p2", 1);

    // p1's start of turn: Moths makes p2's Panther attack it; the Panther destroys it and survives,
    // so p2 draws 2 — on p1's turn (R426: a forced attack is an attack). The first draw is Hinder,
    // cast as it is drawn: p2's play costing 0 (R70), made Radiant by p2's Gifted Program.
    g.startTurn();
    const hinder = g.events.find((event) => event.type === "cardPlayed" && event.defId === HINDER);
    expect(hinder).toMatchObject({ player: "p2", costPaid: 0 });
    expect(g.state.players.p1.mana.nextTurnMod).toBe(-2); // the radiant face ran: "2 lower"

    // p1 takes the Gifted Program, and has played nothing costing 1 or less this turn: Snom Bunny
    // Mind Control cost 4. Stockpile is p1's first cheap card, so it resolves its radiant text.
    g.state.players.p1.mana.current = 5;
    g.play(MIND_CONTROL, { targets: at(gifted) });
    expect(g.card(gifted).controller).toBe("p1");
    const stockpile = g.card(STOCKPILE);
    const handBefore = g.hand("p1").length;
    g.play(STOCKPILE);
    expect(g.card(stockpile).radiant).toBe(true);
    expect(g.hand("p1")).toHaveLength(handBefore - 1 + 5);
  });

  it("R213 a Gifted Program bounced and replayed does not make a second cheap card Radiant in the same turn (R174)", () => {
    const g = scenario({
      p1: {
        hand: [VANILLA, { def: SILAS, radiant: true }, STOCKPILE],
        backrow: [{ def: GIFTED, lane: 5 }],
        mana: 10,
        library: [...LIBRARY],
      },
      p2: { hand: [VANILLA], field: [{ def: VANILLA, lane: 3 }], library: [...LIBRARY] },
    });
    const gifted = g.card(GIFTED);
    const first = g.hand("p1").find((c) => c.defId === VANILLA);
    const second = g.card(STOCKPILE);
    if (first === undefined) throw new Error("setup: hand");

    g.play(first, { zone: 1 });
    expect(g.card(first).radiant).toBe(true);

    // Radiant Silas rotates right: Gifted Program in p1's backrow lane 5 would cross, so it is
    // bounced to p1's hand costing 0 (§8 #52), and p1 plays it again.
    g.play(SILAS, { zone: 2, modes: ["right"] });
    g.expectInZone(gifted, "hand");
    g.play(gifted, { zone: 4 });

    // Stockpile is the second card costing 1 or less p1 has played this turn, not the first.
    g.play(second);
    expect(g.card(second).radiant).toBe(false);
  });

  it("R213 a card costing 1 or less played before Gifted Program arrived was already the turn's first", () => {
    const g = scenario({
      p1: { hand: [STOCKPILE, GIFTED, FRIEND, RENO], field: [TIMMY], mana: 10, library: [...LIBRARY] },
      p2: { hand: [VANILLA], library: [...LIBRARY] },
    });
    const friend = g.card(FRIEND);

    g.play(STOCKPILE);
    g.play(GIFTED);
    g.play(friend);

    // Pint-Sized Summoner reads the same way: the first minion played this turn, whenever it came.
    expect(g.card(friend).radiant).toBe(false);
    g.expectStats(g.card(TIMMY), { attack: 3, maxHealth: 3 });
  });
});

describe("R214: a play's choices are the choices of the face it resolves with", () => {
  it("R214 a Pocket Chaos that radiant Gifted Program will make Radiant is offered, and may carry, the choice to skip the gift (§8 #87 radiant, R81)", () => {
    const g = scenario({
      // Patch v0.2.9 costs Pocket Chaos at (4): a −2 costMod puts the play at (2), inside
      // radiant Gifted Program's threshold, while its base cost stays (4).
      p1: { hand: [{ def: POCKET_CHAOS, costMod: -2 }, STOCKPILE], library: [STOCKPILE], backrow: [{ def: GIFTED, radiant: true }], health: 20 },
      p2: { hand: [STOCKPILE] },
    });
    const chaos = g.card(POCKET_CHAOS);

    // It costs 2, inside radiant Gifted Program's threshold, so step 3 will make it Radiant: the play
    // answers the radiant face's two mode declarations, and `legalActions` offers them.
    expect(offered(g, chaos).map((action) => action.modes)).toContainEqual(["health", "skip"]);
    expect(offered(g, chaos).map((action) => action.modes)).not.toContainEqual(["health"]);

    g.play(chaos, { modes: ["health", "skip"] });

    expect(resolvedFace(g, chaos)).toBe(true);
    g.expectHealth("p1", 30);
    expect(g.hand("p2").filter((card) => card.defId === POCKET_CHAOS)).toHaveLength(0);
    // The radiant face draws nothing since patch v0.1.1: the library's Stockpile stays there.
    expect(g.hand("p1").filter((card) => card.defId === STOCKPILE)).toHaveLength(1);
    expect(g.pile("p1", "library").map((card) => card.defId)).toEqual([STOCKPILE]);
  });

  it("R214 a 5pek Controller that Gifted Program will make Radiant switches the enemy units only when the play says so (§8 #48 radiant)", () => {
    const g = scenario({
      p1: { hand: [PEK_CONTROLLER, STOCKPILE], field: [{ def: RENO, lane: 1 }], backrow: [GIFTED] },
      p2: { hand: [STOCKPILE], field: [{ def: RENO, lane: 1 }] },
    });
    const pek = g.card(PEK_CONTROLLER);
    expect(offered(g, pek).map((action) => action.modes)).toEqual([["enemy"], ["all"]]);

    g.play(pek, { modes: ["enemy"] });

    expect(resolvedFace(g, pek)).toBe(true);
    expect(g.unit("p1", 1)?.position).toBe("ATK");
    expect(g.unit("p2", 1)?.position).toBe("DEF");
  });

  it("R214 a crafted Bigot + Twisted Sorcerer that Gifted Program will make Radiant names the Sorcerer's target alone (R90, R102)", () => {
    const g = scenario({
      seed: "craft-453", // the first Discover offers Bigot, the second Twisted Sorcerer (pools of every set, R380)
      p1: { hand: [CRAFT_A_CARD, RENO], mana: 4, backrow: [GIFTED] },
      p2: { hand: [RENO], field: [PANTHER] },
    });
    g.play(CRAFT_A_CARD);
    g.answer(BIGOT);
    g.answer(TWISTED_SORCERER);
    const card = g.hand("p1").find((held) => held.defId.startsWith("t-"));
    if (card === undefined) throw new Error("setup: the crafted card");
    const panther = unitAt(g, "p2", 1);
    const hero: Selection = { pick: "hero", player: "p2" };

    // It costs 0, so it is the first card costing 1 or less this turn: step 3 makes it Radiant, and
    // radiant Bigot destroys all enemy non-Humans with no target. The flat list is the Sorcerer's
    // alone, so a list built for the base face (Bigot's target, then the Sorcerer's) is refused.
    expect(() => g.play(card, { zone: 2, targets: [...at(panther), hero] })).toThrow();
    g.play(card, { zone: 2, targets: [hero] });

    // Radiant Twisted Sorcerer deals 8 to the target the player named for it: p2's hero, 30 → 22.
    expect(resolvedFace(g, card)).toBe(true);
    g.expectHealth("p2", 22);
  });
});

const GARY = "core-004";
const JEWELOSCO_SCARAB = "core-007";
const CARNIVOROUS_CUBE = "core-022";
const SEVEN_SEVEN = "core-025";
const BIG_FELINOR = "core-043";
const LAVA_GOLEM = "core-055";

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

describe("R214: step 3 applies the face step 1 checked, whatever step 2 put on the board", () => {
  it("R214 a Gifted Program a Tribute's Death puts on the field at step 2 does not change the face the play's choices were checked against (R213, §10.5 step 3)", () => {
    const s = scenario({
      p1: {
        hand: [CARNIVOROUS_CUBE, LAVA_GOLEM, BIGOT],
        field: [GARY, JEWELOSCO_SCARAB, VANILLA],
        backrow: [GIFTED],
        library: [RENO, RENO],
        mana: 4,
      },
      p2: { field: [BIG_FELINOR, SEVEN_SEVEN], hand: [RENO], library: [RENO] },
    });
    const sink: EngineSink = { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
    // A Unit that carries Gifted Program's text: the Gifted Program fused onto p1's Mr. Vanilla, which
    // keeps its instance and its type (R77). #22 eats Units only (R428), and this one is a Unit.
    const vanilla = must(s.unit("p1", 3), "p1's Mr. Vanilla");
    const giftedUnit = must(
      subsystems.fuse(sink, { ingredients: [vanilla, must(s.backrow("p1", 1), "p1's Gifted Program")], target: vanilla }),
      "the Mr. Vanilla carrying Gifted Program",
    );
    // #99's result, built the way `099-craft-a-card.test.ts` builds one: Lava Golem + Bigot, a
    // Unit with Tribute 3 costing 0, whose base face names an enemy non-Human unit to destroy and
    // whose radiant face destroys every enemy non-Human unit and names nothing (R102, R214).
    const crafted = must(
      subsystems.fuse(sink, { ingredients: [s.card(LAVA_GOLEM), s.card(BIGOT)], toHand: "p1" }),
      "the crafted Lava Golem + Bigot",
    );
    // #22 eats the Unit carrying the Gifted Program's text (R41, R428), so none stands on p1's side
    // any more; its Death will summon two copies of it.
    s.play(CARNIVOROUS_CUBE, { targets: [{ pick: "instance", instanceId: giftedUnit.id }] });
    s.expectInZone(giftedUnit, "graveyard");
    const cube = must(s.unit("p1", 4), "p1's Carnivorous Cube");
    const felinor = must(s.unit("p2", 1), "p2's Big Felinor");
    const sevenSeven = must(s.unit("p2", 2), "p2's 4-mana 7/7");

    // Step 1 sees no Gifted Program, so the face that answers the play's choices is the base one
    // (R214), and legalActions offers its single-target Cry with the Tribute paid by the Cube.
    const tributes = [cube.id, must(s.unit("p1", 1), "Gary").id, must(s.unit("p1", 2), "Scarab").id];
    const offered = legalActions(s.state, "p1").some(
      (action) =>
        action.type === "play" &&
        action.instanceId === crafted.id &&
        action.targets?.length === 1 &&
        action.targets[0]?.pick === "instance" &&
        action.targets[0].instanceId === felinor.id &&
        [...(action.tributes ?? [])].sort().join() === [...tributes].sort().join(),
    );
    expect(offered).toBe(true);

    // Step 2 pays the Tribute: the Cube's Death summons two copies of the Unit it ate, each carrying
    // Gifted Program's text.
    s.play(crafted, { tributes, targets: [{ pick: "instance", instanceId: felinor.id }], zone: 5 });
    expect(s.events.filter((event) => event.type === "summoned" && event.defId === giftedUnit.defId)).toHaveLength(2);

    // R214: "step 1 already knows" the face, and step 1 checked the play's choices against the base
    // face, so that is the face step 5 resolves: the chosen Big Felinor is destroyed and nothing
    // else. A Gifted Program that arrived after step 1 cannot turn it into a face whose choices no
    // step checked, destroying the 7/7 the play never named.
    expect(s.card(crafted.id).radiant).toBe(false);
    s.expectInZone(felinor, "graveyard");
    s.expectInZone(sevenSeven, "field");
  });
});
