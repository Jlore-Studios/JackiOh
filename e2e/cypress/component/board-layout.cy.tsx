// BUILD M5-T1 measures the 10-unit, 10-backrow, stacked-pile fixture in a real browser; B28 and B46
// extend it to the mobile and tablet viewports, without overflow either way. Mount `Game` in
// `.app-shell--wide`, its narrowest product container; a bare `Board` gets space it never has.

import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView, heroPower } from "../../../apps/web/src/test/fixtures.ts";

/** BUILD M5-T1's viewports plus B28's landscape phone and portrait tablet. */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "landscape", width: 844, height: 390 },
  { label: "tablet", width: 768, height: 1024 },
  { label: "phone", width: 390, height: 844 },
  { label: "tablet landscape", width: 1024, height: 768 },
  { label: "phone, Safari's visible viewport", width: 390, height: 664 },
] as const;

type Viewport = (typeof VIEWPORTS)[number];

/** `[data-testid="board"]` — `apps/web/src/game/contract.ts` `testid.board`. */
const BOARD = '[data-testid="board"]';

/** BUILD M5-T1's 10 units and 10 backrow cards. */
const FIELD_CARDS = `${BOARD} .field .card`;
const FIELD_CARD_COUNT = 20;

/** #2876's tile width-to-height cap, mirroring `--field-tile-max` (`position-switch.test.tsx` pins it). */
const FIELD_TILE_MAX = 1.25;

type Measurement = {
  viewport: string;
  viewportWidth: number;
  documentScrollWidth: number;
  bodyScrollWidth: number;
  boardScrollWidth: number;
  boardRight: number;
  boardWidth: number;
  documentScrollHeight: number;
  boardHeight: number;
  boardBottom: number;
};

function measure(doc: Document, viewport: Viewport, board: Element): Measurement {
  const box = board.getBoundingClientRect();
  return {
    viewport: `${viewport.label} ${viewport.width}x${viewport.height}`,
    viewportWidth: viewport.width,
    documentScrollWidth: doc.documentElement.scrollWidth,
    bodyScrollWidth: doc.body.scrollWidth,
    boardScrollWidth: board.scrollWidth,
    boardRight: Math.ceil(box.right),
    boardWidth: Math.round(box.width),
    documentScrollHeight: doc.documentElement.scrollHeight,
    boardHeight: Math.round(box.height),
    boardBottom: Math.ceil(box.bottom),
  };
}

describe("BUILD M5-T1 — the full fixture board fits 1280x720, 844x390, 768x1024 and 390x844", () => {
  beforeEach(() => {
    // Assert BUILD M5-T1's dense fixture before measuring it; a sparse board is an easy pass.
    const view = fullBoardView();
    const units = [...view.you.units, ...view.opponent.units].filter((unit) => unit !== null);
    const backrow = [...view.you.backrow, ...view.opponent.backrow].filter((slot) => slot !== null);

    expect(units, "BUILD M5-T1: the fixture board holds 10 units").to.have.length(10);
    expect(backrow, "BUILD M5-T1: the fixture board holds 10 backrow cards").to.have.length(10);
    expect(
      units.filter((unit) => unit.buried > 0),
      "BUILD M5-T1: a stacked pile",
    ).to.have.length.greaterThan(0);

    // `legal: []` changes borders and opacity, never geometry; this fixture has no prompt.
    cy.mount(
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={() => undefined} />
      </div>,
    );
  });

  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`B28 draws all 20 field cards with no horizontal overflow at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);

      cy.get(BOARD).should("be.visible");
      // Count and per-card visibility prevent missing or clipped lanes from making a sparse board pass.
      cy.get(FIELD_CARDS)
        .should("have.length", FIELD_CARD_COUNT)
        .each(($card) => {
          cy.wrap($card, { log: false }).should("be.visible");
        });

      const out: { measured?: Measurement } = {};

      // `should` retries until the viewport change has settled.
      cy.document({ log: false }).should((doc) => {
        const board = doc.querySelector(BOARD);
        expect(board, "the board is mounted").to.not.eq(null);
        if (board === null) return;

        const m = measure(doc, viewport, board);

        expect(m.documentScrollWidth, `the document fits ${where}`).to.be.at.most(viewport.width);
        expect(m.bodyScrollWidth, `the body fits ${where}`).to.be.at.most(viewport.width);
        expect(m.boardScrollWidth, `the board's contents fit ${where}`).to.be.at.most(viewport.width);
        expect(m.boardRight, `the board's right edge is inside ${where}`).to.be.at.most(viewport.width);
        expect(m.documentScrollHeight, `the document fits ${where} vertically`).to.be.at.most(viewport.height);

        // Prevent a collapsed board from trivially fitting.
        expect(m.boardWidth, `the board spans ${where} (minus .app-shell's padding)`).to.be.at.least(
          viewport.width - 40,
        );

        out.measured = m;
      });

      cy.then(() => {
        cy.task("layout:report", out.measured, { log: false });
      });
    });
  }
});

