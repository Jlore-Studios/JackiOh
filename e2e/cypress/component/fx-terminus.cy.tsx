// R3338000: the Terminus cards' set pieces through the real board's effects layer. Every one, and Dice of
// Doom's roll on a 67, a miss and Embiggened, plays at desktop size and on a phone (390x844): the queue
// holds its moment for its own set piece, its cue stands over the board in its own shape, and at rest
// nothing of it is left.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-terminus.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { FX_CARD_TABLE } from "../../../apps/web/src/fx/cards/tables.ts";
import { doomFixture, galleryCardEvents, type DoomRoll } from "../../../apps/web/src/fx/motifs/gallery.ts";
import { SET_PIECES, type SetPieceKey } from "../../../apps/web/src/fx/setPieces.ts";
import { DICE_OF_DOOM } from "../../../apps/web/src/fx/terminus/ids.ts";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const LAYER = '[data-testid="fx-layer"]';
const FX_DOM = '[data-testid="fx-dom"]';
const GAME = '[data-testid="game"]';

const VIEWPORTS = [
  { label: "1280x720", width: 1280, height: 720 },
  { label: "390x844", width: 390, height: 844 },
] as const;

/** The nine Terminus set pieces, in SET_PIECE_KEYS order. */
const KEYS: readonly SetPieceKey[] = ["voidFold", "terminusRift", "farewell", "lastExam", "eventHorizon", "lastWord", "finalDestination", "bigCrunch", "diceOfDoom"];

function cardOf(key: SetPieceKey): string {
  const found = [...FX_CARD_TABLE].find(([, entry]) => entry.kind === "setPiece" && entry.key === key);
  if (found === undefined) throw new Error(`no card names ${key}`);
  return found[0];
}

/** Mounts the board, then plays the scene over it (fx-gallery-captures.cy.tsx's `mountShown`). */
function mountShown(initial: PlayerView, shown: PlayerView): void {
  const lookup = lookupFromDefs(CATALOG);
  const noop = (): void => undefined;
  const board = (view: PlayerView) => (
    <CatalogContext.Provider value={lookup}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={noop} />
      </div>
    </CatalogContext.Provider>
  );
  cy.mount(board(initial)).then(({ rerender }) => {
    cy.get(LAYER).should("have.attr", "data-fx", "on");
    cy.then(() => rerender(board(shown)));
  });
}

/** Dice of Doom played by the viewer, its roll landing as `roll` says. */
function doomPlay(view: PlayerView, roll: DoomRoll): GameEvent[] {
  const me = view.viewer;
  return [
    { type: "cardPlayed", player: me, instanceId: "fx-doom", defId: DICE_OF_DOOM, costPaid: 0 },
    ...doomFixture(view, "fx-doom", me, roll),
    { type: "cardResolved", player: me, instanceId: "fx-doom", defId: DICE_OF_DOOM, permanent: false, costPaid: 0 },
  ];
}

/** Plays `events`, sees `key`'s set piece hold in its shape, then sees it gone at rest. */
function playsThrough(key: SetPieceKey, events: GameEvent[], name: string): void {
  const view = fullBoardView();
  mountShown(view, withEvents(view, events));
  cy.get(GAME, { timeout: 6_000 }).should("have.attr", "data-set-piece", key);
  cy.get(GAME).should("have.attr", "data-set-piece-shape", SET_PIECES[key].shape);
  cy.get(`${FX_DOM} [data-fx="setPiece"][data-shape="${SET_PIECES[key].shape}"]`).should("exist");
  if (key === "diceOfDoom") cy.get(`${FX_DOM} [data-fx="roll"]`).should("have.attr", "data-look", "doom");
  cy.screenshot(`terminus-${name}-mid`, { capture: "viewport", overwrite: true });
  cy.get("[data-animating]", { timeout: 12_000 }).should("not.exist");
  cy.get(GAME).should("not.have.attr", "data-set-piece");
  cy.get(`${FX_DOM} [data-fx="setPiece"]`, { timeout: 4_000 }).should("not.exist");
}

describe("R3338000 the Terminus set pieces play at desktop size and on a phone", () => {
  for (const viewport of VIEWPORTS) {
    for (const key of KEYS) {
      it(`R3338000 ${key} at ${viewport.label}`, () => {
        cy.viewport(viewport.width, viewport.height);
        const view = fullBoardView();
        playsThrough(key, galleryCardEvents(cardOf(key), { kind: "setPiece", key }, view, 1), `${key}-${viewport.label}`);
      });
    }
    for (const roll of ["miss", "embiggened"] as const) {
      it(`R3338000 diceOfDoom ${roll} at ${viewport.label}`, () => {
        cy.viewport(viewport.width, viewport.height);
        playsThrough("diceOfDoom", doomPlay(fullBoardView(), roll), `diceOfDoom-${roll}-${viewport.label}`);
      });
    }
  }
});
