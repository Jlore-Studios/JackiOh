// Issue #3734 (R3734001): a deck is built by touch alone. A pool card is dragged into the deck with pointer events, so a
// finger works as a mouse does: jsdom lays nothing out and sends no touch, so this mounts the real deck workshop at an upright
// iPad's size and sends a finger's pointer events (`pointerType: "touch"`) down, across to the deck and up.

import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";
import { MAX_SAVED_DECKS, MAX_SAVED_TRIOS, DECK_NAME_MAX_LENGTH } from "@jackioh/server-config";
import DeckWorkshop from "../../../apps/web/src/game/deckbuilder/DeckWorkshop.tsx";
import type { DeckSyncInput, DecksResponse } from "../../../apps/web/src/net/api.ts";
import { loadWasm } from "../../../apps/web/src/wasm/index.ts";
import { setShips } from "../../../apps/web/src/wire/catalog.ts";
import { CARD_POOL, DECK_COUNT, DECK_DRAG_GHOST, DECK_DROP, DECK_EDITOR, WORKSHOP, deckCardId, ts } from "../../support/testids.ts";

const DECK_ID = "00000000-0000-4000-8000-000000000001";
const DECKABLE = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token") && setShips(def.set));
const COLLECTION: Record<string, number> = Object.fromEntries(DECKABLE.map((def) => [def.id, 1]));

/** Far enough past the drag threshold (8 px) that no rounding keeps a move below it. */
const NUDGE_PX = 24;

function mountWorkshop(): void {
  const data: DecksResponse = {
    catalogVersion: CATALOG_VERSION,
    decks: [{ id: DECK_ID, name: "Deck 1", cards: [], portrait: null, catalogVersion: CATALOG_VERSION, createdAt: 0, updatedAt: 0 }],
    trios: [],
    limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS, nameLength: DECK_NAME_MAX_LENGTH },
  };
  const api = {
    syncDecks: cy.stub().callsFake((input: DeckSyncInput) =>
      Promise.resolve({
        decks: input.decks.map(({ id }) => ({ id })),
        trios: input.trios.map(({ id }) => ({ id })),
        deletedTrios: input.deletedTrios.map((id) => ({ id })),
        deletedDecks: input.deletedDecks.map((id) => ({ id })),
      }),
    ),
    importTrio: cy.stub().resolves({}),
    getDeckVersions: cy.stub().resolves({ versions: [] }),
    restoreDeckVersion: cy.stub().resolves({}),
  };
  cy.mount(
    <DeckWorkshop
      catalog={{ version: CATALOG_VERSION, cards: CATALOG }}
      collection={COLLECTION}
      data={data}
      profileId="component-spec"
      api={api}
      storage={null}
      initialOpen={{ kind: "deck", id: DECK_ID }}
    />,
  );
}

type Pointer = "mouse" | "touch";

/**
 * The gesture: press the card, a first move of `NUDGE_PX` across or down, a move to the deck's middle and a release there.
 * A finger's `pointerId` is its own, and `force` leaves it to the app (not Cypress) to decide what covers what.
 */
function drag(cardTestid: string, pointer: Pointer, first: "across" | "down"): void {
  const init = { eventConstructor: "PointerEvent", pointerType: pointer, pointerId: 7, isPrimary: true, button: 0, force: true } as const;
  cy.get(cardTestid).then(($card) => {
    const box = $card[0]?.getBoundingClientRect();
    if (box === undefined) throw new Error("the card has no box");
    const x = box.left + box.width / 2;
    const y = box.top + box.height / 2;
    cy.get(cardTestid).trigger("pointerdown", { ...init, clientX: x, clientY: y });
    cy.get(cardTestid).trigger("pointermove", { ...init, clientX: first === "across" ? x + NUDGE_PX : x, clientY: first === "down" ? y + NUDGE_PX : y });
  });
  cy.get(ts(DECK_DROP)).trigger("pointermove", init);
}

