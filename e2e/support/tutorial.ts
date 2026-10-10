// Tutorial e2e driver (SPEC §9.10; specs 22 and 23).
//
// The driver never dispatches; it performs the coach's suggested legal actions through the UI and
// only reads the practice view to await snapshots and locate cards.
//
// Cypress-retried waits observe the board and coach settling (BUILD M8).

import { timeouts, TUTORIAL_PROGRESS_KEY } from "./config.ts";
import {
  ACTION_ERROR,
  ANIMATING,
  BOARD,
  COACH,
  COACH_ACK,
  END_TURN,
  POWER,
  PROMPT,
  PROMPT_SUBMIT,
  TUTORIAL_HUD,
  TUTORIAL_RESULT,
  TUTORIAL_STEP,
  cardId,
  handCardId,
  heroId,
  promptOf,
  promptOptionId,
  switchPositionId,
  ts,
  zoneId,
} from "./testids.ts";
import type { Action, ActionBody, Lane, PlayerId, Row, Selection, Side } from "./types.ts";

// Constants

export const PRACTICE_PATH = "/practice";

/** The module worker loads the engine, card scripts, and AI before its first answer. */
export const TUTORIAL_BOOT_TIMEOUT = 60_000;

/** This can span an AI turn and a coach tip. */
export const MOMENT_TIMEOUT = 90_000;

export const SNAPSHOT_TIMEOUT = 20_000;

/** R81: one pick per choice kind, plus slack. */
const PLAY_PICK_BUDGET = 10;

// Development handles

/** Structural subsets of the `@jackioh/shared` view types (crates/engine/src/wire/view.rs). */
export type CardLike = { instanceId: string; defId: string };
export type PendingOptionLike = {
  key: string;
  label?: string;
  instanceId?: string;
  defId?: string;
  player?: PlayerId;
  row?: Row;
  lane?: number;
};
export type PendingLike =
  | { forYou: true; choiceId: string; kind: string; options: PendingOptionLike[]; min: number; max: number }
  | { forYou: false; pendingFor: PlayerId };
export type SideLike = {
  player: PlayerId;
  hero: { health: number };
  hand: CardLike[] | { count: number };
  units: (CardLike | null)[];
};
export type PlayerViewLike = {
  viewer: PlayerId;
  turn: number;
  active: PlayerId;
  phase: string;
  you: SideLike;
  opponent: SideLike;
  pending: PendingLike | null;
  events: unknown[];
  result: { winner: PlayerId | "draw"; reason: string } | null;
};

/** `PracticeDebug` (apps/web/src/practice/protocol.ts). */
export type PracticeDebugLike = {
  seed: string;
  decks: [string[], string[]];
  handicaps: Partial<Record<PlayerId, Record<string, number>>>;
  log: Action[];
  state: unknown;
  hash: string;
  difficulty: string;
  humanSeat: PlayerId;
  lesson?: string;
};

export type PracticeHandleLike = {
  snapshot(): Promise<PracticeDebugLike>;
  readonly aiSeat: PlayerId | null;
  readonly thinking: boolean;
  readonly view: PlayerViewLike | null;
};

/** `CoachDisplay` (apps/web/src/tutorial/coach.ts), as far as the specs read it. */
export type CoachDisplayLike = {
  mode: "finished" | "waiting" | "tip" | "step";
  id?: string;
  ack?: boolean;
  stepNumber?: number;
  stepCount?: number;
};

export type TutorialHandleLike = {
  readonly lessonId: string;
  readonly display: CoachDisplayLike;
  readonly suggested: ActionBody | null;
};

type TutorialWindow = { __jackiohPractice?: PracticeHandleLike; __jackiohTutorial?: TutorialHandleLike };

function handlesOf(win: Window): TutorialWindow {
  return win as unknown as TutorialWindow;
}

