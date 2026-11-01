// Polish task 7: browser layout checks for B18, B29, B30, B31, B32, B43, B44, B45, B46, and B47.
// B28 is covered by board-layout.cy.tsx; its mount keeps this suite at the product's narrowest container.
// Measurements use `should` retries after each viewport change.

import Game from "../../../apps/web/src/game/Game.tsx";
import "../../../apps/web/src/practice/practice.css";
import { BackLink } from "../../../apps/web/src/routes/nav.tsx";
import { writeSettings, __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import {
  baseView,
  card,
  emptySide,
  faceUpBackrow,
  fullBoardView,
  pendingFor,
  unit,
} from "../../../apps/web/src/test/fixtures.ts";

type GameProps = Parameters<typeof Game>[0];
type View = GameProps["view"];
type Legal = GameProps["legal"];

/** S7: `--glow-ready: #4ade80` and `--glow-condition: #facc15`, as the browser computes them. */
const GREEN = "rgb(74, 222, 128)";
const YELLOW = "rgb(250, 204, 21)";

/** WCAG 2.5.5 / Apple HIG, the floor S11 sets for every touch target. */
const TOUCH_PX = 44;

/** Subpixel slack for edges the browser rounds; never enough to hide a real overlap. */
const EPSILON = 0.5;

const BOARD = '[data-testid="board"]';

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

function mountGame(view: View, legal: Legal = []): void {
  cy.mount(
    <div className="app-shell app-shell--wide">
      <Game view={view} legal={legal} onAction={() => undefined} />
    </div>,
  );
}

function rectOf(element: Element): DOMRect {
  return element.getBoundingClientRect();
}

function boxShadowOf(element: Element): string {
  const win = element.ownerDocument.defaultView;
  return win === null ? "" : win.getComputedStyle(element).boxShadow;
}

// B18: glow colors

/** Fixture separates ready, condition, unavailable, and switch-only cards for glow assertions. */
function glowView(): View {
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "g1", defId: "core-008", cost: 1 }),
        card({ instanceId: "y1", defId: "core-010", cost: 0, conditionActive: true }),
        card({ instanceId: "q1", defId: "core-053", cost: 3, conditionActive: true }),
        card({ instanceId: "r1", defId: "core-019", cost: 4, radiant: true }),
      ],
      units: [
        unit("p1", { instanceId: "fu", defId: "core-004", conditionActive: true }),
        unit("p1", { instanceId: "sw", defId: "core-008" }),
        null,
        null,
        null,
      ],
      backrow: [faceUpBackrow("p1", { instanceId: "fb", defId: "core-093", conditionActive: true }), null, null, null, null],
    }),
  });
}

const GLOW_LEGAL: Legal = [
  { type: "play", instanceId: "g1", zone: { row: "units", lane: 3 } },
  { type: "play", instanceId: "y1" },
  { type: "switchPosition", instanceId: "sw" },
  { type: "endTurn" },
  { type: "offerDraw" },
  { type: "concede" },
];

/** A computed `box-shadow`, one entry per layer (commas inside `rgb(...)` are not separators). */
function shadowLayers(shadow: string): string[] {
  return shadow.split(/,(?![^(]*\))/).map((layer) => layer.trim());
}

function expectShadow(testid: string, check: (shadow: string) => void): void {
  cy.get(ts(testid)).should(($element) => {
    check(boxShadowOf($element[0] as Element));
  });
}

