// 36 — every Meditative card played once through the browser, with the set previewed (#552, MN09).
//
// Key assertions (issue #552, "Every card plays"): "A hotseat run with the set previewed plays each
// Meditative card once through the browser … Each card's animation and cue run with nothing left
// `data-animating`, the log line reads as English, and the inspect view's notes (tuning, counters,
// locks, the Jade counter, the market's stock, a crafted card's parts) read well."
//
// The cards come from the catalog at run time (`crates/cards/catalog.json`): every Meditative entry a
// deck may hold once the set is previewed (R1420), so a card a later part adds is played the day it
// lands, with no list or fixture to edit. They are dealt into seeded /dev/hotseat games whose engine
// previews the set (`cy.seedGame`'s `preview`, the WebAssembly module's `previewSets`), a few cards a
// seat (support/playthrough.ts `SEAT_SHARE`, so no row fills) with every card in the opening hand,
// mana enough for the dearest and heroes no game can kill (R180's handicap), and played by the
// driver in support/playthrough.ts: whoever owes the move answers its prompt with its first legal
// answer, plays a card still to play, or ends the turn. A card another card turned into something
// else (Spiritually 中国 makes a hand Gachaholics) goes into the next game, until every card is done.
// A card cast as it is drawn (Meditative #37 CN in a Bottle, R58) is never played from a hand: it is
// done once it lands in a graveyard.
//
// After every play:
//   - `cy.settled()`: the play's animations and cues (which ride the runner's entries, SPEC §10.11)
//     ran and nothing is left `data-animating`;
//   - every line of the log reads as English (support/playthrough.ts `englishProblems`: it opens with
//     a capital and prints no card id, no engine identifier and no unfilled value; a card shown in
//     Chinese is named in Chinese, R1301);
//   - the card, where it now stands on the field, opens its hover preview, whose face and notes print
//     no id or unfilled value (`noteProblems`), and the Jade Counter badge, where a hero has one, is a
//     number titled "Jade Counter".
// Each prompt a card opens is on the board under the kind the view gives it (`data-prompt-kind`), so
// the crafter, the market and the rest are answered through a board that shows them. At the end of
// each game every card left in either hand (a crafted card, a card bought at the market) opens its
// preview, which reads as the played cards' do.

import {
  deckableOf,
  englishProblems,
  nextGame,
  noteProblems,
  GAME_OVER,
  owedPromptKind,
  playStep,
  castOnDraw,
  printedCost,
  type CatalogRow,
  type PlayStep,
} from "../../support/playthrough.ts";
import { constants, timeouts } from "../../support/config.ts";
import { GAME, INSPECT_HOVER, LOG, cardId, handCardId, handRegionId, ts } from "../../support/testids.ts";
import type { JackiOhDevHandle } from "../../support/types.ts";

/** Where the catalog sits, from e2e/ (Cypress's project root), read at run time. */
const CATALOG_PATH = "../crates/cards/catalog.json";
/** The set under test, and the set that ships whose first card fills a seat left with none. */
const SET = "Meditative";
const FILLER_SET = "Core";
/** More games than the set needs, so a card turned away in one game still has another; spare ones pass at once. */
const MAX_GAMES = 12;
/** A game stops after this many player-turns (§10.1's count) and leaves what it did not play to the next one. */
const TURN_LIMIT = 12;
/** Steps one game may take: moves, answers and hand-overs together. */
const STEP_BUDGET = 400;
/** R180, R290: a hero no game here can kill, so every game lasts until its cards are played. */
const HERO_HEALTH = 999;
/** The hand-card selector the end-of-game reading walks. */
const HAND_CARD = '[data-testid^="hand-card-"]';

type Handle = Parameters<typeof playStep>[0];

/** The deckable Meditative cards, and those not yet played (or, for a Cast on draw card, not yet cast). */
let cards: CatalogRow[] = [];
let filler: CatalogRow | undefined;
const remaining = new Set<string>();

/** The dearest price a card can ask, an Embiggen card's bigger one included. */
function ceiling(def: CatalogRow): number {
  return typeof def.cost === "object" ? Math.max(def.cost.base, def.cost.embiggen) : printedCost(def);
}

/** R180: every card in the opening hand, mana for the dearest from turn 1, and an unkillable hero. */
function handicapFor(size: number, mana: number) {
  const [opening] = constants.OPENING_DRAW;
  return { deckSize: size, manaBonus: mana, manaCap: mana, extraOpeningCards: Math.max(0, size - opening), extraDrawsPerTurn: 0, heroHealth: HERO_HEALTH };
}