// R504: measure empty hand rows in a browser and keep the field in place at every viewport.
describe("R504 an empty hand keeps its place at every size", () => {
  function handRows(doc: Document): { you: number; opponent: number; fieldTop: number } {
    const height = (id: string): number => doc.querySelector(`[data-testid="${id}"]`)?.getBoundingClientRect().height ?? -1;
    return {
      you: height("hand-you"),
      opponent: height("hand-opponent"),
      fieldTop: doc.querySelector(`${BOARD} .field`)?.getBoundingClientRect().top ?? -1,
    };
  }

  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`R504 both hands keep their height with no cards, and the field stays put, at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const full = fullBoardView();
      const empty = { ...full, you: { ...full.you, hand: [] }, opponent: { ...full.opponent, hand: { count: 0 } } };
      const seen: { full?: ReturnType<typeof handRows> } = {};

      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={full} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get(`${BOARD} [data-testid="hand-you"] .hand-slot`).should("have.length", Array.isArray(full.you.hand) ? full.you.hand.length : 0);
      cy.document({ log: false }).then((doc) => {
        seen.full = handRows(doc);
      });

      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={empty} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get('[data-testid="hand-empty-you"]').should("exist");
      cy.get('[data-testid="hand-empty-opponent"]').should("exist");
      cy.document({ log: false }).should((doc) => {
        const before = seen.full;
        expect(before, "the full board was measured").to.not.eq(undefined);
        if (before === undefined) return;
        const after = handRows(doc);
        expect(before.you, `your hand has a height at ${where}`).to.be.greaterThan(0);
        expect(after.you, `your empty hand keeps its height at ${where}`).to.be.closeTo(before.you, 1);
        expect(after.opponent, `their empty hand keeps its height at ${where}`).to.be.closeTo(before.opponent, 1);
        expect(after.fieldTop, `the field does not move at ${where}`).to.be.closeTo(before.fieldTop, 1);
      });
    });
  }
});

// #2877: hero plates keep one size however many effects they carry. The plate (portrait, seat
// label, primary power) is fixed per layout; the seat's modifiers, secrets and Rift flag live in
// one tray hung off it, out of flow, toward the field. Past the layout's allowance the last slot
// is a `+N` chip opening the full list.
describe("#2877 hero plates keep one size however many effects they carry", () => {
  /** Eight modifiers, long labels among them, in the view's order. */
  const EIGHT = [
    { id: "m1", label: "Add a random Call to Chaos to your hand at the start of your every other turn" },
    { id: "m2", label: 'Your cards gain "Combo: draw 1"' },
    { id: "m3", label: "Next Spell gains Echo +1" },
    { id: "m4", label: "Your Heroic Power costs (1) less this turn and your units gain Rush" },
    { id: "m5", label: "+2 Spell Damage while you hold a Secret" },
    { id: "m6", label: "Whenever you draw, give a random unit in your hand +1/+1" },
    { id: "m7", label: "Your traps trigger twice and then keep watching" },
    { id: "m8", label: "At the start of your turn, refresh two spent mana crystals" },
  ];

  /** The tray's allowance per viewport, mirroring `heroLayout.ts`: the phone query first, then
      the desktop query, else tablet. */
  function allowanceFor(viewport: Viewport): number {
    const { width, height } = viewport;
    if (width <= 600 || (width > height && height <= 500)) return 2;
    if (width >= 1025) return 4;
    return 3;
  }

  /** `fullBoardView()` with 8 modifiers, 2 secrets, the Rift flag, 3 Jade and a used power on both seats. */
  function heavyView(): ReturnType<typeof fullBoardView> {
    const base = fullBoardView();
    const used = { ...heroPower, usedThisTurn: true };
    return {
      ...base,
      you: {
        ...base.you,
        hero: { ...base.you.hero, power: used, powers: [used] },
        modifiers: EIGHT,
        secrets: [
          { id: "s-you-1", choice: "greed" as const },
          { id: "s-you-2", choice: "attack" as const },
        ],
        riftExtraTurn: true as const,
        jade: 3,
      },
      opponent: {
        ...base.opponent,
        hero: { health: 30, armor: 0, power: used, powers: [used] },
        modifiers: EIGHT,
        secrets: [{ id: "s-foe-1" }, { id: "s-foe-2" }],
        riftExtraTurn: true as const,
        jade: 3,
      },
    };
  }

  function heroBox(doc: Document, side: "you" | "opponent"): { left: number; top: number; width: number; height: number } {
    const box = doc.querySelector(`[data-testid="hero-${side}"]`)?.getBoundingClientRect();
    return { left: box?.left ?? -1, top: box?.top ?? -1, width: box?.width ?? -1, height: box?.height ?? -1 };
  }

  function fieldTop(doc: Document): number {
    return doc.querySelector(`${BOARD} .field`)?.getBoundingClientRect().top ?? -1;
  }

  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`#2877 each hero's box and the field's top stay within 1px with 0 vs 8+2+1 effects at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={fullBoardView()} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get('[data-testid="hero-you"]').should("be.visible");
      const seen: { you?: ReturnType<typeof heroBox>; opponent?: ReturnType<typeof heroBox>; field?: number } = {};
      cy.document({ log: false }).then((doc) => {
        seen.you = heroBox(doc, "you");
        seen.opponent = heroBox(doc, "opponent");
        seen.field = fieldTop(doc);
      });

      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={heavyView()} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get('[data-testid="tray-more-you"]').should("exist");
      cy.document({ log: false }).should((doc) => {
        expect(seen.you, "the plain board was measured").to.not.eq(undefined);
        if (seen.you === undefined || seen.opponent === undefined || seen.field === undefined) return;
        for (const [name, before, after] of [
          ["your hero's left", seen.you.left, heroBox(doc, "you").left],
          ["your hero's top", seen.you.top, heroBox(doc, "you").top],
          ["your hero's width", seen.you.width, heroBox(doc, "you").width],
          ["your hero's height", seen.you.height, heroBox(doc, "you").height],
          ["their hero's left", seen.opponent.left, heroBox(doc, "opponent").left],
          ["their hero's top", seen.opponent.top, heroBox(doc, "opponent").top],
          ["their hero's width", seen.opponent.width, heroBox(doc, "opponent").width],
          ["their hero's height", seen.opponent.height, heroBox(doc, "opponent").height],
          ["the field's top", seen.field, fieldTop(doc)],
        ] as const) {
          expect(before, `${name} is laid out at ${where}`).to.be.greaterThan(0);
          expect(after, `${name} does not move at ${where}`).to.be.closeTo(before, 1);
        }
      });
    });

    it(`#2877 each tray shows the layout's allowance of chips, the last a +N, at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const allowance = allowanceFor(viewport);
      // 8 modifiers + 2 secrets + the Rift flag: only the first allowance - 1 render as chips.
      const hidden = 11 - (allowance - 1);
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={heavyView()} legal={[]} onAction={() => undefined} />
        </div>,
      );
      for (const side of ["you", "opponent"] as const) {
        cy.get(`[data-testid="tray-${side}"] .modifier-badge`).should("have.length", allowance);
        cy.get(`[data-testid="tray-more-${side}"]`).should("have.text", `+${hidden}`);
      }

      // Exactly the allowance's entries: every one renders, and there is no +N chip.
      const base = fullBoardView();
      const exact = {
        ...base,
        you: { ...base.you, modifiers: EIGHT.slice(0, allowance) },
        opponent: { ...base.opponent, modifiers: EIGHT.slice(0, allowance) },
      };
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={exact} legal={[]} onAction={() => undefined} />
        </div>,
      );
      for (const side of ["you", "opponent"] as const) {
        cy.get(`[data-testid="tray-${side}"] .modifier-badge`).should("have.length", allowance);
        cy.get(`[data-testid="tray-more-${side}"]`).should("not.exist");
      }
    });

    it(`#2877 no tray overlaps a zone or a hand at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={heavyView()} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get('[data-testid="tray-more-you"]').should("exist");
      cy.document({ log: false }).should((doc) => {
        const trays = ["you", "opponent"].map(
          (side) => doc.querySelector(`[data-testid="tray-${side}"]`)?.getBoundingClientRect() ?? null,
        );
        expect(trays, "both trays are mounted").to.not.include(null);
        const solids = [
          ...doc.querySelectorAll(`${BOARD} .zone`),
          ...doc.querySelectorAll('[data-testid="hand-you"], [data-testid="hand-opponent"]'),
        ];
        expect(solids.length, "zones and hands are mounted").to.be.greaterThan(0);
        const hits: string[] = [];
        for (const tray of trays) {
          if (tray === null) continue;
          for (const solid of solids) {
            const box = solid.getBoundingClientRect();
            // Sub-pixel rounding between boxes, too small to hide a real overlap.
            const overlap =
              tray.left < box.right - 1 && tray.right > box.left + 1 && tray.top < box.bottom - 1 && tray.bottom > box.top + 1;
            if (overlap) hits.push(`${(solid as HTMLElement).dataset.testid ?? "a zone"}`);
          }
        }
        expect(hits, `no tray covers a zone or a hand at ${where}`).to.deep.eq([]);
      });
    });
  }
});

