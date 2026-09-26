// #78 /fullsend — SPEC §8.3, R62, R65, R364, §2.3, §10.5 step 5.
//
// BUILD M4-T4: "Refresh 3 mana (R364); −1 cost this turn; hand exiled at end of turn; radiant also
// draws 1 for each card played this turn" (patch v0.1.1).
//
// The discount is read off the hand's own cost, which `viewFor` computes with `mana.effectiveCost`
// (§10.8), and confirmed by paying. R65's X-cost clause is proved by playing an X card for exactly
// X while the discount is live. R62's position for the delayed exile is proved by the event order:
// the exile lands before `turnEnded`, which `turn.ts` emits immediately before cleanup.

import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "./_harness";

const FULLSEND = "core-078";

const COST_0 = "core-010"; // Rapid Replenish — a discount floors at 0 (R65)
const COST_3 = "core-017"; // Flood
const COST_4 = "core-025"; // 4-mana 7/7
const X_CARD = "core-074"; // Adaptive UI, printed cost X

/**
 * R81: #74 Adaptive UI declares one target with its play ("Deal X damage to a target"), so the
 * `play` action has to carry it or the engine refuses the play. It is not part of what this file
 * is about — the enemy hero is always a legal pick and takes the X damage off to one side.
 */
const AT_ENEMY_HERO = [{ pick: "hero", player: "p2" } as const];

const LIBRARY = ["core-008", "core-008", "core-008", "core-008"] as const;

/** The cost a card in the viewer's own hand shows now (§10.8, R65). */
function handCost(s: Scenario, defId: string): number {
  const hand = s.view("p1").you.hand;
  if (!Array.isArray(hand)) throw new Error("§10.8: the viewer's own hand is a list of cards");
  const card = hand.find((entry) => entry.defId === defId);
  if (card === undefined) throw new Error(`${defId} is not in p1's hand`);
  return card.cost;
}

/** Harness gap (reported): no `mods()` accessor, so the test reads `state` — B1.7 covers `src` only. */
function modsOf(s: Scenario, kind: string): unknown[] {
  return s.state.players.p1.mods.filter((mod) => mod.kind === kind);
}

function board(radiant: boolean): Scenario {
  return scenario({
    p1: {
      hand: [{ def: FULLSEND, radiant }, COST_0, COST_3, COST_4, X_CARD],
      library: [...LIBRARY],
    },
    // R82: the opponent keeps something to do, so `endTurn()` does not cascade.
    p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
  });
}

describe("#78 /fullsend — base", () => {
  it("R364 refreshes 3 mana: paying 4 of 4 leaves 0, and 3 come back", () => {
    const s = board(false);

    s.expectMana("p1", 4);
    s.play(FULLSEND);

    s.expectMana("p1", 3);
    s.expectEvents("cardPlayed", "manaChanged");
  });

  it("R364 a Refresh never goes past max: temporary mana above it is not topped up", () => {
    const s = scenario({
      p1: { hand: [FULLSEND, COST_0], library: [...LIBRARY], mana: 6 },
      p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
    });

    // 6 of max 4, pay 4: 2 left, and the Refresh stops at max 4 — a gain of 3 would have made 5.
    s.play(FULLSEND);

    s.expectMana("p1", 4);
  });

  it("this turn your cards cost 1 less, flooring at 0 (R65)", () => {
    const s = board(false);

    s.play(FULLSEND);

    expect(handCost(s, COST_4)).toBe(3);
    expect(handCost(s, COST_3)).toBe(2);
    expect(handCost(s, COST_0)).toBe(0);
    expect(modsOf(s, "costDiscount")).toHaveLength(1);
    expect(s.state.players.p1.mods.find((mod) => mod.kind === "costDiscount")).toMatchObject({
      kind: "costDiscount",
      amount: 1,
      expiry: { until: "thisTurn", turn: 9 },
    });
  });

  it("the discount is really charged: a cost-4 card is paid at 3", () => {
    const s = board(false);
    s.play(FULLSEND);
    s.expectMana("p1", 3);

    s.play(COST_4);

    s.expectMana("p1", 0);
  });

  it("R65 an X-cost card costs exactly X: the discount does not cheapen it", () => {
    const s = board(false);
    s.play(FULLSEND);
    s.expectMana("p1", 3);

    s.play(X_CARD, { x: 2, targets: AT_ENEMY_HERO });

    // Exactly 2, not 1: "costMod and discounts don't change it" (R65).
    s.expectMana("p1", 1);
  });

  it('patch v0.1.1: the base face installs no "Combo: Draw 1", so a play afterwards draws nothing', () => {
    const s = board(false);
    s.play(FULLSEND);
    expect(modsOf(s, "comboDraw")).toHaveLength(0);
    const libraryBefore = s.pile("p1", "library").length;

    s.play(COST_0);

    expect(s.pile("p1", "library")).toHaveLength(libraryBefore);
  });

  it("R62 the hand is exiled at end of turn, before cleanup", () => {
    const s = board(false);
    s.play(FULLSEND);
    const left = s.pile("p1", "hand").map((card) => card.defId);
    expect(left.length).toBeGreaterThan(0);

    s.endTurn();

    expect(s.pile("p1", "hand")).toHaveLength(0);
    expect(s.pile("p1", "exile").map((card) => card.defId).sort()).toEqual([...left].sort());

    // R62 places the exile between the trap window and cleanup, and the log says so — but not by
    // straddling `turnEnded`. `turn.ts` emits that event at the TOP of the window rather than at
    // cleanup, because the window's traps read it (#18 Bread and Butter answers
    // `event.unspentMana`). So the exile comes after `turnEnded`, and cleanup comes after the exile —
    // cleanup being visible as the `modifierChanged` that retires /fullsend's own "this turn" discount.
    const types = s.events.map((event) => event.type);
    const windowOpened = types.indexOf("turnEnded");
    const lastExile = types.lastIndexOf("exiled");
    const cleanupAt = s.events.findIndex(
      (event) => event.type === "modifierChanged" && event.added === false,
    );

    expect(windowOpened).toBeGreaterThanOrEqual(0);
    expect(lastExile).toBeGreaterThan(windowOpened);
    expect(cleanupAt).toBeGreaterThan(lastExile);
    s.expectEvents("cardPlayed", "turnEnded", "exiled", "turnStarted");
  });

  it('§2.2 the "this turn" discount is gone after cleanup', () => {
    const s = board(false);
    s.play(FULLSEND);

    s.endTurn();

    expect(modsOf(s, "costDiscount")).toHaveLength(0);
  });
});