describe("B18 the green and yellow glows in a real browser", () => {
  beforeEach(() => {
    cy.viewport(1280, 720);
    mountGame(glowView(), GLOW_LEGAL);
  });

  it("B18 a card with data-glow=ready has the green in its box-shadow", () => {
    cy.get(ts("hand-card-g1"))
      .should("have.attr", "data-glow", "ready")
      .and("not.have.attr", "data-condition-active");

    expectShadow("hand-card-g1", (shadow) => {
      expect(shadow, "the green glow").to.contain(GREEN);
    });
  });

  it("B18 a playable card whose condition holds is yellow and not green", () => {
    cy.get(ts("hand-card-y1"))
      .should("have.attr", "data-glow", "ready")
      .and("have.attr", "data-condition-active", "true");

    expectShadow("hand-card-y1", (shadow) => {
      expect(shadow, "the yellow glow").to.contain(YELLOW);
      expect(shadow, "yellow replaces green").not.to.contain(GREEN);
    });
  });

  it("B18 a hand card whose condition holds but which cannot be played has neither color", () => {
    cy.get(ts("hand-card-q1"))
      .should("have.attr", "data-condition-active", "true")
      .and("not.have.attr", "data-glow");

    expectShadow("hand-card-q1", (shadow) => {
      expect(shadow).not.to.contain(GREEN);
      expect(shadow).not.to.contain(YELLOW);
    });
  });

  it("B18 a unit on the field whose condition holds is yellow", () => {
    cy.get(ts("card-fu"))
      .should("have.attr", "data-condition-active", "true")
      .and("not.have.attr", "data-glow");

    expectShadow("card-fu", (shadow) => {
      expect(shadow, "the yellow glow").to.contain(YELLOW);
    });
  });

  it("B18 a backrow card whose condition holds is yellow", () => {
    cy.get(ts("card-fb"))
      .should("have.attr", "data-condition-active", "true")
      .and("not.have.attr", "data-glow");

    expectShadow("card-fb", (shadow) => {
      expect(shadow, "the yellow glow").to.contain(YELLOW);
    });
  });

  it("B18 a Radiant card wears no outer glow of its own: yellow outside a card means only a met condition", () => {
    cy.get(ts("hand-card-r1")).should("have.class", "radiant").and("not.have.attr", "data-condition-active");

    expectShadow("hand-card-r1", (shadow) => {
      for (const layer of shadowLayers(shadow).filter((one) => !one.includes("inset"))) {
        // #2164 writes the shadow as a `color-mix` of the palette's black, which Chrome computes as
        // `color(srgb 0 0 0 / 0.45)` rather than `rgba(0, 0, 0, 0.45)`: both are the same plain black.
        // Any colored glow (the yellow, the green, gold) still fails either spelling.
        expect(layer, "an outer shadow layer of a Radiant card is a plain dark shadow").to.match(
          /^(rgba\(0, 0, 0, |color\(srgb 0 0 0 \/ )/,
        );
      }
    });
  });

  it("B18 a legal card without data-glow has neither color", () => {
    cy.get(ts("card-sw"))
      .should("have.attr", "data-legal", "true")
      .and("not.have.attr", "data-glow");

    expectShadow("card-sw", (shadow) => {
      expect(shadow).not.to.contain(GREEN);
      expect(shadow).not.to.contain(YELLOW);
    });
  });
});

/** Nothing but ending the turn is left: `highlightFor` glows `end-turn` (B15). */
const ONLY_END_TURN: Legal = [{ type: "endTurn" }, { type: "offerDraw" }, { type: "concede" }];

describe("B18 End turn turns green once nothing else is left", () => {
  for (const [width, height] of [
    [1280, 720],
    [390, 844],
    [844, 390],
  ] as const) {
    it(`B18 at ${width}x${height} end-turn's computed box-shadow carries the green, and its fill is green too`, () => {
      cy.viewport(width, height);
      mountGame(fullBoardView(), ONLY_END_TURN);

      cy.get(ts("end-turn")).should("have.attr", "data-glow", "ready");
      expectShadow("end-turn", (shadow) => {
        expect(shadow, "End turn's green ring").to.contain(GREEN);
      });
      cy.get(ts("end-turn")).should(($button) => {
        const win = $button[0]?.ownerDocument.defaultView;
        const image = win === null || win === undefined ? "" : win.getComputedStyle($button[0] as Element).backgroundImage;
        expect(image, "the amber fill gave way to a green one").to.contain("rgb(34, 197, 94)");
      });
    });
  }

  it("B18 while a move is left End turn stays amber, with no green", () => {
    cy.viewport(1280, 720);
    mountGame(glowView(), GLOW_LEGAL);

    cy.get(ts("end-turn")).should("not.have.attr", "data-glow");
    expectShadow("end-turn", (shadow) => {
      expect(shadow).not.to.contain(GREEN);
    });
  });
});

// B29: touch targets

const TOUCH_VIEWPORTS = [
  { label: "phone portrait", width: 390, height: 844 },
  { label: "phone landscape", width: 844, height: 390 },
  { label: "tablet", width: 768, height: 1024 },
] as const;

/** B29's list, besides the zones. `power` needs a hero power, which `fullBoardView` has. */
const TOUCH_TARGETS = [
  "end-turn",
  "offer-draw",
  "concede",
  "settings-open-game",
  "power",
  "hero-you",
  "hero-opponent",
] as const;

function expectTouchSize($element: JQuery<HTMLElement>, what: string): void {
  const box = rectOf($element[0] as Element);
  expect(box.width, `${what} is at least ${TOUCH_PX} px wide`).to.be.at.least(TOUCH_PX - EPSILON);
  expect(box.height, `${what} is at least ${TOUCH_PX} px tall`).to.be.at.least(TOUCH_PX - EPSILON);
}

/** Keep switch targets in zone corners, clear of the tile middle. */
const SWITCH_TARGET_PX = 44;
const SWITCH_CLEAR_PX = 6;

function switchesLegal(view: View): Legal {
  return view.you.units.flatMap((u) => (u === null ? [] : [{ type: "switchPosition" as const, instanceId: u.instanceId }]));
}

/** Tile-center taps must remain on cards; disabled enemy switches pass through to their cards. */
function expectMiddlesToCards(): void {
  cy.get(`${BOARD} .field .card-unit`).should("have.length", 10);
  cy.document({ log: false }).should((doc) => {
    const problems: string[] = [];
    for (const card of doc.querySelectorAll<HTMLElement>(`${BOARD} .field .card-unit`)) {
      const id = card.getAttribute("data-testid") ?? "?";
      const box = rectOf(card);
      for (const [label, x, y] of [
        ["middle", box.left + box.width / 2, box.top + box.height / 2],
        ["upper middle", box.left + box.width / 2, box.top + box.height / 3],
      ] as const) {
        const owner = doc.elementFromPoint(x, y)?.closest("[data-testid]") ?? null;
        if (owner !== card) problems.push(`${id} ${label} → ${owner?.getAttribute("data-testid") ?? "nothing"}`);
      }
      // Disabled switches pass through to the card or zone beneath them.
      const zone = card.parentElement;
      const button = zone?.querySelector<HTMLButtonElement>(":scope > .switch-button") ?? null;
      if (button?.disabled === true) {
        const glyph = rectOf(button.querySelector(".switch-glyph") ?? button);
        const owner = doc.elementFromPoint(glyph.left + glyph.width / 2, glyph.top + glyph.height / 2)?.closest("[data-testid]") ?? null;
        if (owner !== card && owner !== zone) problems.push(`${id}'s disabled switch → ${owner?.getAttribute("data-testid") ?? "nothing"}`);
      }
    }
    expect(problems, "taps that miss their unit").to.deep.eq([]);
  });
}

describe("B29 touch targets are at least 44x44 px on phones and tablets", () => {
  for (const viewport of TOUCH_VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`B29 the controls, the gear, the power and both heroes measure at least 44x44 at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      mountGame(fullBoardView());

      // #2645: on a phone Offer draw and Concede are rows of ⋯'s menu, which opens before they are measured.
      if (viewport.label !== "tablet") {
        cy.get(ts("match-menu")).should(($element) => {
          expectTouchSize($element, `match-menu at ${where}`);
        });
        cy.get(ts("match-menu")).click();
      }

      for (const testid of TOUCH_TARGETS) {
        cy.get(ts(testid)).should(($element) => {
          expectTouchSize($element, `${testid} at ${where}`);
        });
      }
    });

    // Switch targets shrink before taking a tile center and accept input only when legal.
    it(`B29 the switch (⟳) takes a tap across a target of up to 44x44 at its zone's corner, short of the tile's middle, at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const view = fullBoardView();
      mountGame(view, switchesLegal(view));

      cy.get(`${BOARD} .zone > [data-testid^="switch-"]:not(:disabled)`)
        .should("have.length.at.least", 3)
        .each(($switch) => {
          cy.wrap($switch, { log: false }).should(($element) => {
            const button = $element[0] as HTMLElement;
            const box = rectOf(button);
            const zoneElement = button.parentElement as HTMLElement;
            const zone = rectOf(zoneElement);
            const inner = {
              top: zone.top + zoneElement.clientTop,
              right: zone.left + zoneElement.clientLeft + zoneElement.clientWidth,
              width: zoneElement.clientWidth,
              height: zoneElement.clientHeight,
            };
            const doc = button.ownerDocument;
            const side = (room: number): number => Math.min(SWITCH_TARGET_PX, room / 2 - SWITCH_CLEAR_PX);
            expect(box.width, "the target's width").to.be.closeTo(side(inner.width), 1);
            expect(box.height, "the target's height").to.be.closeTo(side(inner.height), 1);
            expect(box.right, "at the zone's right edge").to.be.closeTo(inner.right, 1);
            expect(box.top, "at the zone's top edge").to.be.closeTo(inner.top, 1);
            if (viewport.label === "tablet") expectTouchSize($element, `${$switch.attr("data-testid") ?? "a switch"} on a tablet`);
            const inset = 2;
            for (const [px, py] of [
              [box.left + inset, box.top + inset],
              [box.right - inset, box.bottom - inset],
              [box.left + inset, box.bottom - inset],
              [box.left + box.width / 2, box.top + box.height / 2],
            ] as const) {
              const hit = doc.elementFromPoint(px, py)?.closest("button") ?? null;
              expect(hit, `a tap at (${Math.round(px)}, ${Math.round(py)}) near ${$switch.attr("data-testid") ?? "a switch"}`).to.eq(button);
            }
          });
        });
    });

    it(`B29 the middle of every unit's tile takes a tap for the card, and a switch that cannot be pressed takes none, at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const view = fullBoardView();
      mountGame(view, switchesLegal(view));
      expectMiddlesToCards();
    });

    it(`B29 a graveyard or exile pile that holds cards takes a tap across a 44 px band at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      mountGame(fullBoardView());

      // The pill's ::after pseudo-element is its finger-sized target.
      cy.get(`${BOARD} [data-browsable="true"]`)
        .should("have.length", 3)
        .each(($pile) => {
          cy.wrap($pile, { log: false }).should(($element) => {
            const pile = $element[0] as HTMLElement;
            const box = rectOf(pile);
            const doc = pile.ownerDocument;
            const reach = TOUCH_PX / 2 - 2;
            for (const [x, y] of [
              [box.left + box.width / 2, box.top + box.height / 2 - reach],
              [box.left + box.width / 2, box.top + box.height / 2 + reach],
            ] as const) {
              const hit = doc.elementFromPoint(x, y);
              expect(
                hit !== null && (hit === pile || pile.contains(hit)),
                `a tap at (${Math.round(x)}, ${Math.round(y)}) reaches ${$pile.attr("data-testid") ?? "a pile"}`,
              ).to.eq(true);
            }
          });
        });
    });

    it(`B29 every one of the 20 zones measures at least 44x44 at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      mountGame(fullBoardView());

      cy.get(`${BOARD} [data-testid^="zone-"]`)
        .should("have.length", 20)
        .each(($zone) => {
          cy.wrap($zone, { log: false }).should(($element) => {
            expectTouchSize($element, `${$zone.attr("data-testid") ?? "a zone"} at ${where}`);
          });
        });
    });
  }
});

/** Extra-small tiles and desktop must also keep switch targets from their centers. */
const SMALL_TILE_VIEWPORTS = [
  { label: "phone, Safari's visible viewport", width: 390, height: 664 },
  { label: "small phone", width: 360, height: 640 },
  { label: "small phone landscape", width: 667, height: 375 },
  { label: "desktop", width: 1280, height: 720 },
] as const;

describe("B29 the switch never takes the middle of a small tile", () => {
  for (const viewport of SMALL_TILE_VIEWPORTS) {
    it(`B29 the middle of every unit's tile takes a tap for the card at ${viewport.label} ${viewport.width}x${viewport.height}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const view = fullBoardView();
      mountGame(view, switchesLegal(view));
      expectMiddlesToCards();
    });
  }
});