// #2330: where the board has the room the four rows are one height and a DEF card is drawn at full
// size; on a phone the backrow keeps B29's 44 px floor, and no field card leaves its zone. #2556: on a
// phone a DEF card's footprint fills its unit zone's width or height, whichever runs out first, unless
// it is already at full size.
describe("#2330 equal board rows, and DEF cards only as small as their zone needs", () => {
  /** The issue's sizes; `roomy` is the 761px-and-up layouts (a desktop, a tablet held upright). */
  const SIZES = [
    { label: "desktop", width: 1440, height: 900, roomy: true },
    { label: "desktop", width: 1280, height: 720, roomy: true },
    { label: "tablet", width: 768, height: 1024, roomy: true },
    { label: "phone", width: 390, height: 844, roomy: false },
    { label: "landscape", width: 844, height: 390, roomy: false },
    // #2556's phones, upright and on their side.
    { label: "phone", width: 360, height: 780, roomy: false },
    { label: "phone", width: 430, height: 932, roomy: false },
    { label: "landscape", width: 780, height: 360, roomy: false },
    { label: "landscape", width: 932, height: 430, roomy: false },
  ] as const;
  /** B29's touch floor, which a phone's backrow row keeps. */
  const BACKROW_FLOOR_PX = 44;
  /** Sub-pixel rounding between layout boxes and their transformed rects. */
  const SLACK_PX = 1;

  function inside(inner: DOMRect, outer: DOMRect): boolean {
    return (
      inner.left >= outer.left - SLACK_PX &&
      inner.right <= outer.right + SLACK_PX &&
      inner.top >= outer.top - SLACK_PX &&
      inner.bottom <= outer.bottom + SLACK_PX
    );
  }

  /** A zone's inside: its border box less its borders (a zone has no padding). */
  function insideOf(zone: HTMLElement): { width: number; height: number } {
    const box = zone.getBoundingClientRect();
    const style = getComputedStyle(zone);
    return {
      width: box.width - parseFloat(style.borderLeftWidth) - parseFloat(style.borderRightWidth),
      height: box.height - parseFloat(style.borderTopWidth) - parseFloat(style.borderBottomWidth),
    };
  }

  for (const size of SIZES) {
    const where = `${size.label} ${size.width}x${size.height}`;

    it(`#2330 rows, lanes and every DEF card's footprint at ${where}`, () => {
      cy.viewport(size.width, size.height);
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={fullBoardView()} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

      const out: { defs?: unknown[] } = {};

      cy.document({ log: false }).should((doc) => {
        const field = doc.querySelector<HTMLElement>(`${BOARD} .field`);
        expect(field, "the field is mounted").to.not.eq(null);
        if (field === null) return;
        const zones = [...field.querySelectorAll<HTMLElement>(".zone")];
        expect(zones, "4 rows of 5 zones").to.have.length(20);

        if (size.roomy) {
          const first = zones[0]?.getBoundingClientRect().height ?? 0;
          expect(first, `a row has a height at ${where}`).to.be.greaterThan(0);
          for (const zone of zones) {
            expect(zone.getBoundingClientRect().height, `${zone.dataset.testid ?? "a zone"} is as tall as the first at ${where}`).to.be.closeTo(first, SLACK_PX);
          }
          const style = getComputedStyle(field);
          const inner = field.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
          for (const lane of field.querySelectorAll<HTMLElement>(".lane")) {
            expect(lane.getBoundingClientRect().height, `lane ${lane.dataset.lane ?? "?"} fills the field at ${where}`).to.be.closeTo(inner, SLACK_PX);
          }
        } else {
          for (const zone of zones.filter((each) => each.dataset.row === "backrow")) {
            expect(zone.getBoundingClientRect().height, `${zone.dataset.testid ?? "a zone"} keeps the touch floor at ${where}`).to.be.at.least(BACKROW_FLOOR_PX);
          }
        }

        for (const card of field.querySelectorAll<HTMLElement>(".zone > .card")) {
          const zone = card.parentElement;
          if (zone === null) continue;
          expect(inside(card.getBoundingClientRect(), zone.getBoundingClientRect()), `${card.dataset.testid ?? "a back"} stays inside its zone at ${where}`).to.eq(true);
        }

        const defs = [...field.querySelectorAll<HTMLElement>('.card[data-position="DEF"]')];
        expect(defs, "fullBoardView's two DEF units").to.have.length(2);
        const rows: unknown[] = [];
        out.defs = rows;
        for (const def of defs) {
          const name = def.dataset.testid ?? "?";
          const box = def.getBoundingClientRect();
          // Turned at full size, the footprint is the card's own box on its side.
          const full = Math.abs(box.width - def.offsetHeight) <= SLACK_PX && Math.abs(box.height - def.offsetWidth) <= SLACK_PX;
          if (size.roomy) {
            expect(box.width, `${name} footprint width at ${where}`).to.be.closeTo(def.offsetHeight, SLACK_PX);
            expect(box.height, `${name} footprint height at ${where}`).to.be.closeTo(def.offsetWidth, SLACK_PX);
            continue;
          }
          const zone = def.parentElement;
          expect(zone, `${name} sits in a zone`).to.not.eq(null);
          if (zone === null) continue;
          const room = insideOf(zone);
          expect(box.width, `${name} fits its zone's width at ${where}`).to.be.at.most(room.width + SLACK_PX);
          expect(box.height, `${name} fits its zone's height at ${where}`).to.be.at.most(room.height + SLACK_PX);
          const fills = Math.abs(box.width - room.width) <= SLACK_PX || Math.abs(box.height - room.height) <= SLACK_PX;
          expect(fills || full, `#2556 ${name} fills its zone's width or height at ${where}`).to.eq(true);
          rows.push({ issue: 2556, size: where, card: name, scale: Number((box.width / def.offsetHeight).toFixed(3)), footprint: [Math.round(box.width), Math.round(box.height)], zone: [Math.round(room.width), Math.round(room.height)], row: Math.round(zone.getBoundingClientRect().height) });
        }
      });

      cy.then(() => {
        cy.task("layout:report", out.defs ?? [], { log: false });
      });
    });
  }
});

