// R3046000: a full-art card on the board in a real browser. apps/web's vitest suite proves which cards print it, the
// DOM of a Unit's MinionFace and a face-up backrow card's compact CardFace against it, and the rules the stylesheets
// declare (cards/fullArtBoard.test.tsx); where the pieces land, which only a layout and style engine can say, is
// measured here: the art fills the face down to the name's band, the band lies at the foot, the gems stay over the
// art and clear of the name, a finish's band draws over the art and under the parts, a border shows round the art,
// Taunt wears its steel ring, and a card that is not full art keeps its window. Every measurement sits inside
// `.should()`, so it retries while `useFitText` settles.

import type { ReactNode } from "react";

import { CATALOG } from "@jackioh/cards";
import type { Keyword } from "@jackioh/shared";
import { CardFace, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { FULL_ART_FINISH_OPACITY } from "../../../apps/web/src/cards/constants.ts";
import { CARD_SLOT_KINDS, cosmeticsOf, resetCosmeticsForTests, writeCardPick } from "../../../apps/web/src/cards/cosmetics.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { faceUpBackrow, fullBoardView, unit } from "../../../apps/web/src/test/fixtures.ts";

type CardDef = (typeof CATALOG)[string];

/** Gary the Gambler, a full-art Unit; Bread and Butter, a full-art Field Trap; the plain cards beside them. */
const FULL_ART_UNIT = "core-004";
const FULL_ART_BACKROW = "core-018";
const PLAIN_UNIT = "core-002";
const PLAIN_BACKROW = "core-041";

/** A board minion's boxes (keyword-visuals.cy.tsx's three), and a face-up backrow card's: a phone's, a desktop's and a large one. */
const MINION_SIZES = [
  { label: "phone", width: 43, height: 60 },
  { label: "tablet", width: 58, height: 80 },
  { label: "large", width: 81, height: 113 },
] as const;
const BACKROW_SIZES = [
  { label: "phone", width: 43, height: 60 },
  { label: "desktop", width: 71, height: 100 },
  { label: "large", width: 120, height: 168 },
] as const;

/** The name band of a face-up backrow card is this share of the face's height (cards.css `--cf-band`, 24cqh). */
const BACKROW_BAND_SHARE = 0.24;

/** How far two edges that should meet may be apart: a sub-pixel rounding, and the panel's own rim. */
const EDGE_TOLERANCE_PX = 1.5;

/** The nine finishes' ids; the band of one of them is enough to place it. */
const FINISH = cosmeticsOf("finish").find((cosmetic) => !cosmetic.default)?.id;
const BORDER = cosmeticsOf("border").find((cosmetic) => !cosmetic.default)?.id;

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

function clearPicks(): void {
  for (const id of [FULL_ART_UNIT, FULL_ART_BACKROW, PLAIN_UNIT, PLAIN_BACKROW]) {
    for (const kind of CARD_SLOT_KINDS) writeCardPick(id, kind, null);
  }
}

/** A board minion in a box of `width` by `height`, worn as the viewer's own. */
function Minion({
  id,
  width,
  height,
  keywords = [],
}: {
  id: string;
  width: number;
  height: number;
  keywords?: Keyword[];
}) {
  const u = unit("p1", { defId: id, attack: 3, health: 4, maxHealth: 4, keywords });
  const face = faceModel({
    defId: id,
    def: defOf(id),
    radiant: false,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
  });
  return (
    <div data-which={`${id}-${width}`} style={{ width, height, flex: "none", position: "relative" }}>
      <MinionFace face={face} unit={u} worn />
    </div>
  );
}

/** A face-up backrow card in a box of `width` by `height`, worn as the viewer's own. */
function Backrow({ id, width, height }: { id: string; width: number; height: number }) {
  return (
    <div data-which={`${id}-${width}`} style={{ width, height, flex: "none", position: "relative" }}>
      <CardFace face={faceModel({ defId: id, def: defOf(id), radiant: false })} layout="compact" worn />
    </div>
  );
}

function Row({ children }: { children: ReactNode }) {
  return <div style={{ display: "flex", alignItems: "flex-start", gap: 12, padding: 12, background: "#1b1f2b" }}>{children}</div>;
}

function cfIn(doc: Document, which: string): HTMLElement {
  const cf = doc.querySelector<HTMLElement>(`[data-which="${which}"] > .cf`);
  if (cf === null) throw new Error(`no face for ${which}`);
  return cf;
}

function part(cf: HTMLElement, selector: string): HTMLElement {
  const found = cf.querySelector<HTMLElement>(selector);
  if (found === null) throw new Error(`no ${selector}`);
  return found;
}

function rectOf(cf: HTMLElement, selector: string): DOMRect {
  return part(cf, selector).getBoundingClientRect();
}

/** A box's edges in its card's own frame. */
type Edges = { top: number; bottom: number; left: number; right: number; width: number; height: number };

/**
 * `box` as its card stands upright: a DEF card is turned a quarter clockwise (Card.tsx's inline `rotate(90deg)`), so on
 * the screen its top is its right edge and its foot its left, and turned back its art and its name line up as an ATK
 * card's do. Only differences between edges are compared, so the frame's origin does not matter.
 */
function upright(box: DOMRect, turned: boolean): Edges {
  if (!turned) return box;
  return { top: -box.right, bottom: -box.left, left: box.top, right: box.bottom, width: box.height, height: box.width };
}

/** The art and the name's band of a full-art face line up: the art fills the panel down to the band, which lies at the foot. */
function expectArtAboveBand(cf: HTMLElement, artSelector: string, where: string): void {
  const panel = rectOf(cf, ".cf-scale");
  const art = rectOf(cf, artSelector);
  const name = rectOf(cf, ".card-name");
  expect(art.left, `${where}: the art starts at the panel's left`).to.be.closeTo(panel.left, EDGE_TOLERANCE_PX);
  expect(art.right, `${where}: the art reaches the panel's right`).to.be.closeTo(panel.right, EDGE_TOLERANCE_PX);
  expect(art.top, `${where}: the art starts at the panel's top`).to.be.closeTo(panel.top, EDGE_TOLERANCE_PX);
  expect(art.bottom, `${where}: the art ends where the name's band begins`).to.be.closeTo(name.top, EDGE_TOLERANCE_PX);
  expect(name.left, `${where}: the band spans the panel`).to.be.closeTo(panel.left, EDGE_TOLERANCE_PX);
  expect(name.right, `${where}: the band spans the panel`).to.be.closeTo(panel.right, EDGE_TOLERANCE_PX);
  expect(name.bottom, `${where}: the band lies at the foot`).to.be.closeTo(panel.bottom, EDGE_TOLERANCE_PX);
  expect(name.height, `${where}: the band has a height`).to.be.greaterThan(8);
  expect(name.top, `${where}: the name is below the art, not over it`).to.be.at.least(art.bottom - EDGE_TOLERANCE_PX);
}

describe("R3046000 a full-art Unit on the board", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    clearPicks();
    cy.viewport(500, 300);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    clearPicks();
  });

  for (const size of MINION_SIZES) {
    it(`R3046000 the art fills the face down to the name's band and the gems ride the art's corners, at ${size.label} ${String(size.width)}x${String(size.height)}`, () => {
      cy.mount(
        <Row>
          <Minion id={FULL_ART_UNIT} width={size.width} height={size.height} />
          <Minion id={PLAIN_UNIT} width={size.width} height={size.height} />
        </Row>,
      );
      cy.document().should((doc) => {
        const cf = cfIn(doc, `${FULL_ART_UNIT}-${String(size.width)}`);
        expect(cf.getAttribute("data-full-art"), "the Unit is full art").to.eq("true");
        expectArtAboveBand(cf, ".cf-portrait", `full art at ${size.label}`);

        const art = part(cf, ".cf-portrait > .cf-art");
        expect(art.classList.contains("cf-art--full"), "the art is the full face's").to.eq(true);
        expect(getComputedStyle(art).backgroundImage, "the art is painted").to.contain("data:image");
        expect(getComputedStyle(art).borderTopLeftRadius, "no oval").to.eq("0px");

        const portrait = rectOf(cf, ".cf-portrait");
        const name = rectOf(cf, ".card-name");
        const panel = rectOf(cf, ".cf-scale");
        for (const selector of [".stat-attack", ".stat-health"]) {
          const gem = rectOf(cf, selector);
          expect(gem.top, `${selector} is over the art`).to.be.at.least(portrait.top - EDGE_TOLERANCE_PX);
          expect(gem.bottom, `${selector} stands on the band, clear of the name`).to.be.at.most(name.top + EDGE_TOLERANCE_PX);
          expect(gem.left, `${selector} is inside the panel`).to.be.at.least(panel.left - EDGE_TOLERANCE_PX);
          expect(gem.right, `${selector} is inside the panel`).to.be.at.most(panel.right + EDGE_TOLERANCE_PX);
        }
        const cost = rectOf(cf, ".cost-gem");
        expect(cost.top, "the cost gem is over the art").to.be.at.least(portrait.top - EDGE_TOLERANCE_PX);
        expect(cost.left, "the cost gem keeps the top left").to.be.lessThan(panel.left + panel.width / 2);
        expect(cost.bottom, "the cost gem is clear of the name").to.be.at.most(name.top);

        // The control: a Unit that prints no full art draws its art up to the frame (#3341), not an oval in a box: the
        // picture fills the panel's width and starts at its top, square, and ends half a gem above its foot, so the gems
        // straddle its edge and the name's plate lies on it.
        const plain = cfIn(doc, `${PLAIN_UNIT}-${String(size.width)}`);
        expect(plain.hasAttribute("data-full-art"), "the plain Unit is not full art").to.eq(false);
        const plainArt = rectOf(plain, ".cf-portrait");
        const plainPanel = rectOf(plain, ".cf-scale");
        const plainName = rectOf(plain, ".card-name");
        expect(plainArt.width, "the picture fills the panel's sides").to.be.closeTo(plainPanel.width, EDGE_TOLERANCE_PX);
        expect(plainArt.top, "the picture starts at the panel's top").to.be.closeTo(plainPanel.top, EDGE_TOLERANCE_PX);
        expect(getComputedStyle(part(plain, ".cf-portrait > .cf-art")).borderTopLeftRadius, "square").to.eq("0px");
        expect(plain.querySelector(".cf-portrait > .cf-art")?.classList.contains("cf-art--frame"), "the frame shape").to.eq(true);
        expect(plainName.top, "the plate lies on the picture").to.be.at.least(plainArt.top);
        expect(plainName.bottom, "the plate ends on the picture").to.be.at.most(plainArt.bottom + EDGE_TOLERANCE_PX);
        for (const selector of [".stat-attack", ".stat-health"]) {
          const gem = rectOf(plain, selector);
          expect(gem.top, `${selector} straddles the picture's foot: it begins over it`).to.be.lessThan(plainArt.bottom);
          expect(gem.bottom, `${selector} straddles the picture's foot: it ends under it`).to.be.at.least(plainArt.bottom - EDGE_TOLERANCE_PX);
        }
      });
    });
  }

  it("R3046000 Taunt wears a steel ring over the art, its shield not drawn; a Divine Shield glows along the art's edge", () => {
    cy.mount(
      <Row>
        <Minion id={FULL_ART_UNIT} width={81} height={113} keywords={[{ kind: "Taunt" }, { kind: "Divine Shield" }]} />
      </Row>,
    );
    cy.document().should((doc) => {
      const cf = cfIn(doc, `${FULL_ART_UNIT}-81`);
      const ring = getComputedStyle(part(cf, ".cf-portrait"), "::after");
      expect(ring.content, "the scrim and ring layer is drawn").to.not.eq("none");
      expect(ring.boxShadow, "the steel ring").to.contain("inset");
      expect(getComputedStyle(part(cf, '[data-keyword-fx="Taunt"]')).display, "the shield behind the art is not drawn").to.eq("none");
      const bubble = rectOf(cf, ".shield-icon");
      const portrait = rectOf(cf, ".cf-portrait");
      expect(bubble.left, "the bubble lies along the art's edge").to.be.closeTo(portrait.left, EDGE_TOLERANCE_PX);
      expect(bubble.right).to.be.closeTo(portrait.right, EDGE_TOLERANCE_PX);
      expect(bubble.bottom, "above the name's band").to.be.closeTo(portrait.bottom, EDGE_TOLERANCE_PX);
    });
  });

  it("R3046000 a finish draws its band over the art and under the parts; a border shows round the art with no ring", () => {
    if (FINISH === undefined || BORDER === undefined) throw new Error("the registry has no finish or no border");
    cy.then(() => {
      writeCardPick(FULL_ART_UNIT, "finish", FINISH);
      writeCardPick(FULL_ART_UNIT, "border", BORDER);
    });
    cy.mount(
      <Row>
        <Minion id={FULL_ART_UNIT} width={81} height={113} />
      </Row>,
    );
    cy.document().should((doc) => {
      const cf = cfIn(doc, `${FULL_ART_UNIT}-81`);
      expect(cf.getAttribute("data-finish")).to.eq(FINISH);
      expect(cf.getAttribute("data-frame")).to.eq(BORDER);
      const band = getComputedStyle(part(cf, ".cf-scale"), "::before");
      expect(band.content, "the finish's band draws").to.not.eq("none");
      expect(Number(band.opacity), "at most FULL_ART_FINISH_OPACITY").to.be.at.most(FULL_ART_FINISH_OPACITY);
      expect(band.zIndex, "over the art").to.eq("1");
      expect(band.maskImage, "fading toward the middle").to.contain("linear-gradient");
      expect(Number(getComputedStyle(part(cf, ".cf-portrait")).zIndex), "the art is under the band").to.be.lessThan(1);
      for (const selector of [".cost-gem", ".card-name", ".stats"]) {
        expect(Number(getComputedStyle(part(cf, selector)).zIndex), `${selector} is over the band`).to.be.greaterThan(1);
      }
      // The border is the face's edge, as on a full face, round a panel that is inset; no ring is laid over the panel.
      expect(getComputedStyle(cf).backgroundImage, "the border is the face's edge").to.not.eq("none");
      const panel = rectOf(cf, ".cf-scale");
      const face = cf.getBoundingClientRect();
      expect(panel.left - face.left, "the edge shows round the art").to.be.greaterThan(0.4);
      expect(getComputedStyle(part(cf, ".cf-scale")).maskImage, "no ring is cut from the panel").to.not.contain("linear-gradient");
    });
  });
});

