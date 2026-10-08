// #492: every Embiggen card asks whether to pay its embiggen price, against the REAL engine.
//
// The engine lists a play of an "A embiggen B" card with `embiggen: false` and, once B is affordable,
// the same play with `embiggen: true` (R81), for every zone the card may take. The client builds the
// play from those (CLAUDE.md rule 7): with both forms listed it opens the embiggen picker, two cards
// in the middle of the screen, and sends the form picked; with only the normal form listed it asks
// nothing about the price. Here the view and the legal list are `viewFor` and `legalActions` of real
// states (the WebAssembly module, docs/v0.3.0/SURFACE.md §10.3), the board is `<Game/>`, and the play
// is made the two ways a player makes it, click-click and drag to play. The Embiggened card shows the
// view's `embiggenCost`, the engine's price for that play under every cost change (a discount R363's
// "(4)+ Cost" threshold gives the embiggen price alone included), and the play then pays it. Every
// Embiggen card in the catalog is driven, on both faces, so a new one is covered the day it is added.

import { CATALOG } from "@jackioh/cards";
import { DECK_SIZE } from "@jackioh/engine/config";
import type { Action, ActionBody, CardDef, PlayerId } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createGame, legalActions, reduce, registeredCatalog, viewFor } from "../wasm/index.ts";
import { closeInspect } from "../cards/index.ts";
import { __resetSettingsForTests } from "../settings/index.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import Game from "./Game.tsx";

type GameState = Parameters<typeof reduce>[0];

/** The fields of the state this file writes; everything else is the engine's, untouched. */
type Instance = { id: string; defId: string; radiant: boolean; zone: { z: string; player: PlayerId }; [field: string]: unknown };
type Side = { mana: { current: number; max: number }; hand: Instance[]; library: Instance[]; mods: unknown[] };
type Board = { active: PlayerId; turn: number; phase: string; players: Record<PlayerId, Side> };

const lookup = lookupFromDefs(CATALOG);
const SEED = "jackioh-492";
/** A mid-game turn, so nothing about the opening hand or the first turn is in play. */
const TURN = 9;
const el = (id: string): HTMLElement => screen.getByTestId(id);

/** Every "A embiggen B" card in the catalog, with its two printed prices. */
const EMBIGGEN: { def: CardDef; base: number; embiggen: number }[] = Object.values(registeredCatalog()).flatMap((def) =>
  typeof def.cost === "object" ? [{ def, base: def.cost.base, embiggen: def.cost.embiggen }] : [],
);

function fillerDeck(): string[] {
  return Object.values(registeredCatalog())
    .filter((def) => def.set === "Core" && !def.token && !def.tags.includes("Token"))
    .sort((a, b) => Number(a.index) - Number(b.index))
    .slice(0, DECK_SIZE)
    .map((def) => def.id);
}

/** A cost change on the board: the card's own `costMod` (R65), and p1's player modifiers (§10.1). */
type CostChange = { costMod?: number; mods?: unknown[] };

/**
 * p1's main phase on turn 9 with `defId` (on the face asked for) the only card in hand and `mana`
 * to spend, under `change`. The card is a copy of one the engine dealt, so it carries every field a
 * new card has.
 */
function board(defId: string, radiant: boolean, mana: number, change: CostChange = {}): { state: GameState; cardId: string } {
  const deck = fillerDeck();
  const state = createGame({ seed: SEED, decks: [deck, deck] });
  const layout = state as unknown as Board;
  const template = layout.players.p1.library[0];
  if (template === undefined) throw new Error("createGame dealt no library");
  const card: Instance = { ...(JSON.parse(JSON.stringify(template)) as Instance), id: "c1", defId, radiant, zone: { z: "hand", player: "p1" } };
  delete card.knownAs;
  for (const player of ["p1", "p2"] as const) {
    layout.players[player].library = [];
    layout.players[player].hand = [];
  }
  if (change.costMod !== undefined) card.costMod = change.costMod;
  layout.players.p1.hand.push(card);
  layout.players.p1.mods = change.mods ?? [];
  layout.active = "p1";
  layout.turn = TURN;
  layout.phase = "main";
  layout.players.p1.mana.max = mana;
  layout.players.p1.mana.current = mana;
  return { state, cardId: card.id };
}

function render492(state: GameState) {
  const legal = legalActions(state, "p1");
  const onAction = vi.fn<(body: ActionBody) => void>();
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={viewFor(state, "p1")} legal={legal} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return { legal, onAction };
}

/** The `play`s the engine lists for the card. */
function plays(legal: readonly ActionBody[], cardId: string): Extract<ActionBody, { type: "play" }>[] {
  return legal.flatMap((body) => (body.type === "play" && body.instanceId === cardId ? [body] : []));
}

/**
 * The one body the board sent: one the engine listed, and one it accepts. Returns the price the play
 * paid and whether it was embiggened, as the engine's `cardPlayed` says.
 */
