// The five commands BUILD's repo layout names — seedGame, playCard, attack, answerPrompt,
// endTurn — plus the waiting primitives that let the suite obey the M8 house rule: no fixed
// `cy.wait(ms)` anywhere, only assertions Cypress retries. All DOM knowledge lives in
// support/testids.ts, all URL and account knowledge in support/config.ts.

import { CARD_NAMES, allCardIds, cardId as catalogId } from "./cards.ts";
import {
  SESSION_STORAGE_KEY,
  apiUrl,
  bearerHeaders,
  constants,
  hotseatUrl,
  server,
  timeouts,
  type E2EAccount,
} from "./config.ts";
import {
  ANIMATING,
  CONCEDE,
  CONCEDE_CONFIRM,
  CONCEDE_DIALOG,
  DECK_DRAG_MIME,
  DECK_DROP,
  END_TURN,
  OFFER_DRAW,
  POWER,
  PROMPT,
  PROMPT_SUBMIT,
  PROMPT_X_INPUT,
  RESULT_OVERLAY,
  SEAT_SWITCH,
  cardId,
  handCardId,
  heroId,
  animating,
  poolCardId,
  promptOf,
  promptOptionId,
  switchPositionId,
  ts,
  zoneId,
} from "./testids.ts";
import type {
  ActionInput,
  EventType,
  E2EDeckInjection,
  FixtureDeck,
  FixtureHandicap,
  GameStateLike,
  JackiOhDevHandle,
  PlayerId,
  PromptAnswer,
  PromptKind,
  Side,
  ZoneRef,
} from "./types.ts";

/** ASSUMPTION A1: where `seedGame` parks the fixture decks so a reload keeps them. */
export const DECKS_STORAGE_KEY = "jackioh.e2e.decks";

export type SeedGameOptions = {
  /** Every spec sets a seed (BUILD M8). */
  seed: string;
  /** Deck fixture ids, i.e. `e2e/fixtures/decks/<id>.json` without the extension. */
  a: string;
  b: string;
  /** `keep` answers the opening mulligans keeping every card; `manual` leaves them to the spec (02). */
  mulligan?: "keep" | "manual";
  /** Runs in the visit's `onBeforeLoad` after the deck injection, to seed more storage (spec 25). */
  onBeforeLoad?: (win: Cypress.AUTWindow) => void;
  /**
   * Decks given inline, by the ids `a` and `b` name, instead of read from `e2e/fixtures/decks/` (spec
   * 36 builds its decks from the catalog at run time). Each is checked exactly as a fixture is (A9).
   */
  decks?: Readonly<Record<string, Pick<FixtureDeck, "cards" | "handicap"> & Partial<FixtureDeck>>>;
  /** R1420: the sets the hotseat engine previews before it creates the game (spec 36: `["Meditative"]`). */
  preview?: string[];
};

export type PromptStep = PromptAnswer & { kind?: PromptKind };

/** `cy.saveDeck`'s input: `PUT /api/decks/:id`'s body, and the id (minted when omitted). */
export type SaveDeckInput = {
  id?: string;
  name: string;
  cards: readonly string[];
  /** The catalog version to save against; read from `GET /api/decks` when omitted. */
  catalogVersion?: string;
};

/** `cy.saveTrio`'s input: `PUT /api/trios/:id`'s body, and the id (minted when omitted). */
export type SaveTrioInput = { id?: string; name: string; deckIds: (string | null)[] };

/**
 * Every acting command drains `data-animating` (`cy.settled()`), which closes the window BUILD M5-T4 looks through:
 * `expectAnimating` asserts it, `during` runs anything else a spec must see there (retried assertions, never a clock).
 */
export type ActOptions = {
  expectAnimating?: EventType | EventType[];
  during?: () => void;
};

export type PlayCardOptions = ActOptions & {
  /**
   * Click the hand card where it shows rather than at its center: on a phone a full hand fans out and overlaps, so the
   * center is often another card's. It hit-tests as a tap does (`elementFromPoint`).
   */
  visiblePart?: boolean;
  /** R81: the zone travels in the play action; the client builds it from a board click. */
  zone?: ZoneRef;
  /** R81: further play-time pickers (targets, modes, X, embiggen, tribute), in the order shown. */
  answers?: PromptStep[];
};

export type AttackTarget = { card: string } | { hero: Side };

/**
 * Mirrors `WsPlayerCommand` in support/tasks/wsPlayer.ts, which speaks the protocol: whatever the task accepts is
 * declarable here. No `joinRoom`: a room is joined by `POST /api/rooms/:code/join`; sockets open onto existing matches.
 */
export type WsPlayerCommand =
  | {
      action: "connect";
      name: string;
      url?: string;
      token?: string;
      matchId?: string;
      roomCode?: string;
      /** The seat this client holds. Defaults to `p2`, which is the seat a joiner gets (§9.5). */
      seat?: PlayerId;
    }
  | { action: "send"; name: string; body: ActionInput }
  /** R738, R643: an aim or an emote, sent as it is; the server never answers one, so nothing is awaited. */
  | { action: "sendFrame"; name: string; frame: CosmeticFrame }
  | { action: "awaitView"; name: string; where?: ViewPredicate }
  | { action: "view"; name: string }
  | { action: "messages"; name: string }
  | { action: "disconnect"; name: string }
  /** Connect as `token`, concede `matchId` and close, in one task (see `cy.concedeAs`). */
  | { action: "concede"; name: string; url?: string; token: string; matchId: string; seat?: PlayerId }
  | { action: "reset" };

/** R738, R643: a top-level cosmetic frame (an aim or an emote). The server never answers one. */
export type CosmeticFrame = { type: "aim"; aim: Record<string, unknown> | null } | { type: "emote"; emote: string };

/** What `awaitView` waits for. Every field is `AND`ed; an omitted one is not looked at. */
export type ViewPredicate = {
  active?: PlayerId;
  phase?: string;
  /** Only ever satisfied by a prompt THIS client holds: §10.6 hides the kind from the other seat. */
  promptKind?: string;
  hasResult?: boolean;
  /** `view.turn >= n`. §10.1's turn counter is 1-based and counts player-turns (R2). */
  turnAtLeast?: number;
  /** R265, R266: `view.mulligan.opponentReady`; a view outside the mulligan window never matches. */
  mulliganOpponentReady?: boolean;
  /** R269: `view.drawOffer.by` — the seat whose draw offer stands — or `null` for none. */
  drawOfferBy?: PlayerId | null;
};