describe("R3046000 a full-art face-up backrow card", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    clearPicks();
    cy.viewport(500, 300);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    clearPicks();
  });

  for (const size of BACKROW_SIZES) {
    it(`R3046000 the art fills the face down to the name's band, at ${size.label} ${String(size.width)}x${String(size.height)}`, () => {
      cy.mount(
        <Row>
          <Backrow id={FULL_ART_BACKROW} width={size.width} height={size.height} />
          <Backrow id={PLAIN_BACKROW} width={size.width} height={size.height} />
        </Row>,
      );
      cy.document().should((doc) => {
        const cf = cfIn(doc, `${FULL_ART_BACKROW}-${String(size.width)}`);
        expect(cf.getAttribute("data-layout")).to.eq("compact");
        expect(cf.getAttribute("data-full-art"), "the card is full art").to.eq("true");
        expectArtAboveBand(cf, ".cf-art-frame", `backrow full art at ${size.label}`);
        const panel = rectOf(cf, ".cf-scale");
        expect(rectOf(cf, ".card-name").height / panel.height, "the band is its share of the face").to.be.closeTo(BACKROW_BAND_SHARE, 0.03);
        expect(getComputedStyle(part(cf, ".cf-art-frame > .cf-art")).backgroundImage, "the art is painted").to.contain("data:image");
        expect(cf.querySelector(".card-type"), "no type line").to.eq(null);

        const cost = rectOf(cf, ".cost-gem");
        expect(cost.left, "the cost gem keeps the top left").to.be.lessThan(panel.left + panel.width / 2);
        expect(cost.top, "the cost gem is over the art").to.be.lessThan(panel.top + panel.height / 3);
        const gem = cf.querySelector<HTMLElement>(".cf-gem");
        if (gem !== null) {
          const box = gem.getBoundingClientRect();
          expect(box.left, "the rarity gem takes the top right").to.be.greaterThan(panel.left + panel.width / 2);
          expect(box.bottom, "above the name's band").to.be.at.most(rectOf(cf, ".card-name").top);
        }

        // The control: a card that prints no full art keeps its window, with its type line below the name.
        const plain = cfIn(doc, `${PLAIN_BACKROW}-${String(size.width)}`);
        expect(plain.hasAttribute("data-full-art")).to.eq(false);
        expect(rectOf(plain, ".cf-art-frame").width, "the window leaves the panel's sides").to.be.lessThan(rectOf(plain, ".cf-scale").width * 0.95);
      });
    });
  }

  it("R3046000 a finish draws its band over the backrow card's art and under its parts", () => {
    if (FINISH === undefined) throw new Error("the registry has no finish");
    cy.then(() => {
      writeCardPick(FULL_ART_BACKROW, "finish", FINISH);
    });
    cy.mount(
      <Row>
        <Backrow id={FULL_ART_BACKROW} width={100} height={140} />
      </Row>,
    );
    cy.document().should((doc) => {
      const cf = cfIn(doc, `${FULL_ART_BACKROW}-100`);
      const band = getComputedStyle(part(cf, ".cf-scale"), "::before");
      expect(band.content, "the finish's band draws").to.not.eq("none");
      expect(Number(band.opacity), "at most FULL_ART_FINISH_OPACITY").to.be.at.most(FULL_ART_FINISH_OPACITY);
      expect(band.zIndex).to.eq("1");
      for (const selector of [".cost-gem", ".card-name"]) {
        expect(Number(getComputedStyle(part(cf, selector)).zIndex), `${selector} is over the band`).to.be.greaterThan(1);
      }
    });
  });
});