// B30: hand fan

const FAN_DEFS = ["core-002", "core-005", "core-008", "core-011", "core-019", "core-020", "core-043", "core-053", "core-055", "core-077"];

function fanView(n: number): View {
  const full = fullBoardView();
  const hand = Array.from({ length: n }, (_unused, index) =>
    card({ instanceId: `fan${index + 1}`, defId: FAN_DEFS[index % FAN_DEFS.length] ?? "core-001", cost: index % 7 }),
  );
  return {
    ...full,
    you: { ...full.you, hand },
    opponent: { ...full.opponent, hand: { count: n } },
  };
}

function handCards(hand: HTMLElement): DOMRect[] {
  return [...hand.querySelectorAll('[data-testid^="hand-card-"]')].map(rectOf);
}

function exposedStrips(cards: readonly DOMRect[]): number[] {
  const strips: number[] = [];
  for (let index = 0; index + 1 < cards.length; index += 1) {
    const here = cards[index];
    const next = cards[index + 1];
    if (here !== undefined && next !== undefined) strips.push(Math.abs(next.left - here.left));
  }
  return strips;
}

function expectFan(n: number, minimumStrip: number, where: string): void {
  cy.get(ts("hand-you")).should(($hand) => {
    const hand = $hand[0] as HTMLElement;
    const box = rectOf(hand);
    const cards = handCards(hand);
    expect(cards, `${n} cards in your hand`).to.have.length(n);

    cards.forEach((card, index) => {
      expect(card.left, `card ${index + 1} starts inside hand-you at ${where}`).to.be.at.least(box.left - EPSILON);
      expect(card.right, `card ${index + 1} ends inside hand-you at ${where}`).to.be.at.most(box.right + EPSILON);
    });

    const narrowest = Math.min(...exposedStrips(cards));
    expect(narrowest, `each card of a ${n}-card hand shows at least ${minimumStrip} px at ${where}`).to.be.at.least(
      minimumStrip - EPSILON,
    );
  });
}