export type WsPlayerResult = {
  ok: boolean;
  name?: string;
  seat?: string;
  view?: Record<string, unknown> | null;
  messages?: { type: string; [key: string]: unknown }[];
  /** §9.3: the server's reason, relayed verbatim — `ErrorMessage.message`, never a restatement. */
  error?: string;
  /** `ErrorMessage.code`: `illegal_action`, `rate_limited`, `match_over`, `forbidden`, … */
  code?: string;
  /** `AckMessage.seq`: the append-only log seq an accepted action was written at (§9.3). */
  seq?: number;
};

// helpers

/** A non-negative integer, as R180 wants every handicap field to be. */
function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

/**
 * A fixture's `handicap` (R180), checked as the engine's `validateHandicap` checks one (R184, R290),
 * so a bad one fails here with the fixture's name instead of as the route's "no game" panel.
 */
function asHandicap(raw: unknown, id: string): FixtureHandicap {
  expect(raw, `decks/${id}.json handicap`).to.be.an("object");
  const handicap = raw as Partial<Record<keyof FixtureHandicap, unknown>>;
  const fields = ["deckSize", "manaBonus", "manaCap", "extraOpeningCards", "extraDrawsPerTurn"] as const;
  for (const field of fields) {
    expect(isCount(handicap[field]), `decks/${id}.json handicap.${field} is a non-negative integer (R180)`).to.eq(true);
  }
  expect(handicap.deckSize, `decks/${id}.json handicap.deckSize fits a library (R184)`).to.be.within(
    1,
    constants.LIBRARY_CAP,
  );
  if (handicap.heroHealth !== undefined) {
    expect(
      isCount(handicap.heroHealth) && handicap.heroHealth >= 1,
      `decks/${id}.json handicap.heroHealth is a positive integer (R290)`,
    ).to.eq(true);
  }
  return raw as FixtureHandicap;
}

/** SPEC §8's Core ids: what a fixture may hold when no catalog was read (`installLoadout`). */
const CORE_IDS: ReadonlySet<string> = new Set(Object.keys(CARD_NAMES).map((index) => catalogId(Number(index))));

/** Where the catalog sits, from e2e/ (Cypress's project root), read at run time rather than imported. */
const CATALOG_PATH = "../crates/cards/catalog.json";

/**
 * A9: every deckable id of every set (one format, R380), read off the catalog at run time, so `e2e/` stays
 * self-contained and a fixture is checked against the catalog the build serves. Tokens are out: L3 bans them.
 */
function deckableIds(): Cypress.Chainable<ReadonlySet<string>> {
  return cy.readFile(CATALOG_PATH, { log: false }).then((raw: Record<string, { token?: boolean }>) => {
    const ids = Object.entries(raw)
      .filter(([, def]) => def.token !== true)
      .map(([id]) => id);
    return new Set(ids) as ReadonlySet<string>;
  });
}

function asDeck(raw: unknown, id: string, known: ReadonlySet<string> = CORE_IDS): FixtureDeck {
  const deck = raw as Partial<FixtureDeck>;
  expect(deck, `fixture decks/${id}.json`).to.be.an("object");
  expect(deck.cards, `decks/${id}.json cards`).to.be.an("array");
  const cards = deck.cards ?? [];
  const handicap = deck.handicap === undefined ? undefined : asHandicap(deck.handicap, id);
  // L2/L3 (SPEC §9.4) are enforced by the engine's `validateDeck` inside `createGame`; failing here
  // gives a readable message before the app loads. A handicapped deck holds its handicap's size (R184).
  const size = handicap?.deckSize ?? constants.DECK_SIZE;
  expect(
    cards.length,
    handicap === undefined ? `decks/${id}.json holds DECK_SIZE cards` : `decks/${id}.json holds its handicap's deckSize (R184)`,
  ).to.eq(size);
  expect(new Set(cards).size, `decks/${id}.json has no duplicate ids (L3)`).to.eq(cards.length);
  for (const card of cards) {
    expect(known.has(card), `decks/${id}.json: "${card}" is a deckable catalog id (L6)`).to.eq(true);
  }
  return {
    id,
    spec: deck.spec ?? id,
    description: deck.description ?? "",
    cards,
    ...(handicap === undefined ? {} : { handicap }),
  };
}

function exists(selector: string): Cypress.Chainable<boolean> {
  return cy.get("body").then(($body) => $body.find(selector).length > 0);
}

function clickZone(zone: ZoneRef): void {
  cy.get(ts(zoneId(zone.side, zone.row, zone.lane))).click();
}

/** A pick is offered either as a prompt option or as the board card itself (BUILD M5-T2). */
function clickPick(...selectors: string[]): void {
  cy.get("body").then(($body) => {
    const found = selectors.find((selector) => $body.find(selector).length > 0);
    expect(found, `one of ${selectors.join(" | ")} is present`).to.not.eq(undefined);
    cy.get(found ?? selectors[0] ?? "body").click();
  });
}

/** Any `prompt-option-<key>`, so a picker can be answered without naming a key. */
const ANY_PROMPT_OPTION = `[data-testid^="${promptOptionId("")}"]`;

/**
 * Whether a toggle is already chosen. A4: the client marks a picked option with `aria-pressed` (`Prompt.tsx`), the
 * board with `data-selected`; either counts, and it may sit on the clickable ancestor of an inner-label testid.
 */
function isPicked($option: JQuery<HTMLElement>): boolean {
  const marked = $option.closest('[aria-pressed], [data-selected]');
  const element = marked.length > 0 ? marked : $option;
  return element.attr("aria-pressed") === "true" || element.attr("data-selected") === "true";
}

/** BUILD M5-T4: assert the animation this command just caused, and run `during`, before the drain. */
function captureAnimating(options: ActOptions): void {
  const expected = options.expectAnimating;
  if (expected !== undefined) {
    for (const event of Array.isArray(expected) ? expected : [expected]) {
      cy.expectAnimating(event);
    }
  }
  options.during?.();
}

// waiting: BUILD M8 forbids fixed waits, so everything below is a retried assertion

Cypress.Commands.add("settled", () => {
  cy.get(ANIMATING, { timeout: timeouts.animation, log: false }).should("not.exist");
});

Cypress.Commands.add("expectAnimating", (event: EventType) => {
  cy.get(animating(event), { timeout: timeouts.animation }).should("exist");
});

Cypress.Commands.add("waitForPrompt", (kind: PromptKind) => {
  cy.get(promptOf(kind), { timeout: timeouts.view }).should("be.visible");
});

Cypress.Commands.add("noPrompt", () => {
  cy.get(PROMPT, { timeout: timeouts.animation }).should("not.exist");
});

