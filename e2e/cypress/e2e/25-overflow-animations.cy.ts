// Overflow animation coverage (SPEC §2.4; R80, R82, R119, R180, R182, R184, R201, R244, R315,
// R316, R317, R318, R373; BUILD M5-T1, M5-T3, M5-T4).
// Fixtures assert their preconditions because deck order affects each seeded game.
// BUILD M8: each fixture owns its seed and uses retried waits.

import { CARD_NAMES, TOKEN_NAMES } from "../../support/cards.ts";
import { FX_SETTINGS_KEY, constants, seedFor, timeouts } from "../../support/config.ts";
import {
  DAMAGE_POP,
  burnCardId,
  burnNoticeId,
  handCountId,
  handRegionId,
  heroId,
  libraryCountId,
  libraryId,
  overflowCardId,
  pileNoticeId,
  ts,
  type PileNoticeKind,
} from "../../support/testids.ts";
import type { PlayerId, Side } from "../../support/types.ts";

const SEEDS = {
  fatigue: seedFor("25-fatigue"),
  handFull: seedFor("25-hand-full-1"),
  libraryFull: seedFor("25-library-full-1093"),
};

const BYSTANDER = "08-do-nothing-b";

/** Faces print SPEC §8 names (BUILD M5-T1). */
function nameOf(index: number): string {
  const name = CARD_NAMES[index];
  if (name === undefined) throw new Error(`no SPEC §8 card #${String(index)}`);
  return name;
}

const STOCKPILE = nameOf(5);
const MANA_WELL = nameOf(6);
const THE_ROCK = nameOf(66);
const CLONE_MACHINE = nameOf(33);
const GARY = nameOf(4);
const VANILLA = nameOf(8);
const INJECTION = nameOf(90);
const CN_VIRUS = TOKEN_NAMES["90.1"] ?? "CN-Virus";
const THE_COIN = TOKEN_NAMES["T-coin"] ?? "The Coin";

// Screenshot pass

const SHOTS = ["1", "true"].includes(String(Cypress.expose("shots") ?? ""));

type Viewport = { label: string; width: number; height: number; phone: boolean };
const VIEWPORTS: readonly Viewport[] = [
  { label: "1280x720", width: 1280, height: 720, phone: false },
  { label: "390x844", width: 390, height: 844, phone: true },
];
/** R201: half speed keeps notices visible for screenshots. */
const SHOT_FX = { speed: 0.5, intensity: "normal", motion: "system" } as const;

/** Small screens need visible hand clicks. */
function handClick(viewport: Viewport | null): { visiblePart?: boolean } {
  return viewport?.phone === true ? { visiblePart: true } : {};
}

/** Screenshot the notice while its own motion is in flight. */
type Shoot = (name: string, playing?: string) => void;
const NO_SHOTS: Shoot = () => undefined;

function shooterFor(viewport: Viewport): Shoot {
  return (name, playing) => {
    if (playing !== undefined) {
      cy.get(ts(playing), { timeout: timeouts.animation }).should("have.attr", "data-playing", "true");
    }
    // Capturing the motion requires CSS animations to keep running.
    cy.screenshot(`25-overflow-animations/${viewport.label}/${name}`, {
      capture: "viewport",
      disableTimersAndAnimations: false,
    });
  };
}

// Notice recorder

/** Retry assertions can miss transient notice states. */
type NoticeState = {
  testid: string;
  kind: string | null;
  playing: boolean;
  text: string;
  face: string | null;
  outcome: string | null;
};
type NoticeRecorder = { seen: NoticeState[] };
type RecordingWindow = { __overflowNotices?: NoticeRecorder };

const NOTICE_PREFIXES = ["pile-notice-", "burn-notice-"] as const;
const NOTICE_CARD_PREFIXES = ["overflow-card-", "burn-card-"] as const;
const startsWithAny = (prefixes: readonly string[]): string =>
  prefixes.map((prefix) => `[data-testid^="${prefix}"]`).join(", ");

