// BUILD M8: each picker is answered whether it is a `play` choice or a `PendingChoice` (R81, R123; §10.6).
// Mulligans are simultaneous per seat (R9, R265; §2.1, §10.2); short mode menus use Discover.
// Seeded one-kind specs avoid the hand cap (R4; §2.4).
// Fixtures: #7 Jewelosco Scarab, #15 Me and Mr Token, #17 Flood, #24 Efficiency Dividend, and
// #30 Archivist cover choices; #46 Suppressive Aura, #55 Lava Golem, and #67 Zoomerbin Oomen cover zones.

import { seedFor } from "../../support/config.ts";
import {
  GAME,
  MULLIGAN_OPPONENT_READY,
  MULLIGAN_OPPONENT_STATUS,
  PROMPT,
  RADIANT,
  cardId,
  graveyardCountId,
  handCardId,
  heroId,
  promptOf,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { GameStateLike, PlayerId, PromptKind } from "../../support/types.ts";

const SEEDS = {
  mulligan: seedFor("02-mulligan-0"),
  zone: seedFor("02-zone-0"),
  discover: seedFor("02-discover-1"),
  target: seedFor("02-target-70"),
  hand: seedFor("02-hand-1"),
  mode: seedFor("02-mode-16"),
  tribute: seedFor("02-tribute-1359"),
  direction: seedFor("02-direction-5"),
  x: seedFor("02-x-5"),
  embiggen: seedFor("02-embiggen-0"),
};

/** BUILD M5-T4 stat hooks. */
const attackIs = (n: number): string => `[data-attack="${n}"]`;
const healthIs = (n: number): string => `[data-health="${n}"]`;
const maxHealthIs = (n: number): string => `[data-max-health="${n}"]`;

/** Raw state chooses a card; assertions use `viewFor` (CLAUDE.md rule 7). */
function handOf(state: GameStateLike, player: PlayerId): string[] {
  const side = state.players[player] as { hand?: { id: string; defId: string }[] };
  return (side.hand ?? []).map((card) => card.id);
}

/** Discover options are random, so inspect its `PendingChoice` (R60; §6.3, §10.6). */
type PendingPeek = NonNullable<GameStateLike["pending"]> & {
  options?: { key: string; label?: string; selection?: { pick: string; option?: string } }[];
};

function pendingOptions(): Cypress.Chainable<NonNullable<PendingPeek["options"]>> {
  return cy.gameState().then((state) => {
    const pending = state.pending === null ? null : (state.pending as PendingPeek);
    expect(pending, "a PendingChoice is open (§10.6)").to.not.eq(null);
    const options = pending?.options ?? [];
    expect(options, "the open choice offers options").to.not.have.length(0);
    return cy.wrap(options, { log: false });
  });
}

/** BUILD M5-T3: hand the hotseat to the actor. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat (the hotseat handle names the seat holding it)").to.not.eq(
      undefined,
    );
    if (handle.seat !== player) cy.handOver();
  });
}

function openGame(seed: string, mulligan: "keep" | "manual" = "keep"): void {
  cy.seedGame({ seed, a: "02-prompts-a", b: "02-prompts-b", mulligan });
}

/** Reach player 1's turn `k` (odd player-turn; §10.1) with its mana (§2.3). */
function toP1Turn(k: number, budget = 12): void {
  cy.gameState().then((state) => {
    expect(budget, `player 1's turn ${k} is reachable inside the budget`).to.be.greaterThan(0);
    if (state.turn === 2 * k - 1) {
      ensureSeat("p1");
      return;
    }
    ensureSeat(state.active);
    cy.endTurn();
    toP1Turn(k, budget - 1);
  });
}

/** M5-T2: card click starts the R81 play. */
function select(name: string): void {
  cy.handCardByName(name).then((instanceId) => {
    cy.get(ts(handCardId(instanceId))).click();
  });
}

function pickerOpensOnce(kind: PromptKind): void {
  cy.waitForPrompt(kind);
  cy.get(promptOf(kind)).should("have.length", 1);
  cy.get(PROMPT).should("have.length", 1);
}

function pickerAnswered(kind: PromptKind): void {
  cy.get(promptOf(kind)).should("not.exist");
}

describe("BUILD M8 02 — every choice picker is rendered once and answered", () => {
  it("R9 mulligan — both opening mulligans are open at once (R265), and the cards not kept are redrawn", () => {
    openGame(SEEDS.mulligan, "manual");
    pickerOpensOnce("mulligan");

    cy.gameState().then((state) => {
      // R265: simultaneous mulligans live in `state.mulligan`, not `state.pending` (§10.1).
      expect(state.pending, "R265: the mulligans are not `state.pending`").to.eq(null);
      expect(state.mulligan, "R265: both seats' mulligans are open").to.not.eq(undefined);
      expect(state.mulligan?.p1.keep, "player 1 owes its mulligan").to.eq(null);
      expect(state.mulligan?.p2.keep, "and player 2 owes its own at the same time").to.eq(null);
      expect(state.mulligan?.p1.prompt?.kind, "player 1's is a mulligan PendingChoice").to.eq("mulligan");
      expect(state.mulligan?.p2.prompt?.kind, "and so is player 2's").to.eq("mulligan");
      const hand = handOf(state, "p1");
      expect(hand, "player 1's opening hand is three cards (§2.1)").to.have.length(3);
      const kept = hand.slice(0, 1);
      const returned = hand.slice(1);

      cy.get(ts(GAME)).should("have.attr", "data-viewer", "p1");
      cy.get(promptOf("mulligan")).find(ts(MULLIGAN_OPPONENT_STATUS)).should("have.attr", "data-ready", "false");

      // §2.1 / R9: clicks return cards; replacements precede the reshuffle.
      cy.get(promptOf("mulligan"))
        .find('[aria-pressed="true"]')
        .should("have.length", hand.length);
      cy.answerPrompt("mulligan", { cards: returned, submit: true });

      // R266: the first answer is sealed without moving its hand.
      cy.gameState().should((mid) => {
        expect(mid.pending, "still no `state.pending` while player 2 owes its mulligan").to.eq(null);
        expect(mid.mulligan?.p1.keep, "R266: player 1's sealed answer is the ids it keeps").to.deep.eq(kept);
        expect(mid.mulligan?.p2.keep, "player 2 still owes its own").to.eq(null);
        expect(handOf(mid, "p1"), "R266: a sealed answer changes no hand yet").to.deep.eq(hand);
        expect(mid.phase, "the game is still in the mulligan").to.eq("mulligan");
      });

      // BUILD M5-T3 switches to player 2's already-open picker, so it must not be absent.
      cy.get(ts(GAME)).should("have.attr", "data-viewer", "p2");
      pickerOpensOnce("mulligan");
      cy.get(promptOf("mulligan"))
        .find(ts(MULLIGAN_OPPONENT_STATUS))
        .should("have.attr", "data-ready", "true")
        .find(ts(MULLIGAN_OPPONENT_READY))
        .should("be.visible");

      cy.keepMulligans();
      cy.noPrompt();

      cy.gameState().should((after) => {
        const now = handOf(after, "p1");
        // R10 adds turn one's draw to R9's replacements.
        expect(now, "R9's three replacements, plus R10's turn-1 draw").to.have.length(4);
        expect(now, "the kept card stayed in hand").to.include(kept[0]);
        for (const id of returned) {
          expect(now, "a returned card left the hand").to.not.include(id);
        }
        expect(after.phase, "the game leaves the mulligan phase once both seats answer").to.not.eq(
          "mulligan",
        );
        expect(after.mulligan, "R265: the window closes with the second answer").to.eq(undefined);
      });
    });
  });

  it("R81 zone — playing a permanent asks for its zone and the unit lands in the lane chosen", () => {
    openGame(SEEDS.zone);
    ensureSeat("p1");

    // A permanent chooses an unlocked zone (§3.2).
    select("Zoomerbin Oomen");
    pickerOpensOnce("zone");
    cy.gameState().should((state) => {
      expect(state.pending, "R81: a zone is a play choice, so nothing is paused").to.eq(null);
    });

    cy.answerPrompt("zone", { zones: [{ side: "you", row: "units", lane: 3 }] });
    pickerAnswered("zone");

    cy.fieldCardByName("Zoomerbin Oomen").then((oomen) => {
      cy.get(ts(zoneId("you", "units", 3))).find(ts(cardId(oomen))).should("exist");
      // R64 would use the leftmost lane without the answer.
      cy.get(ts(zoneId("you", "units", 1))).find(ts(cardId(oomen))).should("not.exist");
    });
  });

  it("R81 discover — Jewelosco Scarab's Cry opens a Discover and the chosen card reaches the hand", () => {
    openGame(SEEDS.discover);
    ensureSeat("p1");

    // A resolving Cry opens a `PendingChoice` (§9.3).
    select("Jewelosco Scarab");
    pickerOpensOnce("zone");
    cy.answerPrompt("zone", { zones: [{ side: "you", row: "units", lane: 1 }] });

    pickerOpensOnce("discover");
    cy.gameState().should((state) => {
      expect(state.pending, "a Discover pauses resolution (§9.3, §10.6)").to.not.eq(null);
    });

    pendingOptions().then((options) => {
      expect(options, "Discover offers 3 options, drawn without replacement (§6.3)").to.have.length(3);
      const chosen = options[0];
      expect(chosen, "the first option").to.not.eq(undefined);
      const defId = chosen?.selection?.option ?? "";
      expect(defId, "a Discover option names the card it would add").to.not.eq("");

      cy.answerPrompt("discover", { options: options.slice(0, 1).map((option) => option.key) });
      pickerAnswered("discover");
      cy.noPrompt();

      cy.gameState().should((after) => {
        const side = after.players.p1 as { hand?: { defId: string }[] };
        const defIds = (side.hand ?? []).map((card) => card.defId);
        expect(defIds, "the discovered card is in hand").to.include(defId);
      });
    });
  });

  it("R81 target — Hit Job asks which unit to destroy and destroys exactly that one", () => {
    openGame(SEEDS.target);
    ensureSeat("p1");

    // R64 creates a second body, so R90 requires a target choice; use lanes because names collide.
    cy.playByName("Me and Mr Token", { zone: { side: "you", row: "units", lane: 1 } });

    cy.instanceAt("p1", "units", 1).then((victim) => {
      cy.instanceAt("p1", "units", 2).should("not.eq", "");
      // #16 costs (3), so it first fits on player 1's third turn.
      toP1Turn(3);
      select("Hit Job");
      pickerOpensOnce("target");
      cy.gameState().should((state) => {
        expect(state.pending, "R81: a declared target travels in the play action").to.eq(null);
      });

      cy.answerPrompt("target", { cards: [victim] });
      pickerAnswered("target");

      cy.get(ts(cardId(victim))).should("not.exist");
      cy.instanceAt("p1", "units", 2).then((token) => {
        cy.get(ts(cardId(token))).should("exist");
      });
      // The spell and target reach the graveyard (§5.1); R11 keeps the token out.
      cy.get(ts(graveyardCountId("you"))).should("have.text", "2");
    });
  });

  it("R81 hand — Glowy Jelly Bean asks for a card in hand and that card becomes Radiant", () => {
    openGame(SEEDS.hand);
    // #26 first fits on turn three (§2.3).
    toP1Turn(3);

    cy.handCardByName("Jewelosco Scarab").then((scarab) => {
      select("Glowy Jelly Bean");
      // R81 makes a declared hand pick a play choice.
      pickerOpensOnce("hand");
      cy.gameState().should((state) => {
        expect(state.pending, "R81: a declared hand pick is not a PendingChoice").to.eq(null);
      });

      cy.answerPrompt("hand", { cards: [scarab] });
      pickerAnswered("hand");

      // §5.2 changes the hand card's radiant form.
      cy.get(ts(handCardId(scarab))).should("have.attr", "data-radiant", "true");
      cy.get(`${ts(handCardId(scarab))}${RADIANT}`).should("exist");
    });
  });

  it("R81 mode — radiant Flood asks which of its three effects to run and runs that one", () => {
    openGame(SEEDS.mode);
    ensureSeat("p1");

    cy.playByName("Mr. Vanilla", { zone: { side: "you", row: "units", lane: 1 } });

    // Only radiant Flood has this choice (§8.1).
    toP1Turn(3);
    cy.handCardByName("Flood").then((flood) => {
      select("Glowy Jelly Bean");
      cy.answerPrompt("hand", { cards: [flood] });
      cy.get(ts(handCardId(flood))).should("have.attr", "data-radiant", "true");

      toP1Turn(4);
      cy.fieldCardByName("Mr. Vanilla").then((vanilla) => {
        select("Flood");
        pickerOpensOnce("discover");
        cy.gameState().should((state) => {
          expect(state.pending, "R81: a declared mode travels in the play action").to.eq(null);
        });

        // §8.1's card text is the option interface.
        cy.answerPrompt("discover", { options: ["bounce all units"] });
        pickerAnswered("discover");

        cy.get(ts(cardId(vanilla))).should("not.exist");
        cy.get(ts(handCardId(vanilla))).should("exist");
      });
    });
  });

  it("R123 tribute — Lava Golem asks which units pay Tribute 3 and sacrifices exactly those", () => {
    openGame(SEEDS.tribute);
    ensureSeat("p1");

    // R64 makes four bodies; R101 leaves several minimal Tribute 3 sets.
    cy.playByName("Me and Mr Token", { zone: { side: "you", row: "units", lane: 1 } });
    toP1Turn(2);
    cy.playByName("Tempo Timmy", { zone: { side: "you", row: "units", lane: 3 } });
    cy.playByName("Mr. Vanilla", { zone: { side: "you", row: "units", lane: 4 } });

    toP1Turn(3);
    // Identify the token by lane because its name occurs in the card text.
    cy.instanceAt("p1", "units", 1).then((mrToken) => {
      cy.instanceAt("p1", "units", 3).then((timmy) => {
        cy.instanceAt("p1", "units", 4).then((vanilla) => {
          select("Lava Golem");
          pickerOpensOnce("tribute");
          cy.gameState().should((state) => {
            expect(state.pending, "R123: a Tribute is paid inside the play action").to.eq(null);
          });

          // §6.3 Tribute 3; only Sheep Tokens count two (§3.2).
          cy.answerPrompt("tribute", { cards: [mrToken, timmy, vanilla], submit: true });
          pickerAnswered("tribute");

          // R391 lets Tribute pay for its own zone.
          pickerOpensOnce("zone");
          cy.answerPrompt("zone", { zones: [{ side: "you", row: "units", lane: 5 }] });
          pickerAnswered("zone");

          for (const id of [mrToken, timmy, vanilla]) {
            cy.get(ts(cardId(id))).should("not.exist");
          }
          // Sacrifice is death (§6.3); R11 keeps the unpicked token out of the graveyard.
          cy.get(ts(graveyardCountId("you"))).should("have.text", "3");
          cy.instanceAt("p1", "units", 2).then((token) => {
            cy.get(ts(cardId(token))).should("exist");
          });
          // R360 puts the golem in its chosen lane.
          cy.instanceAt("p1", "units", 5).then((golem) => {
            cy.get(ts(cardId(golem))).find(attackIs(10)).should("exist");
            cy.get(ts(cardId(golem))).find(healthIs(5)).should("exist");
          });
        });
      });
    });
  });

  it("R81 direction — Silly Silas asks left or right and every card moves one lane that way", () => {
    openGame(SEEDS.direction);
    ensureSeat("p1");

    cy.playByName("Tempo Timmy", { zone: { side: "you", row: "units", lane: 1 } });

    toP1Turn(3);
    cy.fieldCardByName("Tempo Timmy").then((timmy) => {
      select("Silly Silas");

      // R81 direction is a play choice before the M5-T2 zone click.
      pickerOpensOnce("direction");
      cy.gameState().should((state) => {
        expect(state.pending, "R81: a declared direction travels in the play action").to.eq(null);
      });
      cy.answerPrompt("direction", { options: ["right"] });
      pickerAnswered("direction");

      cy.answerPrompt("zone", { zones: [{ side: "you", row: "units", lane: 2 }] });

      // §3.1 / R14 rotate the unit-zone ring, including Silas.
      cy.fieldCardByName("Silly Silas").then((silas) => {
        cy.get(ts(zoneId("you", "units", 2))).find(ts(cardId(timmy))).should("exist");
        cy.get(ts(zoneId("you", "units", 3))).find(ts(cardId(silas))).should("exist");
      });
    });
  });

  it("R81 x — Efficiency Dividend asks for X, a mode and a target, and deals exactly X", () => {
    openGame(SEEDS.x);
    // X is bounded by current mana (§2.3).
    toP1Turn(2);

    select("Efficiency Dividend");
    pickerOpensOnce("x");
    cy.gameState().should((state) => {
      expect(state.pending, "R81: X is chosen at play time, not by a prompt").to.eq(null);
    });
    cy.answerPrompt("x", { x: 2 });

    // §8.2's mode determines whether this play asks for a target.
    pickerOpensOnce("discover");
    cy.answerPrompt("discover", { options: ["damage"] });
    pickerAnswered("discover");
    pickerOpensOnce("target");
    cy.answerPrompt("target", { hero: "opponent" });

    // §4.4 applies X = 2 without Armor.
    cy.get(ts(heroId("opponent"))).find(healthIs(28)).should("exist");
  });

  it("R81 embiggen — Suppressive Aura asks which price was paid and the aura matches that price", () => {
    openGame(SEEDS.embiggen);
    // Both prices must be affordable (§2.3, R65).
    toP1Turn(4);
    cy.playByName("4-mana 7/7", { zone: { side: "you", row: "units", lane: 1 } });

    toP1Turn(5);
    cy.fieldCardByName("4-mana 7/7").then((sevens) => {
      select("Suppressive Aura");
      pickerOpensOnce("embiggen");
      cy.gameState().should((state) => {
        expect(state.pending, "R81: the embiggen price is a play choice").to.eq(null);
      });
      cy.answerPrompt("embiggen", { options: ["true"] });
      pickerAnswered("embiggen");

      cy.answerPrompt("zone", { zones: [{ side: "you", row: "backrow", lane: 1 }] });

      // §8.2 price changes the aura; §10.4 layer 5 lowers max health.
      cy.get(ts(cardId(sevens))).find(attackIs(5)).should("exist");
      cy.get(ts(cardId(sevens))).find(healthIs(5)).should("exist");
      cy.get(ts(cardId(sevens))).find(maxHealthIs(5)).should("exist");
    });
  });
});
