// The busy-board fixture (issue #3605; SPEC §2's lane, backrow and hand caps, §3–§4's units,
// §6's keywords): both sides at the lane cap with looping keyword treatments, Mythic and
// Legendary animated faces (#3386), face-down backrow traps and a full hand of playable cards.
// Parts 2–6 of #3604 quote this setup; it holds no probe logic. Every precondition asserts loudly,
// so a catalog change that moves a cost, a keyword or a name fails here, in setup, never in a
// measurement. Card facts (costs, keywords, names) are read off the catalog, never restated.

import catalogJson from "../../crates/cards/catalog.json";
import type { KeywordKind } from "@jackioh/shared";

import { KEYWORD_VISUALS } from "../../apps/web/src/cards/keywordVisuals.ts";
import { constants, seedFor } from "./config.ts";
import { handCountId, handRegionId, ts, zoneId } from "./testids.ts";
import type { ActionInput, GameStateLike, Lane, PlayerId } from "./types.ts";

/**
 * Issue #3605: the probe's seed (BUILD M8: every spec sets a seed). The first seed whose
 * shuffle deals every setup card in time: the mulligan keeps the ten wanted cards, both sides
 * play out by player-turn 4, and both hands refill to HAND_CAP by player-turn 7.
 */
export const BUSY_BOARD_SEED = seedFor("50-busy-board-50");

/** Issue #3605: the decks each seat builds the worst case from. */
export const BUSY_BOARD_DECK_A = "50-busy-a";
export const BUSY_BOARD_DECK_B = "50-busy-b";

export type BusyBoardUnit = { defId: string; lane: Lane };

const ANCHORS: readonly BusyBoardUnit[] = [
  /** Megabase: the Legendary wall (Can't attack, Immune, Immutable, Untributable, Can't Defend). */
  { defId: "stinkyscape-116", lane: 1 },
  /** Tylannt: the Mythic anchor (Tradable only, so no loop: see the loop assert below). */
  { defId: "stinkyscape-006", lane: 2 },
  /** Paragon Mk2: the Legendary charger (Flying, Ranged, Charge while it can act). */
  { defId: "stinkyscape-073", lane: 3 },
];

/** P1's five lane units: the anchors plus its two one-cost loopers. */
export const BUSY_BOARD_UNITS_A: readonly BusyBoardUnit[] = [
  ...ANCHORS,
  /** Wrong-House Attacker (Lifesteal, Poisonous) and Stratos (Flying, Ranged). */
  { defId: "classicplus-006", lane: 4 },
  { defId: "stinkyscape-068", lane: 5 },
];

/** P2's five: the same anchors, different loopers (State of the Game, Right-house defender). */
export const BUSY_BOARD_UNITS_B: readonly BusyBoardUnit[] = [
  ...ANCHORS,
  { defId: "classic-041", lane: 4 },
  { defId: "core-003", lane: 5 },
];

/** The Mythic anchor carries the Mythic animated face but no always-loop, and is exempt below. */
export const BUSY_BOARD_MYTHIC_ANCHOR = "stinkyscape-006";

/**
 * The trap pool each side sets two of: My Pawn, Pocket Peptides, Explosive Runes, Prepare For
 * Liftoff, Traitorous Blood. None fires on a Unit play (lethal attack, own-unit combat death,
 * hero attack, Starship death, adjacent attack; §5.1), so unlike Shadowstep or Crop Dusting the
 * set traps survive the setup's own unit plays still to come. The first pool trap in hand takes
 * backrow lane 1, the second lane 2, so whichever two arrive first is the worst case either way.
 */
export const BUSY_BOARD_TRAP_POOL: readonly string[] = [
  "core-096",
  "stinkyscape-138",
  "traininggrounds-013",
  "stinkyscape-144",
  "meditative-064",
];

/** Every card the setup wants in hand: the side's five units plus the trap pool. */
export function busyBoardSetupDefIds(units: readonly BusyBoardUnit[]): readonly string[] {
  return [...units.map((unit) => unit.defId), ...BUSY_BOARD_TRAP_POOL];
}

/**
 * The combat spell, in preference order: the first of these found in p1's hand is cast. Five
 * spells stay out of the busy board, so a ten-card hand always holds one (issue #3605).
 */
export const BUSY_BOARD_COMBAT_SPELL_DEF_IDS: readonly string[] = [
  "core-044",
  "core-005",
  "core-028",
  "classic-026",
  "classic-036",
];