/**
 * One game: the cards it deals that are still to play, those of them cast as they are drawn, and the
 * end steps that say nothing more can come.
 */
type Game = { label: string; toPlay: Set<string>; arrivals: CatalogRow[]; emptyEnds: number };

/** The cards in either graveyard, by definition id: where a card cast as it was drawn lands. */
function graveyardDefIds(handle: JackiOhDevHandle): Set<string> {
  const ids = new Set<string>();
  for (const seat of ["p1", "p2"] as const) {
    const side = handle.state.players[seat] as { graveyard?: { defId: string }[] };
    for (const card of side.graveyard ?? []) ids.add(card.defId);
  }
  return ids;
}

/** After a play: the animations drained, the log in English, and the card's inspect view read. */
function afterPlay(game: Game, step: Extract<PlayStep, { kind: "play" }>): void {
  cy.settled();
  cy.get(`${ts(LOG)} .log-line`).then(($lines) => {
    const problems = [...new Set([...$lines].flatMap((line) => englishProblems(line.textContent ?? "", `${game.label}, after ${step.defId}`)))];
    expect(problems, `the log after ${step.defId}`).to.deep.equal([]);
  });
  cy.get("body").then(($body) => {
    $body.find(".hero-jade").each((_, badge) => {
      expect(badge.textContent ?? "", "the Jade Counter badge is a number").to.match(/^\d+$/);
      expect(badge.getAttribute("title"), "the Jade Counter badge says what it counts").to.eq("Jade Counter");
    });
    if ($body.find(ts(cardId(step.instanceId))).length === 0) return;
    readInspect(ts(cardId(step.instanceId)), `${step.defId} on the field`);
  });
}

/** Rest a mouse on a card, read its hover preview, and leave. */
function readInspect(selector: string, where: string): void {
  cy.get(selector).first().trigger("pointerover", { pointerType: "mouse", force: true });
  // Retried until the preview opens; the message names the card, should it never open.
  cy.get("body", { timeout: timeouts.view }).should(($body) => {
    const hover = $body.find(ts(INSPECT_HOVER));
    expect(hover.length, `${where}'s hover preview opens`).to.be.greaterThan(0);
    expect(noteProblems(hover.text(), `${where}'s inspect view`)).to.deep.equal([]);
  });
  cy.get(selector).first().trigger("pointerout", { pointerType: "mouse", force: true });
  cy.get(ts(INSPECT_HOVER)).should("not.exist");
}

/**
 * Whether the game is done with: every card played, a card won it (a Wincon, such as #8 Reach the
 * Summit's tenth Ascent: the cards it did not play go to the next game), a turn past the limit, or
 * two empty-handed ends with empty decks.
 */
function finished(game: Game, step: PlayStep): boolean {
  if (game.toPlay.size === 0) return true;
  if (step.kind === "over") return true;
  if (step.kind !== "end") return false;
  game.emptyEnds = step.held.length === 0 && step.deck === 0 ? game.emptyEnds + 1 : 0;
  return step.turn > TURN_LIMIT || game.emptyEnds >= 2;
}

/** One step, its checks, and the next step, until the game is done with. */
function drive(game: Game, budget: number): void {
  if (budget <= 0 || game.toPlay.size === 0) return;
  cy.settled();
  cy.jackioh().then((handle: JackiOhDevHandle) => {
    // A prompt the step will answer: the board draws it once the animations queued before it have
    // run, which `cy.settled()` cannot see before they start, so wait on the prompt itself (#617).
    const owed = owedPromptKind(handle as Handle);
    if (owed !== null) cy.get(`[data-testid="prompt-modal"][data-prompt-kind="${owed}"]`, { timeout: timeouts.view }).should("exist");
    cy.document({ log: false }).then((doc) => {
      const step = playStep(handle as Handle, game.toPlay, doc);
      Cypress.log({ name: "step", message: `${game.label}: ${JSON.stringify(step)}` });
      if (step.kind === "over" && step.why !== GAME_OVER) throw new Error(`${game.label}: ${step.why}`);
      if (step.kind === "answer") {
        expect(step.error, `${game.label}: the ${step.promptKind} answer`).to.eq(null);
        expect(step.shown, `${game.label}: the board shows the ${step.promptKind} prompt the view holds`).to.eq(step.drawn);
      }
      if (step.kind === "play") {
        expect(step.error, `${game.label}: playing ${step.defId}`).to.eq(null);
        game.toPlay.delete(step.defId);
        remaining.delete(step.defId);
        afterPlay(game, step);
      }
      // A card cast as it was drawn has had its turn once it is in a graveyard.
      const graveyard = graveyardDefIds(handle);
      for (const def of game.arrivals) {
        if (!graveyard.has(def.id)) continue;
        game.toPlay.delete(def.id);
        remaining.delete(def.id);
      }
      if (finished(game, step)) return;
      drive(game, budget - 1);
    });
  });
}