function installNoticeRecorder(win: Cypress.AUTWindow): void {
  const host = win as unknown as RecordingWindow;
  if (host.__overflowNotices !== undefined) return;
  const recorder: NoticeRecorder = { seen: [] };
  host.__overflowNotices = recorder;
  const last = new Map<string, string>();

  const record = (): void => {
    const present = new Set<string>();
    for (const element of Array.from(win.document.querySelectorAll(startsWithAny(NOTICE_PREFIXES)))) {
      const testid = element.getAttribute("data-testid") ?? "";
      present.add(testid);
      const card = element.querySelector(startsWithAny(NOTICE_CARD_PREFIXES));
      const state: NoticeState = {
        testid,
        kind: element.getAttribute("data-kind"),
        playing: element.getAttribute("data-playing") === "true",
        text: element.textContent ?? "",
        face: card?.getAttribute("data-face") ?? null,
        outcome: card?.getAttribute("data-outcome") ?? null,
      };
      const key = JSON.stringify(state);
      if (last.get(testid) === key) continue;
      last.set(testid, key);
      recorder.seen.push(state);
    }
    // A remounted notice must be recorded again.
    for (const testid of Array.from(last.keys())) if (!present.has(testid)) last.delete(testid);
  };
  new win.MutationObserver(record).observe(win.document, {
    subtree: true,
    childList: true,
    attributes: true,
    characterData: true,
  });
  record();
}

function clearNotices(): void {
  cy.window({ log: false }).then((win) => {
    const recorder = (win as unknown as RecordingWindow).__overflowNotices;
    expect(recorder, "the notice recorder is installed").to.not.eq(undefined);
    if (recorder !== undefined) recorder.seen.length = 0;
  });
}

function recorded(testid: string): Cypress.Chainable<NoticeState[]> {
  return cy.window({ log: false }).then((win) => {
    const seen = (win as unknown as RecordingWindow).__overflowNotices?.seen ?? [];
    return seen.filter((state) => state.testid === testid);
  });
}

// Helpers

type Game = { seed: string; a: string; b: string };

function startGame(game: Game, viewport: Viewport | null): void {
  if (viewport !== null) cy.viewport(viewport.width, viewport.height);
  cy.seedGame({
    ...game,
    onBeforeLoad(win) {
      if (viewport !== null) {
        try {
          win.localStorage.setItem(FX_SETTINGS_KEY, JSON.stringify(SHOT_FX));
        } catch (error) {
          Cypress.log({ name: "fx speed", message: `localStorage unavailable: ${String(error)}` });
        }
      }
      installNoticeRecorder(win);
    },
  });
}

function holdDevice(seat: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== seat) cy.handOver();
  });
  cy.jackioh().its("seat").should("eq", seat);
}

function expectCount(testid: string, count: number): void {
  cy.get(ts(testid), { timeout: timeouts.view }).should("have.text", String(count));
}

function expectPileNotice(side: Side, kind: PileNoticeKind, text: string): void {
  cy.get(ts(libraryId(side)), { timeout: timeouts.animation })
    .find(ts(pileNoticeId(side)), { timeout: timeouts.animation })
    .should("have.attr", "data-kind", kind)
    .and("contain.text", text);
}

function expectOverflowCard(side: Side, name: string, outcome: "notCreated" | "graveyard" | "ceased"): void {
  cy.get(ts(pileNoticeId(side)))
    .find(ts(overflowCardId(side)), { timeout: timeouts.animation })
    .should("have.attr", "data-face", "face")
    .and("have.attr", "data-outcome", outcome)
    .and("contain.text", name);
}

function expectBurnNotice(side: Side, name: string): void {
  cy.get(ts(handRegionId(side)), { timeout: timeouts.animation })
    .find(ts(burnNoticeId(side)), { timeout: timeouts.animation })
    .should("contain.text", "Hand full");
  cy.get(ts(burnNoticeId(side)))
    .find(ts(burnCardId(side)))
    .should("have.attr", "data-face", "face")
    .and("contain.text", name);
}

