// BUILD M8: §5.2 swaps a field card's §10.4 printed stats without rebuilding it, so its damage remains.
// R60 makes Knockoff Temu deterministic after #94 leaves Mr. Vanilla as the sole non-Radiant candidate.
// BUILD M5-T4: use `radiantSet` for the animation; `contain.text` is necessary until testids has stat selectors.
// `cy.playByName` settles before the animation; Knockoff Temu's click is expanded to observe it.

import { CARDS, CARD_NAMES, cardId as catalogId } from "../../support/cards.ts";
import { seedFor } from "../../support/config.ts";
import { RADIANT, cardId, handCardId, ts } from "../../support/testids.ts";
import type { GameStateLike, Lane, PlayerId } from "../../support/types.ts";

/** BUILD M8 seed: the required #8, #26, #31 and #94 draws arrive before §2.3's mana cap. */
const SEED = seedFor("11-radiant-442");

const TURN_BUDGET = 34;
const UNIT_LANE: Lane = 1;

const GENNS_GREED = catalogId(94);
const MATH_EQUATION = catalogId(31);

/** SPEC §8 #8 Mr. Vanilla's base and Radiant stats. */
const VANILLA = { attack: 4, health: 4 } as const;
const VANILLA_RADIANT = { attack: 12, health: 12 } as const;
/** SPEC §8 #31 deals Fib(2) = 1 at its printed cost (R25). */
const MATH_DAMAGE = 1;

/** Costs come from SPEC §8; §2.3 caps max mana at 4, so each of these waits for its own turn. */
const COSTS: Record<string, number> = {
  [CARDS.mrVanilla]: 1,
  [CARDS.glowyJellyBean]: 3,
  [CARDS.knockoffTemu]: 2,
  [GENNS_GREED]: 4,
  [MATH_EQUATION]: 1,
};

/** SPEC §8 card names are what BUILD M5-T1 renders. */
function nameOf(index: number): string {
  const name = CARD_NAMES[index];
  if (name === undefined) throw new Error(`no SPEC §8 card #${index}`);
  return name;
}

/** The state peek drives actions; assertions against player-visible state use `viewFor` (CLAUDE.md rule 7). */
type Held = { id: string; defId: string; radiant?: boolean };
type FieldUnit = Held & { damage?: number; buffs?: { attack: number; health: number } };
type SidePeek = {
  mana?: { current: number };
  hand?: Held[];
  library?: Held[];
  units?: (FieldUnit[] | null)[];
  backrow?: (Held | null)[];
};

function peek(state: GameStateLike, player: PlayerId): SidePeek {
  return state.players[player] as SidePeek;
}

function handOf(state: GameStateLike, player: PlayerId): Held[] {
  return peek(state, player).hand ?? [];
}

/** §3.2: only a unit pile's top card is active. */
function unitsOf(state: GameStateLike, player: PlayerId): FieldUnit[] {
  return (peek(state, player).units ?? []).flatMap((pile) => {
    const top = pile === null ? undefined : pile[0];
    return top === undefined ? [] : [top];
  });
}

function unitById(state: GameStateLike, player: PlayerId, instanceId: string): FieldUnit {
  const unit = unitsOf(state, player).find((candidate) => candidate.id === instanceId);
  expect(unit, `${instanceId} is on ${player}'s field`).to.not.eq(undefined);
  return unit ?? { id: instanceId, defId: "" };
}

/** BUILD M5-T3: hand the hotseat device to the acting seat. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat names the seat holding the device").to.not.eq(undefined);
    if (handle.seat !== player) cy.handOver();
  });
}

function passTurn(): void {
  cy.gameState().then((state) => {
    if (state.result !== null) return;
    ensureSeat(state.active);
    cy.endTurn();
  });
}

/** Take turns until `ready`, leaving the device with the actor. */
function advanceUntil(label: string, ready: (state: GameStateLike) => boolean): void {
  const step = (left: number): void => {
    cy.gameState().then((state) => {
      expect(state.result, `${label}: the game ended first`).to.eq(null);
      if (ready(state)) return;
      expect(left, `${label}: not reached inside ${TURN_BUDGET} player-turns`).to.be.greaterThan(0);
      passTurn();
      step(left - 1);
    });
  };
  step(TURN_BUDGET);
}

/** Wait for the player to hold `defId` and afford its SPEC §8 cost. */
function waitToPlay(player: PlayerId, defId: string): void {
  const cost = COSTS[defId] ?? 0;
  advanceUntil(`${player} can pay ${cost} for ${defId}`, (state) => {
    if (state.active !== player) return false;
    if ((peek(state, player).mana?.current ?? 0) < cost) return false;
    return handOf(state, player).some((card) => card.defId === defId);
  });
  ensureSeat(player);
}

/** The support map has no stat selector, so shown health uses rendered `{health}/{maxHealth}` text. */
function expectShownHealth(instanceId: string, health: number, maxHealth: number): void {
  cy.get(ts(cardId(instanceId))).should("contain.text", `${health}/${maxHealth}`);
}