/** The preference spells that declare a target (True Strike, Burn): answered, not clicked. */
export const BUSY_BOARD_TARGETED_SPELLS: readonly string[] = ["core-044", "classic-036"];

type CatalogCard = {
  name?: unknown;
  cost?: unknown;
  type?: unknown;
  base?: { keywords?: { kind?: unknown }[] };
};

const CATALOG = catalogJson as unknown as Record<string, CatalogCard>;

function catalogCard(defId: string): CatalogCard {
  const card = CATALOG[defId];
  if (card === undefined) throw new Error(`busy board: ${defId} is not in the catalog`);
  return card;
}

/** The catalog's name: the combat spell preference resolves defIds through this. */
export function busyBoardNameOf(defId: string): string {
  const name = catalogCard(defId).name;
  if (typeof name !== "string" || name.length === 0) throw new Error(`busy board: ${defId} has no name`);
  return name;
}

function busyBoardCostOf(defId: string): number {
  const cost = catalogCard(defId).cost;
  if (!Number.isInteger(cost)) throw new Error(`busy board: ${defId} has no integer cost`);
  return cost as number;
}

/**
 * The looping treatments a unit draws: its keywords whose visual loops while drawn
 * (`keywordVisuals.ts` `when: "always"`, on the field). Mirrors `keywordFxPlan`'s motion rule.
 */
export function busyBoardLoopCount(defId: string): number {
  const keywords = catalogCard(defId).base?.keywords ?? [];
  return keywords.filter(({ kind }) => {
    const visual = KEYWORD_VISUALS[kind as KeywordKind];
    return visual !== undefined && visual.onField && visual.motion?.when === "always";
  }).length;
}

type SetupCard = { id: string; defId: string };

type SetupSide = {
  hand?: SetupCard[];
  units?: (SetupCard[] | null)[];
  backrow?: (SetupCard | null)[];
};

function setupSide(state: GameStateLike, player: PlayerId): SetupSide {
  return state.players[player] as SetupSide;
}

/**
 * This turn's mana with the fixture's handicap: min(turn + 9, 10), so 10 on each setup turn
 * (player-turns 1–4). A catalog change that moves a setup cost fails loudly at dispatch.
 */
const SETUP_TURN_MANA = 10;

/** The legal setup plays still missing from `player`'s board that this turn's mana covers. */
function plannedPlays(
  player: PlayerId,
  units: readonly BusyBoardUnit[],
  state: GameStateLike,
): ActionInput[] {
  const side = setupSide(state, player);
  const hand = side.hand ?? [];
  const used = new Set<string>();
  const take = (defId: string): string | null => {
    const found = hand.find((card) => card.defId === defId && !used.has(card.id));
    if (found === undefined) return null;
    used.add(found.id);
    return found.id;
  };
  const boardUnits = new Set((side.units ?? []).flatMap((pile) => pile ?? []).map((card) => card.defId));
  const backrow = side.backrow ?? [];
  let spent = 0;
  const plays: ActionInput[] = [];
  const afford = (defId: string): boolean => {
    const cost = busyBoardCostOf(defId);
    if (spent + cost > SETUP_TURN_MANA) return false;
    spent += cost;
    return true;
  };
  for (const unit of units) {
    if (boardUnits.has(unit.defId)) continue;
    const instanceId = take(unit.defId);
    if (instanceId === null || !afford(unit.defId)) continue;
    plays.push({ type: "play", playerId: player, instanceId, zone: { row: "units", lane: unit.lane } });
  }
  const takenLanes = new Set(
    [1, 2].filter((lane) => backrow[lane - 1] !== null && backrow[lane - 1] !== undefined),
  );
  for (const defId of BUSY_BOARD_TRAP_POOL) {
    const lane = ([1, 2] as const).find((candidate) => !takenLanes.has(candidate));
    if (lane === undefined) break;
    const instanceId = take(defId);
    if (instanceId === null || !afford(defId)) continue;
    plays.push({ type: "play", playerId: player, instanceId, zone: { row: "backrow", lane } });
    takenLanes.add(lane);
  }
  return plays;
}

function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== player) cy.handOver();
  });
}

/** One setup turn: play what is still missing and affordable, then hand over. */
function setupTurn(player: PlayerId, units: readonly BusyBoardUnit[]): void {
  ensureSeat(player);
  cy.gameState()
    .then((state) => plannedPlays(player, units, state))
    .then((plays) => {
      for (const play of plays) cy.dispatchAction(play);
    });
  cy.endTurn();
}

