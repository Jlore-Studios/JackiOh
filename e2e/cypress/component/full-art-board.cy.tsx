// R3046000: a full-art card on the board in a real browser. apps/web's vitest suite proves which cards print it, the
// DOM of a Unit's MinionFace and a face-up backrow card's compact CardFace against it, and the rules the stylesheets
// declare (cards/fullArtBoard.test.tsx); where the pieces land, which only a layout and style engine can say, is
// measured here: the art fills the face down to the name's band, the band lies at the foot, the gems stay over the
// art and clear of the name, a finish's band draws over the art and under the parts, a border shows round the art,
// Taunt wears its steel ring, and a card that is not full art keeps its window. Every measurement sits inside
// `.should()`, so it retries while `useFitText` settles.
//
// R3339000 amends it: a card in play that is art first (a full-art-shaped level, or Legendary or Mythic) draws no name
// band, its art edge to edge, and shows its name on a slim hover plate (here: a keyboard focus on its card, the same
// rule as a hover); the band is the face of a card whose name shows (Card settings, "Names on the board": Always), so
// R3046000's measurements below run with that setting.

import type { ReactNode } from "react";

import { CATALOG } from "@jackioh/cards";
import type { Keyword } from "@jackioh/shared";
import { CardFace, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { FULL_ART_FINISH_OPACITY } from "../../../apps/web/src/cards/constants.ts";
import { CARD_SLOT_KINDS, cosmeticsOf, resetCosmeticsForTests, writeCardPick } from "../../../apps/web/src/cards/cosmetics.ts";
import { isFullArtCard } from "../../../apps/web/src/cards/fullArt.ts";
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

/** R3339000: a Legendary or Mythic Unit and Spell that print no full art (art first by their rarity alone), and a Common Unit. */
const LEGENDARY_UNIT = Object.values(CATALOG).find((def) => (def.rarity === "Legendary" || def.rarity === "Mythic") && def.type === "Unit" && !isFullArtCard(def.id))?.id ?? "";
const LEGENDARY_BACKROW = Object.values(CATALOG).find((def) => (def.rarity === "Legendary" || def.rarity === "Mythic") && def.type !== "Unit" && !isFullArtCard(def.id))?.id ?? "";

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
  for (const id of [FULL_ART_UNIT, FULL_ART_BACKROW, PLAIN_UNIT, PLAIN_BACKROW, LEGENDARY_UNIT, LEGENDARY_BACKROW]) {
    for (const kind of CARD_SLOT_KINDS) writeCardPick(id, kind, null);
  }
}

/**
 * The box a face stands in. With `host`, it is a focusable `.card`, as Card.tsx's root is (its look stripped to a bare
 * box), so `:focus-within` reveals an art-first face's hover plate as the board's keyboard focus does.
 */
function Box({ which, width, height, host, children }: { which: string; width: number; height: number; host: boolean; children: ReactNode }) {
  if (!host) {
    return (
      <div data-which={which} style={{ width, height, flex: "none", position: "relative" }}>
        {children}
      </div>
    );
  }
  return (
    <div
      className="card"
      data-which={which}
      tabIndex={0}
      style={{ width, height, flex: "none", position: "relative", display: "block", maxWidth: "none", padding: 0, border: 0, background: "none", boxShadow: "none", overflow: "visible" }}
    >
      {children}
    </div>
  );
}

/** A board minion in a box of `width` by `height`, worn as the viewer's own. */
function Minion({
  id,
  width,
  height,
  keywords = [],
  host = false,
}: {
  id: string;
  width: number;
  height: number;
  keywords?: Keyword[];
  host?: boolean;
}) {
  const u = unit("p1", { defId: id, attack: 3, health: 4, maxHealth: 4, keywords });
  const face = faceModel({
    defId: id,
    def: defOf(id),
    radiant: false,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
  });
  return (
    <Box which={`${id}-${width}`} width={width} height={height} host={host}>
      <MinionFace face={face} unit={u} worn />
    </Box>
  );
}

/** A face-up backrow card in a box of `width` by `height`, worn as the viewer's own: a card in play on the board (`onBoard`). */
function Backrow({ id, width, height, host = false }: { id: string; width: number; height: number; host?: boolean }) {
  return (
    <Box which={`${id}-${width}`} width={width} height={height} host={host}>
      <CardFace face={faceModel({ defId: id, def: defOf(id), radiant: false })} layout="compact" worn onBoard />
    </Box>
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
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, boardNames: "always" });
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
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, boardNames: "always" });
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
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, boardNames: "always" });
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