describe("B30 the hand fan fits a phone and never overlaps on a desktop", () => {
  it("B30 at 390x844 a 10-card hand stays inside hand-you with at least 28 px of each card showing", () => {
    cy.viewport(390, 844);
    mountGame(fanView(10));

    expectFan(10, 28, "390x844");
  });

  it("B30 at 390x844 a 7-card hand shows at least 44 px of each card", () => {
    cy.viewport(390, 844);
    mountGame(fanView(7));

    expectFan(7, TOUCH_PX, "390x844");
  });

  it("B30 at 1280x720 no two cards of a 10-card hand overlap", () => {
    cy.viewport(1280, 720);
    mountGame(fanView(10));

    cy.get(ts("hand-you")).should(($hand) => {
      const cards = handCards($hand[0] as HTMLElement).sort((a, b) => a.left - b.left);
      expect(cards).to.have.length(10);
      for (let index = 0; index + 1 < cards.length; index += 1) {
        const here = cards[index];
        const next = cards[index + 1];
        if (here === undefined || next === undefined) continue;
        expect(here.right, `card ${index + 1} ends before card ${index + 2} starts`).to.be.at.most(next.left + EPSILON);
      }
    });
  });
});

// B31: phone grids and height budget

describe("B31 the phone layouts fit the height and hide the log", () => {
  it("B31 at 844x390 the controls sit right of the field, the board ends on the screen, and the log is hidden", () => {
    cy.viewport(844, 390);
    mountGame(fullBoardView());

    cy.get(BOARD).should(($board) => {
      const board = $board[0] as HTMLElement;
      const field = board.querySelector(".field");
      const controls = board.querySelector(".control-bar");
      expect(field, "the field").to.not.eq(null);
      expect(controls, "the control bar").to.not.eq(null);
      if (field === null || controls === null) return;

      expect(rectOf(controls).left, "the control bar starts at or right of the field's right edge").to.be.at.least(
        rectOf(field).right - EPSILON,
      );
      // B46's vertical shell padding leaves 378px for the board.
      expect(rectOf(board).height, "the board fits 390 px of height less the shell's padding").to.be.at.most(378);
      expect(rectOf(board).bottom, "the board ends inside the viewport").to.be.at.most(390 + EPSILON);
    });
    cy.get(ts("log")).should("not.be.visible");
  });

  it("B31 at 390x844 the board is at most 820 px tall and the log is hidden", () => {
    cy.viewport(390, 844);
    mountGame(fullBoardView());

    cy.get(BOARD).should(($board) => {
      expect(rectOf($board[0] as Element).height).to.be.at.most(820);
    });
    cy.get(ts("log")).should("not.be.visible");
  });

  it("B31 at 1280x720 the log is visible", () => {
    cy.viewport(1280, 720);
    mountGame(fullBoardView());

    cy.get(ts("log")).should("be.visible");
  });
});

// B32: prompt as bottom sheet

type PromptKindUnderTest = "discover" | "target" | "mode";

function promptView(kind: PromptKindUnderTest): View {
  const options = {
    discover: [
      { key: "mode:core-043", label: "Flood", defId: "core-043" },
      { key: "mode:core-055", label: "Archivist", defId: "core-055" },
      { key: "mode:core-066", label: "Lava Golem", defId: "core-066" },
    ],
    target: [
      { key: "instance:pe1", label: "Enemy unit", instanceId: "pe1" },
      { key: "hero:p2", label: "Enemy hero", player: "p2" as const },
    ],
    mode: [
      { key: "mode:deal", label: "Deal 2" },
      { key: "mode:draw", label: "Draw 1" },
    ],
  }[kind];

  return baseView({
    you: emptySide("p1", { hand: [card({ instanceId: "ph1", defId: "core-002" })] }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "pe1", defId: "core-008" }), null, null, null, null],
    }),
    pending: pendingFor(kind, options),
  });
}

describe("B32 a prompt is a bottom sheet on a phone and a centered modal on a desktop", () => {
  for (const kind of ["discover", "target", "mode"] as const) {
    it(`B32 at 390x844 a ${kind} prompt spans the width, sits on the bottom edge, and its buttons are at least 44 px tall`, () => {
      cy.viewport(390, 844);
      mountGame(promptView(kind));

      cy.get(ts("prompt-modal")).should(($modal) => {
        const box = rectOf($modal[0] as Element);
        expect(box.left, "the sheet starts at the viewport's left edge").to.be.at.most(EPSILON);
        expect(box.right, "the sheet ends at the viewport's right edge").to.be.at.least(390 - EPSILON);
        expect(Math.abs(box.bottom - 844), "the sheet's bottom edge is the viewport's").to.be.at.most(1);
      });

      cy.get(ts("prompt-submit")).should(($submit) => {
        expect(rectOf($submit[0] as Element).height, "prompt-submit").to.be.at.least(TOUCH_PX - EPSILON);
      });

      cy.get('[data-testid^="prompt-option-"]')
        .should("have.length.at.least", 2)
        .each(($option) => {
          cy.wrap($option, { log: false }).should(($element) => {
            expect(
              rectOf($element[0] as Element).height,
              `${$option.attr("data-testid") ?? "an option"} is at least ${TOUCH_PX} px tall`,
            ).to.be.at.least(TOUCH_PX - EPSILON);
          });
        });
    });
  }

  it("B32 at 1280x720 the prompt is a centered modal narrower than the viewport, not a sheet on the bottom edge", () => {
    cy.viewport(1280, 720);
    mountGame(promptView("discover"));

    cy.get(ts("prompt-modal")).should(($modal) => {
      const box = rectOf($modal[0] as Element);
      expect(box.width, "narrower than the viewport").to.be.below(1280);
      expect(Math.abs((box.left + box.right) / 2 - 640), "centered horizontally").to.be.at.most(2);
      expect(box.bottom, "not resting on the bottom edge").to.be.below(720 - 1);
    });
  });
});