export function practiceHandle(): Cypress.Chainable<PracticeHandleLike> {
  return cy
    .window({ timeout: TUTORIAL_BOOT_TIMEOUT, log: false })
    .should((win) => {
      expect(handlesOf(win).__jackiohPractice, "window.__jackiohPractice (dev builds)").to.not.eq(undefined);
      expect(handlesOf(win).__jackiohPractice?.view, "the practice game's first snapshot").to.not.eq(null);
    })
    .then((win) => handlesOf(win).__jackiohPractice as PracticeHandleLike);
}

export function tutorialHandle(): Cypress.Chainable<TutorialHandleLike> {
  return cy
    .window({ timeout: TUTORIAL_BOOT_TIMEOUT, log: false })
    .should((win) => {
      expect(handlesOf(win).__jackiohTutorial, "window.__jackiohTutorial (dev builds, in a lesson)").to.not.eq(undefined);
    })
    .then((win) => handlesOf(win).__jackiohTutorial as TutorialHandleLike);
}

export function currentView(win: Window): PlayerViewLike | null {
  return handlesOf(win).__jackiohPractice?.view ?? null;
}

export function debugSnapshot(): Cypress.Chainable<PracticeDebugLike> {
  return practiceHandle().then((handle) => cy.wrap(handle.snapshot(), { log: false, timeout: timeouts.task }));
}

export function handOf(view: PlayerViewLike): CardLike[] {
  return Array.isArray(view.you.hand) ? view.you.hand : [];
}

// Visiting, progress, and motion

export function lessonUrl(lessonId: string): string {
  const params = new URLSearchParams({ lesson: lessonId, pace: "fast" });
  return `${PRACTICE_PATH}?${params.toString()}`;
}

export type TutorialVisit = {
  /** Seed progress before boot; strings remain raw so corrupt storage can be tested. */
  progress?: unknown;
  /** `prefers-reduced-motion: reduce`, as spec 13's Hard game uses it. */
  reducedMotion?: boolean;
  onBeforeLoad?: (win: Cypress.AUTWindow) => void;
};

export function writeProgress(win: Window, progress: unknown): void {
  try {
    if (progress === undefined || progress === null) win.localStorage.removeItem(TUTORIAL_PROGRESS_KEY);
    else win.localStorage.setItem(TUTORIAL_PROGRESS_KEY, typeof progress === "string" ? progress : JSON.stringify(progress));
  } catch (error) {
    Cypress.log({ name: "tutorial", message: `localStorage unavailable: ${String(error)}` });
  }
}