/** How far a plate that fades in must have come after a focus: its transition is `--motion-fast`, so the retry covers it. */
const FOCUS_PLATE_OPACITY = "1";

/**
 * The bare `.card` host of a face in a box (`Box`), focused: the keyboard's way to what a hover does. A headless browser
 * does not hold the page's focus, so `:focus-within` would never match: the browser is asked to behave as focused.
 */
function focusHost(which: string): void {
  cy.then(() => Cypress.automation("remote:debugger:protocol", { command: "Emulation.setFocusEmulationEnabled", params: { enabled: true } }));
  cy.get(`[data-which="${which}"]`).focus();
}

describe("R3339000 an art-first Unit on the board", () => {
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
    for (const id of [FULL_ART_UNIT, LEGENDARY_UNIT]) {
      it(`R3339000 ${id}'s art fills the whole face with no name band, and its name shows on a slim plate on focus, at ${size.label} ${String(size.width)}x${String(size.height)}`, () => {
        const which = `${id}-${String(size.width)}`;
        cy.mount(
          <Row>
            <Minion id={id} width={size.width} height={size.height} host />
          </Row>,
        );
        cy.document().should((doc) => {
          const cf = cfIn(doc, which);
          expect(cf.getAttribute("data-art-first"), "the Unit is drawn art first").to.eq("true");
          expect(cf.querySelector(".card-name:not(.card-name-hover)"), "no static name").to.eq(null);
          const panel = rectOf(cf, ".cf-scale");
          const art = rectOf(cf, ".cf-portrait");
          expect(art.left, "the art starts at the panel's left").to.be.closeTo(panel.left, EDGE_TOLERANCE_PX);
          expect(art.right, "the art reaches the panel's right").to.be.closeTo(panel.right, EDGE_TOLERANCE_PX);
          expect(art.top, "the art starts at the panel's top").to.be.closeTo(panel.top, EDGE_TOLERANCE_PX);
          expect(art.bottom, "the art runs to the panel's foot: no band").to.be.closeTo(panel.bottom, EDGE_TOLERANCE_PX);
          expect(getComputedStyle(part(cf, ".cf-portrait > .cf-art")).borderTopLeftRadius, "no oval").to.eq("0px");
          expect(getComputedStyle(part(cf, ".card-name-hover")).opacity, "the plate is clear until the card is hovered or focused").to.eq("0");
          for (const selector of [".stat-attack", ".stat-health", ".cost-gem"]) {
            const gem = rectOf(cf, selector);
            expect(gem.left, `${selector} is inside the panel`).to.be.at.least(panel.left - EDGE_TOLERANCE_PX);
            expect(gem.right, `${selector} is inside the panel`).to.be.at.most(panel.right + EDGE_TOLERANCE_PX);
            expect(gem.bottom, `${selector} is inside the panel`).to.be.at.most(panel.bottom + EDGE_TOLERANCE_PX);
          }
        });
        focusHost(which);
        cy.document().should((doc) => {
          const cf = cfIn(doc, which);
          const plate = part(cf, ".card-name-hover");
          expect(getComputedStyle(plate).opacity, "a focused card shows its name").to.eq(FOCUS_PLATE_OPACITY);
          const box = plate.getBoundingClientRect();
          const panel = rectOf(cf, ".cf-scale");
          expect(box.height, "the plate has a height").to.be.greaterThan(8);
          expect(box.height, "the plate is slim").to.be.lessThan(panel.height / 3);
          expect(box.left, "the plate is inside the panel").to.be.at.least(panel.left - EDGE_TOLERANCE_PX);
          expect(box.right, "the plate is inside the panel").to.be.at.most(panel.right + EDGE_TOLERANCE_PX);
          expect(box.bottom, "the plate lies over the art's foot").to.be.at.most(panel.bottom + EDGE_TOLERANCE_PX);
          expect(box.top, "the plate lies over the art").to.be.at.least(panel.top);
          expect(plate.textContent, "the plate says the card's name").to.eq(defOf(id).name);
        });
      });
    }
  }

  for (const size of MINION_SIZES) {
    it(`R3339000 a Common Unit keeps a name plate, smaller and translucent over the art's foot, at ${size.label} ${String(size.width)}x${String(size.height)}`, () => {
      cy.mount(
        <Row>
          <Minion id={PLAIN_UNIT} width={size.width} height={size.height} />
        </Row>,
      );
      cy.document().should((doc) => {
        const cf = cfIn(doc, `${PLAIN_UNIT}-${String(size.width)}`);
        expect(cf.hasAttribute("data-art-first"), "a Common is not art first").to.eq(false);
        const name = part(cf, ".card-name");
        expect(name.classList.contains("card-name-compact-translucent"), "the small plate").to.eq(true);
        const box = name.getBoundingClientRect();
        const art = rectOf(cf, ".cf-portrait");
        expect(box.height, "the plate has a height").to.be.greaterThan(6);
        expect(box.top, "the plate lies on the art").to.be.at.least(art.top);
        expect(box.bottom, "the plate ends on the art (#3341)").to.be.at.most(art.bottom + EDGE_TOLERANCE_PX);
        const background = getComputedStyle(name).backgroundColor;
        expect(background, "the plate is translucent").to.match(/rgba\(/);
        expect(art.width, "the art fills the panel's sides (#3341)").to.be.closeTo(rectOf(cf, ".cf-scale").width, EDGE_TOLERANCE_PX);
      });
    });
  }
});

