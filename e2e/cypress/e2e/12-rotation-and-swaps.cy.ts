// BUILD M8: R14 and §3.1 rotate each row's ten zones as one ring; crossing its centre changes control (§3.2), not ownership (R12).
// R33 and §10.8 require a face-down trap's controller alone to see its identity. R73 swaps each lane across the board with ownership intact.
// R81 declared `direction` and `mode` choices travel in `play`; §10.6 supplies no prompt. BUILD M5-T4 observes the resulting zone testids.

import { CARDS, CARD_NAMES, cardId as catalogId } from "../../support/cards.ts";
import type { PlayCardOptions } from "../../support/commands.ts";
import { seedFor } from "../../support/config.ts";
import { BOARD, cardId, ts, zoneId } from "../../support/testids.ts";
import type { GameStateLike, Lane, PlayerId, Side, ZoneRef } from "../../support/types.ts";

/** BUILD M8 seed supplies the ordered trap and unit plays before R2's cap (SPEC §8). */
const SEED = seedFor("12-rotation-4");

const TURN_BUDGET = 34;

const BEAR_HONEYPOT = catalogId(60);

/** SPEC §8 choice-free 2-cost units avoid Bear Honeypot's 1-cost trigger. */
const PLAIN_UNITS: readonly string[] = [
  catalogId(1),
  catalogId(20),
  catalogId(32),
  catalogId(45),
  catalogId(56),
  catalogId(91),
];
const PLAIN_UNIT_COST = 2;

/** SPEC §8 costs require waiting within §2.3's mana cap. */
const COSTS: Record<string, number> = {
  [CARDS.myPawn]: 1,
  [BEAR_HONEYPOT]: 1,
  [CARDS.sillySilas]: 3,
  [CARDS.pocketChaos]: 4,
};

/** R14 / §3.1 ring, from seat 1's `you` view (ASSUMPTION A3). */
const RING: readonly { side: Side; lane: Lane }[] = [
  { side: "you", lane: 1 },
  { side: "you", lane: 2 },
  { side: "you", lane: 3 },
  { side: "you", lane: 4 },
  { side: "you", lane: 5 },
  { side: "opponent", lane: 5 },
  { side: "opponent", lane: 4 },
  { side: "opponent", lane: 3 },
  { side: "opponent", lane: 2 },
  { side: "opponent", lane: 1 },
];

/** R14: move right within `zone.row`'s ring. */
function rotateRight(zone: ZoneRef): ZoneRef {
  const at = RING.findIndex((slot) => slot.side === zone.side && slot.lane === zone.lane);
  expect(at, `${zone.side} lane ${zone.lane} is on the ring`).to.be.at.least(0);
  const next = RING[(at + 1) % RING.length];
  if (next === undefined) throw new Error("the ring is empty");
  return { side: next.side, row: zone.row, lane: next.lane };
}

/** R73: the board swap is lane-preserving, so a card only crosses the centre line. */
function acrossTheLine(zone: ZoneRef): ZoneRef {
  return { side: zone.side === "you" ? "opponent" : "you", row: zone.row, lane: zone.lane };
}

/** The owner of each side in seat 1's view (ASSUMPTION A3). */
function seatOf(side: Side): PlayerId {
  return side === "you" ? "p1" : "p2";
}

/** SPEC §8 card names are what BUILD M5-T1 renders. */
function nameOf(index: number): string {
  const name = CARD_NAMES[index];
  if (name === undefined) throw new Error(`no SPEC §8 card #${index}`);
  return name;
}

type Held = { id: string; defId: string };
type SidePeek = {
  mana?: { current: number };
  hand?: Held[];
  units?: ({ id: string }[] | null)[];
  backrow?: ({ id: string } | null)[];
};

function peek(state: GameStateLike, player: PlayerId): SidePeek {
  return state.players[player] as SidePeek;
}

function handOf(state: GameStateLike, player: PlayerId): Held[] {
  return peek(state, player).hand ?? [];
}

/** BUILD M5-T3: hand the hotseat device to the actor. */
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