// the dev handle (BUILD M5-T3)

Cypress.Commands.add("jackioh", () => {
  return cy
    .window({ timeout: timeouts.view, log: false })
    .should((win) => {
      expect(win.__jackioh, "window.__jackioh (BUILD M5-T3)").to.not.eq(undefined);
    })
    .then((win) => win.__jackioh as JackiOhDevHandle);
});

Cypress.Commands.add("gameState", () => {
  return cy.jackioh().then((handle) => handle.state);
});

/**
 * Setup shortcut only: `reduce` refuses illegal actions itself (SPEC §9.3), so dispatching is a way past the UI, never
 * past the rules; specs use it to reach a pre-condition, never to exercise the behavior under test.
 */
Cypress.Commands.add("dispatchAction", (action: ActionInput) => {
  cy.jackioh().then((handle) => {
    handle.dispatch(action);
  });
  cy.settled();
});

// finding cards: by name through the DOM, or by defId/zone through the dev handle

/** The instance id in `hand-card-<instanceId>` / `card-<instanceId>`. */
function instanceIdOf($element: JQuery<HTMLElement>): string {
  const testid = $element.attr("data-testid") ?? "";
  const id = testid.replace(/^hand-card-/, "").replace(/^card-/, "");
  expect(id, `an instance id inside "${testid}"`).to.not.eq("");
  return id;
}

/** The instance id of the named card in the shown hand (BUILD M5-T1: hand cards show the name). */
Cypress.Commands.add("handCardByName", (name: string) => {
  return cy
    .get('[data-testid^="hand-card-"]', { timeout: timeouts.view })
    .contains(name)
    .closest('[data-testid^="hand-card-"]')
    .then(($card) => instanceIdOf($card));
});

/** The instance id of the named card on the field, either side. */
Cypress.Commands.add("fieldCardByName", (name: string) => {
  return cy
    .get('[data-testid^="card-"]', { timeout: timeouts.view })
    .contains(name)
    .closest('[data-testid^="card-"]')
    .then(($card) => instanceIdOf($card));
});

/** The instance id of the first `defId` in a player's hand, off the dev handle, where the DOM cannot answer. */
Cypress.Commands.add("instanceInHand", (player: PlayerId, defId: string) => {
  return cy.gameState().then((state) => {
    const side = state.players[player] as { hand?: { id: string; defId: string }[] };
    const found = (side.hand ?? []).find((card) => card.defId === defId);
    expect(found, `${defId} in ${player}'s hand`).to.not.eq(undefined);
    return found?.id ?? "";
  });
});

/** The instance id occupying a zone: the top card of a Stack pile for units (SPEC §3.2). */
Cypress.Commands.add("instanceAt", (player: PlayerId, row: "units" | "backrow", lane: number) => {
  return cy.gameState().then((state) => {
    const side = state.players[player] as {
      units?: ({ id: string }[] | null)[];
      backrow?: ({ id: string } | null)[];
    };
    const slot =
      row === "units" ? (side.units ?? [])[lane - 1]?.[0] ?? null : (side.backrow ?? [])[lane - 1] ?? null;
    expect(slot, `a card at ${player} ${row} lane ${lane}`).to.not.eq(null);
    return slot?.id ?? "";
  });
});

/** Play the named hand card. The readable form of `handCardByName` + `playCard`. */
Cypress.Commands.add("playByName", (name: string, options: PlayCardOptions = {}) => {
  cy.handCardByName(name).then((instanceId) => {
    cy.playCard(instanceId, options);
  });
});

// seedGame

Cypress.Commands.add("seedGame", (options: SeedGameOptions) => {
  const { seed, a, b, mulligan = "keep" } = options;
  /** A deck given inline (spec 36), else its fixture file. */
  const deckFor = (id: string): Cypress.Chainable<unknown> => {
    const inline = options.decks?.[id];
    if (inline === undefined) return cy.fixture(`decks/${id}.json`);
    const deck: unknown = { id, spec: id, description: "", ...inline };
    return cy.wrap(deck, { log: false });
  };

  deckableIds().then((known) => {
    deckFor(a).then((rawA) => {
      deckFor(b).then((rawB) => {
        const deckA = asDeck(rawA, a, known);
        const deckB = asDeck(rawB, b, known);
        const decks = {
          [a]: deckA.cards,
          [b]: deckB.cards,
        };
        // R180: fixture A seats p1 and fixture B seats p2, so their handicaps are p1's and p2's.
        const handicaps: Partial<Record<PlayerId, FixtureHandicap>> = {
          ...(deckA.handicap === undefined ? {} : { p1: deckA.handicap }),
          ...(deckB.handicap === undefined ? {} : { p2: deckB.handicap }),
        };
        const injection: E2EDeckInjection = {
          seed,
          decks,
          ...(Object.keys(handicaps).length === 0 ? {} : { handicaps }),
          ...(options.preview === undefined ? {} : { preview: options.preview }),
        };
        cy.visit(hotseatUrl(seed, a, b), {
          onBeforeLoad(win) {
            // ASSUMPTION A1: in E2E mode the hotseat route resolves `a=`/`b=` from this injection
            // before falling back to its built-in dev decks, and creates the game with its handicaps.
            win.__jackiohE2E = injection;
            try {
              win.localStorage.setItem(DECKS_STORAGE_KEY, JSON.stringify(injection));
            } catch (error) {
              Cypress.log({ name: "seedGame", message: `localStorage unavailable: ${String(error)}` });
            }
            options.onBeforeLoad?.(win);
          },
        });
      });
    });
  });

  cy.jackioh().should((handle) => {
    expect(handle.seed, "seed reached the engine").to.eq(seed);
  });
  cy.settled();
  if (mulligan === "keep") cy.keepMulligans();
});

/**
 * R9 / §2.1: the opening mulligans, kept whole; both seats mulligan at once (R265), each answer sealed until the other
 * is in (R266), so on `/dev/hotseat` the device follows the seat that owes one and networked the loop ends at this
 * seat's `mulligan-waiting`. THE ANSWER NAMES THE CARDS KEPT, NOT THE CARDS RETURNED (`crates/engine/src/setup.rs`):
 * Confirm with nothing toggled sends `keep: []` and mulligans the whole hand.
 */