// B43: log on a phone

describe("B43 a phone opens the log from the control bar", () => {
  for (const [width, height] of [
    [390, 844],
    [844, 390],
  ] as const) {
    it(`B43 at ${width}x${height} log-toggle is a 44 px button that shows the log over the board and hides it again`, () => {
      cy.viewport(width, height);
      mountGame(fullBoardView());

      cy.get(ts("log")).should("not.be.visible");
      cy.get(ts("log-toggle")).should(($toggle) => {
        expectTouchSize($toggle, "log-toggle");
      });
      cy.get(ts("log-toggle")).click();
      cy.get(ts("log")).should("be.visible").and(($log) => {
        const box = rectOf($log[0] as Element);
        expect(box.top, "the log opens over the top of the board").to.be.below(height / 2);
      });
      cy.get(ts("log-toggle")).click();
      cy.get(ts("log")).should("not.be.visible");
    });
  }

  it("B43 at 1280x720 the log has its own place and there is no toggle", () => {
    cy.viewport(1280, 720);
    mountGame(fullBoardView());

    cy.get(ts("log")).should("be.visible");
    cy.get(ts("log-toggle")).should("not.be.visible");
  });
});

// B44 and B45: tapping a hand card on a phone

/** Seven crowded hand cards include board-target, hero-target, and unplayable cases. */
function tapView(): View {
  return baseView({
    you: emptySide("p1", {
      mana: { current: 5, max: 5 },
      hand: [
        card({ instanceId: "t2", defId: "core-053", cost: 2 }),
        card({ instanceId: "t1", defId: "core-011", cost: 1 }),
        card({ instanceId: "t5", defId: "core-043", cost: 8 }),
        card({ instanceId: "t4", defId: "core-020", cost: 8 }),
        card({ instanceId: "t6", defId: "core-055", cost: 8 }),
        card({ instanceId: "t7", defId: "core-008", cost: 8 }),
        card({ instanceId: "t3", defId: "core-077", cost: 9 }),
      ],
    }),
    opponent: emptySide("p2", { hand: { count: 5 }, units: [unit("p2", { instanceId: "e1" }), null, null, null, null] }),
  });
}

const TAP_LEGAL: Legal = [
  { type: "play", instanceId: "t1", zone: { row: "units", lane: 2 } },
  { type: "play", instanceId: "t1", zone: { row: "units", lane: 3 } },
  { type: "play", instanceId: "t2", targets: [{ pick: "hero", player: "p1" }] },
  { type: "play", instanceId: "t2", targets: [{ pick: "hero", player: "p2" }] },
  { type: "endTurn" },
];

const PHONES = [
  [390, 844],
  [844, 390],
] as const;

/** Wait for the lift transition to settle before measuring its card. */
function settledBox(testid: string, check: (box: DOMRect, element: HTMLElement) => void): void {
  let last = "";
  cy.get(ts(testid)).should(($card) => {
    const box = rectOf($card[0] as Element);
    const now = `${box.left.toFixed(1)},${box.top.toFixed(1)},${box.width.toFixed(1)},${box.height.toFixed(1)}`;
    const same = now === last;
    last = now;
    expect(same, "the lift has settled").to.eq(true);
    check(box, $card[0] as HTMLElement);
  });
}

describe("B45 lifting a hand card on a phone", () => {
  afterEach(() => {
    __resetSettingsForTests();
    localStorage.clear();
  });

  for (const [width, height] of PHONES) {
    it(`B45 at ${width}x${height} a tap on a card you cannot play lifts it at least 1.5 times as big, opaque, and on the screen`, () => {
      cy.viewport(width, height);
      mountGame(tapView(), TAP_LEGAL);

      const resting: { box?: DOMRect } = {};
      settledBox("hand-card-t4", (box) => {
        resting.box = box;
      });
      cy.get(ts("hand-card-t4")).click();
      cy.get(ts("hand-card-t4")).closest(".hand-slot").should("have.attr", "data-lifted", "true");
      settledBox("hand-card-t4", (box, element) => {
        const before = resting.box as DOMRect;
        expect(box.width, "lifted to be read").to.be.at.least(before.width * 1.5 - EPSILON);
        expect(element.ownerDocument.defaultView?.getComputedStyle(element).opacity, "not see-through").to.eq("1");
        expect(box.left, "inside the screen's left edge").to.be.at.least(-EPSILON);
        expect(box.right, "inside the screen's right edge").to.be.at.most(width + EPSILON);
        expect(box.top, "inside the screen's top edge").to.be.at.least(-EPSILON);
      });
    });

    it(`B45 at ${width}x${height} the card at the end of the hand grows away from the screen's edge`, () => {
      cy.viewport(width, height);
      mountGame(tapView(), TAP_LEGAL);

      cy.get(ts("hand-card-t3")).click();
      settledBox("hand-card-t3", (box) => {
        expect(box.right, "inside the screen's right edge").to.be.at.most(width + EPSILON);
      });
    });

    it(`B45 at ${width}x${height} a card being played stays low, and your hero glowing as its target is never under it`, () => {
      cy.viewport(width, height);
      mountGame(tapView(), TAP_LEGAL);

      cy.get(ts("hand-card-t2")).click();
      cy.get(ts("hero-you")).should("have.attr", "data-glow", "ready");
      settledBox("hand-card-t2", () => undefined);
      cy.get(ts("hero-you")).should(($hero) => {
        const hero = $hero[0] as HTMLElement;
        const box = rectOf(hero);
        const doc = hero.ownerDocument;
        for (const fx of [0.2, 0.5, 0.8]) {
          for (const fy of [0.3, 0.5, 0.7]) {
            const hit = doc.elementFromPoint(box.left + box.width * fx, box.top + box.height * fy);
            expect(hit !== null && hero.contains(hit), `hero-you is on top at (${fx}, ${fy})`).to.eq(true);
          }
        }
      });
    });
  }

  it("B45 at 390x844 keyboard focus on a card its neighbour covers raises it into view", () => {
    cy.viewport(390, 844);
    mountGame(tapView(), TAP_LEGAL);

    // Legal t1 takes focus while t2 overlaps it.
    cy.get(ts("hand-card-t1")).then(($card) => {
      ($card[0] as HTMLElement & { focus(options?: { focusVisible?: boolean }): void }).focus({ focusVisible: true });
    });
    cy.get(ts("hand-card-t1")).should(($card) => {
      const card = $card[0] as HTMLElement;
      expect(card.matches(":focus-visible"), "the card has keyboard focus").to.eq(true);
      const box = rectOf(card);
      // At rest, t5 covers the card's right edge.
      const hit = card.ownerDocument.elementFromPoint(box.left + box.width * 0.85, box.top + box.height / 2);
      expect(hit !== null && card.contains(hit), "the focused card is on top of its neighbours").to.eq(true);
    });
  });
});