function sent(
  state: GameState,
  legal: readonly ActionBody[],
  onAction: ReturnType<typeof vi.fn>,
): { body: ActionBody; paid: number | undefined; embiggened: boolean | undefined } {
  expect(onAction).toHaveBeenCalledTimes(1);
  const body = onAction.mock.calls[0]?.[0] as ActionBody;
  expect(legal).toContainEqual(body);
  const result = reduce(state, { ...body, playerId: "p1", nonce: "492" } as Action);
  expect(result.error).toBeUndefined();
  const played = result.events.find((event) => event.type === "cardPlayed");
  return played?.type === "cardPlayed"
    ? { body, paid: played.costPaid, embiggened: played.embiggened }
    : { body, paid: undefined, embiggened: undefined };
}

/** Finishes a play whose price is picked: a Field Spell still takes its zone, clicked on the board. */
function placeIfAsked(): void {
  if (screen.queryByTestId("prompt-modal")?.getAttribute("data-prompt-kind") === "zone") {
    fireEvent.click(el(testid.zone("you", "backrow", 1)));
  }
}

beforeEach(() => {
  __resetSettingsForTests();
});

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
});

describe("#492 R81 every Embiggen card asks for its price when both are affordable", () => {
  it("the catalog has Embiggen cards to drive", () => {
    expect(EMBIGGEN.length).toBeGreaterThan(0);
  });

  for (const { def, base, embiggen } of EMBIGGEN) {
    for (const radiant of [false, true]) {
      const name = `${def.name}${radiant ? " (Radiant)" : ""}`;

      it(`${name}: a click opens the two cards, and each sends its own form at its own price`, () => {
        for (const form of [true, false]) {
          const { state, cardId } = board(def.id, radiant, embiggen);
          const { legal, onAction } = render492(state);
          // The engine lists both forms, for every zone alike.
          const listed = plays(legal, cardId);
          expect(new Set(listed.map((play) => play.embiggen))).toEqual(new Set([false, true]));
          expect(listed.filter((play) => play.embiggen === true)).toHaveLength(listed.length / 2);

          fireEvent.click(el(testid.handCard(cardId)));
          const modal = el("prompt-modal");
          expect(modal).toHaveAttribute("data-prompt-kind", "embiggen");
          expect(modal).toHaveAttribute("data-prompt-layout", "cards");
          expect(el("prompt-option-false")).toHaveTextContent("Normal");
          expect(el("prompt-option-false")).toHaveTextContent(`Pay (${String(base)})`);
          expect(el("prompt-option-true")).toHaveTextContent("Embiggened");
          expect(el("prompt-option-true")).toHaveTextContent(`Pay (${String(embiggen)})`);
          // Each option is the card's own face, on the face it is played with.
          expect(el("prompt-option-true").getAttribute("aria-label")).toContain(def.name);
          expect(onAction).not.toHaveBeenCalled();

          fireEvent.click(el(`prompt-option-${String(form)}`));
          placeIfAsked();
          const { body, paid, embiggened } = sent(state, legal, onAction);
          expect(body).toEqual(expect.objectContaining({ type: "play", instanceId: cardId, embiggen: form }));
          expect(paid).toBe(form ? embiggen : base);
          expect(embiggened).toBe(form);
          cleanup();
        }
      });

      it(`${name}: with mana for its normal price only, it plays at that price and nothing asks about the price`, () => {
        const { state, cardId } = board(def.id, radiant, embiggen - 1);
        const { legal, onAction } = render492(state);
        expect(plays(legal, cardId).every((play) => play.embiggen === false)).toBe(true);

        fireEvent.click(el(testid.handCard(cardId)));
        expect(screen.queryByTestId("prompt-option-true")).toBeNull();
        expect(screen.queryByTestId("prompt-modal")?.getAttribute("data-prompt-kind")).not.toBe("embiggen");
        placeIfAsked();
        const { body, paid } = sent(state, legal, onAction);
        expect(body).toEqual(expect.objectContaining({ type: "play", instanceId: cardId, embiggen: false }));
        expect(paid).toBe(base);
      });
    }
  }
});