/** `prefers-reduced-motion: reduce`: the board draws every view as it arrives (BUILD M5-T4). */
export function preferReducedMotion(win: Cypress.AUTWindow): void {
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

export function visitTutorial(path: string, options: TutorialVisit = {}): void {
  cy.visit(path, {
    onBeforeLoad(win) {
      writeProgress(win, options.progress);
      if (options.reducedMotion === true) preferReducedMotion(win);
      options.onBeforeLoad?.(win);
    },
  });
}

export function storedProgress(): Cypress.Chainable<unknown> {
  return cy.window({ log: false }).then((win) => {
    const raw = win.localStorage.getItem(TUTORIAL_PROGRESS_KEY);
    return raw === null ? null : (JSON.parse(raw) as unknown);
  });
}

// Coach and HUD

export type StepCounter = { step: number; of: number };

/** Read each child because responsive long and short forms concatenate at the root. */
export function stepCounterIn(doc: Document): StepCounter | null {
  const root = doc.querySelector(ts(TUTORIAL_STEP));
  if (root === null) return null;
  const texts = [root, ...Array.from(root.querySelectorAll("*"))].map((element) => (element.textContent ?? "").trim());
  for (const pattern of [/^Step\s+(\d+)\s+of\s+(\d+)$/, /^(\d+)\s*\/\s*(\d+)$/]) {
    for (const text of texts) {
      const match = pattern.exec(text);
      if (match !== null) return { step: Number(match[1]), of: Number(match[2]) };
    }
  }
  return null;
}

export function stepCounter(): Cypress.Chainable<StepCounter> {
  let found: StepCounter | null = null;
  return cy
    .document({ log: false })
    .should((doc) => {
      found = stepCounterIn(doc);
      expect(found, `"Step k of n" in ${TUTORIAL_STEP}`).to.not.eq(null);
    })
    .then(() => found as StepCounter);
}

export type CoachMark = { mode: string; step: string | null };

export function coachMarkIn(doc: Document): CoachMark | null {
  const coach = doc.querySelector(ts(COACH));
  if (coach === null) return null;
  return { mode: coach.getAttribute("data-coach-mode") ?? "", step: coach.getAttribute("data-coach-step") };
}

function sameMark(a: CoachMark | null, b: CoachMark | null): boolean {
  return a === null || b === null ? a === b : a.mode === b.mode && a.step === b.step;
}

// Moments when the human has something to do

export type Moment =
  | { kind: "over"; outcome: string | null }
  | { kind: "ack"; coach: CoachMark | null; counter: StepCounter | null }
  | { kind: "prompt"; promptKind: string; coach: CoachMark | null; counter: StepCounter | null; suggested: ActionBody | null }
  | { kind: "turn"; coach: CoachMark | null; counter: StepCounter | null; suggested: ActionBody | null };

type Waiting = { waiting: string };

function momentIn(win: Window): Moment | Waiting {
  const doc = win.document;
  const result = doc.querySelector(ts(TUTORIAL_RESULT));
  if (result !== null) return { kind: "over", outcome: result.getAttribute("data-outcome") };
  const busy = doc.querySelector(ANIMATING);
  if (busy !== null) return { waiting: `the board is animating ${busy.getAttribute("data-animating") ?? ""}` };
  const coach = doc.querySelector(ts(COACH));
  if (coach?.getAttribute("data-stale") === "true") return { waiting: "the coach has not caught up with the board" };
  const tutorial = handlesOf(win).__jackiohTutorial;
  if (tutorial === undefined) return { waiting: "window.__jackiohTutorial is not set" };
  const mark = coachMarkIn(doc);
  const counter = stepCounterIn(doc);
  if (doc.querySelector(ts(COACH_ACK)) !== null) return { kind: "ack", coach: mark, counter };

  const hud = doc.querySelector(ts(TUTORIAL_HUD));
  if (hud?.getAttribute("data-thinking") !== "false") return { waiting: "the AI owes an action" };
  const prompt = doc.querySelector(PROMPT);
  if (prompt !== null) {
    return {
      kind: "prompt",
      promptKind: prompt.getAttribute("data-prompt-kind") ?? "",
      coach: mark,
      counter,
      suggested: tutorial.suggested,
    };
  }
  const board = doc.querySelector(ts(BOARD));
  const endTurn = doc.querySelector<HTMLButtonElement>(ts(END_TURN));
  const mine = board?.getAttribute("data-active") === "you" && board.getAttribute("data-phase") === "main";
  if (mine && endTurn !== null && !endTurn.disabled) {
    return { kind: "turn", coach: mark, counter, suggested: tutorial.suggested };
  }
  return { waiting: "neither the human's turn nor a prompt for the human" };
}

/** Wait until the human has a stable action to take. */
export function waitForMoment(timeout = MOMENT_TIMEOUT): Cypress.Chainable<Moment> {
  let found: Moment | Waiting = { waiting: "not looked yet" };
  return cy
    .window({ timeout, log: false })
    .should((win) => {
      found = momentIn(win);
      expect("kind" in found ? "" : found.waiting, "waiting for the human's next moment").to.eq("");
    })
    .then(() => found as Moment);
}

// Acting through the UI

const ANY_PROMPT_OPTION = `[data-testid^="${promptOptionId("")}"]`;

function optionKey($option: JQuery<HTMLElement>): string {
  return ($option.attr("data-testid") ?? "").slice(promptOptionId("").length);
}

/** A4: a chosen option is `aria-pressed="true"` (or `data-selected`), on it or an ancestor. */
function isPicked($option: JQuery<HTMLElement>): boolean {
  const marked = $option.closest("[aria-pressed], [data-selected]");
  const element = marked.length > 0 ? marked : $option;
  return element.attr("aria-pressed") === "true" || element.attr("data-selected") === "true";
}

/** Matches `selectionKey` in apps/web/src/game/actions.ts. */
function selectionKey(selection: Selection): string {
  switch (selection.pick) {
    case "instance":
      return `instance:${selection.instanceId}`;
    case "hero":
      return `hero:${selection.player}`;
    case "zone":
      return `zone:${selection.player}:${selection.row}:${String(selection.lane)}`;
    case "mode":
      return `mode:${selection.option}`;
    case "none":
      return "none";
  }
}

function sideOf(view: PlayerViewLike, player: PlayerId): Side {
  return player === view.viewer ? "you" : "opponent";
}

function boardTestidOf(view: PlayerViewLike, selection: Selection): string | null {
  switch (selection.pick) {
    case "instance":
      return handOf(view).some((card) => card.instanceId === selection.instanceId)
        ? handCardId(selection.instanceId)
        : cardId(selection.instanceId);
    case "hero":
      return heroId(sideOf(view, selection.player));
    case "zone":
      return zoneId(sideOf(view, selection.player), selection.row, selection.lane as Lane);
    case "mode":
    case "none":
      return null;
  }
}

function attackTargetTestid(view: PlayerViewLike, targetId: string): string {
  for (const side of [view.you, view.opponent]) {
    if (targetId === `hero-${side.player}`) return heroId(sideOf(view, side.player));
  }
  return cardId(targetId);
}

/** Reverse mapping for `selectionForOption`. */
function optionFor(options: readonly PendingOptionLike[], selection: Selection): PendingOptionLike | undefined {
  return options.find((option) => {
    switch (selection.pick) {
      case "zone":
        return option.row === selection.row && option.lane === selection.lane && option.player === selection.player;
      case "instance":
        return option.instanceId === selection.instanceId && option.row === undefined;
      case "hero":
        return option.player === selection.player && option.instanceId === undefined && option.row === undefined;
      case "mode":
        return option.defId === selection.option || option.key === selection.option || option.key.endsWith(`:${selection.option}`);
      case "none":
        return option.key === "none" || option.key.startsWith("none:");
    }
  });
}

/** Prefer a reachable board target; otherwise use the picker. */
function reachable(element: Element): boolean {
  const box = element.getBoundingClientRect();
  if (box.width <= 0 || box.height <= 0) return false;
  const hit = element.ownerDocument.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
  return hit !== null && (hit === element || element.contains(hit));
}

function submitIfStillOpen(root: string): void {
  cy.get("body", { log: false }).then(($body) => {
    const submit = $body.find(`${root} ${ts(PROMPT_SUBMIT)}`);
    if (submit.length > 0 && submit.attr("aria-disabled") !== "true") cy.wrap(submit.first(), { log: false }).click();
  });
}

/** R9: toggle exactly the requested keep set, then confirm. */
export function mulliganThroughUi(keep: readonly string[]): void {
  const wanted = [...keep].sort();
  cy.get(promptOf("mulligan"), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("be.visible");
  cy.get(promptOf("mulligan")).within(() => {
    cy.get(ANY_PROMPT_OPTION).each(($option) => {
      if (isPicked($option) !== keep.includes(optionKey($option))) cy.wrap($option, { log: false }).click();
    });
    cy.get(ANY_PROMPT_OPTION).should(($options) => {
      const kept = $options
        .toArray()
        .map((element) => Cypress.$(element))
        .filter(($option) => isPicked($option))
        .map(($option) => optionKey($option))
        .sort();
      expect(kept, "the cards marked Keep are exactly the ones the coach keeps").to.deep.eq(wanted);
    });
    cy.get(ts(PROMPT_SUBMIT)).click();
  });
}

function answerThroughUi(action: Extract<ActionBody, { type: "answer" }>): void {
  cy.window({ log: false }).then((win) => {
    const pending = currentView(win)?.pending ?? null;
    expect(pending !== null && pending.forYou, "a prompt for the human is open").to.eq(true);
    const options = pending !== null && pending.forYou ? pending.options : [];
    for (const selection of action.selection) {
      const option = optionFor(options, selection);
      expect(option, `a prompt option for ${JSON.stringify(selection)}`).to.not.eq(undefined);
      cy.get(`${PROMPT} ${ts(promptOptionId(option?.key ?? ""))}`).click();
    }
    submitIfStillOpen(PROMPT);
  });
}

export function answerFirstOption(): void {
  cy.get("body", { log: false }).then(($body) => {
    if ($body.find(promptOf("mulligan")).length > 0) {
      cy.get(promptOf("mulligan")).within(() => {
        cy.get(ANY_PROMPT_OPTION).each(($option) => {
          if (!isPicked($option)) cy.wrap($option, { log: false }).click();
        });
        cy.get(ts(PROMPT_SUBMIT)).click();
      });
      return;
    }
    cy.get(PROMPT).first().find(ANY_PROMPT_OPTION).first().click();
    submitIfStillOpen(PROMPT);
  });
}

type PlayAction = Extract<ActionBody, { type: "play" }>;

/** R81: complete play picks in client order, preferring reachable board targets. */
function finishPlay(play: PlayAction, view: PlayerViewLike, remaining: number): void {
  cy.get("body", { log: false }).then(($body) => {
    const card = $body.find(ts(handCardId(play.instanceId)));
    if (card.length === 0 || card.attr("data-selected") !== "true") return;
    expect(remaining, `the play of ${play.instanceId} is complete within ${String(PLAY_PICK_BUDGET)} picks`).to.be.greaterThan(0);

    const picker = $body.find(`${PROMPT}[data-prompt-source="play"]`);
    const kind = picker.attr("data-prompt-kind");
    const inPicker = (key: string): string => `${PROMPT}[data-prompt-source="play"] ${ts(promptOptionId(key))}`;
    const onBoard = (testid: string): boolean => {
      const element = $body.find(`${ts(testid)}[data-legal="true"]`)[0];
      return element !== undefined && reachable(element);
    };

    switch (kind) {
      case "zone": {
        const zone = play.zone;
        expect(zone, "the play names its zone").to.not.eq(undefined);
        if (zone === undefined) return;
        const where = zoneId("you", zone.row, zone.lane as Lane);
        cy.get(onBoard(where) ? ts(where) : inPicker(`${zone.row}:${String(zone.lane)}`)).click();
        break;
      }
      case "x":
        cy.get(inPicker(String(play.x))).click();
        break;
      case "embiggen":
        cy.get(inPicker(String(play.embiggen))).click();
        break;
      case "tribute":
        for (const id of play.tributes ?? []) cy.get(inPicker(id)).click();
        submitIfStillOpen(`${PROMPT}[data-prompt-source="play"]`);
        break;
      case "target":
      case "hand": {
        // Prefer a reachable board target to a picker option.
        const next = (play.targets ?? []).find((selection) => {
          const where = boardTestidOf(view, selection);
          return where === null || $body.find(`${ts(where)}[data-selected="true"]`).length === 0;
        });
        expect(next, "the play names a target still to pick").to.not.eq(undefined);
        if (next === undefined) return;
        const where = boardTestidOf(view, next);
        if (where !== null && onBoard(where)) cy.get(ts(where)).click();
        else {
          cy.get(inPicker(selectionKey(next))).click();
          submitIfStillOpen(`${PROMPT}[data-prompt-source="play"]`);
        }
        break;
      }
      case "discover":
      case "mode":
      case "direction":
        for (const mode of play.modes ?? []) cy.get(inPicker(mode)).click();
        submitIfStillOpen(`${PROMPT}[data-prompt-source="play"]`);
        break;
      default:
        break;
    }
    finishPlay(play, view, remaining - 1);
  });
}

export type PerformHooks = {
  afterPickUp?: (play: PlayAction) => void;
};

export function performThroughUi(action: ActionBody, hooks: PerformHooks = {}): void {
  cy.window({ log: false }).then((win) => {
    const view = currentView(win);
    expect(view, "the page holds a view").to.not.eq(null);
    if (view === null) return;
    switch (action.type) {
      case "mulligan":
        mulliganThroughUi(action.keep);
        return;
      case "play":
        cy.get(ts(handCardId(action.instanceId))).should("have.attr", "data-legal", "true").click();
        hooks.afterPickUp?.(action);
        finishPlay(action, view, PLAY_PICK_BUDGET);
        return;
      case "attack":
        cy.get(ts(cardId(action.attackerId))).should("have.attr", "data-legal", "true").click();
        cy.get(ts(attackTargetTestid(view, action.targetId))).should("have.attr", "data-legal", "true").click();
        return;
      case "endTurn":
        cy.get(ts(END_TURN)).should("not.be.disabled").click();
        return;
      case "switchPosition":
        cy.get(ts(switchPositionId(action.instanceId))).should("not.be.disabled").click();
        return;
      case "activatePower":
        expect(action.targets ?? [], "a power the coach asks for names no target (the spec clicks `power` alone)").to.deep.eq([]);
        cy.get(ts(POWER)).should("have.attr", "data-legal", "true").click();
        return;
      case "answer":
        answerThroughUi(action);
        return;
      default:
        throw new Error(`the coach suggested ${JSON.stringify(action)}, which no lesson step asks a player to do`);
    }
  });
}

export function fallbackTurn(): void {
  cy.get("body", { log: false }).then(($body) => {
    const attackers = $body
      .find(`[data-testid^="zone-you-units-"] [data-testid^="${cardId("")}"][data-glow="ready"]`)
      .toArray()
      .map((element) => element.getAttribute("data-testid") ?? "")
      .filter((testid) => testid !== "");
    const first = attackers[0];
    if (first === undefined) {
      cy.get(ts(END_TURN)).should("not.be.disabled").click();
      return;
    }
    cy.get(ts(first)).click();
    cy.get(ts(heroId("opponent"))).then(($hero) => {
      if ($hero.attr("data-legal") === "true") {
        cy.wrap($hero, { log: false }).click();
        return;
      }
      cy.get(ts(first)).click();
      cy.get(ts(END_TURN)).should("not.be.disabled").click();
    });
  });
}

// One step of the driver

export type Taken = "ack" | "coach" | "fallback";

export function takeMoment(moment: Exclude<Moment, { kind: "over" }>, hooks: PerformHooks = {}): Cypress.Chainable<Taken> {
  if (moment.kind === "ack") {
    const before = moment.coach;
    cy.get(ts(COACH_ACK)).click();
    cy.document({ log: false, timeout: MOMENT_TIMEOUT }).should((doc) => {
      expect(sameMark(coachMarkIn(doc), before), `the coach moved on from ${JSON.stringify(before)}`).to.eq(false);
    });
    return cy.wrap<Taken>("ack", { log: false });
  }

  let before: PlayerViewLike | null = null;
  cy.window({ log: false }).then((win) => {
    before = currentView(win);
  });
  const suggested = moment.suggested;
  if (suggested !== null) performThroughUi(suggested, hooks);
  else if (moment.kind === "prompt") answerFirstOption();
  else fallbackTurn();
  cy.window({ log: false, timeout: SNAPSHOT_TIMEOUT }).should((win) => {
    expect(currentView(win) !== before, "the action came back from the worker as a new snapshot").to.eq(true);
  });
  cy.get(ts(ACTION_ERROR)).should("not.exist");
  return cy.wrap<Taken>(suggested === null ? "fallback" : "coach", { log: false });
}