Cypress.Commands.add("keepMulligans", () => {
  // WAIT FOR THE CLIENT TO HAVE A VIEW BEFORE ASKING WHETHER A PICKER IS OPEN. `exists()` is one non-retrying DOM read,
  // wrong for the first seat's mulligan: a networked board has no view until the actor's first full view lands (§9.5),
  // and asking then silently skips seat 1's mulligan. The handle's `state` is null until `net.ts`'s first `view` frame
  // and never null in hotseat, so waiting on it closes the race for every spec at once.
  cy.window({ timeout: timeouts.view, log: false }).should((win) => {
    const handle = win.__jackioh;
    expect(handle, "window.__jackioh (BUILD M5-T3)").to.not.eq(undefined);
    expect(
      handle?.state,
      "the client has a view to render (§9.5: the actor pushes one on attach)",
    ).to.not.eq(null);
  });

  const drain = (remaining: number): void => {
    if (remaining === 0) return;
    exists(promptOf("mulligan")).then((open) => {
      if (!open) return;
      cy.get(promptOf("mulligan")).within(() => {
        // The mulligan is per-card toggles plus a confirm, whatever its max (BUILD M5-T2).
        cy.get(ANY_PROMPT_OPTION).each(($option) => {
          if (isPicked($option)) return;
          cy.wrap($option, { log: false }).click();
        });
        cy.get(ts(PROMPT_SUBMIT)).click();
      });
      cy.settled();
      drain(remaining - 1);
    });
  };
  // At most one mulligan per seat (SPEC §2.1).
  drain(2);
});

/**
 * §2.5: concede, through the confirmation the `concede` control opens. The control only asks;
 * `concede-confirm` sends `{ type: "concede" }`, so clicking `concede` alone concedes nothing.
 */
Cypress.Commands.add("concede", () => {
  cy.get(ts(CONCEDE), { timeout: timeouts.view }).should("not.be.disabled").click();
  cy.get(ts(CONCEDE_DIALOG)).should("be.visible");
  cy.get(ts(CONCEDE_CONFIRM)).click();
  cy.get(ts(CONCEDE_DIALOG)).should("not.exist");
  cy.settled();
});

// playCard / attack / answerPrompt / endTurn

/** A point of `element` no other element covers, relative to its box, hit-tested on a grid; else `null`. */
function uncoveredPoint(element: HTMLElement): { x: number; y: number } | null {
  const rect = element.getBoundingClientRect();
  const steps = 8;
  for (let row = 1; row < steps; row += 1) {
    for (let col = 1; col < steps; col += 1) {
      const x = rect.left + (rect.width * col) / steps;
      const y = rect.top + (rect.height * row) / steps;
      const hit = element.ownerDocument.elementFromPoint(x, y);
      if (hit !== null && element.contains(hit)) return { x: x - rect.left, y: y - rect.top };
    }
  }
  return null;
}

/** Click the part of an element that shows (see `PlayCardOptions.visiblePart`). */
function clickUncovered(selector: string): void {
  cy.get(selector, { timeout: timeouts.view })
    .should(($element) => {
      expect(uncoveredPoint($element[0] as HTMLElement), `some part of ${selector} is not covered`).to.not.eq(null);
    })
    .then(($element) => {
      const point = uncoveredPoint($element[0] as HTMLElement) ?? { x: 0, y: 0 };
      cy.wrap($element, { log: false }).click(point.x, point.y);
    });
}

Cypress.Commands.add("playCard", (instanceId: string, options: PlayCardOptions = {}) => {
  if (options.visiblePart === true) clickUncovered(ts(handCardId(instanceId)));
  else cy.get(ts(handCardId(instanceId)), { timeout: timeouts.view }).click();
  if (options.zone !== undefined) clickZone(options.zone);
  for (const step of options.answers ?? []) {
    cy.answerPrompt(step.kind ?? null, step);
  }
  captureAnimating(options);
  cy.settled();
});

Cypress.Commands.add("attack", (attackerId: string, target: AttackTarget, options: ActOptions = {}) => {
  cy.get(ts(cardId(attackerId)), { timeout: timeouts.view }).click();
  if ("hero" in target) {
    cy.get(ts(heroId(target.hero))).click();
  } else {
    cy.get(ts(cardId(target.card))).click();
  }
  captureAnimating(options);
  cy.settled();
});

Cypress.Commands.add("answerPrompt", (kind: PromptKind | null, answer: PromptAnswer = {}) => {
  const root = kind === null ? PROMPT : promptOf(kind);
  cy.get(root, { timeout: timeouts.view }).should("be.visible");

  // §10.6: a Discover's three options are drawn by the match rng, so a spec cannot name a key, only
  // "the first": DOM order, which is `PendingChoice.options` order on every machine.
  if (answer.first !== undefined) {
    const wanted = answer.first;
    cy.get(root)
      .find(ANY_PROMPT_OPTION)
      .should("have.length.at.least", wanted)
      .then(($options) => {
        for (let at = 0; at < wanted; at += 1) {
          cy.wrap($options.eq(at), { log: false }).click();
        }
      });
  }
  // R81's embiggen price: the picker's two options are the two prices, keyed by the boolean.
  if (answer.embiggen !== undefined) {
    const key = String(answer.embiggen);
    clickPick(`${PROMPT} ${ts(promptOptionId(key))}`, ts(promptOptionId(key)));
  }
  for (const key of answer.options ?? []) {
    clickPick(`${PROMPT} ${ts(promptOptionId(key))}`, ts(promptOptionId(key)));
  }
  for (const instanceId of answer.cards ?? []) {
    clickPick(
      `${PROMPT} ${ts(promptOptionId(instanceId))}`,
      `${PROMPT} ${ts(cardId(instanceId))}`,
      ts(cardId(instanceId)),
      ts(handCardId(instanceId)),
    );
  }
  for (const zone of answer.zones ?? []) {
    clickZone(zone);
  }
  if (answer.hero !== undefined) {
    cy.get(ts(heroId(answer.hero))).click();
  }
  // #492: a few X values are cards, one click each, and more keep the stepper, typed and confirmed.
  if (answer.x !== undefined) {
    const x = String(answer.x);
    cy.get(root).then(($root) => {
      if ($root.find(ts(PROMPT_X_INPUT)).length === 0) {
        cy.wrap($root, { log: false }).find(ts(promptOptionId(x))).click();
        return;
      }
      cy.wrap($root, { log: false }).within(() => {
        cy.get(ts(PROMPT_X_INPUT)).clear();
        cy.get(ts(PROMPT_X_INPUT)).type(x);
        cy.get(ts(PROMPT_SUBMIT)).click();
      });
    });
  }

  const picks =
    (answer.options?.length ?? 0) +
    (answer.cards?.length ?? 0) +
    (answer.zones?.length ?? 0) +
    (answer.first ?? 0) +
    (answer.embiggen === undefined ? 0 : 1);
  const needsSubmit = answer.submit ?? picks > 1;
  if (needsSubmit) {
    cy.get(root).within(() => {
      cy.get(ts(PROMPT_SUBMIT)).click();
    });
  }
  cy.settled();
});