function canPay(state: GameStateLike, player: PlayerId, cost: number): boolean {
  return (peek(state, player).mana?.current ?? 0) >= cost;
}

/** Play the held SPEC §8 card by id, preserving its pre-play id for later assertions. */
function playWhenDrawn(
  player: PlayerId,
  defId: string,
  options: PlayCardOptions = {},
): Cypress.Chainable<string> {
  const cost = COSTS[defId] ?? 0;
  advanceUntil(`${player} can pay ${cost} for ${defId}`, (state) => {
    if (state.active !== player) return false;
    if (!canPay(state, player, cost)) return false;
    return handOf(state, player).some((card) => card.defId === defId);
  });
  ensureSeat(player);
  return cy.instanceInHand(player, defId).then((instanceId) => {
    cy.playCard(instanceId, options);
    // R227: a set trap gets a fresh id, so track the id in its zone.
    const zone = options.zone;
    if (zone !== undefined && zone.row === "backrow" && zone.side === "you") {
      return cy.instanceAt(player, "backrow", zone.lane);
    }
    return cy.wrap(instanceId, { log: false });
  });
}

/** Require an id captured by an earlier step. */
function need(value: string | undefined, what: string): string {
  if (value === undefined) throw new Error(`${what} was never captured`);
  return value;
}

/** Put a choice-free unit in a lane; only its zone matters here. */
function summonPlainUnit(player: PlayerId, lane: Lane): Cypress.Chainable<string> {
  advanceUntil(`${player} can summon a plain unit into lane ${lane}`, (state) => {
    if (state.active !== player) return false;
    if (!canPay(state, player, PLAIN_UNIT_COST)) return false;
    return handOf(state, player).some((card) => PLAIN_UNITS.includes(card.defId));
  });
  ensureSeat(player);
  return cy.gameState().then((state) => {
    const unit = handOf(state, player).find((card) => PLAIN_UNITS.includes(card.defId));
    expect(unit, `${player} holds one of the choice-free units`).to.not.eq(undefined);
    const instanceId = unit?.id ?? "";
    // `viewFor` makes the acting seat `you` (ASSUMPTION A3).
    cy.playCard(instanceId, { zone: { side: "you", row: "units", lane } });
    return cy.wrap(instanceId, { log: false });
  });
}

/** BUILD M5-T1 nests each card inside its zone. */
function expectCardAt(instanceId: string, zone: ZoneRef): void {
  cy.get(ts(cardId(instanceId)))
    .closest(ts(zoneId(zone.side, zone.row, zone.lane)))
    .should("exist");
}

/** Confirm the engine's zone after a crossed card changes control. */
function expectEngineAt(instanceId: string, zone: ZoneRef): void {
  cy.instanceAt(seatOf(zone.side), zone.row, zone.lane).should("eq", instanceId);
}

/** R33: this seat is the controller, so the trap's face is theirs to read. */
function expectTrapReadable(instanceId: string, zone: ZoneRef, name: string): void {
  expectCardAt(instanceId, zone);
  cy.get(ts(zoneId(zone.side, zone.row, zone.lane))).should("contain.text", name);
}

/** R33 / §10.8: a non-controller cannot see the trap, but the engine confirms its zone. */
function expectTrapHidden(instanceId: string, zone: ZoneRef, name: string): void {
  cy.get(ts(cardId(instanceId))).should("not.exist");
  cy.get(ts(zoneId(zone.side, zone.row, zone.lane))).should("not.contain.text", name);
  expectEngineAt(instanceId, zone);
}

// BUILD M5-T1 layout check: Cypress measures responsive overflow at both viewports.

/** BUILD M5-T1 viewports. */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