/**
 * R169, BUILD M5-T4 ("badge list equals the view's modifiers"): /fullsend's riders are badges, on
 * both seats, from the moment the spell resolves until the cleanup that retires them.
 */
describe("#78 /fullsend — visible to the player while active (R169, §10.8)", () => {
  it("shows the discount as a badge the moment the spell resolves", () => {
    const s = board(false);

    s.play(FULLSEND);

    const badges = s.view("p1").you.modifiers;
    expect(badges.map((modifier) => modifier.label)).toEqual(["Your cards cost 1 less"]);
    // The ids are the engine's, so a `modifierChanged` animation lands on the badge it names.
    expect(badges.map((modifier) => modifier.id)).toEqual(s.state.players.p1.mods.map((mod) => mod.id));
  });

  it("the badges go at cleanup, with the modifiers they stand for (§2.2)", () => {
    const s = board(true);
    s.play(FULLSEND);
    expect(s.view("p1").you.modifiers).toHaveLength(2);

    s.endTurn();

    expect(s.view("p1").you.modifiers).toEqual([]);
  });

  it("the opponent sees them too: playing a Spell is public (§10.5 step 4)", () => {
    const s = board(true);

    s.play(FULLSEND);

    expect(s.view("p2").opponent.modifiers.map((modifier) => modifier.label)).toEqual([
      "Your cards cost 1 less",
      'Your cards gain "Combo: draw 1"',
    ]);
  });
});

describe("#78 /fullsend — radiant", () => {
  it("R364 radiant refreshes 3 mana and its cards still cost only 1 less (patch v0.1.1)", () => {
    const s = board(true);

    s.play(FULLSEND);

    s.expectMana("p1", 3);
    expect(handCost(s, COST_4)).toBe(3);
    expect(handCost(s, COST_3)).toBe(2);
    expect(handCost(s, COST_0)).toBe(0);
  });

  it('installs the "Combo: Draw 1" rider as a turn-scoped player modifier (§10.5 step 5)', () => {
    const s = board(true);

    s.play(FULLSEND);

    expect(modsOf(s, "comboDraw")).toHaveLength(1);
    expect(s.state.players.p1.mods.find((mod) => mod.kind === "comboDraw")).toMatchObject({
      kind: "comboDraw",
      amount: 1,
      expiry: { until: "thisTurn", turn: 9 },
    });
  });

  it("each card played this turn draws 1 (§10.5 step 5)", () => {
    const s = board(true);
    s.play(FULLSEND);
    const libraryBefore = s.pile("p1", "library").length;
    const handBefore = s.pile("p1", "hand").length;

    s.play(COST_0);

    // One draw for the play: the library is one shorter, and the hand is the played card plus the
    // drawn one.
    expect(s.pile("p1", "library")).toHaveLength(libraryBefore - 1);
    expect(s.pile("p1", "hand")).toHaveLength(handBefore - 1 + 1);
  });

  it("R65 radiant does not cheapen an X-cost card either", () => {
    const s = board(true);
    s.play(FULLSEND);

    s.play(X_CARD, { x: 2, targets: AT_ENEMY_HERO });

    s.expectMana("p1", 1);
  });

  it("R62 radiant still exiles the hand at end of turn, and both riders are gone after cleanup", () => {
    const s = board(true);
    s.play(FULLSEND);
    const left = s.pile("p1", "hand").map((card) => card.defId);

    s.endTurn();

    expect(s.pile("p1", "hand")).toHaveLength(0);
    expect(s.pile("p1", "exile").map((card) => card.defId).sort()).toEqual([...left].sort());
    expect(modsOf(s, "costDiscount")).toHaveLength(0);
    expect(modsOf(s, "comboDraw")).toHaveLength(0);
  });
});
