// BUILD M5-T1 measures the 10-unit, 10-backrow, stacked-pile fixture in a real browser; B28 and B46
// extend it to the mobile and tablet viewports, without overflow either way. Mount `Game` in
// `.app-shell--wide`, its narrowest product container; a bare `Board` gets space it never has.

import type { ReactElement } from "react";

import Username from "../../../apps/web/src/auth/Username.tsx";
import Game from "../../../apps/web/src/game/Game.tsx";
import RankCrest from "../../../apps/web/src/rank/RankCrest.tsx";
import { baseView, emptySide, faceDownBackrow, fullBoardView, heroPower, unit } from "../../../apps/web/src/test/fixtures.ts";

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
        (["you", "opponent"] as const).forEach((side, i) => {
          const tray = trays[i];
          if (tray === undefined || tray === null) return;
          for (const solid of solids) {
            const box = solid.getBoundingClientRect();
            // Sub-pixel rounding between boxes, too small to hide a real overlap.
            const overlap =
              tray.left < box.right - 1 && tray.right > box.left + 1 && tray.top < box.bottom - 1 && tray.bottom > box.top + 1;
            if (overlap) hits.push(`${(solid as HTMLElement).dataset.testid ?? "a zone"}-by-tray-${side}`);
          }
        });
        expect(hits.join(" "), `no tray covers a zone or a hand at ${where}`).to.eq("");
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