describe("#492 the Embiggened card shows the embiggen price as it stands, and the play pays it", () => {
  /** p1's player modifiers live this turn (§10.1 `mods`, as the state holds them). */
  const thisTurn = { until: "thisTurn", turn: TURN };
  type Case = { name: string; change: CostChange; normal: (a: number) => number; embiggened: (b: number) => number; tone: string };
  const cases: Case[] = [
    { name: "its own discount (costMod -1)", change: { costMod: -1 }, normal: (a) => a - 1, embiggened: (b) => b - 1, tone: "down" },
    {
      // R363: Professor Curvature's shape reaches a price of 4 or more: the embiggen price, not the base one.
      name: "a (4)+ Cost discount (R363)",
      change: { mods: [{ id: "m492-curve", expiry: thisTurn, kind: "costDiscount", amount: 1, minCurrentCost: 4 }] },
      normal: (a) => a,
      embiggened: (b) => b - 1,
      tone: "down",
    },
    {
      name: "a surcharge of 1 (R455)",
      change: { mods: [{ id: "m492-tax", expiry: thisTurn, kind: "costRule", rule: { amount: 1 } }] },
      normal: (a) => a + 1,
      embiggened: (b) => b + 1,
      tone: "up",
    },
  ];

  for (const { def, base, embiggen } of EMBIGGEN) {
    for (const { name, change, normal, embiggened, tone } of cases) {
      const price = embiggened(embiggen);
      it(`${def.name} under ${name}: Normal says (${String(normal(base))}), Embiggened (${String(price)}), and the play pays that`, () => {
        const { state, cardId } = board(def.id, false, price, change);
        const { legal, onAction } = render492(state);
        expect(viewFor(state, "p1").you.hand).toEqual(
          expect.arrayContaining([expect.objectContaining({ instanceId: cardId, cost: normal(base), embiggenCost: price })]),
        );

        fireEvent.click(el(testid.handCard(cardId)));
        expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "embiggen");
        expect(el("prompt-option-false")).toHaveTextContent(`Pay (${String(normal(base))})`);
        expect(el("prompt-option-true")).toHaveTextContent(`Pay (${String(price)})`);
        const gem = el("prompt-option-true").querySelector(".cost-gem");
        expect(gem).toHaveAttribute("data-cost", String(price));
        expect(gem).toHaveAttribute("data-tone", tone);

        fireEvent.click(el("prompt-option-true"));
        placeIfAsked();
        const { paid, embiggened: was } = sent(state, legal, onAction);
        expect(paid).toBe(price);
        expect(was).toBe(true);
      });
    }
  }
});

// ---------------------------------------------------------------------------------------------
// Drag to play. jsdom has no layout, so `document.elementsFromPoint` answers whatever the pointer is
// "over" (drag-layer.test.tsx does the same); the press goes to the card, the rest to `window`.
// ---------------------------------------------------------------------------------------------

let under: Element[] = [];
const START = { x: 200, y: 400 };

/**
 * A click as a pointer makes one, its press first: the layer swallows the click a drag's release
 * produces (DragLayer.tsx), and a new press is what tells it the next click is the player's.
 */
function tap(target: Element): void {
  fireEvent.pointerDown(target, { pointerId: 2, button: 0, clientX: 600, clientY: 300 });
  fireEvent.pointerUp(window, { pointerId: 2, button: 0, clientX: 600, clientY: 300 });
  fireEvent.click(target);
}

function drag(source: Element, onto: Element): void {
  under = [source];
  fireEvent.pointerDown(source, { pointerId: 1, button: 0, clientX: START.x, clientY: START.y });
  fireEvent.pointerMove(window, { pointerId: 1, clientX: START.x, clientY: START.y - 20 });
  under = [onto];
  fireEvent.pointerMove(window, { pointerId: 1, clientX: 420, clientY: 180 });
  fireEvent.pointerUp(window, { pointerId: 1, button: 0, clientX: 420, clientY: 180 });
}

describe("#492 R81 dragging an Embiggen card asks for its price too", () => {
  beforeEach(() => {
    under = [];
    document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
  });

  afterEach(() => {
    delete (document as Partial<Document>).elementsFromPoint;
    document.documentElement.removeAttribute("data-dragging");
  });

  for (const { def, base, embiggen } of EMBIGGEN) {
    it(`${def.name}: dropped on the board, it opens the two cards, and the card picked sends that form`, () => {
      const { state, cardId } = board(def.id, false, embiggen);
      const { legal, onAction } = render492(state);
      const zoned = plays(legal, cardId).some((play) => play.zone !== undefined);
      // A Field Spell lands in a zone it may take; a Spell anywhere on the board.
      drag(el(testid.handCard(cardId)), el(zoned ? testid.zone("you", "backrow", 2) : "board"));

      expect(onAction).not.toHaveBeenCalled();
      expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "embiggen");
      tap(el("prompt-option-true"));
      const { body, paid } = sent(state, legal, onAction);
      expect(body).toEqual(
        expect.objectContaining({
          type: "play",
          instanceId: cardId,
          embiggen: true,
          ...(zoned ? { zone: { row: "backrow", lane: 2 } } : {}),
        }),
      );
      expect(paid).toBe(embiggen);
    });

    it(`${def.name}: dropped with mana for its normal price only, it plays at that price without asking`, () => {
      const { state, cardId } = board(def.id, false, embiggen - 1);
      const { legal, onAction } = render492(state);
      const zoned = plays(legal, cardId).some((play) => play.zone !== undefined);
      drag(el(testid.handCard(cardId)), el(zoned ? testid.zone("you", "backrow", 2) : "board"));

      expect(screen.queryByTestId("prompt-modal")).toBeNull();
      const { body, paid } = sent(state, legal, onAction);
      expect(body).toEqual(expect.objectContaining({ type: "play", instanceId: cardId, embiggen: false }));
      expect(paid).toBe(base);
    });
  }
});
