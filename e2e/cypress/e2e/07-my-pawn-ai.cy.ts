// BUILD M8 07: #96 My Pawn (§8.5) cancels a lethal declaration and lets the §10.7 AI finish it.
// R44 defines lethal; the fixture excludes armor #84/#1/#73. R99 keeps the trap armed until lethal.
// R121 makes forced attackers #9/#60 ineligible; attacks here are player-declared. R33/§10.8 hide
// the face-down trap from seat 1. Use testids and settled/retried waits, never fixed sleeps.
// The fixture depends on the §9.4 L6 catalog contract and the §4.2 step-4 trap window (B-1).
// Lethal uses clicks instead of `cy.attack` because its `cy.settled()` would hide queued AI animation.

import { CARD_NAMES, cardId as catalogId } from "../../support/cards.ts";
import { seedFor, timeouts } from "../../support/config.ts";
import {
  ANIMATING,
  END_TURN,
  ILLEGAL,
  OFFER_DRAW,
  animating,
  cardId,
  handCardId,
  heroId,
  ts,
} from "../../support/testids.ts";
import type { GameStateLike, Lane, PlayerId } from "../../support/types.ts";

/** BUILD M8 seed: #96 opens for p2 and #20 makes a grinder affordable before R2's cap. */
const SEED = seedFor("07-my-pawn-10");

/** Budgets: R2 caps the game at 30 player-turns, so nothing here may loop forever. */
const TURN_BUDGET = 34;
const SWING_BUDGET = 8;

const TRAP_LANE: Lane = 3;
const GRINDER_LANE: Lane = 1;

/** SPEC §8 printed attacks; no #14 aura or buff means §10.4 layer 1 is sufficient. */
const GRINDERS: readonly { defId: string; name: string; attack: number; cost: number }[] = [
  { defId: catalogId(19), name: nameOf(19), attack: 9, cost: 3 },
  { defId: catalogId(20), name: nameOf(20), attack: 7, cost: 2 },
  { defId: catalogId(25), name: nameOf(25), attack: 7, cost: 4 },
];

/** SPEC §8's name for card #index, which is what the client prints on a card (BUILD M5-T1). */
function nameOf(index: number): string {
  const name = CARD_NAMES[index];
  if (name === undefined) throw new Error(`no SPEC §8 card #${index}`);
  return name;
}

/** Engine peeks choose actions; player-visible assertions use `viewFor` (CLAUDE.md rule 7). */
type SidePeek = {
  hero?: { health: number };
  mana?: { current: number };
  hand?: { id: string; defId: string }[];
  units?: ({ id: string; summonedTurn?: number; exertion?: { attacked: boolean } }[] | null)[];
  backrow?: ({ id: string } | null)[];
  graveyard?: { defId: string }[];
};

function peek(state: GameStateLike, player: PlayerId): SidePeek {
  return state.players[player] as SidePeek;
}

function heroHealth(state: GameStateLike, player: PlayerId): number {
  const health = peek(state, player).hero?.health;
  expect(health, `${player}'s hero health`).to.be.a("number");
  return health ?? 0;
}

function handOf(state: GameStateLike, player: PlayerId): { id: string; defId: string }[] {
  return peek(state, player).hand ?? [];
}

/** §3.2: only the top card of a unit pile is active, so that is the one this spec ever clicks. */
function unitAt(state: GameStateLike, player: PlayerId, lane: Lane) {
  return (peek(state, player).units ?? [])[lane - 1]?.[0];
}

/** BUILD M5-T3: a hotseat device is handed over, so make sure it is on the seat that has to act. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat names the seat holding the device").to.not.eq(undefined);
    if (handle.seat !== player) cy.handOver();
  });
}

/** R82 guarantees active players can end turn; a disabled control is a real failure. */
function passTurn(): void {
  cy.gameState().then((state) => {
    if (state.result !== null) return;
    ensureSeat(state.active);
    cy.endTurn();
  });
}

/** Take turns until `ready` holds, then leave the device on the seat that has to act. */
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