describe("B44 a play's board pick on a phone held upright", () => {
  afterEach(() => {
    __resetSettingsForTests();
    localStorage.clear();
  });

  it("B44 with drag to play on, a tap on a playable card opens a slim bar that covers neither that card nor the hand", () => {
    cy.viewport(390, 844);
    mountGame(tapView(), TAP_LEGAL);

    cy.get(ts("hand-card-t1")).click();
    cy.get(ts("zone-you-units-2")).should("have.attr", "data-glow", "ready");
    cy.get(ts("prompt-modal")).should("have.attr", "data-prompt-source", "play").find(".prompt-zones").should("not.be.visible");
    settledBox("hand-card-t1", () => undefined);
    cy.get(ts("prompt-modal")).should(($bar) => {
      const bar = rectOf($bar[0] as Element);
      const hand = rectOf($bar[0]?.ownerDocument.querySelector(ts("hand-you")) as Element);
      const lifted = rectOf($bar[0]?.ownerDocument.querySelector(ts("hand-card-t1")) as Element);
      expect(bar.height, "a bar, not a sheet").to.be.at.most(72);
      expect(bar.top, "below the hand").to.be.at.least(hand.bottom - EPSILON);
      expect(bar.top, "below the card that was tapped").to.be.at.least(lifted.bottom - EPSILON);
    });
    // Without options, the prompt describes board targets within the bar.
    cy.get(ts("prompt-modal"))
      .find(".prompt-board-hint")
      .should("be.visible")
      .and("have.text", "Tap a highlighted zone on the board.")
      .and(($hint) => {
        const hint = rectOf($hint[0] as Element);
        const bar = rectOf($hint[0]?.closest(ts("prompt-modal")) as Element);
        expect(hint.bottom, "inside the bar").to.be.at.most(bar.bottom + EPSILON);
      });
    cy.get(ts("zone-you-units-3")).click();
    cy.get(ts("prompt-modal")).should("not.exist");
  });

  it("B44 with drag to play off, the same tap opens the full picker sheet", () => {
    cy.viewport(390, 844);
    cy.then(() => {
      writeSettings({ dragToPlay: false });
    });
    mountGame(tapView(), TAP_LEGAL);

    cy.get(ts("hand-card-t1")).click();
    cy.get(ts("prompt-modal")).find(".prompt-zones").should("be.visible");
    cy.get(ts("prompt-modal")).find(".prompt-board-hint").should("not.be.visible");
  });
});

// B47: a card picker folded away to show the board

/** `transparent`, as the browser computes a background. */
const CLEAR = "rgba(0, 0, 0, 0)";

function backgroundOf(element: Element): string {
  const win = element.ownerDocument.defaultView;
  return win === null ? "" : win.getComputedStyle(element).backgroundColor;
}

describe("B47 a card picker folds away to show the board", () => {
  it("B47 at 390x844 a folded Discover is a bar below the field over an undimmed board, and unfolds with its options", () => {
    cy.viewport(390, 844);
    mountGame(promptView("discover"));

    cy.get(ts("prompt-scrim")).should(($scrim) => {
      expect(backgroundOf($scrim[0] as Element), "the open picker dims the board").not.to.eq(CLEAR);
    });
    cy.get(ts("prompt-peek")).should(($peek) => {
      expectTouchSize($peek, "prompt-peek");
    });
    cy.get(ts("prompt-peek")).click();

    cy.get(ts("prompt-scrim")).should("have.attr", "data-collapsed", "true").and(($scrim) => {
      expect(backgroundOf($scrim[0] as Element), "the folded picker leaves the board undimmed").to.eq(CLEAR);
    });
    cy.get(ts("prompt-modal"))
      .should("have.attr", "data-collapsed", "true")
      .and(($bar) => {
        const bar = rectOf($bar[0] as Element);
        const field = rectOf($bar[0]?.ownerDocument.querySelector(`${BOARD} .field`) as Element);
        expect(bar.height, "a bar, not a sheet").to.be.at.most(72);
        expect(bar.top, "below the field").to.be.at.least(field.bottom - EPSILON);
      });
    cy.get('[data-testid^="prompt-option-"]').should("not.exist");
    cy.get(ts("prompt-peek")).should("have.attr", "aria-pressed", "true").click();

    cy.get(ts("prompt-modal")).should("not.have.attr", "data-collapsed");
    cy.get('[data-testid^="prompt-option-"]').should("have.length", 3);
  });
});

// B46: whole game screen fits