// #2876: on a phone held upright every field card is a tile that fills its zone: face-up units,
// face-down backs and backrow cards are as wide as their zone's inside or the tile ratio cap,
// the four rows share the field's height, and an ATK unit's drawn area is at least 1.5x its size
// on `main`. The hand, `--card-h` and the screen budget stay exactly as they are.
describe("#2876 phone portrait tiles fill their zones", () => {
  /** The issue's phones, upright. */
  const SIZES = [
    { label: "phone", width: 390, height: 844 },
    { label: "phone, Safari's visible viewport", width: 390, height: 664 },
    { label: "phone", width: 360, height: 780 },
    { label: "phone", width: 430, height: 932 },
  ] as const;
  /** Sub-pixel rounding between layout boxes and their measured rects. */
  const SLACK_PX = 1;

  /** A zone's inside: its border box less its borders (a zone has no padding). */
  function insideOf(zone: HTMLElement): { width: number; height: number } {
    const box = zone.getBoundingClientRect();
    const style = getComputedStyle(zone);
    return {
      width: box.width - parseFloat(style.borderLeftWidth) - parseFloat(style.borderRightWidth),
      height: box.height - parseFloat(style.borderTopWidth) - parseFloat(style.borderBottomWidth),
    };
  }

  function mountBoard(rail: boolean): void {
    cy.mount(
      <div className="app-shell app-shell--wide">
        <Game
          view={fullBoardView()}
          legal={[]}
          onAction={() => undefined}
          boardRail={rail ? <button type="button">Back</button> : undefined}
        />
      </div>,
    );
  }

  /**
   * Resolve `--card-h` to pixels with a hidden probe in the board's scope. Custom properties
   * compute to their specified value (`clamp(44px, …)`), so `getComputedStyle` never yields
   * pixels for one; a probe with `height: var(--card-h)` does. The probe is absolute, so it
   * takes no space, and it is removed before the assertion returns.
   */
  function cardHPx(board: HTMLElement): number {
    const probe = board.ownerDocument.createElement("div");
    probe.style.position = "absolute";
    probe.style.visibility = "hidden";
    probe.style.height = "var(--card-h)";
    board.appendChild(probe);
    const px = probe.getBoundingClientRect().height;
    probe.remove();
    return px;
  }

  for (const size of SIZES) {
    const where = `${size.label} ${size.width}x${size.height}`;

    it(`#2876 field cards are as wide as their zone or the tile cap at ${where}`, () => {
      cy.viewport(size.width, size.height);
      mountBoard(false);
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

      cy.document({ log: false }).should((doc) => {
        const field = doc.querySelector<HTMLElement>(`${BOARD} .field`);
        expect(field, "the field is mounted").to.not.eq(null);
        if (field === null) return;
        const board = field.closest<HTMLElement>(BOARD);
        expect(board, "the board is mounted").to.not.eq(null);
        if (board === null) return;
        // Rows may grow past the zone's width (tall viewports), so a tile filling its zone can be
        // narrower than 0.74 of its height; what proves the 0.72 shape is gone is beating it.
        const cardH = cardHPx(board);
        expect(cardH, "--card-h resolves to pixels").to.be.greaterThan(0);
        const oldWidth = 0.72 * cardH;
        for (const card of field.querySelectorAll<HTMLElement>('.zone > .card:not([data-position="DEF"])')) {
          const zone = card.parentElement;
          expect(zone, "a field card sits in a zone").to.not.eq(null);
          if (zone === null) continue;
          const room = insideOf(zone);
          const box = card.getBoundingClientRect();
          const expected = Math.min(room.width, FIELD_TILE_MAX * box.height);
          expect(box.width, `${card.dataset.testid ?? "a back"} is inside its zone at ${where}`).to.be.at.most(
            room.width + SLACK_PX,
          );
          expect(box.width, `${card.dataset.testid ?? "a back"} fills its zone or the cap at ${where}`).to.be.at.least(
            expected - 1.5,
          );
          // Where the zone is wider than the old cap, the old shape would have bound: beating it
          // proves the tile. Where the zone binds the old shape too (390x844, 360x780), old and
          // new widths coincide and the taller rows (the lanes and area its) carry the proof.
          if (oldWidth < room.width - 1) {
            expect(box.width, `the 0.72 portrait shape no longer applies at ${where}`).to.be.greaterThan(oldWidth);
          }
        }
      });
    });

    it(`#2876 lanes fill the field's inside height at ${where}`, () => {
      cy.viewport(size.width, size.height);
      mountBoard(false);
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

      cy.document({ log: false }).should((doc) => {
        const field = doc.querySelector<HTMLElement>(`${BOARD} .field`);
        expect(field, "the field is mounted").to.not.eq(null);
        if (field === null) return;
        const style = getComputedStyle(field);
        const inner = field.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
        expect(inner, `the field has a height at ${where}`).to.be.greaterThan(0);
        for (const lane of field.querySelectorAll<HTMLElement>(".lane")) {
          expect(lane.getBoundingClientRect().height, `lane ${lane.dataset.lane ?? "?"} fills the field at ${where}`).to.be.closeTo(
            inner,
            SLACK_PX,
          );
        }
      });
    });
  }

  it("#2876 an ATK unit's drawn area is at least 1.5x main's at 390x664", () => {
    cy.viewport(390, 664);
    mountBoard(false);
    cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

    const out: { measured?: { mainArea: number; newArea: number } } = {};

    cy.document({ log: false }).should((doc) => {
      const board = doc.querySelector<HTMLElement>(BOARD);
      expect(board, "the board is mounted").to.not.eq(null);
      if (board === null) return;
      const cardH = cardHPx(board);
      expect(cardH, "--card-h resolves to pixels").to.be.greaterThan(0);
      const mainArea = 0.72 * cardH * cardH;
      const card = doc.querySelector<HTMLElement>(`${BOARD} .zone[data-row="units"] > .card[data-position="ATK"]`);
      expect(card, "an ATK unit is on the field").to.not.eq(null);
      if (card === null) return;
      const box = card.getBoundingClientRect();
      const newArea = box.width * box.height;
      expect(newArea, "an ATK unit's drawn area at 390x664").to.be.at.least(1.5 * mainArea);
      out.measured = { mainArea: Math.round(mainArea), newArea: Math.round(newArea) };
    });

    cy.then(() => {
      cy.task("layout:report", { issue: 2876, size: "phone 390x664", ...out.measured }, { log: false });
    });
  });

  it("#2876 End turn and your hand stay on screen at 390x664, with and without a rail", () => {
    for (const rail of [false, true]) {
      const where = rail ? "with a rail" : "with no rail";
      cy.viewport(390, 664);
      mountBoard(rail);
      cy.get('[data-testid="end-turn"]').should("be.visible");

      cy.document({ log: false }).should((doc) => {
        const endTurn = doc.querySelector("[data-testid=\"end-turn\"]");
        expect(endTurn, `End turn is mounted ${where}`).to.not.eq(null);
        if (endTurn === null) return;
        expect(endTurn.getBoundingClientRect().bottom, `End turn stays on screen at 390x664 ${where}`).to.be.at.most(
          664 + SLACK_PX,
        );
        const cards = [...doc.querySelectorAll('[data-testid="hand-you"] [data-testid^="hand-card-"]')];
        expect(cards.length, `your hand has cards ${where}`).to.be.greaterThan(0);
        for (const card of cards) {
          expect(
            card.getBoundingClientRect().bottom,
            `a hand card's bottom edge stays on screen at 390x664 ${where}`,
          ).to.be.at.most(664 + SLACK_PX);
        }
      });
    }
  });

  it("#2876 your hand keeps its size at 390x844", () => {
    cy.viewport(390, 844);
    mountBoard(false);
    cy.get('[data-testid="hand-you"] [data-testid^="hand-card-"]').should("have.length.greaterThan", 0);

    cy.document({ log: false }).should((doc) => {
      const board = doc.querySelector<HTMLElement>(BOARD);
      expect(board, "the board is mounted").to.not.eq(null);
      if (board === null) return;
      const cardH = cardHPx(board);
      expect(cardH, "--card-h resolves to pixels").to.be.greaterThan(0);
      const card = doc.querySelector<HTMLElement>('[data-testid="hand-you"] [data-testid^="hand-card-"]');
      expect(card, "your hand has a card").to.not.eq(null);
      if (card === null) return;
      // The fan tilts each slot, so the rect is the tilted box; the offsets are the card itself.
      expect(card.offsetHeight, "your hand's card height is unchanged at 390x844").to.be.closeTo(
        1.24 * cardH,
        SLACK_PX,
      );
      expect(card.offsetWidth, "your hand's card width is unchanged at 390x844").to.be.closeTo(
        0.68 * 1.24 * cardH,
        SLACK_PX,
      );
    });
  });
});