Cypress.Commands.add("endTurn", (options: ActOptions & { handOver?: boolean } = {}) => {
  cy.get(ts(END_TURN), { timeout: timeouts.view }).should("not.be.disabled");
  cy.get(ts(END_TURN)).click();
  captureAnimating(options);
  cy.settled();
  if (options.handOver ?? true) cy.handOver();
});

/** BUILD M5-T3: the hotseat seat-switch button. A no-op when the client switches by itself. */
Cypress.Commands.add("handOver", () => {
  exists(ts(SEAT_SWITCH)).then((present) => {
    if (!present) return;
    cy.get(ts(SEAT_SWITCH)).click();
    cy.settled();
  });
});

Cypress.Commands.add("switchPosition", (instanceId: string, options: ActOptions = {}) => {
  cy.get(ts(switchPositionId(instanceId))).click();
  captureAnimating(options);
  cy.settled();
});

Cypress.Commands.add("offerDraw", () => {
  cy.get(ts(OFFER_DRAW)).click();
  cy.settled();
});

Cypress.Commands.add("usePower", (options: ActOptions = {}) => {
  cy.get(ts(POWER)).click();
  captureAnimating(options);
  cy.settled();
});

// Reading a seat's piles, and getting to a turn

type SidePeek = {
  hand?: { id: string; defId: string }[];
  units?: ({ id: string }[] | null)[];
};

function peek(state: GameStateLike, player: PlayerId): SidePeek {
  return state.players[player] as SidePeek;
}

/**
 * The instance ids in a seat's hand, read off `window.__jackioh.state`, where the DOM cannot answer. CLAUDE.md rule 7:
 * it answers only "which instance ids might I click?"; every assertion belongs on the DOM, which is `viewFor`.
 */
Cypress.Commands.add("handIds", (player: PlayerId) => {
  return cy.gameState().then((state) => (peek(state, player).hand ?? []).map((card) => card.id));
});

/** The top card of each of a seat's unit zones, in lane order (§3.2: only a Stack pile's top is active). */
Cypress.Commands.add("unitIds", (player: PlayerId) => {
  return cy.gameState().then((state) =>
    (peek(state, player).units ?? []).flatMap((pile) => {
      const top = pile === null ? undefined : pile[0];
      return top === undefined ? [] : [top.id];
    }),
  );
});

/**
 * End turns until `turn` is current (§10.1: 1-based, p1 takes the odd ones). R82-SAFE: a turn can end by itself, so
 * each step re-reads state, hands the device to the active seat (BUILD M5-T3) and presses `end-turn` only if offered.
 */
Cypress.Commands.add("advanceToTurn", (turn: number, options: { budget?: number } = {}) => {
  const budget = options.budget ?? turn * 2 + 4;
  const step = (remaining: number): void => {
    cy.gameState().then((state) => {
      expect(state.turn, `player-turn ${String(turn)} has not already gone by`).to.be.at.most(turn);
      if (state.turn >= turn) return;
      expect(remaining, `player-turn ${String(turn)} is reachable inside the budget`).to.be.greaterThan(0);
      cy.jackioh().then((handle) => {
        // In hotseat the handle names the seat holding the device; networked, there is nothing to
        // hand over and `seat` is this client's own seat for the whole match.
        if (handle.seat !== undefined && handle.seat !== state.active) cy.handOver();
      });
      exists(ts(END_TURN)).then((present) => {
        if (present) cy.endTurn();
        else cy.handOver();
      });
      step(remaining - 1);
    });
  };
  step(budget);
});

// Sessions (A10): the networked specs sign in before they visit

/**
 * The account `cy.signIn` last named, for the rest of this spec file. `testIsolation: true` clears the browser's copy
 * between tests, so it is re-installed on every load, which makes `cy.signIn` usable from a `before` hook.
 */
let signedIn: E2EAccount | null = null;

/** A10: the shape `apps/web/src/net/session.ts` parses — an access token and nothing else. */
function writeSession(win: Window, account: E2EAccount): void {
  try {
    win.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: account.token }));
  } catch (error) {
    Cypress.log({ name: "signIn", message: `localStorage unavailable: ${String(error)}` });
  }
}

/**
 * Sign in as a fixture account. The session is `localStorage[SESSION_STORAGE_KEY]`, so it survives spec 05's mid-match
 * reload, written in `onBeforeLoad` so the first boot has it (§9.4's gate). `visit` below carries the last session.
 */
Cypress.Commands.add("signIn", (account: E2EAccount) => {
  signedIn = account;
  return cy.wrap(account, { log: false });
});

/** Forget the session, so a later `cy.visit` boots signed out (§9.4's login screen). */
Cypress.Commands.add("signOut", () => {
  signedIn = null;
  cy.window({ log: false }).then((win) => {
    try {
      win.localStorage.removeItem(SESSION_STORAGE_KEY);
    } catch {
      // A private window or blocked site data: there was no session to remove.
    }
  });
});

/** `cy.signIn(account)` + `cy.visit(path)`, which is how the networked specs open a screen. */
Cypress.Commands.add("visitAs", (account: E2EAccount, path: string, options: Partial<Cypress.VisitOptions> = {}) => {
  cy.signIn(account);
  cy.visit(path, options);
});

// A signed-in session is installed by the visit itself, chained onto whatever `onBeforeLoad` the
// caller passed (so `seedGame`'s deck injection still runs); with none signed in it passes through.
const carrySession = (originalFn: (...args: unknown[]) => unknown, ...args: unknown[]): unknown => {
  const account = signedIn;
  if (account === null) return originalFn(...args);
  const [first, second] = args;
  const given = (typeof first === "string" ? second : first) as Partial<Cypress.VisitOptions> | undefined;
  const merged: Partial<Cypress.VisitOptions> = {
    ...(given ?? {}),
    onBeforeLoad(win: Cypress.AUTWindow) {
      writeSession(win, account);
      given?.onBeforeLoad?.(win);
    },
  };
  return typeof first === "string" ? originalFn(first, merged) : originalFn(merged);
};

// `as never` only silences the overload gymnastics of Commands.overwrite, as support/e2e.ts does.
Cypress.Commands.overwrite("visit", carrySession as never);

// Saved decks and trios (A12) and the deck workshop (A11)