function expectDamagePop(side: Side, amount: number): void {
  cy.get(ts(heroId(side)), { timeout: timeouts.animation })
    .find(DAMAGE_POP, { timeout: timeouts.animation })
    .should("contain.text", String(amount));
}

/** R318: a drained runner removes notices. */
function expectNoticesGone(side: Side): void {
  cy.settled();
  cy.get(ts(pileNoticeId(side)), { timeout: timeouts.animation }).should("not.exist");
  cy.get(ts(burnNoticeId(side)), { timeout: timeouts.animation }).should("not.exist");
}

function expectPlayed(testid: string): void {
  recorded(testid).then((states) => {
    expect(states.length, `${testid} was drawn`).to.be.greaterThan(0);
    expect(
      states.some((state) => state.playing),
      `${testid} carried data-playing="true" while its entry ran`,
    ).to.eq(true);
  });
}

// The three games

/** R315: fatigue animation runs on the device that sees its event. */
function fatigue(viewport: Viewport | null, shoot: Shoot): void {
  startGame({ seed: SEEDS.fatigue, a: "25-fatigue-a", b: BYSTANDER }, viewport);

  // R184: the opening hand and first draw exhaust this four-card library.
  holdDevice("p1");
  expectCount(libraryCountId("you"), 0);
  expectCount(handCountId("you"), 4);
  cy.endTurn();

  holdDevice("p2");
  clearNotices();
  cy.endTurn({
    handOver: false,
    expectAnimating: "fatigue",
    during: () => {
      expectPileNotice("opponent", "fatigue", "Fatigue 1");
      expectDamagePop("opponent", 1);
      shoot("fatigue-opponent");
    },
  });
  expectNoticesGone("opponent");
  expectPlayed(pileNoticeId("opponent"));

  holdDevice("p1");
  clearNotices();
  cy.playByName(STOCKPILE, {
    ...handClick(viewport),
    expectAnimating: "fatigue",
    during: () => {
      expectPileNotice("you", "fatigue", "Fatigue 3");
      expectDamagePop("you", 3);
      shoot("fatigue-you");
    },
  });
  expectNoticesGone("you");
  recorded(pileNoticeId("you")).then((states) => {
    const counts = states
      .map((state) => /Fatigue (\d+)/.exec(state.text)?.[1])
      .filter((count): count is string => count !== undefined)
      .filter((count, at, all) => at === 0 || all[at - 1] !== count);
    expect(counts, "R315: each fatigue draw's badge, in draw order").to.deep.eq(["2", "3"]);
  });
  expectPlayed(pileNoticeId("you"));

  cy.replayCheck("25-fatigue");
}

/** R317: a full hand burns on the device that sees the draw. */
function handFull(viewport: Viewport | null, shoot: Shoot): void {
  startGame({ seed: SEEDS.handFull, a: "25-hand-full-a", b: BYSTANDER }, viewport);

  // R182: opening cards and the first draw fill the hand.
  holdDevice("p1");
  expectCount(handCountId("you"), constants.HAND_CAP);
  cy.handCardByName(STOCKPILE);
  cy.endTurn();

  holdDevice("p2");
  clearNotices();
  cy.endTurn({
    handOver: false,
    expectAnimating: "burned",
    during: () => {
      shoot("hand-full-opponent", burnNoticeId("opponent"));
      expectBurnNotice("opponent", MANA_WELL);
    },
  });
  expectNoticesGone("opponent");
  expectPlayed(burnNoticeId("opponent"));
  expectCount(handCountId("opponent"), constants.HAND_CAP);

  // Stockpile refills the hand before its second draw burns.
  holdDevice("p1");
  clearNotices();
  cy.playByName(STOCKPILE, {
    ...handClick(viewport),
    expectAnimating: "burned",
    during: () => {
      shoot("hand-full-you", burnNoticeId("you"));
      expectBurnNotice("you", THE_ROCK);
    },
  });
  expectNoticesGone("you");
  expectPlayed(burnNoticeId("you"));
  expectCount(handCountId("you"), constants.HAND_CAP);

  cy.replayCheck("25-hand-full");
}