describe("BUILD M8 11 — Glowy Jelly Bean on a hand card, Knockoff Temu on a damaged field unit", () => {
  beforeEach(() => {
    cy.seedGame({ seed: SEED, a: "11-radiant-a", b: "11-radiant-b" });
  });

  it("§5.2 flips a hand card and a damaged field unit, keeping the damage on the field card", () => {
    // Damage Mr. Vanilla before testing its Radiant flip.
    waitToPlay("p1", CARDS.mrVanilla);
    cy.playByName(nameOf(8), { zone: { side: "you", row: "units", lane: UNIT_LANE } });

    cy.instanceAt("p1", "units", UNIT_LANE).then((vanillaId) => {
      cy.get(`${ts(cardId(vanillaId))}${RADIANT}`).should("not.exist");
      expectShownHealth(vanillaId, VANILLA.health, VANILLA.health);

      waitToPlay("p2", MATH_EQUATION);
      // R81: the declared target travels in the play action.
      cy.playByName(nameOf(31), { answers: [{ kind: "target", cards: [vanillaId] }] });

      cy.gameState().then((damaged) => {
        expect(unitById(damaged, "p1", vanillaId).damage, "§4.4: the spell dealt Fib(2) = 1").to.eq(
          MATH_DAMAGE,
        );
      });
      // §4.1: current health subtracts persistent damage.
      ensureSeat("p1");
      expectShownHealth(vanillaId, VANILLA.health - MATH_DAMAGE, VANILLA.health);

      // §8 #26's R81 declared `hand` choice remains its own §10.6 picker kind.
      waitToPlay("p1", CARDS.glowyJellyBean);
      cy.gameState().then((state) => {
        // Avoid Radiant #94, #28, and Mr. Vanilla: each changes this test's setup.
        const steered = [CARDS.mrVanilla, CARDS.glowyJellyBean, CARDS.knockoffTemu, GENNS_GREED];
        const target = handOf(state, "p1").find((card) => !steered.includes(card.defId));
        expect(target, "seat 1 holds a card for Glowy Jelly Bean to convert").to.not.eq(undefined);
        if (target === undefined) return;

        cy.get(`${ts(handCardId(target.id))}${RADIANT}`).should("not.exist");
        // Use def id: §8 gives #26, #27, and #28 ambiguous names.
        cy.instanceInHand("p1", CARDS.glowyJellyBean).then((beanId) => {
          cy.playCard(beanId, { answers: [{ kind: "hand", cards: [target.id] }] });
        });

        // BUILD M5-T4's `radiantSet` confirms §5.2's hand flip.
        cy.get(`${ts(handCardId(target.id))}${RADIANT}`).should("exist");
        cy.gameState().then((after) => {
          const held = handOf(after, "p1").find((card) => card.id === target.id);
          expect(held?.radiant, "§5.2: Radiant is a flag on the instance").to.eq(true);
        });
      });

      // #94 leaves Knockoff Temu as the sole candidate.
      waitToPlay("p1", GENNS_GREED);
      cy.playByName(nameOf(94));

      cy.gameState().then((state) => {
        const side = peek(state, "p1");
        expect(side.library ?? [], "#94 exiled the last of the library (every other card is odd-cost)")
          .to.have.length(0);
        expect(
          handOf(state, "p1").map((card) => card.defId),
          "#94 drew the deck's only 2-cost card and exiled the rest of the hand",
        ).to.deep.eq([CARDS.knockoffTemu]);
        expect(
          unitsOf(state, "p1").map((unit) => unit.id),
          "Mr. Vanilla is the only card seat 1 has on the field",
        ).to.deep.eq([vanillaId]);
        expect(
          (side.backrow ?? []).filter((slot) => slot !== null),
          "seat 1's backrow is empty, so the union holds nothing else",
        ).to.have.length(0);
        expect(unitById(state, "p1", vanillaId).radiant, "Mr. Vanilla is still on its base face").to.eq(
          false,
        );
        expect(side.mana?.current, "#94's +2 mana pays for Knockoff Temu on the same turn").to.be.at.least(
          COSTS[CARDS.knockoffTemu] ?? 2,
        );
      });

      // #28 has one candidate and no play-time choice; expand its click to observe the animation.
      cy.instanceInHand("p1", CARDS.knockoffTemu).then((temuId) => {
        cy.get(ts(handCardId(temuId))).click();

        // BUILD M5-T4 `radiantSet` must be observable before settlement.
        cy.expectAnimating("radiantSet");
        cy.settled();

        cy.get(`${ts(cardId(vanillaId))}${RADIANT}`).should("exist");

        expectShownHealth(vanillaId, VANILLA_RADIANT.health - MATH_DAMAGE, VANILLA_RADIANT.health);
        cy.gameState().then((after) => {
          const unit = unitById(after, "p1", vanillaId);
          expect(unit.radiant, "§5.2: the flag is set on the same instance").to.eq(true);
          expect(unit.damage, "§5.2: damage taken is kept across the flip").to.eq(MATH_DAMAGE);
          expect(unit.buffs, "§5.2: buffs are kept too, and this unit never had any").to.deep.eq({
            attack: 0,
            health: 0,
          });
          // §10.1: a converted instance retains its id.
          expect(unit.defId, "the instance was converted in place (§5.2)").to.eq(CARDS.mrVanilla);
        });
      });
    });
  });
});