/** Mirror the match route's BackLink and match bar. */
function mountMatch(view: View, legal: Legal = []): void {
  cy.mount(
    <div className="app-shell app-shell--wide">
      <BackLink />
      <header className="match-bar">
        <span>
          match <code>m-4f2c9e1a</code> · seat <code>p1</code> · turn 5 · <code>open</code>
        </span>
        <span className="clock">0:42 · grace 1:30</span>
      </header>
      <Game view={view} legal={legal} onAction={() => undefined} />
    </div>,
  );
}

/** Mirror the hotseat route's hand-over bar. */
function mountHotseat(view: View, legal: Legal = []): void {
  cy.mount(
    <div className="app-shell app-shell--wide">
      <header className="hotseat-bar">
        <span>
          seed <code>b40-drag</code> · seat <code>p1</code> · turn 5 · active <code>p1</code>
        </span>
        <button type="button">Hand over to p2</button>
      </header>
      <Game view={view} legal={legal} onAction={() => undefined} />
    </div>,
  );
}

const SCREENS = [
  { label: "phone portrait", width: 390, height: 844 },
  { label: "phone landscape", width: 844, height: 390 },
  { label: "tablet", width: 768, height: 1024 },
  { label: "desktop", width: 1280, height: 720 },
  { label: "tablet landscape", width: 1024, height: 768 },
  { label: "phone, Safari's visible viewport", width: 390, height: 664 },
] as const;

function expectScreenFits(width: number, height: number, where: string): void {
  cy.document().should((doc) => {
    expect(doc.documentElement.scrollHeight, `nothing scrolls vertically at ${where}`).to.be.at.most(height);
    expect(doc.documentElement.scrollWidth, `nothing scrolls sideways at ${where}`).to.be.at.most(width);
    const endTurn = rectOf(doc.querySelector(ts("end-turn")) as Element);
    expect(endTurn.bottom, `End turn is on the screen at ${where}`).to.be.at.most(height + EPSILON);
    const cards = [...doc.querySelectorAll(`${ts("hand-you")} [data-testid^="hand-card-"]`)].map(rectOf);
    expect(cards.length, "your hand has cards").to.be.greaterThan(0);
    for (const card of cards) {
      // Enough of each card to see and to press: its top 24 px (the cost and the name).
      expect(card.top + 24, `a hand card shows its top at ${where}`).to.be.at.most(height + EPSILON);
    }
  });
}

describe("B46 the game screen, route bar included, fits the viewport", () => {
  for (const screen of SCREENS) {
    const where = `${screen.label} ${screen.width}x${screen.height}`;

    it(`B46 the match route fits at ${where}`, () => {
      cy.viewport(screen.width, screen.height);
      mountMatch(fullBoardView(), ONLY_END_TURN);
      expectScreenFits(screen.width, screen.height, where);
    });

    it(`B46 the hotseat route fits at ${where}`, () => {
      cy.viewport(screen.width, screen.height);
      mountHotseat(fullBoardView(), ONLY_END_TURN);
      expectScreenFits(screen.width, screen.height, where);
    });
  }

  it("B46 a tablet held landscape gets the desktop board: the controls in a sidebar right of the field", () => {
    cy.viewport(1024, 768);
    mountGame(fullBoardView());

    cy.get(BOARD).should(($board) => {
      const board = $board[0] as HTMLElement;
      const field = board.querySelector(".field");
      const controls = board.querySelector(".control-bar");
      if (field === null || controls === null) throw new Error("no field or control bar");
      expect(rectOf(controls).left, "the control bar starts right of the field").to.be.at.least(rectOf(field).right - EPSILON);
    });
  });
});

// #2645: the phone's match controls keep to one row

const BAR_VIEWPORTS = [
  { width: 320, height: 568 },
  { width: 360, height: 740 },
  { width: 375, height: 667 },
  { width: 390, height: 844 },
  { width: 412, height: 915 },
  { width: 844, height: 390 },
] as const;

/** The same board with the Marketplace open for the rest of the game (R1853000). */
function withMarketplace(): View {
  return { ...fullBoardView(), marketplace: { offers: [], sales: [] } };
}

/** An online match: Back leads the rail that rides in the control bar (Game.tsx's `boardRail`). */
function mountOnline(view: View, legal: Legal = []): void {
  cy.mount(
    <div className="app-shell app-shell--wide">
      <Game view={view} legal={legal} onAction={() => undefined} boardRail={<button type="button">Back</button>} />
    </div>,
  );
}

/** Mirror the practice route's table (practice-table.cy.tsx), whose stylesheet hides Offer draw. */
function mountPractice(view: View, legal: Legal = []): void {
  cy.mount(
    <div className="app-shell app-shell--wide practice practice--game">
      <header className="practice-hud" data-testid="practice-hud">
        <span className="practice-hud__tier">Practice Easy</span>
      </header>
      <div className="practice-board practice-table" data-difficulty="easy" data-thinking="false">
        <Game view={view} legal={legal} onAction={() => undefined} />
      </div>
    </div>,
  );
}

const BAR_ROUTES: readonly { label: string; mount: (view: View, legal?: Legal) => void }[] = [
  { label: "online, with a rail", mount: mountOnline },
  { label: "practice, with none", mount: mountPractice },
  { label: "hotseat, with none", mount: mountHotseat },
];

function overlap(a: DOMRect, b: DOMRect): boolean {
  return a.left < b.right - EPSILON && b.left < a.right - EPSILON && a.top < b.bottom - EPSILON && b.top < a.bottom - EPSILON;
}

