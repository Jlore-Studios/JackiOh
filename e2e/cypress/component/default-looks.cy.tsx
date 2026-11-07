// R3343001, R3343002 (#3343): the default looks in a real browser. apps/web's vitest suite proves which card draws which
// look and what each face's markup says (cards/defaultLooks.test.tsx, cards/versions.test.ts, collection/collection.test.tsx);
// what only a layout engine can say is measured here, at a desktop's size and a phone's (390x844), on Ceaseless Void, a
// Mythic Unit whose Default is Full art, Silly Silas, a Legendary Unit whose Default is Extended art, and Felinor Fiender,
// a Legendary Unit worn at Prismatic's first level, a full art:
//  - in the viewer's own hand: the Mythic bare, its art across the face (R2723000), the Legendary's picture past its
//    window with its name and words, Prismatic across the face; every part drawn inside its face;
//  - on the board: the Mythic and Prismatic full art, their name in a band below the art, the Legendary keeping its
//    window (R3046000);
//  - in the Collection: the large face wearing the default look, the Default's first rung named and pictured by it, and
//    the ladder above it starting at its next level, next with nothing owned (R3343002).
// A screenshot of each is kept.

import { CATALOG } from "@jackioh/cards";
import "../../../apps/web/src/auth/tavern.css";
import { resetCosmeticsForTests, writeCardLevel } from "../../../apps/web/src/cards/cosmetics.ts";
import { FullArtOwned } from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import {
  VERSION_EXTENDED_ART_ID,
  VERSION_FULL_ART_ANIMATED_ID,
  VERSION_FULL_ART_ID,
  VERSION_PRISMATIC_ID,
} from "../../../apps/web/src/cards/versions.ts";
import CardCosmeticsView from "../../../apps/web/src/collection/CardCosmeticsView.tsx";
import { NO_FULL_ART_ID, collectionTestid as T } from "../../../apps/web/src/collection/collection.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { baseView, card, emptySide, resetIds, unit } from "../../../apps/web/src/test/fixtures.ts";

type CardDef = (typeof CATALOG)[string];

/** Ceaseless Void, a Mythic Unit: its Default draws Full art. */
const MYTHIC = "core-100";
/** Silly Silas, a Legendary Unit: its Default draws Extended art. */
const LEGENDARY = "core-052";
/** Felinor Fiender, a Legendary Unit offered Prismatic, worn at its first level. */
const PRISMATIC = "core-092";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 800 },
  { label: "phone", width: 390, height: 844 },
] as const;

const BOARD = '[data-testid="board"]';

/** Sub-pixel rounding between rects. */
const EPSILON = 0.5;

/** How long the faces may take to settle (`useFitText`, the lazy art). */
const SETTLE_TIMEOUT_MS = 10_000;

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return def;
}

function must<T extends Element>(found: T | null | undefined, what: string): T {
  if (found === null || found === undefined) throw new Error(`no ${what}`);
  return found;
}

function clean(): void {
  localStorage.clear();
  __resetSettingsForTests();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
}

/** Each of `parts` that `face` holds and draws has a box with an area, whole inside `face`. */
function expectPartsInside(face: Element, parts: readonly string[], what: string): void {
  const outer = face.getBoundingClientRect();
  expect(outer.width, `${what}: the face has a width`).to.be.greaterThan(0);
  for (const selector of parts) {
    for (const part of face.querySelectorAll<HTMLElement>(selector)) {
      if (getComputedStyle(part).display === "none" || getComputedStyle(part).visibility === "hidden") continue;
      const inner = part.getBoundingClientRect();
      if (inner.width <= 1 + EPSILON && inner.height <= 1 + EPSILON) continue; // a visually hidden word
      expect(inner.left, `${what}: ${selector} inside the face, left`).to.be.at.least(outer.left - EPSILON);
      expect(inner.right, `${what}: ${selector} inside the face, right`).to.be.at.most(outer.right + EPSILON);
      expect(inner.top, `${what}: ${selector} inside the face, top`).to.be.at.least(outer.top - EPSILON);
      expect(inner.bottom, `${what}: ${selector} inside the face, bottom`).to.be.at.most(outer.bottom + EPSILON);
    }
  }
}

/** The three cards in the viewer's hand. */
function handView() {
  resetIds();
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "mythic", defId: MYTHIC }),
        card({ instanceId: "legendary", defId: LEGENDARY }),
        card({ instanceId: "prismatic", defId: PRISMATIC }),
      ],
    }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
  });
}