describe("R3046000 the real board", () => {
  const BOARD = '[data-testid="board"]';
  const VIEWPORTS = [
    { label: "desktop", width: 1280, height: 720 },
    { label: "phone", width: 390, height: 844 },
  ] as const;

  /** `fullBoardView()` with Bread and Butter face-up in lane 1 of the viewer's backrow. */
  function view() {
    const base = fullBoardView();
    return { ...base, you: { ...base.you, backrow: [faceUpBackrow("p1", { defId: FULL_ART_BACKROW, type: "Field Trap" }), ...base.you.backrow.slice(1)] } };
  }

  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    clearPicks();
  });

  for (const viewport of VIEWPORTS) {
    it(`R3046000 every full-art Unit and backrow card on the board has its art above its name and its name at its foot at ${viewport.label}`, () => {
      cy.viewport(viewport.width, viewport.height);
      cy.mount(
        <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
          <div className="app-shell app-shell--wide">
            <Game view={view()} legal={[]} onAction={() => undefined} />
          </div>
        </CatalogContext.Provider>,
      );
      cy.get(`${BOARD} .field .card`).should("have.length", 20);
      cy.document().should((doc) => {
        const units = [...doc.querySelectorAll<HTMLElement>(`${BOARD} .field .card[data-face="minion"] > .cf[data-full-art="true"]`)];
        // Gary and Tempo Timmy on your side, Mr. Vanilla and Jlockeed Shredder-10 on the other's.
        expect(units.length, "the board's full-art Units").to.be.at.least(4);
        // Jlockeed Shredder-10 stands in DEF, turned sideways: it is measured upright, in its own frame.
        expect(units.some((cf) => cf.parentElement?.getAttribute("data-position") === "DEF"), "a full-art Unit in DEF").to.eq(true);
        for (const cf of units) {
          const turned = cf.parentElement?.getAttribute("data-position") === "DEF";
          const where = `${cf.parentElement?.getAttribute("data-def-id") ?? "?"}${turned ? " in DEF" : ""} at ${viewport.label}`;
          const artBox = cf.querySelector<HTMLElement>(".cf-portrait")?.getBoundingClientRect();
          const nameBox = cf.querySelector<HTMLElement>(".card-name")?.getBoundingClientRect();
          const panelBox = cf.querySelector<HTMLElement>(".cf-scale")?.getBoundingClientRect();
          if (artBox === undefined || nameBox === undefined || panelBox === undefined) throw new Error(`${where}: no art, name or panel`);
          const art = upright(artBox, turned);
          const name = upright(nameBox, turned);
          const panel = upright(panelBox, turned);
          expect(art.bottom, `${where}: the art ends where the name's band begins`).to.be.closeTo(name.top, EDGE_TOLERANCE_PX);
          expect(name.bottom, `${where}: the band lies at the foot`).to.be.closeTo(panel.bottom, EDGE_TOLERANCE_PX);
          expect(art.width, `${where}: the art fills the panel`).to.be.closeTo(panel.width, EDGE_TOLERANCE_PX);
          for (const stat of cf.querySelectorAll<HTMLElement>(".stat-attack, .stat-health")) {
            expect(upright(stat.getBoundingClientRect(), turned).bottom, `${where}: a stat is clear of the name`).to.be.at.most(name.top + EDGE_TOLERANCE_PX);
          }
        }
        const backrow = doc.querySelector<HTMLElement>(`${BOARD} .field .card[data-face="compact"] > .cf[data-full-art="true"]`);
        expect(backrow, `the full-art backrow card at ${viewport.label}`).to.not.eq(null);
        if (backrow !== null) {
          const frame = rectOf(backrow, ".cf-art-frame");
          const name = rectOf(backrow, ".card-name");
          expect(frame.bottom, `the backrow art ends where its name's band begins at ${viewport.label}`).to.be.closeTo(name.top, EDGE_TOLERANCE_PX);
          expect(name.bottom, "the band lies at the foot").to.be.closeTo(rectOf(backrow, ".cf-scale").bottom, EDGE_TOLERANCE_PX);
        }
        // The cards that print none draw their art up to the frame (#3341).
        const plain = doc.querySelectorAll(`${BOARD} .field .card[data-face="minion"] > .cf:not([data-full-art]) .cf-art--frame`);
        expect(plain.length, "plain Units draw their art up to the frame").to.be.at.least(1);
      });
    });
  }
});