/** The seat's own hand in its newest view, as testids, in the order the board draws it. */
function handCardIds(handle: Handle): string[] {
  const hand = (handle.view() as unknown as { you: { hand: { instanceId: string }[] | { count: number } } }).you.hand;
  return Array.isArray(hand) ? hand.map((card) => handCardId(card.instanceId)) : [];
}

/**
 * Every card left in either hand opens a preview that reads (a crafted card's parts, a market lot).
 * The ids are the seat's newest view's, read once the board draws them: after the device moves, or a
 * game's last play, the board shows the view before it for a moment with nothing animating, which
 * `cy.settled()` cannot see (#659).
 */
function readHands(): void {
  for (const seat of ["p1", "p2"] as const) {
    cy.jackioh().then((handle) => {
      handle.setSeat?.(seat);
    });
    cy.get(ts(GAME), { timeout: timeouts.view }).should("have.attr", "data-viewer", seat);
    cy.settled();
    cy.jackioh().then((handle) => {
      const ids = handCardIds(handle as Handle);
      cy.get(ts(handRegionId("you")), { timeout: timeouts.view }).should(($hand) => {
        const drawn = $hand
          .find(HAND_CARD)
          .toArray()
          .map((card) => card.getAttribute("data-testid") ?? "");
        expect(drawn, `${seat}'s hand on the board is its view's`).to.deep.equal(ids);
      });
      for (const id of ids) readInspect(ts(id), `${seat}'s ${id}`);
    });
  }
}

describe("36 — every Meditative card plays once on /dev/hotseat with the set previewed", () => {
  before(() => {
    cy.readFile(CATALOG_PATH, { log: false }).then((catalog: Record<string, CatalogRow>) => {
      cards = deckableOf(catalog, SET);
      filler = deckableOf(catalog, FILLER_SET)[0];
      remaining.clear();
      for (const def of cards) remaining.add(def.id);
    });
  });

  it("the premise: the catalog holds the set, and a card of a set that ships stands by to fill a seat", () => {
    expect(cards.length, `the deckable ${SET} cards`).to.be.greaterThan(0);
    expect(filler, `a deckable ${FILLER_SET} card`).to.not.eq(undefined);
  });

  for (let index = 1; index <= MAX_GAMES; index += 1) {
    it(`game ${String(index)}: the cards still to play are played, each settling, logged in English and inspected`, () => {
      if (remaining.size === 0 || filler === undefined) return;
      const [a, b] = nextGame(
        cards.filter((def) => remaining.has(def.id)),
        filler,
      );
      const dealt = [...a, ...b];
      const mana = Math.max(constants.MAX_MANA, ...dealt.map(ceiling));
      const game: Game = {
        label: `game ${String(index)}`,
        toPlay: new Set(dealt.filter((def) => remaining.has(def.id)).map((def) => def.id)),
        arrivals: dealt.filter((def) => remaining.has(def.id) && castOnDraw(def)),
        emptyEnds: 0,
      };
      const [idA, idB] = [`36-game-${String(index)}-a`, `36-game-${String(index)}-b`];
      cy.seedGame({
        seed: `meditative-play-through-${String(index)}`,
        a: idA,
        b: idB,
        decks: {
          [idA]: { cards: a.map((def) => def.id), handicap: handicapFor(a.length, mana) },
          [idB]: { cards: b.map((def) => def.id), handicap: handicapFor(b.length, mana) },
        },
        preview: [SET],
        // The driver answers both mulligans itself, keeping every card.
        mulligan: "manual",
      });
      drive(game, STEP_BUDGET);
      readHands();
    });
  }

  it("every deckable Meditative card was played, or was cast as it was drawn", () => {
    expect([...remaining], "cards no game played").to.deep.equal([]);
  });
});