/**
 * A12: one scenario deck, padded into a trio R253's Best of 3 accepts (L1's `DECKS_PER_LOADOUT` decks, no card in two
 * by L4) with `DECK_SIZE * 2` unused Core ids, which with R111's one copy of each non-token card stays inside L5.
 */
export function loadoutFrom(deck: readonly string[], deckIndex = 0): string[][] {
  const used = new Set(deck);
  const spare = allCardIds().filter((id) => !used.has(id));
  const size = constants.DECK_SIZE;
  expect(spare.length, "enough spare Core ids to pad a loadout").to.be.at.least(size * 2);
  const padding = [spare.slice(0, size), spare.slice(size, size * 2)];
  const decks: string[][] = [];
  for (let at = 0; at < constants.DECKS_PER_LOADOUT; at += 1) {
    decks.push(at === deckIndex ? [...deck] : (padding.shift() ?? []));
  }
  return decks;
}

/** `GET /api/decks` (`DecksResponse` in apps/web/src/net/api.ts), as far as the suite reads it. */
export type SavedDecksBody = {
  catalogVersion: string;
  decks: { id: string; name: string; cards: string[]; catalogVersion: string; createdAt: number; updatedAt: number }[];
  trios: { id: string; name: string; deckIds: (string | null)[]; createdAt: number; updatedAt: number }[];
  limits: { decks: number; trios: number; nameLength: number };
};

/** What `cy.installLoadout` saved: three decks, oldest first, and the trio that holds them. */
export type InstalledLoadout = {
  /** In saved order, `GET /api/decks`'s (oldest first, ties on id): `deckIds[n]` is R257's `{ deckIndex: n }`. */
  deckIds: [string, string, string];
  /** The trio ("E2E trio") holding `deckIds` in slot order. */
  trioId: string;
  /** The cards of each deck, in the same order; `decks[deckIndex]` is the fixture's. */
  decks: string[][];
  /** The catalog version the decks were saved against. */
  catalogVersion: string;
};

/** The deck names `cy.installLoadout` saves, in order. */
export const INSTALLED_DECK_NAMES = ["Deck 1", "Deck 2", "Deck 3"] as const;
/** The trio name `cy.installLoadout` saves. */
export const INSTALLED_TRIO_NAME = "E2E trio";

/** R256: a deck or trio id, minted as the workshop mints one; hand-built when not a secure context. */
export function mintId(): string {
  const webCrypto = globalThis.crypto as Crypto | undefined;
  if (webCrypto !== undefined && typeof webCrypto.randomUUID === "function") return webCrypto.randomUUID();
  const hex = Array.from({ length: 32 }, () => Math.floor(Math.random() * 16));
  hex[12] = 4;
  hex[16] = ((hex[16] ?? 0) & 0x3) | 0x8;
  const text = hex.map((digit) => digit.toString(16)).join("");
  return `${text.slice(0, 8)}-${text.slice(8, 12)}-${text.slice(12, 16)}-${text.slice(16, 20)}-${text.slice(20)}`;
}

/**
 * `count` fresh ids in ascending order: `GET /api/decks` breaks a `createdAt` tie on the id (Postgres sorts a `uuid` as
 * its lower-case text) and three saves can land in one millisecond, so decks saved in these ids' order list in it.
 */
export function mintIds(count: number): string[] {
  return Array.from({ length: count }, mintId)
    .map((id) => id.toLowerCase())
    .sort();
}

Cypress.Commands.add("savedDecks", (account: E2EAccount) => {
  return cy
    .request<SavedDecksBody>({ method: "GET", url: apiUrl("/api/decks"), headers: bearerHeaders(account.token) })
    .then((response) => {
      expect(response.status, "GET /api/decks").to.eq(200);
      return response.body;
    });
});

/**
 * Delete every trio and deck an account has saved, and yield the catalog version the server serves (a save needs it: a
 * stale one is 409 `update_required`, R253). Deleting a deck empties the slots that named it anyway (R252).
 */
Cypress.Commands.add("clearDecks", (account: E2EAccount) => {
  return cy.savedDecks(account).then((saved) => {
    for (const trio of saved.trios) {
      cy.request({ method: "DELETE", url: apiUrl(`/api/trios/${trio.id}`), headers: bearerHeaders(account.token) })
        .its("status")
        .should("eq", 200);
    }
    for (const deck of saved.decks) {
      cy.request({ method: "DELETE", url: apiUrl(`/api/decks/${deck.id}`), headers: bearerHeaders(account.token) })
        .its("status")
        .should("eq", 200);
    }
    return cy.wrap(saved.catalogVersion, { log: false });
  });
});

/** One `PUT /api/decks/:id` (R250, R256): a create for a new id, else a replace. Yields the id. */
Cypress.Commands.add("saveDeck", (account: E2EAccount, deck: SaveDeckInput) => {
  const id = deck.id ?? mintId();
  const put = (catalogVersion: string): Cypress.Chainable<string> =>
    cy
      .request({
        method: "PUT",
        url: apiUrl(`/api/decks/${id}`),
        headers: bearerHeaders(account.token),
        body: { name: deck.name, cards: deck.cards, catalogVersion },
      })
      .then((response) => {
        expect(response.status, `PUT /api/decks/${id} ("${deck.name}")`).to.eq(200);
        return id;
      });
  if (deck.catalogVersion !== undefined) return put(deck.catalogVersion);
  return cy.savedDecks(account).then((saved) => put(saved.catalogVersion));
});

/** One `PUT /api/trios/:id` (R252, R256). Yields the id. */
Cypress.Commands.add("saveTrio", (account: E2EAccount, trio: SaveTrioInput) => {
  const id = trio.id ?? mintId();
  return cy
    .request({
      method: "PUT",
      url: apiUrl(`/api/trios/${id}`),
      headers: bearerHeaders(account.token),
      body: { name: trio.name, deckIds: trio.deckIds },
    })
    .then((response) => {
      expect(response.status, `PUT /api/trios/${id} ("${trio.name}")`).to.eq(200);
      return id;
    });
});

/**
 * Replace everything an account saved with a fixture deck as deck `deckIndex` (default 0), two padding decks and a trio
 * (SPEC §9.4, R250–R253), saved in `mintIds` order so `deckIds[n]` is what `{ deckIndex: n }` names (R257, §9.5).
 */
