// Spec 13: practice against the AI without a server (SPEC §9.9, R187; B40).
// It observes R265 mulligan thinking, R745 log retention, R181/R180 handicaps, R668 reload, and R765 save/leave.
// BUILD M8 uses deterministic seeds, retried testids, and no `cy.wait(ms)`; recorder history and reduced motion make BUILD M5-T4 AI turns observable.

import { seedFor, timeouts } from "../../support/config.ts";
import {
  ACTION_ERROR,
  ANIMATING,
  BOARD,
  END_TURN,
  GAME,
  LOG,
  MULLIGAN_OPPONENT_READY,
  MULLIGAN_OPPONENT_STATUS,
  PRACTICE_DECK,
  PRACTICE_ERROR,
  PRACTICE_HUD,
  PRACTICE_LEAVE,
  PRACTICE_LEAVE_CONFIRM,
  PRACTICE_LEAVE_SAVE,
  PRACTICE_MENU,
  PRACTICE_NEW_GAME,
  PRACTICE_RESULT,
  PRACTICE_RESUME,
  PRACTICE_RESUME_BANNER,
  PRACTICE_SETUP,
  PRACTICE_START,
  PRACTICE_THINKING,
  PROMPT,
  PROMPT_SUBMIT,
  RESULT_OVERLAY,
  cardId,
  handCountId,
  manaId,
  practiceDifficultyId,
  promptOf,
  promptOptionId,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { Action, Lane, PlayerId } from "../../support/types.ts";

/** BUILD M8 seed keeps the Easy game alive for four human turns despite R635's cost-bucket deal. */
const SEED = seedFor("13-practice-t");

/** The Hard seed yields an observable first AI turn. */
const HARD_SEED = `${seedFor("13-practice")}:hard`;

const HUMAN_TURNS = 3;

/** The worker loads engine, card scripts, and AI before its first answer. */
const BOOT_TIMEOUT = 60_000;

/** Bound rare cast-on-draw prompts (R58). */
const PROMPT_BUDGET = 10;

type Difficulty = "easy" | "medium" | "hard";

// Page dev handle.

/** Structural subset of `PracticeDebug`. */
type PracticeDebugLike = {
  seed: string;
  decks: [string[], string[]];
  handicaps: Partial<Record<PlayerId, unknown>>;
  log: Action[];
  state: unknown;
  hash: string;
  difficulty: Difficulty;
  humanSeat: PlayerId;
  /** R433: dealt seats for the random deck. */
  dealt?: PlayerId[];
};

type PracticeHandleLike = {
  snapshot(): Promise<PracticeDebugLike>;
  readonly aiSeat: PlayerId | null;
  readonly thinking: boolean;
};

// Recorder.

type Sample = {
  thinking: boolean;
  turn: string | null;
  active: string | null;
  phase: string | null;
  opponentMax: string | null;
  opponentHand: string | null;
  opponentUnits: number;
  prompt: string | null;
  /** R265: the human picker reports the AI mulligan with `data-ready`. */
  opponentReady: string | null;
};

type Recorder = { samples: Sample[]; sockets: number };

type PracticeWindow = { __jackiohPractice?: PracticeHandleLike; __practiceRecorder?: Recorder };

const ANY_CARD = `[data-testid^="${cardId("")}"]`;

const LANES: readonly Lane[] = [1, 2, 3, 4, 5];

function opponentUnitCount(doc: Document): number {
  return LANES.reduce(
    (count, lane) => count + (doc.querySelector(ts(zoneId("opponent", "units", lane)))?.querySelectorAll(ANY_CARD).length ?? 0),
    0,
  );
}

function sampleOf(doc: Document): Sample {
  const board = doc.querySelector(ts(BOARD));
  return {
    thinking: doc.querySelector(ts(PRACTICE_THINKING)) !== null,
    turn: board?.getAttribute("data-turn") ?? null,
    active: board?.getAttribute("data-active") ?? null,
    phase: board?.getAttribute("data-phase") ?? null,
    opponentMax: doc.querySelector(ts(manaId("opponent")))?.getAttribute("data-max") ?? null,
    opponentHand: doc.querySelector(ts(handCountId("opponent")))?.textContent?.trim() ?? null,
    opponentUnits: opponentUnitCount(doc),
    prompt: doc.querySelector(PROMPT)?.getAttribute("data-prompt-kind") ?? null,
    opponentReady: doc.querySelector(ts(MULLIGAN_OPPONENT_STATUS))?.getAttribute("data-ready") ?? null,
  };
}

/** Record page states and WebSockets from `onBeforeLoad`. */
function installRecorder(win: Cypress.AUTWindow): void {
  const recorder: Recorder = { samples: [], sockets: 0 };
  (win as unknown as PracticeWindow).__practiceRecorder = recorder;

  const RealSocket = win.WebSocket;
  win.WebSocket = new Proxy(RealSocket, {
    construct(target, args: unknown[]) {
      recorder.sockets += 1;
      return Reflect.construct(target, args) as object;
    },
  });

  const record = (): void => {
    const next = sampleOf(win.document);
    const last = recorder.samples.at(-1);
    if (last === undefined || JSON.stringify(last) !== JSON.stringify(next)) recorder.samples.push(next);
  };
  new win.MutationObserver(record).observe(win.document, {
    subtree: true,
    childList: true,
    attributes: true,
    characterData: true,
  });
}

/** Reduce motion so the recorder observes every drawn view. */
function preferReducedMotion(win: Cypress.AUTWindow): void {
  const real = win.matchMedia.bind(win);
  win.matchMedia = (query: string): MediaQueryList => {
    if (query.replace(/\s+/g, "") !== "(prefers-reduced-motion:reduce)") return real(query);
    return {
      matches: true,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      dispatchEvent: () => false,
    } as unknown as MediaQueryList;
  };
}

function recorder(): Cypress.Chainable<Recorder> {
  return cy.window({ log: false }).then((win) => {
    const found = (win as unknown as PracticeWindow).__practiceRecorder;
    expect(found, "the recorder installed in onBeforeLoad").to.not.eq(undefined);
    return found as Recorder;
  });
}

function practiceHandle(): Cypress.Chainable<PracticeHandleLike> {
  return cy
    .window({ timeout: timeouts.view })
    .should((win) => {
      expect((win as unknown as PracticeWindow).__jackiohPractice, "window.__jackiohPractice (dev builds)").to.not.eq(
        undefined,
      );
    })
    .then((win) => (win as unknown as PracticeWindow).__jackiohPractice as PracticeHandleLike);
}

// Drive the page.

function practiceUrl(seed: string, difficulty: Difficulty, seat: PlayerId): string {
  const params = new URLSearchParams({ seed, difficulty, deck: "random", seat, pace: "fast" });
  return `/practice?${params.toString()}`;
}

function visitPractice(url: string, options: { reducedMotion?: boolean } = {}): void {
  cy.visit(url, {
    onBeforeLoad(win) {
      installRecorder(win);
      if (options.reducedMotion === true) preferReducedMotion(win);
    },
  });
}

const ANY_PROMPT_OPTION = `[data-testid^="${promptOptionId("")}"]`;

function isPicked($option: JQuery<HTMLElement>): boolean {
  const marked = $option.closest("[aria-pressed], [data-selected]");
  const element = marked.length > 0 ? marked : $option;
  return element.attr("aria-pressed") === "true" || element.attr("data-selected") === "true";
}

/** The AI can take whole turns between board settlements. */
function settledThroughAiTurn(): void {
  cy.get(ANIMATING, { timeout: timeouts.game, log: false }).should("not.exist");
}

/** R9: select every mulligan card, then confirm. */
function keepWholeHand(): void {
  cy.get(promptOf("mulligan"), { timeout: BOOT_TIMEOUT }).should("be.visible");
  cy.get(promptOf("mulligan")).within(() => {
    cy.get(ANY_PROMPT_OPTION).each(($option) => {
      if (isPicked($option)) return;
      cy.wrap($option, { log: false }).click();
    });
    cy.get(ts(PROMPT_SUBMIT)).click();
  });
  settledThroughAiTurn();
}

function answerFirstOption(): void {
  cy.get(PROMPT).first().within(() => {
    cy.get(ANY_PROMPT_OPTION).first().click();
  });
  cy.get("body").then(($body) => {
    const submit = $body.find(ts(PROMPT_SUBMIT));
    if (submit.length > 0 && !submit.is(":disabled")) cy.wrap(submit.first()).click();
  });
  settledThroughAiTurn();
}

function opponentSide(): Cypress.Chainable<string> {
  return cy.document({ log: false }).then((doc) => {
    const sample = sampleOf(doc);
    return JSON.stringify({ max: sample.opponentMax, hand: sample.opponentHand, units: sample.opponentUnits });
  });
}

type Waited = "turn" | "prompt" | "over";

function waitForHuman(): Cypress.Chainable<Waited> {
  return cy
    .get("body", { timeout: timeouts.game })
    .should(($body) => {
      const board = $body.find(ts(BOARD));
      const endTurn = $body.find(ts(END_TURN));
      const myTurn =
        board.attr("data-active") === "you" &&
        board.attr("data-phase") === "main" &&
        endTurn.length > 0 &&
        !endTurn.is(":disabled");
      const prompt = $body.find(PROMPT).length > 0;
      const over = $body.find(ts(RESULT_OVERLAY)).length > 0;
      expect(myTurn || prompt || over, "the human's main phase, a prompt for the human, or a result").to.eq(true);
    })
    .then(($body): Waited => {
      if ($body.find(ts(RESULT_OVERLAY)).length > 0) return "over";
      if ($body.find(PROMPT).length > 0) return "prompt";
      return "turn";
    });
}

/** Reach the human main phase, resolving at most PROMPT_BUDGET prompts. */
function reachHumanTurn(prompts = PROMPT_BUDGET): void {
  waitForHuman().then((waited) => {
    expect(waited, "the game is still on when the human is due to act").to.not.eq("over");
    if (waited === "prompt") {
      expect(prompts, "the human's prompts run out").to.be.greaterThan(0);
      answerFirstOption();
      reachHumanTurn(prompts - 1);
      return;
    }
    cy.settled();
  });
}

function endHumanTurns(remaining: number): void {
  if (remaining === 0) return;
  reachHumanTurn();
  cy.get(ts(ACTION_ERROR)).should("not.exist");
  cy.get(ts(END_TURN)).should("not.be.disabled").click();
  settledThroughAiTurn();
  cy.get(ts(ACTION_ERROR)).should("not.exist");
  endHumanTurns(remaining - 1);
}

// No server.

let apiRequests = 0;

beforeEach(() => {
  apiRequests = 0;
  cy.intercept({ url: "**/api/**" }, () => {
    apiRequests += 1;
  });
});

function expectNoServer(): void {
  cy.then(() => {
    expect(apiRequests, "requests to /api").to.eq(0);
  });
  recorder().then((recorded) => {
    expect(recorded.sockets, "WebSockets opened").to.eq(0);
  });
}

// Spec.

describe("13 — practice against the AI, with no account and no server (§9.9, B40)", () => {
  it("B40 anonymous /practice shows setup", () => {
    visitPractice("/practice");

    cy.get(ts(PRACTICE_SETUP), { timeout: BOOT_TIMEOUT }).should("be.visible");
    for (const difficulty of ["easy", "medium", "hard"] as const) {
      cy.get(ts(practiceDifficultyId(difficulty))).should("have.attr", "type", "radio");
    }
    cy.get(ts(practiceDifficultyId("easy"))).should("be.checked");
    cy.get(ts(PRACTICE_DECK)).should(($select) => {
      const values = Array.from(($select[0] as HTMLSelectElement).options).map((option) => option.value);
      expect(values, "a random deck is always offered").to.include("random");
      expect(values.filter((value) => value.startsWith("saved:")), "no saved decks without an account").to.deep.eq([]);
    });
    cy.get(ts(PRACTICE_START)).should("be.visible");
    cy.get(ts(PRACTICE_HUD)).should("not.exist");
    cy.get(ts(PRACTICE_ERROR)).should("not.exist");

    expectNoServer();
  });

  it("B40 an Easy game seated p2: the AI thinks and plays, human turns end cleanly, the log replays, and a concede is a Loss", () => {
    visitPractice(practiceUrl(SEED, "easy", "p2"));

    cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT })
      .should("have.attr", "data-difficulty", "easy")
      .and("have.attr", "data-human-seat", "p2")
      .and("have.attr", "data-ai-seat", "p1");
    cy.get(ts(GAME)).should("have.attr", "data-viewer", "p2");
    cy.get(ts(PRACTICE_SETUP)).should("not.exist");

    // R265: the AI answers its mulligan while the human picker remains open.
    cy.get(promptOf("mulligan"), { timeout: BOOT_TIMEOUT }).should("be.visible");
    cy.get(promptOf("mulligan"))
      .find(ts(MULLIGAN_OPPONENT_READY), { timeout: timeouts.view })
      .should("be.visible");
    recorder().then((recorded) => {
      const firstPrompt = recorded.samples.findIndex((sample) => sample.prompt === "mulligan");
      expect(firstPrompt, "the human's mulligan was drawn").to.be.greaterThan(-1);
      const aiReady = recorded.samples.findIndex(
        (sample) => sample.prompt === "mulligan" && sample.opponentReady === "true",
      );
      expect(aiReady, "the AI answered its mulligan while the human's picker was open (R265)").to.be.greaterThan(-1);
      expect(
        recorded.samples.slice(0, aiReady).some((sample) => sample.thinking),
        "practice-thinking showed while the AI mulliganed",
      ).to.eq(true);
    });

    opponentSide().then((beforeTheAiTurn) => {
      keepWholeHand();
      reachHumanTurn();
      cy.get(ts(ACTION_ERROR)).should("not.exist");
      opponentSide().then((afterTheAiTurn) => {
        expect(afterTheAiTurn, "the AI's first turn changed its mana, hand or board").to.not.eq(beforeTheAiTurn);
      });
    });

    let firstEnded = "";
    reachHumanTurn();
    cy.get(ts(BOARD))
      .invoke("attr", "data-turn")
      .then((turn) => {
        firstEnded = String(turn);
      });

    endHumanTurns(HUMAN_TURNS);

    recorder().then((recorded) => {
      let lastMulligan = -1;
      recorded.samples.forEach((sample, index) => {
        if (sample.prompt === "mulligan") lastMulligan = index;
      });
      expect(
        recorded.samples.slice(lastMulligan + 1).some((sample) => sample.thinking),
        "practice-thinking showed while the AI played its turns",
      ).to.eq(true);
    });

    // R745: the log retains early turns.
    reachHumanTurn();
    cy.then(() => {
      cy.get(`${ts(LOG)} .log-line[data-event="turnEnded"]`).should("contain.text", `You ended turn ${firstEnded} with`);
    });
    cy.get(`${ts(LOG)} .log-line[data-event="turnStarted"]`).should("contain.text", "Turn 1: Opponent");

    // §10.8 makes the concede overlay viewer-relative.
    reachHumanTurn();
    cy.concede();
    cy.get(ts(RESULT_OVERLAY), { timeout: timeouts.view }).should("be.visible").and("contain.text", "Loss");
    cy.get(ts(PRACTICE_RESULT), { timeout: timeouts.view })
      .should("be.visible")
      .and("have.attr", "data-outcome", "loss")
      .and("contain.text", "Defeat");
    cy.get(ts(ACTION_ERROR)).should("not.exist");

    // R187: fold the browser log and reported handicaps to its hash.
    practiceHandle()
      .then((handle) => handle.snapshot())
      .then((debug) => {
        expect(debug.seed, "the URL's seed reached the core").to.eq(SEED);
        expect(debug.difficulty).to.eq("easy");
        expect(debug.humanSeat).to.eq("p2");
        expect(debug.log.at(-1), "the game ended on the human's concede").to.include({ type: "concede", playerId: "p2" });
        expect(debug.log.some((action) => action.playerId === "p1"), "the AI acted").to.eq(true);

        cy.task<{ replayHash: string; browserHash: string; errors: unknown[] }>(
          "replayHash",
          {
            label: "13-practice",
            seed: debug.seed,
            decks: debug.decks,
            log: debug.log,
            state: debug.state,
            handicaps: debug.handicaps,
            ...(debug.dealt === undefined ? {} : { dealt: debug.dealt }),
          },
          { timeout: timeouts.task },
        ).then((result) => {
          expect(result.errors, "the recorded log replays with no rejected action").to.deep.eq([]);
          expect(result.browserHash, "hashState of the page's final state is the page's own hash").to.eq(debug.hash);
          expect(result.replayHash, "the Node fold reaches the browser's final state").to.eq(result.browserHash);
        });
      });

    expectNoServer();
  });

  it("B40 a Hard game seated p2: the AI's first turn shows mana-opponent data-max=2", () => {
    visitPractice(practiceUrl(HARD_SEED, "hard", "p2"), { reducedMotion: true });

    cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT })
      .should("have.attr", "data-difficulty", "hard")
      .and("have.attr", "data-ai-seat", "p1");
    keepWholeHand();
    reachHumanTurn();

    recorder().then((recorded) => {
      const firstAiTurn = recorded.samples.find(
        (sample) => sample.turn === "1" && sample.active === "opponent" && sample.phase === "main",
      );
      expect(firstAiTurn, "the board showed the AI's first turn").to.not.eq(undefined);
      expect(firstAiTurn?.opponentMax, "Hard refreshes to min(turns + 1, 7) = 2 on its first turn (R181)").to.eq("2");
    });

    // R180 / R187: this Hard game carries its handicap through replay.
    practiceHandle()
      .then((handle) => handle.snapshot())
      .then((debug) => {
        expect(debug.difficulty).to.eq("hard");
        expect(debug.handicaps, "the AI seat's handicap reached the snapshot").to.have.property("p1");
        expect(debug.handicaps.p1, "Hard's handicap (R180)").to.deep.include({
          deckSize: 30,
          manaBonus: 1,
          manaCap: 7,
          extraOpeningCards: 1,
          extraDrawsPerTurn: 1,
        });
        expect(debug.decks[0], "the Hard AI's 30-card deck (R184)").to.have.length(30);
        expect(debug.log.some((action) => action.playerId === "p1"), "the AI acted").to.eq(true);

        cy.task<{ replayHash: string; browserHash: string; errors: unknown[] }>(
          "replayHash",
          {
            label: "13-practice-hard",
            seed: debug.seed,
            decks: debug.decks,
            log: debug.log,
            state: debug.state,
            handicaps: debug.handicaps,
            ...(debug.dealt === undefined ? {} : { dealt: debug.dealt }),
          },
          { timeout: timeouts.task },
        ).then((result) => {
          expect(result.errors, "the Hard game's log replays with no rejected action").to.deep.eq([]);
          expect(result.browserHash, "hashState of the page's state is the page's own hash").to.eq(debug.hash);
          expect(result.replayHash, "the Node fold, handicaps included, reaches the browser's state").to.eq(
            result.browserHash,
          );
        });
      });

    expectNoServer();
  });
  it("R668 a reload in the middle of a game picks it up on the same state", () => {
    visitPractice(practiceUrl(`${SEED}:reload`, "medium", "p2"), { reducedMotion: true });
    cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT }).should("have.attr", "data-difficulty", "medium");
    keepWholeHand();
    reachHumanTurn();

    practiceHandle()
      .then((handle) => handle.snapshot())
      .then((before) => {
        expect(before.log.some((action) => action.playerId === "p1"), "the AI acted").to.eq(true);

        // R668: without a new-game parameter, the worker restores its saved game.
        visitPractice("/practice?pace=fast");
        cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT })
          .should("have.attr", "data-difficulty", "medium")
          .and("have.attr", "data-human-seat", "p2");
        cy.get(ts(PRACTICE_SETUP)).should("not.exist");
        cy.get(ts(PRACTICE_ERROR)).should("not.exist");
        reachHumanTurn();
        practiceHandle()
          .then((handle) => handle.snapshot())
          .then((after) => {
            expect(after.seed).to.eq(before.seed);
            expect(after.log, "the same log, folded back").to.deep.eq(before.log);
            expect(after.hash, "the same state").to.eq(before.hash);
          });
      });

    expectNoServer();
  });

  it("R765 Save and leave keeps the game for the menu's banner, Resume picks it up on the same state, and Leave without saving keeps nothing", () => {
    visitPractice(practiceUrl(`${SEED}:save`, "medium", "p2"), { reducedMotion: true });
    cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT }).should("have.attr", "data-difficulty", "medium");
    keepWholeHand();
    reachHumanTurn();

    practiceHandle()
      .then((handle) => handle.snapshot())
      .then((before) => {
        expect(before.log.some((action) => action.playerId === "p1"), "the AI acted").to.eq(true);

        cy.get(ts(PRACTICE_NEW_GAME)).click();
        cy.get(ts(PRACTICE_LEAVE)).should("have.attr", "data-can-save", "true");
        cy.get(ts(PRACTICE_LEAVE_SAVE)).click();
        cy.get(ts(PRACTICE_SETUP), { timeout: BOOT_TIMEOUT }).should("be.visible");
        cy.get(ts(PRACTICE_RESUME_BANNER)).should("have.attr", "data-difficulty", "medium");
        cy.get(ts(PRACTICE_HUD)).should("not.exist");

        visitPractice("/practice?pace=fast");
        cy.get(ts(PRACTICE_SETUP), { timeout: BOOT_TIMEOUT }).should("be.visible");
        cy.get(ts(PRACTICE_RESUME_BANNER)).should("be.visible");
        cy.get(ts(PRACTICE_HUD)).should("not.exist");

        cy.get(ts(PRACTICE_RESUME)).click();
        cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT })
          .should("have.attr", "data-difficulty", "medium")
          .and("have.attr", "data-human-seat", "p2");
        cy.get(ts(PRACTICE_ERROR)).should("not.exist");
        reachHumanTurn();
        practiceHandle()
          .then((handle) => handle.snapshot())
          .then((after) => {
            expect(after.seed).to.eq(before.seed);
            expect(after.log, "the same log, folded back").to.deep.eq(before.log);
            expect(after.hash, "the same state").to.eq(before.hash);
          });
      });

    cy.get(ts(PRACTICE_MENU)).click();
    cy.get(ts(PRACTICE_LEAVE)).should("be.visible");
    cy.get(ts(PRACTICE_LEAVE_CONFIRM)).click();
    cy.location("pathname").should("eq", "/");
    visitPractice("/practice?pace=fast");
    cy.get(ts(PRACTICE_SETUP), { timeout: BOOT_TIMEOUT }).should("be.visible");
    cy.get(ts(PRACTICE_RESUME_BANNER)).should("not.exist");
    cy.get(ts(PRACTICE_HUD)).should("not.exist");

    expectNoServer();
  });
});