/** The three cards as Units in the viewer's first three lanes. */
function boardView() {
  resetIds();
  return baseView({
    you: emptySide("p1", {
      units: [unit("p1", { defId: MYTHIC }), unit("p1", { defId: LEGENDARY }), unit("p1", { defId: PRISMATIC }), null, null],
    }),
  });
}

function Table({ view }: { view: ReturnType<typeof baseView> }) {
  return (
    <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={() => undefined} wearCosmetics />
      </div>
    </CatalogContext.Provider>
  );
}

/** `id`'s Collection view, owning `levels` on it alone. */
function View({ id, levels }: { id: string; levels: readonly string[] }) {
  return (
    <div className="tavern app-shell">
      <FullArtOwned owned={new Map([[id, new Set(levels)]])}>
        <CardCosmeticsView def={defOf(id)} owned={false} onClose={() => undefined} />
      </FullArtOwned>
    </div>
  );
}

/** The parts a face draws that must stay inside it. */
const PARTS = [".cost-gem", ".cf-art-frame", ".card-name", ".card-text", ".cf-stats"] as const;

describe("R3343001 R3343002 the default looks in hand, on the board and in the Collection", () => {
  beforeEach(() => {
    clean();
    writeCardLevel(PRISMATIC, VERSION_PRISMATIC_ID);
  });

  after(clean);

  for (const viewport of VIEWPORTS) {
    it(`R3343001 in the viewer's own hand at ${viewport.label}: the Mythic bare in full art, the Legendary in Extended art with its words, Prismatic in full art`, () => {
      cy.viewport(viewport.width, viewport.height);
      cy.mount(<Table view={handView()} />);
      cy.get('[data-testid="hand-you"]', { timeout: SETTLE_TIMEOUT_MS }).should(($hand) => {
        const hand = must($hand.get(0), "hand");
        const face = (instanceId: string): HTMLElement => must(hand.querySelector<HTMLElement>(`[data-testid="hand-card-${instanceId}"] > .cf`), `face of ${instanceId}`);

        const mythic = face("mythic");
        expect(mythic.getAttribute("data-full-art"), "the Mythic").to.eq("true");
        expect(mythic.getAttribute("data-full-art-hand"), "the Mythic is bare in hand").to.eq("true");
        expect(mythic.getAttribute("data-version"), "the Mythic").to.eq(VERSION_FULL_ART_ID);
        expect(mythic.hasAttribute("data-level"), "the Mythic names no level").to.eq(false);
        expectPartsInside(mythic, PARTS, MYTHIC);

        const legendary = face("legendary");
        expect(legendary.hasAttribute("data-full-art"), "the Legendary").to.eq(false);
        expect(legendary.getAttribute("data-version"), "the Legendary").to.eq(VERSION_EXTENDED_ART_ID);
        expect(legendary.querySelector(".card-text"), "the Legendary keeps its words").to.not.eq(null);
        expect(must(legendary.querySelector(".card-name"), "the Legendary's name").getBoundingClientRect().height, "its name draws").to.be.greaterThan(1);
        expect(must(legendary.querySelector(".cf-art"), "the Legendary's art").classList.contains("cf-art--full"), "its picture runs past its window").to.eq(true);
        expectPartsInside(legendary, PARTS, LEGENDARY);

        const prismatic = face("prismatic");
        expect(prismatic.getAttribute("data-full-art"), "Prismatic").to.eq("true");
        expect(prismatic.getAttribute("data-version"), "Prismatic").to.eq(VERSION_PRISMATIC_ID);
        expect(prismatic.querySelector(".cf-art")?.getAttribute("data-art-picture"), "Prismatic's chrome").to.eq("prismatic");
        expectPartsInside(prismatic, PARTS, PRISMATIC);
      });
      cy.get('[data-testid="hand-you"]').screenshot(`R3343001-default-looks-hand-${viewport.label}`, { overwrite: true });
    });

    it(`R3343001 on the board at ${viewport.label}: the Mythic and Prismatic full art, the Legendary keeping its window`, () => {
      cy.viewport(viewport.width, viewport.height);
      cy.mount(<Table view={boardView()} />);
      cy.get(`${BOARD} .field .card[data-face="minion"]`, { timeout: SETTLE_TIMEOUT_MS }).should("have.length", 3);
      cy.document().should((doc) => {
        const minion = (id: string): HTMLElement =>
          must(doc.querySelector<HTMLElement>(`${BOARD} .field .card[data-face="minion"][data-def-id="${id}"] > .cf`), `the minion of ${id}`);
        for (const [id, version] of [
          [MYTHIC, VERSION_FULL_ART_ID],
          [PRISMATIC, VERSION_PRISMATIC_ID],
        ] as const) {
          const cf = minion(id);
          expect(cf.getAttribute("data-full-art"), id).to.eq("true");
          expect(cf.getAttribute("data-version"), id).to.eq(version);
          const art = must(cf.querySelector<HTMLElement>(".cf-portrait"), `${id}'s art`).getBoundingClientRect();
          const name = must(cf.querySelector<HTMLElement>(".card-name"), `${id}'s name`).getBoundingClientRect();
          expect(name.top, `${id}: the name's band below the art`).to.be.at.least(art.top);
          expectPartsInside(cf, [".cf-portrait", ".card-name", ".cost-gem"], id);
        }
        const windowed = minion(LEGENDARY);
        expect(windowed.hasAttribute("data-full-art"), LEGENDARY).to.eq(false);
        expect(windowed.querySelector(".cf-art--oval"), `${LEGENDARY} keeps its window`).to.not.eq(null);
        expectPartsInside(windowed, [".cf-portrait", ".card-name", ".cost-gem"], LEGENDARY);
      });
      cy.get(BOARD).screenshot(`R3343001-default-looks-board-${viewport.label}`, { overwrite: true });
    });

    it(`R3343002 in the Collection at ${viewport.label}: each card's Default rung named and pictured by its look, its ladder starting above it`, () => {
      cy.viewport(viewport.width, viewport.height);
      for (const [id, levels, version, ladder, next] of [
        [MYTHIC, [], VERSION_FULL_ART_ID, "full-art", VERSION_FULL_ART_ANIMATED_ID],
        [LEGENDARY, [], VERSION_EXTENDED_ART_ID, "extended-art", `${VERSION_EXTENDED_ART_ID}-animated`],
        [PRISMATIC, [VERSION_PRISMATIC_ID], VERSION_PRISMATIC_ID, "prismatic", `${VERSION_PRISMATIC_ID}-animated`],
      ] as const) {
        cy.mount(<View id={id} levels={levels} />);
        cy.get(`[data-testid="${T.rung(NO_FULL_ART_ID)}"]`, { timeout: SETTLE_TIMEOUT_MS }).should("exist");
        cy.document().should((doc) => {
          const face = must(doc.querySelector<HTMLElement>(`[data-testid="${T.cardViewFace}"] .cf`), `${id}'s large face`);
          expect(face.getAttribute("data-version"), `${id}'s large face`).to.eq(version);
          expect(face.getAttribute("data-full-art"), `${id}'s large face`).to.eq(version === VERSION_EXTENDED_ART_ID ? null : "true");
          expectPartsInside(face, PARTS, id);
          const normal = must(doc.querySelector<HTMLElement>(`[data-testid="${T.rung(NO_FULL_ART_ID)}"]`), `${id}'s Default rung`);
          if (id !== PRISMATIC) {
            expect(normal.textContent ?? "", `${id}'s Default rung`).to.contain(version === VERSION_FULL_ART_ID ? "Full art" : "Extended art");
            expect(normal.querySelector(".collection-strip__picture--full-art"), `${id}'s Default rung pictures its look`).to.not.eq(null);
          }
          const section = must(doc.querySelector<HTMLElement>(`[data-testid="${T.ladder(ladder)}"]`), `${id}'s ${ladder} ladder`);
          const first = must(section.querySelector<HTMLElement>(".collection-rung"), `${id}'s ${ladder} first rung`);
          expect(first.dataset.level, `${id}'s ${ladder} first rung`).to.eq(id === PRISMATIC ? VERSION_PRISMATIC_ID : next);
          const nextRung = must(doc.querySelector<HTMLElement>(`[data-testid="${T.rung(next)}"]`), `${id}'s ${next}`);
          expect(nextRung.dataset.next, `${id}: ${next} is next`).to.eq("true");
        });
        cy.get(`[data-testid="${T.cardViewFace}"]`).screenshot(`R3343002-default-looks-collection-${id}-${viewport.label}`, { overwrite: true });
      }
    });
  }
});