Cypress.Commands.add(
  "installLoadout",
  (account: E2EAccount, fixtureId: string, options: { deckIndex?: number } = {}) => {
    const deckIndex = options.deckIndex ?? 0;
    expect(deckIndex, `a deck index inside L1's ${String(constants.DECKS_PER_LOADOUT)} decks`).to.be.within(
      0,
      constants.DECKS_PER_LOADOUT - 1,
    );
    return cy.fixture(`decks/${fixtureId}.json`).then((raw) => {
      const decks = loadoutFrom(asDeck(raw, fixtureId).cards, deckIndex);
      const ids = mintIds(constants.DECKS_PER_LOADOUT);
      const [first, second, third] = ids;
      if (first === undefined || second === undefined || third === undefined) {
        throw new Error("mintIds(3) did not yield three ids");
      }
      return cy.clearDecks(account).then((catalogVersion) => {
        decks.forEach((cards, at) => {
          cy.saveDeck(account, {
            id: ids[at] ?? mintId(),
            name: INSTALLED_DECK_NAMES[at] ?? `Deck ${String(at + 1)}`,
            cards,
            catalogVersion,
          });
        });
        return cy.saveTrio(account, { name: INSTALLED_TRIO_NAME, deckIds: [first, second, third] }).then(
          (trioId): InstalledLoadout => ({ deckIds: [first, second, third], trioId, decks, catalogVersion }),
        );
      });
    });
  },
);

// Leaving nothing behind on the E2E server (§9.5, R259–R261)

/** `GET /api/auth/me`, as far as the cleanup reads it. */
type MeBody = { currentMatchId: string | null; currentSeriesId?: string | null };

/** `GET /api/series/:id` (`SeriesView`), as far as the cleanup reads it. */
type SeriesStatusBody = { status: "picking" | "playing" | "over"; currentMatchId: string | null };

/**
 * §2.5: concede `matchId` as `account`, from a socket of its own in one task, so the account's
 * browser cannot take the seat back in between. A match that was already over is not an error.
 */
Cypress.Commands.add("concedeAs", (account: E2EAccount, matchId: string) => {
  return cy
    .task<WsPlayerResult>(
      "wsPlayer",
      { action: "concede", name: `concede-${account.email}`, url: server.ws(), token: account.token, matchId },
      { timeout: timeouts.task },
    )
    .then((result) => {
      const refused = result.code === "match_over";
      expect(
        result.ok || refused,
        `${account.email} conceded ${matchId}: ${result.code ?? ""} ${result.error ?? "ok"}`,
      ).to.eq(true);
      return result;
    });
});

/**
 * How many things `cy.freeAccount` may have to undo: a match, then (in a series) the forfeit that
 * follows it, with room for a concede its own browser raced. A bound on a cleanup loop, not a rule.
 */
const FREE_ACCOUNT_STEPS = 8;

/**
 * Take an account out of an open queue ticket, a live match (§2.5) and a series not over (R261): the server keeps them,
 * so a failed spec draws a 409 on the next `POST /api/queue`; the loop's budget fails loudly on one it cannot clear.
 */
Cypress.Commands.add("freeAccount", (account: E2EAccount) => {
  cy.request({ method: "DELETE", url: apiUrl("/api/queue"), headers: bearerHeaders(account.token), failOnStatusCode: false });
  const step = (left: number): void => {
    cy.request<MeBody>({ method: "GET", url: apiUrl("/api/auth/me"), headers: bearerHeaders(account.token) }).then(
      (response) => {
        const matchId = response.body.currentMatchId;
        const seriesId = response.body.currentSeriesId ?? null;
        if (matchId === null && seriesId === null) return;
        expect(left, `${account.email} is out of every match and series in time`).to.be.greaterThan(0);
        if (matchId !== null) {
          cy.concedeAs(account, matchId);
        } else if (seriesId !== null) {
          cy.request<SeriesStatusBody>({
            method: "GET",
            url: apiUrl(`/api/series/${seriesId}`),
            headers: bearerHeaders(account.token),
            failOnStatusCode: false,
          }).then((series) => {
            if (series.status !== 200) return;
            if (series.body.status === "picking") {
              cy.request({
                method: "POST",
                url: apiUrl(`/api/series/${seriesId}/forfeit`),
                headers: bearerHeaders(account.token),
                failOnStatusCode: false,
              });
            } else if (series.body.status === "playing" && series.body.currentMatchId !== null) {
              cy.concedeAs(account, series.body.currentMatchId);
            }
          });
        }
        step(left - 1);
      },
    );
  };
  step(FREE_ACCOUNT_STEPS);
});

/**
 * R897000: stand in for a queued client. The queue cancels a ticket unheard from for `QUEUE_TICKET_STALE_MS`, so an
 * account enqueued over HTTP polls as a client does, via the `stayQueued` task, which ends once the ticket is gone.
 */
Cypress.Commands.add("stayQueued", (account: E2EAccount, everyMs: number) => {
  cy.task("stayQueued", { url: server.http(), token: account.token, everyMs }, { log: false });
});

/**
 * A11: drag a card from the pool into the open deck, the whole multi-event gesture so no spec
 * hand-rolls it. The `DataTransfer` is built in the app's own window so the page can read it.
 */
Cypress.Commands.add("dragCardToDeck", (catalogCardId: string) => {
  const source = ts(poolCardId(catalogCardId));
  const target = ts(DECK_DROP);
  cy.get(source, { timeout: timeouts.view }).should("exist");
  cy.get(target).should("exist");

  cy.window({ log: false }).then((win) => {
    const dataTransfer = new win.DataTransfer();
    dataTransfer.setData(DECK_DRAG_MIME, catalogCardId);
    dataTransfer.setData("text/plain", catalogCardId);

    cy.get(source).trigger("dragstart", { dataTransfer, eventConstructor: "DragEvent" });
    cy.get(target).trigger("dragover", { dataTransfer, eventConstructor: "DragEvent" });
    cy.get(target).trigger("drop", { dataTransfer, eventConstructor: "DragEvent" });
    // The source may be gone or greyed out after a successful drop, so letting go is best-effort.
    exists(source).then((present) => {
      if (present) cy.get(source).trigger("dragend", { dataTransfer, eventConstructor: "DragEvent" });
    });
  });
});

// replay determinism (spec 01) and the Node WebSocket player (spec 06)

/**
 * BUILD M8 spec 01: "final state hash equals the vitest replay of the recorded actions". The log is
 * folded with the Rust engine (`jackioh replay`, support/tasks/replay-runner.ts) and both states hashed.
 */