describe("R3339000 an art-first face-up backrow card", () => {
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
    for (const id of [FULL_ART_BACKROW, LEGENDARY_BACKROW]) {
      it(`R3339000 ${id}'s art fills the whole face with no band or type line, and its name shows on focus, at ${size.label} ${String(size.width)}x${String(size.height)}`, () => {
        const which = `${id}-${String(size.width)}`;
        cy.mount(
          <Row>
            <Backrow id={id} width={size.width} height={size.height} host />
          </Row>,
        );
        cy.document().should((doc) => {
          const cf = cfIn(doc, which);
          expect(cf.getAttribute("data-art-first"), "the card is drawn art first").to.eq("true");
          expect(cf.querySelector(".card-name:not(.card-name-hover)"), "no static name").to.eq(null);
          expect(cf.querySelector(".card-type"), "no type line").to.eq(null);
          const panel = rectOf(cf, ".cf-scale");
          const frame = rectOf(cf, ".cf-art-frame");
          expect(frame.top, "the art starts at the panel's top").to.be.closeTo(panel.top, EDGE_TOLERANCE_PX);
          expect(frame.bottom, "the art runs to the panel's foot: no band").to.be.closeTo(panel.bottom, EDGE_TOLERANCE_PX);
          expect(frame.left).to.be.closeTo(panel.left, EDGE_TOLERANCE_PX);
          expect(frame.right).to.be.closeTo(panel.right, EDGE_TOLERANCE_PX);
          expect(getComputedStyle(part(cf, ".cf-art-frame > .cf-art")).backgroundImage, "the art is painted").to.contain("data:image");
          expect(getComputedStyle(part(cf, ".card-name-hover")).opacity, "the plate is clear until the card is hovered or focused").to.eq("0");
        });
        focusHost(which);
        cy.document().should((doc) => {
          const cf = cfIn(doc, which);
          const plate = part(cf, ".card-name-hover");
          expect(getComputedStyle(plate).opacity, "a focused card shows its name").to.eq(FOCUS_PLATE_OPACITY);
          const box = plate.getBoundingClientRect();
          const panel = rectOf(cf, ".cf-scale");
          // The plate has a height of its own: the face's band is `--cf-band`, which an art-first face sets to 0.
          expect(box.height, "the plate has a height").to.be.greaterThan(8);
          expect(box.bottom, "the plate lies at the foot").to.be.closeTo(panel.bottom, EDGE_TOLERANCE_PX);
          expect(box.left).to.be.closeTo(panel.left, EDGE_TOLERANCE_PX);
          expect(box.right).to.be.closeTo(panel.right, EDGE_TOLERANCE_PX);
          expect(plate.textContent, "the plate says the card's name").to.eq(defOf(id).name);
        });
      });
    }
  }

  for (const size of BACKROW_SIZES) {
    it(`R3339000 under Always an art-first backrow card keeps its name, a band with a height, at ${size.label} ${String(size.width)}x${String(size.height)}`, () => {
      cy.then(() => {
        writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, boardNames: "always" });
      });
      cy.mount(
        <Row>
          <Backrow id={FULL_ART_BACKROW} width={size.width} height={size.height} />
          <Backrow id={LEGENDARY_BACKROW} width={size.width} height={size.height} />
        </Row>,
      );
      cy.document().should((doc) => {
        for (const id of [FULL_ART_BACKROW, LEGENDARY_BACKROW]) {
          const cf = cfIn(doc, `${id}-${String(size.width)}`);
          expect(cf.hasAttribute("data-art-first"), `${id} is not drawn art first`).to.eq(false);
          const name = part(cf, ".card-name");
          expect(name.classList.contains("card-name-hover"), `${id}: a static name`).to.eq(false);
          expect(name.textContent).to.eq(defOf(id).name);
          expect(name.getBoundingClientRect().height, `${id}: the name has a height`).to.be.greaterThan(8);
        }
        // A full-art one's band is R3046000's, its share of the face.
        const full = cfIn(doc, `${FULL_ART_BACKROW}-${String(size.width)}`);
        expect(rectOf(full, ".card-name").height / rectOf(full, ".cf-scale").height, "the band is its share of the face").to.be.closeTo(BACKROW_BAND_SHARE, 0.03);
      });
    });
  }
});

