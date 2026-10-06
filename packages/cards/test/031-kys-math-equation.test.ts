// #31 KY's Math Equation — SPEC §8.2 row 31, patch v0.2.0 (R429): "Deal Fib(times played + 1) damage
// to a target. End of turn: Return this to your hand. It costs (1) more, to a maximum of (4)."; radiant
// Fib(times played + 3) (R275).
//
// BUILD M4-T4 must-pass row 31, as R429 rewrites it: "1st play → 1 damage, 2nd → 2, 3rd → 3, 4th → 5,
// 5th → 8; clamps at 89 (R25); its cost — costMod, discounts, the cost paid — changes its price and
// never its damage (R67); the return costs (1) more, never above (4)". Times played counts this card's
// plays, the current one included, on the instance (`timesPlayed`), in every zone. The damage it
// would deal now, its R280 `preview`, is proved in test/preview.test.ts.
//
// Every side gets a unit on the board so §2.5's auto-end-turn does not run the turn on by itself
// (see the harness header): a unit with an unspent exertion is always a meaningful action.

import { describe, expect, it } from "vitest";
import { effectiveCost, timesPlayedOf } from "@jackioh/engine";
import { scenario, type Scenario } from "./_harness";
import { base, radiant } from "../src/scripts/031-kys-math-equation";

const AT_ENEMY_HERO = [{ pick: "hero", player: "p2" } as const];

/** A board where p1 can play the equation and neither side auto-ends its turn. */
function board(options: { timesPlayed?: number; radiant?: boolean; costMod?: number; health?: number } = {}): Scenario {
  const s = scenario({
    seed: "ky-math",
    p1: {
      hand: [{ def: "31", radiant: options.radiant === true, costMod: options.costMod ?? 0 }],
      field: ["15"],
      library: ["15", "15", "15", "15"],
      mana: 10,
    },
    p2: { field: ["15"], library: ["15", "15", "15", "15"], ...(options.health === undefined ? {} : { health: options.health }) },
  });
  // The plays it has had before this fixture's own (R429): what the harness cannot seed.
  if (options.timesPlayed !== undefined) s.card("31").timesPlayed = options.timesPlayed;
  return s;
}

/** Play, end p1's turn (the return), let p2's turn pass, and come back to p1's main phase. */
function playAndComeBack(s: Scenario): Scenario {
  s.play("31", { targets: AT_ENEMY_HERO });
  s.endTurn();
  s.endTurn();
  expect(s.state.active).toBe("p1");
  return s;
}