Cypress.Commands.add("replayCheck", (label: string) => {
  cy.jackioh().then((handle) => {
    const { seed, decks, log, state } = handle;
    expect(decks, "window.__jackioh.decks (ASSUMPTION A2)").to.be.an("array");
    expect(log, "window.__jackioh.log (ASSUMPTION A2)").to.be.an("array");
    // R180: a handicapped game folds only with the handicaps it was created with; none is `{}`.
    const handicaps = handle.handicaps ?? {};
    return cy
      .task<{ replayHash: string; browserHash: string; errors: unknown[] }>(
        "replayHash",
        { label, seed, decks, log, state, ...(Object.keys(handicaps).length === 0 ? {} : { handicaps }) },
        { timeout: timeouts.task },
      )
      .should((result) => {
        expect(result.errors, "the recorded log replays with no rejected action").to.deep.eq([]);
        expect(result.replayHash, "engine replay hash equals the browser's final state hash").to.eq(
          result.browserHash,
        );
      });
  });
});

Cypress.Commands.add("wsPlayer", (command: WsPlayerCommand) => {
  return cy.task<WsPlayerResult>("wsPlayer", command, { timeout: timeouts.task }).should((result) => {
    // §9.3: the server's own words. The code is a label on the failure, not a rewording of it.
    const why = result.code === undefined ? (result.error ?? "ok") : `[${result.code}] ${result.error ?? ""}`;
    expect(result.ok, `wsPlayer ${command.action}: ${why}`).to.eq(true);
  });
});

Cypress.Commands.add("expectResult", (text: "Win" | "Loss" | "Draw") => {
  cy.get(ts(RESULT_OVERLAY), { timeout: timeouts.game }).should("be.visible");
  cy.get(ts(RESULT_OVERLAY)).should("contain.text", text);
});

// Augmenting Cypress's own `Chainable` means using its namespace and repeating its
// `Subject = any` default exactly, or declaration merging fails.
/* eslint-disable @typescript-eslint/no-namespace, @typescript-eslint/no-explicit-any, @typescript-eslint/no-unused-vars */
declare global {
  namespace Cypress {
    interface Chainable<Subject = any> {
      /** Visit the hotseat route with a seed and two fixture decks (BUILD M5-T3). */
      seedGame(options: SeedGameOptions): Chainable<void>;
      /** Keep every card in any open opening-mulligan prompt (R9, R265). */
      keepMulligans(): Chainable<void>;
      /** Click `concede`, then `concede-confirm` in the dialog it opens (§2.5). */
      concede(): Chainable<void>;
      /** Play a hand card, building its R81 play-time choices from the UI. */
      playCard(instanceId: string, options?: PlayCardOptions): Chainable<void>;
      /** Declare an attack against a unit or a hero (SPEC §4.2). */
      attack(attackerId: string, target: AttackTarget, options?: ActOptions): Chainable<void>;
      /** Answer the open prompt; `null` accepts whatever kind is open (SPEC §10.6). */
      answerPrompt(kind: PromptKind | null, answer?: PromptAnswer): Chainable<void>;
      /** Click `end-turn`, then hand the device over if the client asks for it. */
      endTurn(options?: ActOptions & { handOver?: boolean }): Chainable<void>;
      handOver(): Chainable<void>;
      switchPosition(instanceId: string, options?: ActOptions): Chainable<void>;
      offerDraw(): Chainable<void>;
      usePower(options?: ActOptions): Chainable<void>;
      /** Every `data-animating` element has drained (BUILD M5-T4). */
      settled(): Chainable<void>;
      /** An element is currently animating this event type. */
      expectAnimating(event: EventType): Chainable<void>;
      waitForPrompt(kind: PromptKind): Chainable<void>;
      noPrompt(): Chainable<void>;
      handCardByName(name: string): Chainable<string>;
      fieldCardByName(name: string): Chainable<string>;
      instanceInHand(player: PlayerId, defId: string): Chainable<string>;
      instanceAt(player: PlayerId, row: "units" | "backrow", lane: number): Chainable<string>;
      playByName(name: string, options?: PlayCardOptions): Chainable<void>;
      /** The instance ids in a seat's hand (setup only: assertions belong on the DOM). */
      handIds(player: PlayerId): Chainable<string[]>;
      /** The top card of each of a seat's unit zones, in lane order (§3.2). */
      unitIds(player: PlayerId): Chainable<string[]>;
      /** End turns until `turn` is the current player-turn; R82-safe (§10.1). */
      advanceToTurn(turn: number, options?: { budget?: number }): Chainable<void>;
      /** Remember a fixture account, so every later visit boots signed in as it (A10). */
      signIn(account: E2EAccount): Chainable<E2EAccount>;
      /** Forget it again, so a later visit boots signed out. */
      signOut(): Chainable<void>;
      /** `cy.signIn(account)` then `cy.visit(path)`, session installed before the page runs. */
      visitAs(account: E2EAccount, path: string, options?: Partial<Cypress.VisitOptions>): Chainable<void>;
      /** Replace an account's saved decks with a fixture's loadout (§9.4, R250–R253); yields the ids. */
      installLoadout(account: E2EAccount, fixtureId: string, options?: { deckIndex?: number }): Chainable<InstalledLoadout>;
      /** `GET /api/decks` as `account`. */
      savedDecks(account: E2EAccount): Chainable<SavedDecksBody>;
      /** Delete every saved trio and deck of `account`; yields the server's catalog version. */
      clearDecks(account: E2EAccount): Chainable<string>;
      /** `PUT /api/decks/:id` as `account`; yields the id. */
      saveDeck(account: E2EAccount, deck: SaveDeckInput): Chainable<string>;
      /** `PUT /api/trios/:id` as `account`; yields the id. */
      saveTrio(account: E2EAccount, trio: SaveTrioInput): Chainable<string>;
      /** Concede `matchId` as `account` from a socket of its own (§2.5). */
      concedeAs(account: E2EAccount, matchId: string): Chainable<WsPlayerResult>;
      /** Dequeue, concede a live match and forfeit a series between games (§9.5, R261). */
      freeAccount(account: E2EAccount): Chainable<void>;
      /** Poll as `account` every `everyMs` while its queue ticket is open, as a client does (R897000). */
      stayQueued(account: E2EAccount, everyMs: number): Chainable<void>;
      /** Drag a pool card into the open deck in the workshop: the whole gesture, not one event (A11). */
      dragCardToDeck(catalogCardId: string): Chainable<void>;
      jackioh(): Chainable<JackiOhDevHandle>;
      gameState(): Chainable<GameStateLike>;
      dispatchAction(action: ActionInput): Chainable<void>;
      replayCheck(label: string): Chainable<void>;
      wsPlayer(command: WsPlayerCommand): Chainable<WsPlayerResult>;
      expectResult(text: "Win" | "Loss" | "Draw"): Chainable<void>;
    }
  }
}