/** The precondition asserts: five units and two face-down traps a side, a full playable hand. */
function assertBusyBoard(): void {
  cy.gameState().should((state) => {
    for (const [player, units] of [
      ["p1", BUSY_BOARD_UNITS_A],
      ["p2", BUSY_BOARD_UNITS_B],
    ] as const) {
      const side = setupSide(state, player);
      const piles = (side.units ?? []).map((pile) => pile?.length ?? 0);
      expect(piles, `${player} holds all five lanes (§2.4)`).to.deep.eq([1, 1, 1, 1, 1]);
      for (const unit of units) {
        const top = (side.units ?? [])[unit.lane - 1]?.[0];
        expect(top?.defId, `${player} lane ${unit.lane} holds ${unit.defId}`).to.eq(unit.defId);
        if (unit.defId !== BUSY_BOARD_MYTHIC_ANCHOR) {
          expect(
            busyBoardLoopCount(unit.defId),
            `${unit.defId} carries two always-loops (§6)`,
          ).to.be.at.least(2);
        }
      }
      const traps = (side.backrow ?? []).slice(0, 2).map((slot) => slot?.defId ?? null);
      expect(traps, `${player} set two traps face-down (§5.1)`).to.have.length(2);
      for (const trap of traps) {
        expect(BUSY_BOARD_TRAP_POOL, `${player}'s ${trap} never fires mid-setup (§5.1)`).to.include(trap);
      }
    }
  });
  for (const side of ["you", "opponent"] as const) {
    for (const lane of [1, 2, 3, 4, 5] as const) {
      cy.get(ts(zoneId(side, "units", lane))).find('[data-testid^="card-"]').should("exist");
    }
    for (const lane of [1, 2] as const) {
      cy.get(ts(zoneId(side, "backrow", lane))).find('[data-face-down="true"]').should("exist");
    }
    cy.get(ts(handCountId(side))).should("have.text", String(constants.HAND_CAP));
  }
  // SPEC §2.4: the full hand every card of which is playable (CLAUDE.md rule 7: the green is
  // `legalActions`', never the client's).
  cy.get(ts(handRegionId("you")))
    .find('[data-testid^="hand-card-"]')
    .should("have.length", constants.HAND_CAP)
    .each(($card) => {
      expect($card.attr("data-glow"), "every card in the full hand is playable").to.eq("ready");
    });
}

/**
 * Build the worst case both sides share: five looping units and two set traps a side over the
 * first two turns, measuring at player-turn 7 with p1 to move. Turns 5–7 play nothing: each
 * side's start-of-turn draw refills its hand to HAND_CAP after its last setup play (p1 spends
 * player-turns 1 and 3, p2 player-turns 2 and 4), and nothing triggers on an empty turn, so the
 * board stands exactly as built. Setup only: `dispatchAction` reaches the precondition, never
 * exercises the behavior under test.
 */
export function setupBusyBoard(): void {
  cy.seedGame({ seed: BUSY_BOARD_SEED, a: BUSY_BOARD_DECK_A, b: BUSY_BOARD_DECK_B, mulligan: "manual" });
  // The setup mulligan keeps what the board needs and redraws the rest, so the two setup turns
  // start from a hand already holding most of it. Answering a mulligan is setup, not the
  // behavior under test, and `dispatchAction` reaches it the same way the picker would.
  for (const [player, units] of [
    ["p1", BUSY_BOARD_UNITS_A],
    ["p2", BUSY_BOARD_UNITS_B],
  ] as const) {
    cy.gameState().then((state) => {
      const wanted = busyBoardSetupDefIds(units);
      return (setupSide(state, player).hand ?? [])
        .filter((card) => wanted.includes(card.defId))
        .map((card) => card.id);
    }).then((keep) => {
      cy.dispatchAction({ type: "mulligan", playerId: player, keep });
    });
  }
  setupTurn("p1", BUSY_BOARD_UNITS_A);
  setupTurn("p2", BUSY_BOARD_UNITS_B);
  setupTurn("p1", BUSY_BOARD_UNITS_A);
  setupTurn("p2", BUSY_BOARD_UNITS_B);
  cy.advanceToTurn(7);
  ensureSeat("p1");
  assertBusyBoard();
}