/** BUILD M5-T1: each viewport must fit without clipping the tracked cards. */
function expectFitsViewport(
  viewport: { label: string; width: number; height: number },
  cards: readonly string[],
): void {
  cy.viewport(viewport.width, viewport.height);
  cy.get(ts(BOARD)).should("be.visible");
  for (const instanceId of cards) {
    cy.get(ts(cardId(instanceId))).should("be.visible");
  }
  // `should` retries until asynchronous resize layout settles.
  cy.document({ log: false }).should((doc) => {
    const where = `${viewport.label} ${String(viewport.width)}x${String(viewport.height)}`;
    expect(doc.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(
      viewport.width,
    );
    expect(doc.body.scrollWidth, `the body fits ${where}`).to.be.at.most(viewport.width);

    const board = doc.querySelector(ts(BOARD));
    expect(board, "the board is rendered").to.not.eq(null);
    expect(board?.scrollWidth ?? 0, `the board fits ${where}`).to.be.at.most(viewport.width);
    expect(
      Math.ceil(board?.getBoundingClientRect().right ?? 0),
      `the board's right edge is inside ${where}`,
    ).to.be.at.most(viewport.width);
  });
}

/** Tracked card zones in seat 1's view. */
type Board = {
  yourLane1: ZoneRef;
  silas: ZoneRef;
  yourLane5: ZoneRef;
  theirLane1: ZoneRef;
  myPawn: ZoneRef;
  honeypot: ZoneRef;
};

function mapBoard(board: Board, move: (zone: ZoneRef) => ZoneRef): Board {
  return {
    yourLane1: move(board.yourLane1),
    silas: move(board.silas),
    yourLane5: move(board.yourLane5),
    theirLane1: move(board.theirLane1),
    myPawn: move(board.myPawn),
    honeypot: move(board.honeypot),
  };
}

describe("BUILD M8 12 — Silly Silas rotates one step around the ring and Pocket Chaos swaps the boards", () => {
  beforeEach(() => {
    cy.seedGame({ seed: SEED, a: "12-rotation-a", b: "12-rotation-b" });
  });

  it("R14/R33/R73 moves every card one lane, then flips both sides of the board", () => {
    const ids: {
      myPawn?: string;
      honeypot?: string;
      yourLane1?: string;
      yourLane5?: string;
      theirLane1?: string;
      silas?: string;
    } = {};

    // R14's centre crossings make control changes visible in seat 1 lane 5 and seat 2 lane 1.
    const built: Board = {
      yourLane1: { side: "you", row: "units", lane: 1 },
      silas: { side: "you", row: "units", lane: 3 },
      yourLane5: { side: "you", row: "units", lane: 5 },
      theirLane1: { side: "opponent", row: "units", lane: 1 },
      myPawn: { side: "you", row: "backrow", lane: 2 },
      honeypot: { side: "opponent", row: "backrow", lane: 2 },
    };

    // Set My Pawn before #60 Bear Honeypot can trigger on its 1-cost play.
    playWhenDrawn("p1", CARDS.myPawn, { zone: { side: "you", row: "backrow", lane: 2 } }).then((id) => {
      ids.myPawn = id;
    });

    playWhenDrawn("p2", BEAR_HONEYPOT, { zone: { side: "you", row: "backrow", lane: 2 } }).then((id) => {
      ids.honeypot = id;
    });

    summonPlainUnit("p1", 1).then((id) => {
      ids.yourLane1 = id;
    });
    summonPlainUnit("p1", 5).then((id) => {
      ids.yourLane5 = id;
    });
    summonPlainUnit("p2", 1).then((id) => {
      ids.theirLane1 = id;
    });

    // R33: seat 1 reads only its own face-down trap.
    ensureSeat("p1");
    cy.then(() => {
      expectCardAt(need(ids.yourLane1, "seat 1's lane-1 unit"), built.yourLane1);
      expectCardAt(need(ids.yourLane5, "seat 1's lane-5 unit"), built.yourLane5);
      expectCardAt(need(ids.theirLane1, "seat 2's lane-1 unit"), built.theirLane1);
      expectTrapReadable(need(ids.myPawn, "seat 1's My Pawn"), built.myPawn, nameOf(96));
      expectTrapHidden(need(ids.honeypot, "seat 2's Bear Honeypot"), built.honeypot, nameOf(60));
    });

    // §10.5 puts Silas on field before its §8 #52 Cry rotates it.
    playWhenDrawn("p1", CARDS.sillySilas, {
      zone: { side: "you", row: "units", lane: 3 },
      // R81: the declared direction travels in `modes`.
      answers: [{ kind: "direction", options: ["right"] }],
    }).then((id) => {
      ids.silas = id;
    });

    const rotated = mapBoard(built, rotateRight);

    // BUILD M5-T4: R14 crossings move cards under the other side's zone.
    ensureSeat("p1");
    cy.then(() => {
      expectCardAt(need(ids.yourLane1, "seat 1's lane-1 unit"), rotated.yourLane1);
      expectEngineAt(need(ids.yourLane1, "seat 1's lane-1 unit"), rotated.yourLane1);
      expectCardAt(need(ids.silas, "Silly Silas"), rotated.silas);
      expectEngineAt(need(ids.silas, "Silly Silas"), rotated.silas);
      expectCardAt(need(ids.yourLane5, "seat 1's lane-5 unit"), rotated.yourLane5);
      expectEngineAt(need(ids.yourLane5, "seat 1's lane-5 unit"), rotated.yourLane5);
      expectCardAt(need(ids.theirLane1, "seat 2's lane-1 unit"), rotated.theirLane1);
      expectEngineAt(need(ids.theirLane1, "seat 2's lane-1 unit"), rotated.theirLane1);
      // R14 rotates the backrow too; R33 visibility is unchanged without a crossing.
      expectTrapReadable(need(ids.myPawn, "seat 1's My Pawn"), rotated.myPawn, nameOf(96));
      expectTrapHidden(need(ids.honeypot, "seat 2's Bear Honeypot"), rotated.honeypot, nameOf(60));
    });

    // #87's board mode applies R73's lane-preserving swap.
    playWhenDrawn("p1", CARDS.pocketChaos, {
      // R81 declared mode uses #88's Discover picker.
      answers: [{ kind: "discover", options: ["board"] }],
    });

    const swapped = mapBoard(rotated, acrossTheLine);
    ensureSeat("p1");
    cy.then(() => {
      expectCardAt(need(ids.yourLane1, "seat 1's lane-1 unit"), swapped.yourLane1);
      expectEngineAt(need(ids.yourLane1, "seat 1's lane-1 unit"), swapped.yourLane1);
      expectCardAt(need(ids.silas, "Silly Silas"), swapped.silas);
      expectEngineAt(need(ids.silas, "Silly Silas"), swapped.silas);
      expectCardAt(need(ids.yourLane5, "seat 1's lane-5 unit"), swapped.yourLane5);
      expectEngineAt(need(ids.yourLane5, "seat 1's lane-5 unit"), swapped.yourLane5);
      expectCardAt(need(ids.theirLane1, "seat 2's lane-1 unit"), swapped.theirLane1);
      expectEngineAt(need(ids.theirLane1, "seat 2's lane-1 unit"), swapped.theirLane1);

      // R33 swaps trap readability; R12 ownership remains while R73 control moves.
      expectTrapHidden(need(ids.myPawn, "seat 1's My Pawn"), swapped.myPawn, nameOf(96));
      expectTrapReadable(need(ids.honeypot, "seat 2's Bear Honeypot"), swapped.honeypot, nameOf(60));
      cy.fieldCardByName(nameOf(60)).should("eq", need(ids.honeypot, "seat 2's Bear Honeypot"));
    });

    // BUILD M5-T1 measures this spec's fullest reachable board.
    cy.then(() => {
      expect(Cypress.config("viewportWidth"), "the desktop viewport is the suite's default").to.eq(
        VIEWPORTS[0].width,
      );
      expect(Cypress.config("viewportHeight"), "and so is its height").to.eq(VIEWPORTS[0].height);

      // R33 hides seat 1's My Pawn after the swap, leaving five visible cards to measure.
      const onScreen = [
        need(ids.yourLane1, "seat 1's lane-1 unit"),
        need(ids.silas, "Silly Silas"),
        need(ids.yourLane5, "seat 1's lane-5 unit"),
        need(ids.theirLane1, "seat 2's lane-1 unit"),
        need(ids.honeypot, "seat 2's Bear Honeypot"),
      ];

      for (const viewport of VIEWPORTS) {
        expectFitsViewport(viewport, onScreen);
      }
    });
  });
});