describe("BUILD M8 07 — My Pawn cancels the lethal swing and an AI plays out the rest of the turn", () => {
  beforeEach(() => {
    cy.seedGame({ seed: SEED, a: "07-lethal-a", b: "07-my-pawn-b" });
  });

  it("R99/R44 cancels the attack, locks P1 out, animates the AI turn and ends it", () => {
    // 1. Seat 2 sets My Pawn face-down.
    const myPawn = catalogId(96);
    advanceUntil(
      "p2 holds My Pawn on its own turn",
      (state) => state.active === "p2" && handOf(state, "p2").some((card) => card.defId === myPawn),
    );
    ensureSeat("p2");
    cy.playByName(nameOf(96), { zone: { side: "you", row: "backrow", lane: TRAP_LANE } });

    // §10.8/R33 hide the face-down trap's id from seat 1, so capture it here.
    cy.instanceAt("p2", "backrow", TRAP_LANE).then((trapId) => {
      // 2. Seat 1 summons an available grinder.
      advanceUntil("p1 can summon a grinder", (state) => {
        if (state.active !== "p1") return false;
        const mana = peek(state, "p1").mana?.current ?? 0;
        return handOf(state, "p1").some((card) =>
          GRINDERS.some((g) => g.defId === card.defId && g.cost <= mana),
        );
      });
      ensureSeat("p1");

      cy.gameState().then((state) => {
        const mana = peek(state, "p1").mana?.current ?? 0;
        const grinder = GRINDERS.find(
          (g) => g.cost <= mana && handOf(state, "p1").some((card) => card.defId === g.defId),
        );
        expect(grinder, "one of 07-lethal-a's three grinders is in hand and affordable").to.not.eq(
          undefined,
        );
        if (grinder === undefined) return;

        cy.playByName(grinder.name, { zone: { side: "you", row: "units", lane: GRINDER_LANE } });

        cy.instanceAt("p1", "units", GRINDER_LANE).then((attackerId) => {
          // 3. Below R44 lethal, R99 must leave the trap armed; §4.2 makes the hero the only target.
          const swing = (left: number): void => {
            cy.gameState().then((state2) => {
              expect(state2.result, "the grind did not end the game").to.eq(null);
              if (heroHealth(state2, "p2") <= grinder.attack) return; // the next one is lethal
              expect(left, "the hero came into lethal range inside the swing budget").to.be.greaterThan(0);

              advanceUntil("p1 can swing again", (s) => {
                if (s.active !== "p1") return false;
                const unit = unitAt(s, "p1", GRINDER_LANE);
                // §4.1: summoning sickness on the turn it entered, one exertion per turn after.
                return unit !== undefined && unit.summonedTurn !== s.turn && unit.exertion?.attacked !== true;
              });
              ensureSeat("p1");

              cy.gameState().then((atSwing) => {
                const before = heroHealth(atSwing, "p2");
                cy.attack(attackerId, { hero: "opponent" });

                cy.gameState().then((after) => {
                  // R44's projection matches this unmitigated nonlethal hit.
                  expect(heroHealth(after, "p2"), "a non-lethal swing deals its full attack").to.eq(
                    before - grinder.attack,
                  );
                  // R99: the condition was not met, so the trap was NOT spent by the event.
                  expect(
                    (peek(after, "p2").backrow ?? [])[TRAP_LANE - 1]?.id,
                    "R99: a non-lethal declaration leaves My Pawn armed in its zone",
                  ).to.eq(trapId);
                  expect(
                    (peek(after, "p2").graveyard ?? []).some((card) => card.defId === myPawn),
                    "R99: a non-lethal declaration does not send My Pawn to the graveyard",
                  ).to.eq(false);
                });
                // R33: still face-down and nameless to seat 1.
                cy.get(ts(cardId(trapId))).should("not.exist");

                swing(left - 1);
              });
            });
          };
          swing(SWING_BUDGET);

          // 4. R44 now reaches lethal; §4.2 fires the trap after exertion.
          advanceUntil("p1 can declare the lethal swing", (s) => {
            if (s.active !== "p1") return false;
            const unit = unitAt(s, "p1", GRINDER_LANE);
            return unit !== undefined && unit.summonedTurn !== s.turn && unit.exertion?.attacked !== true;
          });
          ensureSeat("p1");

          cy.gameState().then((before) => {
            const health = heroHealth(before, "p2");
            expect(health, "R44: the projected damage now reaches the hero").to.be.at.most(grinder.attack);
            const turn = before.turn;

            // Do not settle `cy.attack`: §4.2's two clicks expose the queued animation.
            cy.get(ts(cardId(attackerId))).click();
            cy.get(ts(heroId("opponent"))).click();

            // BUILD M5-T4 animates the visible attacker; face-down `trapFired` has no declarer-side card.
            cy.expectAnimating("attackCancelled");

            // §10.7 AI actions replay from one `reduce`; await its M5-T4 turn-ended animation.
            cy.get(ANIMATING).should("exist");
            cy.get(animating("turnEnded"), { timeout: timeouts.view }).should("exist");
            cy.settled();

            // §8.5 leaves p1 with no legal action; BUILD M5-T2 reflects `legalActions`.
            cy.get(ts(END_TURN)).should("be.disabled");
            cy.get(ts(OFFER_DRAW)).should("be.disabled");
            cy.gameState().then((after) => {
              const held = handOf(after, "p1")[0];
              expect(held, "seat 1 still holds cards it cannot play").to.not.eq(undefined);
              if (held !== undefined) {
                cy.get(`${ts(handCardId(held.id))}${ILLEGAL}`).should("exist");
              }
            });

            cy.gameState().should((after) => {
              // The cancelled hit never happened.
              expect(heroHealth(after, "p2"), "the cancelled attack dealt no damage").to.eq(health);
              expect(after.result, "the cancelled swing did not end the game").to.eq(null);
              // §5.1 spends the trap to its owner's graveyard.
              expect(
                (peek(after, "p2").backrow ?? [])[TRAP_LANE - 1],
                "My Pawn left the backrow when it fired",
              ).to.eq(null);
              expect(
                (peek(after, "p2").graveyard ?? []).some((card) => card.defId === myPawn),
                "My Pawn is in its owner's graveyard (§5.1: a Trap goes there after it fires)",
              ).to.eq(true);
              // The AI ended seat 1's turn.
              expect(after.turn, "the player-turn advanced").to.be.greaterThan(turn);
              expect(after.active, "the turn ended and seat 2 is to act").to.eq("p2");
            });
          });
        });
      });
    });
  });
});