/** R80, R316: test each full-library refusal. */
function libraryFull(viewport: Viewport | null, shoot: Shoot): void {
  startGame({ seed: SEEDS.libraryFull, a: "25-library-full-a", b: "25-library-full-b" }, viewport);

  holdDevice("p1");
  expectCount(libraryCountId("you"), constants.LIBRARY_CAP - 4);
  for (const name of [CLONE_MACHINE, GARY, VANILLA]) cy.handCardByName(name);

  cy.playByName(CLONE_MACHINE, { ...handClick(viewport), zone: { side: "you", row: "backrow", lane: 1 } });
  // R119: Clone Machine does not copy itself; Gary leaves 59.
  cy.playByName(GARY, { ...handClick(viewport), zone: { side: "you", row: "units", lane: 1 } });
  expectCount(libraryCountId("you"), constants.LIBRARY_CAP - 1);

  // R80: only Vanilla's first copy fills the library.
  clearNotices();
  cy.playByName(VANILLA, {
    ...handClick(viewport),
    zone: { side: "you", row: "units", lane: 2 },
    expectAnimating: "libraryOverflow",
    during: () => {
      shoot("library-full-you", pileNoticeId("you"));
      expectPileNotice("you", "libraryFull", "Deck full");
      expectOverflowCard("you", VANILLA, "notCreated");
    },
  });
  expectNoticesGone("you");
  expectPlayed(pileNoticeId("you"));
  expectCount(libraryCountId("you"), constants.LIBRARY_CAP);

  // Seat 2's handicap funds Injection; R244 leaves Coin in hand and R82 keeps the turn open.
  cy.endTurn();
  holdDevice("p2");
  cy.handCardByName(INJECTION);
  cy.handCardByName(THE_COIN);
  clearNotices();
  cy.playByName(INJECTION, {
    ...handClick(viewport),
    expectAnimating: "libraryOverflow",
    during: () => {
      shoot("library-full-opponent", pileNoticeId("opponent"));
      expectPileNotice("opponent", "libraryFull", "Deck full");
      expectOverflowCard("opponent", CN_VIRUS, "notCreated");
    },
  });
  expectNoticesGone("opponent");
  expectPlayed(pileNoticeId("opponent"));
  expectCount(libraryCountId("opponent"), constants.LIBRARY_CAP);

  cy.replayCheck("25-library-full");
}

describe("Spec 25 — fatigue, a full hand and a full library on the board (R315–R318)", () => {
  it("R315 fatigue: 'Fatigue 1' and the hit on the other seat's device, 'Fatigue 2' then 'Fatigue 3' on the owner's", () => {
    fatigue(null, NO_SHOTS);
  });

  it("R317 hand full: the burned card's face under 'Hand full', on the other seat's device and then the owner's", () => {
    handFull(null, NO_SHOTS);
  });

  it("R316 library full: the refused copy under 'Deck full', the owner's own play and then the other seat's CN-Virus", () => {
    libraryFull(null, NO_SHOTS);
  });
});

// Screenshot pass only with `--expose shots=1`.
if (SHOTS) {
  describe("Spec 25 — screenshots of each overflow mid-flight", () => {
    for (const viewport of VIEWPORTS) {
      it(`R318 at ${viewport.label}: fatigue`, () => {
        fatigue(viewport, shooterFor(viewport));
      });
      it(`R318 at ${viewport.label}: hand full`, () => {
        handFull(viewport, shooterFor(viewport));
      });
      it(`R318 at ${viewport.label}: library full`, () => {
        libraryFull(viewport, shooterFor(viewport));
      });
    }
  });
}