// #2932: on a phone held upright a field card keeps a card shape (its width over its height between
// 0.72, the phone's card, and 0.86, the shape the desktop draws units at) and grows by height; the four
// rows share the field's height; the seats are thinner and give theirs to the field; the match bar
// rides the top strip; and a unit's Activate control and keyword marks stand in its zone's corner. #2876's
// other claims hold as they were: no mat above or below the lanes, and the hand, `--card-h` and the
// screen budget unchanged.
describe("#2876, #2932 phone portrait cards keep a card shape and the field keeps the height", () => {
  /** The issue's phones, upright. */
  const SIZES = [
    { label: "phone", width: 390, height: 844 },
    { label: "phone, Safari's visible viewport", width: 390, height: 664 },
    { label: "phone", width: 360, height: 780 },
    { label: "phone", width: 430, height: 932 },
  ] as const;
  /** Sub-pixel rounding between layout boxes and their measured rects. */
  const SLACK_PX = 1;
  /** `--field-ratio-min` and `--field-ratio-max` (`position-switch.test.tsx` pins them), and what a measured ratio may stray. */
  const FIELD_RATIO_MIN = 0.72;
  const FIELD_RATIO_MAX = 0.86;
  const RATIO_SLACK = 0.02;
  /** What a card-shaped placeholder's ratio may stray from the card's, for the 2px inset it keeps off each side (#3257). */
  const INSET_RATIO_SLACK = 0.03;
  /** How far the badge stack's first mark may stand from its zone's inner top-left corner: its 2px inset and a little more (#3257). */
  const CORNER_PX = 4;
  /** The lanes that carry a badged unit in the specs of #3257: the first, a middle one and the last. */
  const BADGED_LANES = [1, 3, 5] as const;
  /** A badge stack's items: the Activate control, the keyword marks and the "+n" (`FieldBadges.tsx`). */
  const MARKS = ".activate-control, .keyword-icon, .cf-kw-more";
  /** B29's touch floor. */
  const TOUCH_PX = 44;
  /** How much of current `main`'s seat a plate may still be: #2932 asks for at least 20% off. */
  const PLATE_MAX_SHARE = 0.8;

  /**
   * The numbers before this change, measured in this very harness (this file's board, `fullBoardView()`
   * with no rail, mounted in `.app-shell--wide`, in Cypress's browser) with the stylesheets of the
   * commit in question swapped in: `board.css`, `activate.css` and `cards.css` of `main` (c95f4c3ea),
   * and of the commit before #2900 (67fa9e5d3^). They are constants because those stylesheets cannot be
   * mounted beside this change. Main's seats at every phone, and at 390x664 an ATK unit's drawn box and
   * its row (the zone's height); before #2900, the same unit's box.
   */
  const MAIN: Readonly<Record<string, { you: number; opponent: number }>> = {
    "390x844": { you: 103.2, opponent: 103.2 },
    "390x664": { you: 103.2, opponent: 103.2 },
    "360x780": { you: 103.2, opponent: 103.2 },
    "430x932": { you: 103.2, opponent: 103.2 },
  };
  const MAIN_ATK_390X664 = { width: 63.6, height: 59.1, row: 62.1 };
  const BEFORE_2900_ATK_390X664 = { width: 41.3, height: 54.3 };

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

  /** A match's rail as a phone draws it: Back and the compact rank line, as `routes/match.tsx` builds them. */
  function RankRail(): ReactElement {
    return (
      <>
        <nav className="row screen-nav">
          <button type="button" className="link-button" data-testid="nav-back">
            ← Back
          </button>
        </nav>
        <p className="match-ranks" data-testid="match-ranks">
          <span className="match-ranks__full">Ranked match · MihoMain (you) Raisin vs LazyPirate Raisin</span>
          <span className="match-ranks__short">
            <span className="match-ranks__label">Ranked · </span>
            <Username name="MihoMain" /> (you) <RankCrest rank={{ tier: "raisin", placementsPlayed: 2, placementGames: 5 }} size="small" />{" "}
            <span className="match-ranks__tier">Raisin</span> vs <Username name="LazyPirate" />{" "}
            <RankCrest rank={{ tier: "raisin", placementsPlayed: 2, placementGames: 5 }} size="small" />{" "}
            <span className="match-ranks__tier">Raisin</span>
          </span>
        </p>
      </>
    );
  }

  function mountTopRail(view: ReturnType<typeof fullBoardView> = fullBoardView()): void {
    cy.mount(
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={() => undefined} phoneRailTop boardRail={<RankRail />} />
      </div>,
    );
  }

  /** The probe's height, in pixels: a hidden, absolutely placed `div` in the board's scope takes no space. */
  function probePx(board: HTMLElement, height: string): number {
    const probe = board.ownerDocument.createElement("div");
    probe.style.position = "absolute";
    probe.style.visibility = "hidden";
    probe.style.height = height;
    board.appendChild(probe);
    const px = probe.getBoundingClientRect().height;
    probe.remove();
    return px;
  }

  /**
   * Resolve `--card-h` to pixels with a hidden probe in the board's scope. Custom properties
   * compute to their specified value (`clamp(44px, …)`), so `getComputedStyle` never yields
   * pixels for one; a probe with `height: var(--card-h)` does.
   */
  function cardHPx(board: HTMLElement): number {
    return probePx(board, "var(--card-h)");
  }

  function overlaps(a: DOMRect, b: DOMRect): boolean {
    return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
  }

  /** Your units row's zone in `lane`, from 1. */
  function unitZone(doc: Document, lane: number): HTMLElement {
    const zone = doc.querySelector<HTMLElement>(`${BOARD} .zone[data-side="you"][data-row="units"][data-lane="${lane}"]`);
    expect(zone, `your lane ${lane} unit zone is mounted`).to.not.eq(null);
    return zone as HTMLElement;
  }

  const PING = { ability: "ping", label: "Deal 1 damage.", usesLeft: 1, usable: true } as const;

  /**
   * The full board with an Activate ability and two keywords on your units in lanes 1, 3 and 5, all in
   * `position`: lane 2's unit already carries every keyword, lane 4's only a Stack (no mark).
   */
  function badged(position: "ATK" | "DEF"): ReturnType<typeof fullBoardView> {
    const base = fullBoardView();
    const units = base.you.units.map((each, index) =>
      each === null || !BADGED_LANES.some((lane) => lane - 1 === index)
        ? each
        : { ...each, position, keywords: [{ kind: "Rush" as const }, { kind: "Can't Defend" as const }], activations: [PING] },
    );
    return { ...base, you: { ...base.you, units } };
  }

  for (const size of SIZES) {
    const where = `${size.label} ${size.width}x${size.height}`;
    const key = `${size.width}x${size.height}`;

    it(`#2932 every field card's width over height is 0.72 to 0.86, in ATK and DEF, in all five lanes, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      mountBoard(false);
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

      cy.document({ log: false }).should((doc) => {
        const cards = [...doc.querySelectorAll<HTMLElement>(`${BOARD} .field .zone > .card`)];
        expect(cards, "twenty cards in their zones").to.have.length(FIELD_CARD_COUNT);
        expect(
          cards.filter((card) => card.dataset.position === "DEF").length,
          "fullBoardView's DEF units are among them",
        ).to.be.at.least(2);
        expect(new Set(cards.map((card) => card.parentElement?.dataset.lane)).size, "all five lanes").to.eq(5);
        for (const card of cards) {
          // The layout box, not the rect: a DEF card's rect is its box turned on its side.
          const ratio = card.offsetWidth / card.offsetHeight;
          expect(ratio, `${card.dataset.testid ?? "a back"}'s width over height at ${where}`).to.be.within(
            FIELD_RATIO_MIN - RATIO_SLACK,
            FIELD_RATIO_MAX + RATIO_SLACK,
          );
        }
      });
    });

    it(`#2932 every ATK unit and backrow card is as wide as its zone's inside, or at the 0.86 cap at its row's height, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      mountBoard(false);
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

      cy.document({ log: false }).should((doc) => {
        const cards = [...doc.querySelectorAll<HTMLElement>(`${BOARD} .field .zone > .card:not([data-position="DEF"])`)];
        expect(cards.length, "ATK units and backrow cards are mounted").to.be.greaterThan(0);
        for (const card of cards) {
          const zone = card.parentElement;
          expect(zone, "a field card sits in a zone").to.not.eq(null);
          if (zone === null) continue;
          const room = insideOf(zone);
          const name = card.dataset.testid ?? "a back";
          expect(card.offsetWidth, `${name} is as wide as its zone or the cap at ${where}`).to.be.closeTo(
            Math.min(room.width, FIELD_RATIO_MAX * room.height),
            SLACK_PX,
          );
          expect(card.offsetHeight, `${name} is as tall as its row or the shape floor lets it at ${where}`).to.be.closeTo(
            Math.min(room.height, room.width / FIELD_RATIO_MIN),
            SLACK_PX,
          );
        }
      });
    });

    it(`#2932 all four rows are one height, and every lane fills the field's inside height, at ${where}`, () => {
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
        const zones = [...field.querySelectorAll<HTMLElement>(".zone")];
        expect(zones, "4 rows of 5 zones").to.have.length(20);
        const first = zones[0]?.getBoundingClientRect().height ?? 0;
        for (const zone of zones) {
          // The backrow rows are not shrunk to make the unit rows bigger: every zone is the first one's height.
          expect(zone.getBoundingClientRect().height, `${zone.dataset.testid ?? "a zone"} is as tall as the first at ${where}`).to.be.closeTo(
            first,
            SLACK_PX,
          );
        }
        for (const lane of field.querySelectorAll<HTMLElement>(".lane")) {
          expect(lane.getBoundingClientRect().height, `lane ${lane.dataset.lane ?? "?"} fills the field at ${where}`).to.be.closeTo(
            inner,
            SLACK_PX,
          );
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

    it(`#2932 each hero plate is at least 20% shorter than on main, and every button in one is a 44px target, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      mountBoard(false);
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

      cy.document({ log: false }).should((doc) => {
        const main = MAIN[key];
        expect(main, `main's seat heights are recorded for ${where}`).to.not.eq(undefined);
        if (main === undefined) return;
        for (const side of ["you", "opponent"] as const) {
          const seat = doc.querySelector<HTMLElement>(`${BOARD} .seat-${side}`);
          expect(seat, `the ${side} seat is mounted`).to.not.eq(null);
          if (seat === null) continue;
          expect(seat.getBoundingClientRect().height, `the ${side} seat's height at ${where} (main ${String(main[side])}px)`).to.be.at.most(
            PLATE_MAX_SHARE * main[side],
          );
          const hero = doc.querySelector<HTMLElement>(`[data-testid="hero-${side}"]`);
          expect(hero?.getBoundingClientRect().height ?? 0, `hero-${side} keeps B29's floor`).to.be.at.least(TOUCH_PX - SLACK_PX);
          for (const button of seat.querySelectorAll<HTMLElement>("button")) {
            const box = button.getBoundingClientRect();
            if (box.width === 0 || box.height === 0) continue;
            const name = button.dataset.testid ?? button.className;
            if (box.width >= TOUCH_PX - SLACK_PX && box.height >= TOUCH_PX - SLACK_PX) continue;
            // The tray's chips were never 44px targets: an ellipsis needs `overflow: hidden`, which clips the
            // band, and the opponent's chips lie under the pills' own 44px band (inspectable.css). Both hold on
            // `main` (18.5px chips, the same clipped band), so they are not this change's to measure here.
            if (button.classList.contains("modifier-badge")) continue;
            // A smaller button takes its tap across a 44px band round its center (index.css, R2122000).
            const reach = TOUCH_PX / 2 - 2;
            for (const [x, y] of [
              [box.left + box.width / 2, box.top + box.height / 2 - reach],
              [box.left + box.width / 2, box.top + box.height / 2 + reach],
            ] as const) {
              const hit = doc.elementFromPoint(x, y);
              expect(hit !== null && (hit === button || button.contains(hit)), `a tap 22px off ${name}'s center reaches it at ${where}`).to.eq(true);
            }
          }
        }
      });
    });

    it(`#2932 the rail and Back share one top strip above the opponent's seat, and nothing sits below End turn, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      mountTopRail();
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

      cy.document({ log: false }).should((doc) => {
        const game = doc.querySelector<HTMLElement>('[data-testid="game"]');
        const rail = doc.querySelector<HTMLElement>(".game > .board-rail");
        const back = doc.querySelector<HTMLElement>('[data-testid="nav-back"]');
        const ranks = doc.querySelector<HTMLElement>('[data-testid="match-ranks"]');
        const banner = doc.querySelector<HTMLElement>(".game > .turn-banner");
        const seat = doc.querySelector<HTMLElement>(`${BOARD} .seat-opponent`);
        const endTurn = doc.querySelector<HTMLElement>('[data-testid="end-turn"]');
        expect(game?.dataset.railTop, "the game is in its top-strip layout").to.eq("true");
        expect(doc.querySelector(".control-bar .board-rail"), "the control bar holds no rail").to.eq(null);
        for (const [name, element] of [["the rail", rail], ["Back", back], ["the ranks", ranks], ["the banner", banner], ["the opponent's seat", seat], ["End turn", endTurn]] as const) {
          expect(element, `${name} is mounted at ${where}`).to.not.eq(null);
        }
        if (rail === null || back === null || ranks === null || banner === null || seat === null || endTurn === null) return;
        const [backBox, ranksBox, bannerBox, seatBox, endBox] = [back, ranks, banner, seat, endTurn].map((element) => element.getBoundingClientRect());
        if (backBox === undefined || ranksBox === undefined || bannerBox === undefined || seatBox === undefined || endBox === undefined) return;
        // One strip: Back and the ranks overlap vertically, both above the opponent's seat.
        expect(ranksBox.top < backBox.bottom && backBox.top < ranksBox.bottom, `the ranks and Back share a row at ${where}`).to.eq(true);
        expect(ranksBox.bottom, `the ranks sit above the opponent's seat at ${where}`).to.be.at.most(seatBox.top + SLACK_PX);
        expect(backBox.bottom, `Back sits above the opponent's seat at ${where}`).to.be.at.most(seatBox.top + SLACK_PX);
        // The shell's turn banner is the visually-hidden live region (board.css, #2917): no pill is
        // drawn, so the strip is the rail's alone and the banner takes no room in it.
        expect(Math.max(bannerBox.width, bannerBox.height), `the turn banner stays a hidden live region at ${where}`).to.be.at.most(1 + SLACK_PX);
        // One line: the rank line's visible parts share a row.
        const parts = [...ranks.querySelectorAll<HTMLElement>(".match-ranks__short > :not(.match-ranks__label):not(.match-ranks__tier)")];
        expect(parts.length, "the rank line has its names and crests").to.be.greaterThan(2);
        const middles = parts.map((part) => {
          const box = part.getBoundingClientRect();
          return box.top + box.height / 2;
        });
        expect(Math.max(...middles) - Math.min(...middles), `the rank line is one line at ${where}`).to.be.at.most(4);
        expect(ranksBox.height, `the rank line is one line tall at ${where}`).to.be.at.most(24);
        // The strip's words stay in the page for a screen reader, hidden the visually-hidden way.
        expect(ranks.querySelector(".match-ranks__label")?.textContent, "the label stays in the page").to.contain("Ranked");
        // Nothing below the End-turn row.
        for (const element of doc.querySelectorAll<HTMLElement>(`${BOARD} .hand, ${BOARD} .seat, ${BOARD} .field, .game > .board-rail, .game > .turn-banner`)) {
          expect(element.getBoundingClientRect().bottom, `${element.className} is not below End turn at ${where}`).to.be.at.most(endBox.bottom + SLACK_PX);
        }
        expect(endBox.bottom, `End turn is on screen at ${where}`).to.be.at.most(size.height + SLACK_PX);
      });
    });

    it(`#3257 every empty zone keeps a card-shaped placeholder inside it, in all four rows and all five lanes, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      const view = baseView({
        you: emptySide("p1", {
          units: [null, null, unit("p1", { instanceId: "yu3" }), null, null],
          backrow: [null, null, null, null, faceDownBackrow],
        }),
        opponent: emptySide("p2", { hand: { count: 0 }, units: [null, unit("p2", { instanceId: "ou2" }), null, null, null] }),
      });
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={view} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get(FIELD_CARDS).should("have.length", 3);
      cy.document({ log: false }).should((doc) => {
        const zones = [...doc.querySelectorAll<HTMLElement>(`${BOARD} .field .zone`)];
        expect(zones.length, `four rows of five zones at ${where}`).to.eq(20);
        const rows = new Set<string>();
        const lanes = new Set<string>();
        let empty = 0;
        for (const zone of zones) {
          const at = `${zone.dataset.side} ${zone.dataset.row} lane ${zone.dataset.lane} at ${where}`;
          const placeholder = getComputedStyle(zone, "::before");
          if (zone.querySelector(":scope > .card") !== null) {
            expect(placeholder.display, `an occupied zone shows no placeholder: ${at}`).to.eq("none");
            continue;
          }
          empty += 1;
          rows.add(`${zone.dataset.side}-${zone.dataset.row}`);
          lanes.add(zone.dataset.lane ?? "");
          expect(placeholder.display, `an empty zone shows its placeholder: ${at}`).to.not.eq("none");
          expect(placeholder.pointerEvents, `the placeholder takes no hit: ${at}`).to.eq("none");
          const width = parseFloat(placeholder.width);
          const height = parseFloat(placeholder.height);
          expect(width, `the placeholder has a width: ${at}`).to.be.greaterThan(0);
          expect(height, `the placeholder has a height: ${at}`).to.be.greaterThan(0);
          // A card's shape, less what the inset takes off each side.
          expect(width / height, `the placeholder is no thinner than a card: ${at}`).to.be.at.least(FIELD_RATIO_MIN - INSET_RATIO_SLACK);
          expect(width / height, `the placeholder is no wider than a card: ${at}`).to.be.at.most(FIELD_RATIO_MAX + INSET_RATIO_SLACK);
          const inside = insideOf(zone);
          expect(width, `the placeholder is no wider than its zone: ${at}`).to.be.at.most(inside.width + SLACK_PX);
          expect(height, `the placeholder is no taller than its zone: ${at}`).to.be.at.most(inside.height + SLACK_PX);
        }
        expect(rows.size, `empty zones in all four rows at ${where}`).to.eq(4);
        expect(lanes.size, `empty zones in all five lanes at ${where}`).to.eq(5);
        expect(empty, `the empty zones are all but three at ${where}`).to.eq(17);
      });
    });

    it(`#3257 a unit's Activate control and keyword marks stand inside its own zone's top-left corner, on and off DEF, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      const seen: { atk?: Record<number, DOMRect[]> } = {};
      const mountBadged = (position: "ATK" | "DEF"): void => {
        const view = badged(position);
        // Every unit's switch is live, so each zone holds its ⟳ at the top-right.
        const switches = view.you.units.flatMap((each) => (each === null ? [] : [{ type: "switchPosition" as const, instanceId: each.instanceId }]));
        cy.mount(
          <div className="app-shell app-shell--wide">
            <Game view={view} legal={switches} onAction={() => undefined} />
          </div>,
        );
        cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);
      };

      const check = (position: "ATK" | "DEF"): void => {
        mountBadged(position);
        cy.document({ log: false }).should((doc) => {
          const zones = [...doc.querySelectorAll<HTMLElement>(`${BOARD} .field .zone`)];
          for (const lane of BADGED_LANES) {
            const at = `${position} lane ${lane} at ${where}`;
            const zone = unitZone(doc, lane);
            const stack = zone.querySelector<HTMLElement>(":scope > .field-badges");
            const card = zone.querySelector<HTMLElement>(":scope > .card");
            const control = stack?.querySelector<HTMLElement>(".activate-control") ?? null;
            const flip = zone.querySelector<HTMLElement>(":scope > .switch-button");
            expect(stack, `the zone holds its badge stack: ${at}`).to.not.eq(null);
            expect(card, `the zone holds its card: ${at}`).to.not.eq(null);
            expect(control, `the stack holds the Activate control: ${at}`).to.not.eq(null);
            expect(flip, `the zone holds its switch: ${at}`).to.not.eq(null);
            if (stack === null || card === null || control === null || flip === null) return;
            expect(card.dataset.position, `the unit is in ${position}: ${at}`).to.eq(position);
            const marks = [...stack.querySelectorAll<HTMLElement>(MARKS)];
            // Lanes 1, 3 and 5 hold an ability and keywords: in DEF too the stack keeps the control and a mark or the +n (#3315).
            expect(marks.length, `the control and at least one mark or the +n: ${at}`).to.be.at.least(2);
            expect(card.querySelectorAll(".activate-control, .keyword-icon").length, `no mark stays on the face: ${at}`).to.eq(0);
            const zoneBox = zone.getBoundingClientRect();
            const flipBox = flip.getBoundingClientRect();
            const boxes = marks.map((mark) => mark.getBoundingClientRect());
            for (const box of boxes) {
              // Wholly inside its own zone.
              expect(box.left, `a mark is inside its zone's left: ${at}`).to.be.at.least(zoneBox.left - SLACK_PX);
              expect(box.top, `a mark is inside its zone's top: ${at}`).to.be.at.least(zoneBox.top - SLACK_PX);
              expect(box.right, `a mark is inside its zone's right: ${at}`).to.be.at.most(zoneBox.right + SLACK_PX);
              expect(box.bottom, `a mark is inside its zone's bottom: ${at}`).to.be.at.most(zoneBox.bottom + SLACK_PX);
              // Over no other zone, and not over this zone's own ⟳.
              expect(overlaps(box, flipBox), `a mark overlaps its zone's switch: ${at}`).to.eq(false);
              for (const other of zones) {
                if (other !== zone) expect(overlaps(box, other.getBoundingClientRect()), `a mark overlaps another zone: ${at}`).to.eq(false);
              }
            }
            // The first mark starts at the zone's inner top-left corner.
            const style = getComputedStyle(zone);
            const first = boxes[0];
            expect(first, `a first mark: ${at}`).to.not.eq(undefined);
            if (first === undefined) return;
            expect(first.left - (zoneBox.left + parseFloat(style.borderLeftWidth)), `the stack is at the zone's inner left: ${at}`).to.be.within(-SLACK_PX, CORNER_PX);
            expect(first.top - (zoneBox.top + parseFloat(style.borderTopWidth)), `the stack is at the zone's inner top: ${at}`).to.be.within(-SLACK_PX, CORNER_PX);

            // The Activate control's press band (its ::before) is inside the zone too, and no taller than a touch target.
            const controlBox = control.getBoundingClientRect();
            const controlStyle = getComputedStyle(control);
            const band = getComputedStyle(control, "::before");
            // The band is positioned from the control's padding box: past its border.
            const bandTop = controlBox.top + parseFloat(controlStyle.borderTopWidth) + parseFloat(band.top);
            const bandLeft = controlBox.left + parseFloat(controlStyle.borderLeftWidth) + parseFloat(band.left);
            const bandRight = controlBox.right - parseFloat(controlStyle.borderRightWidth) - parseFloat(band.right);
            const bandBottom = bandTop + parseFloat(band.height);
            expect(bandTop, `the band starts inside its zone's top: ${at}`).to.be.at.least(zoneBox.top - SLACK_PX);
            expect(bandLeft, `the band starts inside its zone's left: ${at}`).to.be.at.least(zoneBox.left - SLACK_PX);
            expect(bandRight, `the band ends inside its zone's right: ${at}`).to.be.at.most(zoneBox.right + SLACK_PX);
            expect(bandBottom, `the band ends inside its zone's bottom: ${at}`).to.be.at.most(zoneBox.bottom + SLACK_PX);
            expect(bandBottom - bandTop, `the band is no taller than a touch target: ${at}`).to.be.at.most(TOUCH_PX + SLACK_PX / 2);
            expect(bandBottom - bandTop, `the band is at least as tall as the control: ${at}`).to.be.at.least(controlBox.height - SLACK_PX / 2);
            expect(
              doc.elementFromPoint(controlBox.left + controlBox.width / 2, controlBox.top + controlBox.height / 2)?.closest(".activate-control") ?? null,
              `the Activate control takes a press on its own box: ${at}`,
            ).to.eq(control);

            const name = card.querySelector<HTMLElement>(".card-name")?.getBoundingClientRect();
            const stats = [...card.querySelectorAll<HTMLElement>(".stat")].map((stat) => stat.getBoundingClientRect());
            expect(name, `the card has its name: ${at}`).to.not.eq(undefined);
            expect(stats.length, `the card has its stat badges: ${at}`).to.be.at.least(2);
            // A DEF card is turned, so its name, attack and health stand in a strip down its left, under the stack's corner:
            // the stack stops above the turned card's box (board.css), whose top is where the room above the plate ends in ATK.
            const cardBox = card.getBoundingClientRect();
            const limit = position === "ATK" ? (name?.top ?? 0) : cardBox.top;
            // The Activate control is never dropped: on the shortest phone the band above a turned card is shorter than
            // the control, which then overlaps the attack gem's top (the owner's call on #3268, option 1) and only there.
            const controlFits = controlBox.bottom <= limit + SLACK_PX / 2;
            expect(controlFits || (position === "DEF" && size.height === 664), `the control stops above the name plate or the turned card, but on the shortest phone's DEF card: ${at}`).to.eq(true);
            // FieldBadges.tsx never drops the control, nor the +n while the unit has a mark to draw, so over a DEF card whose band
            // is shorter than they are they hang over the turned card (#3315); a named mark is only drawn where it fits.
            const mayHang = (mark: HTMLElement, box: DOMRect): boolean =>
              position === "DEF" && (mark === control || mark.matches(".cf-kw-more")) && box.bottom > limit + SLACK_PX / 2;
            marks.forEach((mark, index) => {
              const box = boxes[index];
              if (box === undefined) return;
              if (mayHang(mark, box)) return;
              if (name !== undefined) expect(overlaps(box, name), `a mark overlaps the name: ${at}`).to.eq(false);
              for (const stat of stats) expect(overlaps(box, stat), `a mark overlaps a stat: ${at}`).to.eq(false);
              expect(box.bottom, `a mark is above the name plate or the turned card: ${at}`).to.be.at.most(limit + SLACK_PX / 2);
            });
            if (position === "ATK") {
              (seen.atk ??= {})[lane] = boxes;
            } else {
              // DEF rotates neither the control nor the marks, and moves neither: the stack starts where it did, and is cut
              // shorter, so the marks that no longer fit are the "+n" (which may be as wide as it likes).
              const before = seen.atk?.[lane] ?? [];
              expect(boxes.length, `no more marks in DEF than in ATK: ${at}`).to.be.at.most(before.length);
              const was = before[0];
              expect(was, `the control was measured in ATK: ${at}`).to.not.eq(undefined);
              const now = boxes[0];
              if (was === undefined || now === undefined) return;
              for (const side of ["left", "top", "right", "bottom"] as const) {
                expect(now[side], `the control's ${side} is the same in DEF: ${at}`).to.be.closeTo(was[side], 0.5);
              }
              boxes.forEach((box, index) => {
                const earlier = before[index];
                if (earlier === undefined) return;
                expect(box.left, `a mark's left is the same in DEF: ${at}`).to.be.closeTo(earlier.left, 0.5);
                expect(box.top, `a mark's top is the same in DEF: ${at}`).to.be.closeTo(earlier.top, 0.5);
              });
            }
          }
        });
      };

      check("ATK");
      check("DEF");
    });

    it(`#3257 marks that do not fit above the name plate fold into the +n, and the stack stays inside its zone, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      const base = fullBoardView();
      const first = base.you.units[0];
      if (first === null || first === undefined) throw new Error("fullBoardView has a unit in lane 1");
      const crowded = {
        ...first,
        keywords: (["Rush", "Can't Defend", "Charge", "First Strike", "Poisonous", "Lifesteal"] as const).map((kind) => ({ kind })),
        activations: [PING],
      };
      const view = { ...base, you: { ...base.you, units: base.you.units.map((each, index) => (index === 0 ? crowded : each)) } };
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={view} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);
      cy.document({ log: false }).should((doc) => {
        const zone = unitZone(doc, 1);
        const stack = zone.querySelector<HTMLElement>(":scope > .field-badges");
        const card = zone.querySelector<HTMLElement>(":scope > .card");
        expect(stack, `lane 1 holds its badge stack at ${where}`).to.not.eq(null);
        expect(card, `lane 1 holds its card at ${where}`).to.not.eq(null);
        if (stack === null || card === null) return;
        const more = stack.querySelector<HTMLElement>(".cf-kw-more");
        expect(more, `the marks that do not fit fold into a +n at ${where}`).to.not.eq(null);
        const folded = /^\+(\d+)$/.exec(more?.textContent ?? "");
        expect(folded, `the +n reads as a count at ${where}`).to.not.eq(null);
        // Six marks, of which the face's own cap draws at most two: four or more fold.
        expect(Number(folded?.[1] ?? 0), `at least four marks folded at ${where}`).to.be.at.least(4);
        const zoneBox = zone.getBoundingClientRect();
        const boxes = [...stack.querySelectorAll<HTMLElement>(MARKS)].map((each) => each.getBoundingClientRect());
        for (const box of boxes) {
          expect(box.left, `a mark is inside its zone's left at ${where}`).to.be.at.least(zoneBox.left - SLACK_PX);
          expect(box.top, `a mark is inside its zone's top at ${where}`).to.be.at.least(zoneBox.top - SLACK_PX);
          expect(box.right, `a mark is inside its zone's right at ${where}`).to.be.at.most(zoneBox.right + SLACK_PX);
          expect(box.bottom, `a mark is inside its zone's bottom at ${where}`).to.be.at.most(zoneBox.bottom + SLACK_PX);
        }
        const name = card.querySelector<HTMLElement>(".card-name")?.getBoundingClientRect();
        expect(name, `the card has its name at ${where}`).to.not.eq(undefined);
        if (name !== undefined) {
          expect(Math.max(...boxes.map((box) => box.bottom)), `the lowest mark is above the name at ${where}`).to.be.at.most(name.top + SLACK_PX / 2);
        }
      });
    });

    it(`#3315 a DEF unit's keyword marks never vanish: the stack keeps a mark or the +n, with and without Activate, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      const base = fullBoardView();
      const first = base.you.units[0];
      if (first === null || first === undefined) throw new Error("fullBoardView has a unit in lane 1");
      const check = (withActivate: boolean): void => {
        const turned = {
          ...first,
          position: "DEF" as const,
          keywords: [{ kind: "Rush" as const }, { kind: "Can't Defend" as const }],
          activations: withActivate ? [PING] : [],
        };
        const view = { ...base, you: { ...base.you, units: base.you.units.map((each, index) => (index === 0 ? turned : each)) } };
        cy.mount(
          <div className="app-shell app-shell--wide">
            <Game view={view} legal={[]} onAction={() => undefined} />
          </div>,
        );
        cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);
        cy.document({ log: false }).should((doc) => {
          const at = `${withActivate ? "with" : "without"} Activate at ${where}`;
          const zone = unitZone(doc, 1);
          const stack = zone.querySelector<HTMLElement>(":scope > .field-badges");
          const card = zone.querySelector<HTMLElement>(":scope > .card");
          expect(stack, `lane 1 holds its badge stack ${at}`).to.not.eq(null);
          expect(card?.dataset.position, `lane 1's unit is in DEF ${at}`).to.eq("DEF");
          if (stack === null) return;
          expect(stack.querySelector(".activate-control") !== null, `the control is there if and only if the unit has an ability ${at}`).to.eq(withActivate);
          const more = stack.querySelector<HTMLElement>(".cf-kw-more");
          const folded = more === null ? 0 : Number(/^\+(\d+)$/.exec(more.textContent ?? "")?.[1] ?? 0);
          expect(stack.querySelectorAll(".keyword-icon").length + folded, `both marks are named or counted in the +n ${at}`).to.eq(2);
          // The band above a turned card is about 4px at 390x664: no named mark fits there, so the +n must stand for them.
          if (size.height === 664) expect(more, `the +n stands for the marks that do not fit ${at}`).to.not.eq(null);
          const zoneBox = zone.getBoundingClientRect();
          for (const mark of stack.querySelectorAll<HTMLElement>(MARKS)) {
            const box = mark.getBoundingClientRect();
            expect(box.left, `a mark is inside its zone's left ${at}`).to.be.at.least(zoneBox.left - SLACK_PX);
            expect(box.top, `a mark is inside its zone's top ${at}`).to.be.at.least(zoneBox.top - SLACK_PX);
            expect(box.right, `a mark is inside its zone's right ${at}`).to.be.at.most(zoneBox.right + SLACK_PX);
            expect(box.bottom, `a mark is inside its zone's bottom ${at}`).to.be.at.most(zoneBox.bottom + SLACK_PX);
          }
        });
      };
      check(true);
      check(false);
    });

    it(`#3257 the cost steps aside under the marks, and stays where there are none, at ${where}`, () => {
      cy.viewport(size.width, size.height);
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={badged("ATK")} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);
      cy.document({ log: false }).should((doc) => {
        const costOf = (lane: number): HTMLElement => {
          const cost = unitZone(doc, lane).querySelector<HTMLElement>(":scope > .card .cost-gem");
          expect(cost, `lane ${lane}'s unit has a cost gem at ${where}`).to.not.eq(null);
          return cost as HTMLElement;
        };
        expect(unitZone(doc, 3).querySelector(":scope > .field-badges"), `lane 3 holds marks at ${where}`).to.not.eq(null);
        expect(getComputedStyle(costOf(3)).visibility, `the cost under the marks is hidden at ${where}`).to.eq("hidden");
        // Lane 4's unit has only a Stack, which is no mark: no stack, and its cost stays.
        expect(unitZone(doc, 4).querySelector(":scope > .field-badges"), `lane 4 holds no marks at ${where}`).to.eq(null);
        expect(getComputedStyle(costOf(4)).visibility, `a cost with no marks over it stays at ${where}`).to.eq("visible");
      });
    });
  }

  it("#2932 an ATK unit's drawn area at 390x664 is larger than before #2900, and its height beats main's row", () => {
    cy.viewport(390, 664);
    mountBoard(false);
    cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

    const out: { measured?: Record<string, number | boolean> } = {};

    cy.document({ log: false }).should((doc) => {
      const board = doc.querySelector<HTMLElement>(BOARD);
      expect(board, "the board is mounted").to.not.eq(null);
      if (board === null) return;
      const beforeArea = BEFORE_2900_ATK_390X664.width * BEFORE_2900_ATK_390X664.height;
      const mainArea = MAIN_ATK_390X664.width * MAIN_ATK_390X664.height;
      const card = doc.querySelector<HTMLElement>(`${BOARD} .zone[data-row="units"] > .card[data-position="ATK"]`);
      expect(card, "an ATK unit is on the field").to.not.eq(null);
      if (card === null) return;
      const box = card.getBoundingClientRect();
      const newArea = box.width * box.height;
      expect(newArea, "an ATK unit's drawn area is larger than before #2900").to.be.greaterThan(beforeArea);
      expect(box.height, "an ATK unit is taller than main's row").to.be.greaterThan(MAIN_ATK_390X664.row);
      out.measured = {
        beforeArea: Math.round(beforeArea),
        mainArea: Math.round(mainArea),
        newArea: Math.round(newArea),
        newOverBefore: Number((newArea / beforeArea).toFixed(2)),
        newOverMain: Number((newArea / mainArea).toFixed(2)),
        reached1_5x: newArea >= 1.5 * beforeArea,
      };
    });

    cy.then(() => {
      cy.task("layout:report", { issue: 2932, size: "phone 390x664", ...out.measured }, { log: false });
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