function release(pointer: Pointer): void {
  cy.get(ts(DECK_DROP)).trigger("pointerup", { eventConstructor: "PointerEvent", pointerType: pointer, pointerId: 7, isPrimary: true, button: 0, force: true });
}

/** The first card of the pool the deck would take. */
function firstLegal(): Cypress.Chainable<string> {
  return cy
    .get(`${ts(CARD_POOL)} .db-card[data-legal="true"]`, { timeout: 30_000 })
    .first()
    .invoke("attr", "data-card")
    .then((id) => String(id));
}

describe("R3734001 a deck is built by touch drag alone", () => {
  before(() => {
    cy.wrap(loadWasm(), { timeout: 60_000 });
  });

  beforeEach(() => {
    cy.viewport(820, 1180);
    mountWorkshop();
    cy.get(ts(WORKSHOP)).should("be.visible");
    cy.get(ts(DECK_EDITOR)).should("have.attr", "data-deck", DECK_ID);
    cy.get(ts(DECK_COUNT)).should("have.attr", "data-count", "0");
    cy.get(ts(DECK_DROP)).should("be.visible");
  });

  it("R3734001 a finger that moves across lifts the card, and releasing it over the deck adds it", () => {
    firstLegal().then((id) => {
      const card = ts(`card-pool-${id}`);
      drag(card, "touch", "across");
      // The card is in flight: its ghost follows the finger, and the deck marks where it will land.
      cy.get(ts(DECK_DRAG_GHOST)).should("be.visible").and("have.attr", "data-pointer", "touch");
      cy.get(ts(DECK_DROP)).should("have.attr", "data-drop", "over");
      release("touch");
      cy.get(ts(DECK_COUNT)).should("have.attr", "data-count", "1");
      cy.get(ts(deckCardId(id))).should("exist");
      cy.get(ts(DECK_DRAG_GHOST)).should("not.exist");
      cy.get(ts(DECK_DROP)).should("not.have.attr", "data-drop");
    });
  });

  it("R3734001 a finger that moves down first is the pool's scroll: nothing lifts and nothing is added", () => {
    firstLegal().then((id) => {
      drag(ts(`card-pool-${id}`), "touch", "down");
      cy.get(ts(DECK_DRAG_GHOST)).should("not.exist");
      release("touch");
      cy.get(ts(DECK_COUNT)).should("have.attr", "data-count", "0");
      cy.get(ts(deckCardId(id))).should("not.exist");
    });
  });

  it("R3734001 a release anywhere but over the deck adds nothing", () => {
    firstLegal().then((id) => {
      const card = ts(`card-pool-${id}`);
      drag(card, "touch", "across");
      cy.get(ts(DECK_DRAG_GHOST)).should("be.visible");
      cy.get(ts(CARD_POOL)).trigger("pointerup", { eventConstructor: "PointerEvent", pointerType: "touch", pointerId: 7, isPrimary: true, button: 0, force: true, clientX: 5, clientY: 5 });
      cy.get(ts(DECK_COUNT)).should("have.attr", "data-count", "0");
      cy.get(ts(DECK_DRAG_GHOST)).should("not.exist");
    });
  });

  it("R3734001 a mouse drags the same way, in any direction", () => {
    firstLegal().then((id) => {
      drag(ts(`card-pool-${id}`), "mouse", "down");
      cy.get(ts(DECK_DRAG_GHOST)).should("have.attr", "data-pointer", "mouse");
      release("mouse");
      cy.get(ts(DECK_COUNT)).should("have.attr", "data-count", "1");
    });
  });

  it("R3734001 the pool keeps the card's native image drag and callout off, and pans only up and down", () => {
    firstLegal().then((id) => {
      cy.get(ts(`card-pool-${id}`)).should("have.css", "touch-action", "pan-y").and("have.css", "user-select", "none");
    });
  });
});