describe("#31 KY's Math Equation — base", () => {
  it("R429 its 1st play deals Fib(1 + 1) = Fib(2) = 1 damage to the target", () => {
    const s = board();
    s.play("31", { targets: AT_ENEMY_HERO });
    s.expectHealth("p2", 29);
    s.expectEvents("cardPlayed", "damage");
  });

  it("R429 §10.5 step 4 counts the play on the instance, and the count rides it through every zone", () => {
    const s = board();
    const equation = s.card("31");
    expect(timesPlayedOf(equation)).toBe(0);

    s.play(equation, { targets: AT_ENEMY_HERO });
    s.expectInZone(equation, "graveyard");
    expect(timesPlayedOf(s.card(equation))).toBe(1);

    s.endTurn(); // the return
    s.expectInZone(equation, "hand");
    expect(timesPlayedOf(s.card(equation))).toBe(1);
  });

  it("R429 its 2nd play, once it has come back, deals Fib(3) = 2", () => {
    const s = playAndComeBack(board());
    s.expectHealth("p2", 29);

    s.play("31", { targets: AT_ENEMY_HERO });

    s.expectHealth("p2", 27);
    expect(timesPlayedOf(s.card("31"))).toBe(2);
  });

  it("R429 the 3rd play deals 3, the 4th 5 and the 5th 8", () => {
    for (const [before, dealt] of [
      [2, 3],
      [3, 5],
      [4, 8],
    ] as const) {
      const s = board({ timesPlayed: before });
      s.play("31", { targets: AT_ENEMY_HERO });
      s.expectHealth("p2", 30 - dealt);
    }
  });

  it("R25 the Fib index still clamps at 11, so the damage clamps at 89", () => {
    const s = board({ timesPlayed: 20, health: 200 });
    s.play("31", { targets: AT_ENEMY_HERO });
    s.expectHealth("p2", 111);
  });

  it("R67 R429 its cost plays no part: a (4) Equation on its 1st play deals 1, and a discount changes only the price", () => {
    const pricey = board({ costMod: 3 });
    expect(effectiveCost(pricey.state, pricey.card("31"))).toBe(4);
    pricey.play("31", { targets: AT_ENEMY_HERO });
    pricey.expectMana("p1", 6);
    pricey.expectHealth("p2", 29);

    const cheap = board();
    cheap.state.players.p1.mods.push({
      id: "test-spell-discount",
      kind: "costDiscount",
      amount: 1,
      onlyType: "Spell",
      expiry: { until: "never" },
    });
    cheap.play("31", { targets: AT_ENEMY_HERO });
    cheap.expectMana("p1", 10);
    cheap.expectHealth("p2", 29);
  });

  it("R429 R78 end of turn: it returns to hand and costs (1) more, for good", () => {
    const s = board();
    const equation = s.card("31");
    s.play("31", { targets: AT_ENEMY_HERO });
    s.expectInZone(equation, "graveyard");

    s.endTurn();

    s.expectInZone(equation, "hand");
    expect(s.card(equation).costMod).toBe(1);
    expect(effectiveCost(s.state, s.card(equation))).toBe(2);
  });

  it("R429 the return never lifts its cost above (4): from (3) it reaches (4), and at (4) it adds nothing", () => {
    const three = board({ costMod: 2 });
    three.play("31", { targets: AT_ENEMY_HERO });
    three.endTurn();
    three.expectInZone("31", "hand");
    expect(effectiveCost(three.state, three.card("31"))).toBe(4);

    const four = board({ costMod: 3 });
    four.play("31", { targets: AT_ENEMY_HERO });
    four.endTurn();
    four.expectInZone("31", "hand");
    expect(four.card("31").costMod).toBe(3);
    expect(effectiveCost(four.state, four.card("31"))).toBe(4);
  });

  it("R429, R65 a (0) Equation — #95's `costOverride` — comes back costing (1)", () => {
    const s = board();
    s.card("31").costOverride = 0;
    s.play("31", { targets: AT_ENEMY_HERO });
    s.expectMana("p1", 10);
    s.endTurn();
    expect(effectiveCost(s.state, s.card("31"))).toBe(1);
  });

  it("R429, R30 an Echo repeat is the same play: Twinspell's repeat deals the same Fib(2) again, and the count is 1", () => {
    const s = scenario({
      seed: "ky-math-echo",
      p1: { hand: ["31"], field: ["15"], backrow: ["79"], library: ["15", "15"], mana: 10 },
      p2: { field: ["15"], library: ["15", "15"] },
    });
    s.play("31", { targets: AT_ENEMY_HERO });
    // The repeat asks its target afresh (§10.6).
    expect(s.state.pending?.kind).toBe("target");
    const labels = s.state.pending?.options.map((o) => o.label) ?? [];
    expect(labels).toContain("Enemy hero");
    expect(labels.some((l) => /^p[12]\b/.test(l))).toBe(false);
    expect(s.state.pending?.options.map((o) => o.key)).toContain("hero:p2");
    s.answer("hero:p2");

    s.expectHealth("p2", 28);
    expect(timesPlayedOf(s.card("31"))).toBe(1);
  });

  it("the return belongs to the turn it was played on, not to every graveyard copy", () => {
    // A copy that was already in the graveyard when the turn began was not played this turn, so
    // ending the turn leaves it there — otherwise it would climb a cost every turn for the game.
    const s = scenario({
      seed: "ky-math-stale",
      p1: { hand: ["15"], field: ["15"], graveyard: ["31"], library: ["15", "15"] },
      p2: { field: ["15"], library: ["15", "15"] },
    });
    const stale = s.card("31");

    s.endTurn();

    s.expectInZone(stale, "graveyard");
    expect(s.card(stale).costMod).toBe(0);
  });

  it("R429 both faces ask the engine to count their plays", () => {
    expect(base.staticFlags?.countsPlays).toBe(true);
    expect(radiant.staticFlags?.countsPlays).toBe(true);
  });
});

describe("#31 KY's Math Equation — radiant", () => {
  it("R429 radiant: the 1st play deals Fib(1 + 3) = Fib(4) = 3", () => {
    const s = board({ radiant: true });
    s.play("31", { targets: AT_ENEMY_HERO });
    s.expectHealth("p2", 27);
  });

  it("R429 radiant: the 2nd play deals Fib(5) = 5, the 3rd Fib(6) = 8", () => {
    const second = board({ radiant: true, timesPlayed: 1 });
    second.play("31", { targets: AT_ENEMY_HERO });
    second.expectHealth("p2", 25);

    const third = board({ radiant: true, timesPlayed: 2 });
    third.play("31", { targets: AT_ENEMY_HERO });
    third.expectHealth("p2", 22);
  });

  it("R25 radiant clamps at 89 too", () => {
    const s = board({ radiant: true, timesPlayed: 20, health: 200 });
    s.play("31", { targets: AT_ENEMY_HERO });
    s.expectHealth("p2", 111);
  });

  it("§8 Conventions: the radiant face changes only the offset, so the return clause and its cap are kept", () => {
    const s = playAndComeBack(board({ radiant: true }));
    const equation = s.card("31");
    expect(s.card(equation).radiant).toBe(true);
    expect(effectiveCost(s.state, s.card(equation))).toBe(2);
    s.expectHealth("p2", 27);

    // The 2nd play: Fib(2 + 3) = 5.
    s.play(equation, { targets: AT_ENEMY_HERO });
    s.expectHealth("p2", 22);
  });
});