describe("R3339000 art-first cards on the real board", () => {
  const BOARD = '[data-testid="board"]';
  const VIEWPORTS = [
    { label: "desktop", width: 1280, height: 720 },
    { label: "phone", width: 390, height: 844 },
  ] as const;
  const SETTINGS = ["artFirst", "always", "never"] as const;

  /** `fullBoardView()` with Bread and Butter face-up in lane 1 of the viewer's backrow. */
  function view() {
    const base = fullBoardView();
    return { ...base, you: { ...base.you, backrow: [faceUpBackrow("p1", { defId: FULL_ART_BACKROW, type: "Field Trap" }), ...base.you.backrow.slice(1)] } };
  }

  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    clearPicks();
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    clearPicks();
  });

  for (const viewport of VIEWPORTS) {
    for (const setting of SETTINGS) {
      it(`R3339000 renders board faces at ${viewport.label} with boardNames=${setting}`, () => {
        writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, boardNames: setting });
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
          const faces = [...doc.querySelectorAll<HTMLElement>(`${BOARD} .field .card[data-face="minion"] > .cf, ${BOARD} .field .card[data-face="compact"] > .cf`)];
          const where = (cf: HTMLElement): string => `${cf.parentElement?.getAttribute("data-def-id") ?? "?"} at ${viewport.label}, ${setting}`;
          const drawnArtFirst = faces.filter((cf) => cf.getAttribute("data-art-first") === "true");
          const artFirstUnits = drawnArtFirst.filter((cf) => cf.getAttribute("data-layout") === "minion");

          if (setting === "always") {
            expect(drawnArtFirst.length, "Always draws no card art first").to.eq(0);
          } else {
            // Gary and Tempo Timmy on your side, Mr. Vanilla and Jlockeed Shredder-10 on the other's: both seats.
            expect(artFirstUnits.length, "art-first units on the board").to.be.at.least(4);
            const seats = new Set(artFirstUnits.map((cf) => cf.parentElement?.getAttribute("data-owner")));
            expect(seats.size, "art-first units of both seats").to.be.at.least(2);
          }

          for (const cf of faces) {
            const staticName = cf.querySelector<HTMLElement>(".card-name:not(.card-name-hover)");
            const hoverName = cf.querySelector<HTMLElement>(".card-name-hover");
            const artFirst = cf.getAttribute("data-art-first") === "true";
            if (setting === "always") {
              expect(staticName !== null, `${where(cf)}: Always shows every name`).to.eq(true);
              expect(hoverName, `${where(cf)}: no hover plate`).to.eq(null);
              expect(staticName?.getBoundingClientRect().height, `${where(cf)}: the name has a height`).to.be.greaterThan(6);
            } else if (setting === "never" || artFirst) {
              expect(staticName, `${where(cf)}: no static name`).to.eq(null);
              expect(hoverName !== null, `${where(cf)}: a hover plate`).to.eq(true);
              expect(hoverName?.textContent?.length, `${where(cf)}: it says a name`).to.be.greaterThan(0);
            } else {
              expect(staticName !== null, `${where(cf)}: a card that is not art first keeps its name`).to.eq(true);
            }
            if (artFirst) {
              expect(cf.querySelector(".card-type"), `${where(cf)}: no type line`).to.eq(null);
            }
          }

          const normalUnits = faces.filter((cf) => cf.getAttribute("data-layout") === "minion" && cf.getAttribute("data-art-first") !== "true");
          expect(normalUnits.length, "normal units on the board").to.be.at.least(1);

          // Keyword marks are icon marks with accessible labels.
          const marks = [...doc.querySelectorAll<HTMLElement>(`${BOARD} .field .card[data-face="minion"] .keywords .keyword`)];
          for (const mark of marks) {
            expect(mark.getAttribute("role")).to.eq("img");
            expect(Boolean(mark.getAttribute("aria-label")), "has accessible label").to.eq(true);
          }
        });
      });
    }
  }
});