/** The bar's controls as one row of them: End turn and every shown button that is the bar's own child. */
function expectOneRow(doc: Document, width: number, height: number, market: boolean, where: string): void {
  const bar = doc.querySelector<HTMLElement>(`${BOARD} .control-bar`);
  if (bar === null) throw new Error(`no control bar at ${where}`);
  const own = (testid: string): HTMLElement | null => bar.querySelector<HTMLElement>(`:scope > ${ts(testid)}`);
  const endTurn = bar.querySelector<HTMLElement>(ts("end-turn"));
  const named = { "log-toggle": own("log-toggle"), "match-menu": own("match-menu"), "settings-open-game": own("settings-open-game") };
  const shop = own("marketplace-button");

  expect(endTurn, `End turn at ${where}`).not.to.eq(null);
  for (const [testid, element] of Object.entries(named)) expect(element, `${testid} is a button of the bar at ${where}`).not.to.eq(null);
  expect(shop !== null, `the Marketplace's toggle is in the bar exactly when the view has one, at ${where}`).to.eq(market);
  for (const testid of ["offer-draw", "concede", "friends-button", "audio-toggle"]) {
    expect(own(testid), `${testid} is not a button of the bar at ${where}`).to.eq(null);
  }

  const controls = [
    endTurn,
    ...[...bar.querySelectorAll<HTMLElement>(":scope > button")].filter((button) => getComputedStyle(button).display !== "none"),
  ].filter((element): element is HTMLElement => element !== null);
  const boxes = controls.map((control) => ({ id: control.getAttribute("data-testid") ?? control.tagName, box: rectOf(control) }));

  for (const { id, box } of boxes) {
    expect(box.height, `${id} is one 44px line tall at ${where}`).to.be.within(TOUCH_PX - EPSILON, TOUCH_PX + EPSILON);
    expect(box.width, `${id} is at least 44px wide at ${where}`).to.be.at.least(TOUCH_PX - EPSILON);
    expect(box.left, `${id} starts on the screen at ${where}`).to.be.at.least(-EPSILON);
    expect(box.right, `${id} ends on the screen at ${where}`).to.be.at.most(width + EPSILON);
  }
  const landscape = width > height;
  if (landscape) {
    // The column is 176px: End turn and the gear on a first row, the rest on one below it.
    const first = boxes.filter(({ id }) => id === "end-turn" || id === "settings-open-game");
    const rest = boxes.filter(({ id }) => id !== "end-turn" && id !== "settings-open-game");
    for (const { id, box } of first) expect(box.top, `${id} shares End turn's row at ${where}`).to.be.closeTo(first[0]?.box.top ?? 0, 1);
    for (const { id, box } of rest) {
      expect(box.top, `${id} shares the second row at ${where}`).to.be.closeTo(rest[0]?.box.top ?? 0, 1);
      expect(box.top, `${id} is below End turn's row at ${where}`).to.be.at.least((first[0]?.box.bottom ?? 0) - EPSILON);
    }
  } else {
    for (const { id, box } of boxes) expect(box.top, `${id} shares one top with End turn at ${where}`).to.be.closeTo(boxes[0]?.box.top ?? 0, 1);
  }
  for (const [index, one] of boxes.entries()) {
    for (const other of boxes.slice(index + 1)) {
      expect(overlap(one.box, other.box), `${one.id} and ${other.id} overlap at ${where}`).to.eq(false);
    }
  }
  expect(doc.documentElement.scrollWidth, `nothing scrolls sideways at ${where}`).to.be.at.most(width);
}

describe("#2645 the phone's match controls keep to their row", () => {
  for (const viewport of BAR_VIEWPORTS) {
    for (const route of BAR_ROUTES) {
      const where = `${route.label} at ${String(viewport.width)}x${String(viewport.height)}`;

      it(`#2645 ${where}: End turn, the log, ⋯ and the gear share the bar's row, with the Marketplace's toggle or without`, () => {
        cy.viewport(viewport.width, viewport.height);
        route.mount(fullBoardView(), ONLY_END_TURN);
        cy.document().should((doc) => {
          expectOneRow(doc, viewport.width, viewport.height, false, `${where}, no marketplace`);
        });

        route.mount(withMarketplace(), ONLY_END_TURN);
        cy.document().should((doc) => {
          expectOneRow(doc, viewport.width, viewport.height, true, `${where}, with a marketplace`);
        });
      });
    }
  }

  it("#2645 at 360x740 the match menu opens above the bar with Offer draw and Concede on one 44px line each, inside the screen", () => {
    cy.viewport(360, 740);
    mountGame(fullBoardView(), ONLY_END_TURN);
    cy.get(ts("match-menu")).click();

    cy.document().should((doc) => {
      const bar = rectOf(doc.querySelector(`${BOARD} .control-bar`) as Element);
      const menu = rectOf(doc.querySelector(ts("match-menu-panel")) as Element);
      expect(menu.bottom, "the menu ends above the bar").to.be.at.most(bar.top + EPSILON);
      expect(menu.left, "the menu starts on the screen").to.be.at.least(-EPSILON);
      expect(menu.right, "the menu ends on the screen").to.be.at.most(360 + EPSILON);
      expect(menu.top, "the menu's top is on the screen").to.be.at.least(-EPSILON);
      for (const testid of ["offer-draw", "concede"]) {
        const row = rectOf(doc.querySelector(ts(testid)) as Element);
        expect(row.height, `${testid} is one 44px line`).to.be.within(TOUCH_PX - EPSILON, TOUCH_PX + EPSILON);
        expect(row.left, `${testid} is inside the menu`).to.be.at.least(menu.left - EPSILON);
        expect(row.right, `${testid} is inside the menu`).to.be.at.most(menu.right + EPSILON);
      }
    });
  });

  it("#2645 at 390x844 with a rail the Marketplace panel opens above the bar's top edge, inside the screen", () => {
    cy.viewport(390, 844);
    mountOnline(withMarketplace(), ONLY_END_TURN);
    cy.get(ts("marketplace-button")).click();

    cy.document().should((doc) => {
      const bar = rectOf(doc.querySelector(`${BOARD} .control-bar`) as Element);
      const panel = rectOf(doc.querySelector(ts("marketplace-panel")) as Element);
      expect(panel.bottom, "the panel ends above the bar, rail included").to.be.at.most(bar.top + EPSILON);
      expect(panel.left, "the panel starts on the screen").to.be.at.least(-EPSILON);
      expect(panel.right, "the panel ends on the screen").to.be.at.most(390 + EPSILON);
      expect(panel.top, "the panel's top is on the screen").to.be.at.least(-EPSILON);
    });
  });
});
