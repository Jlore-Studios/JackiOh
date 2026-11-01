// MN09 (#552), mobile: the prompt kinds the Meditative set adds, at phone width with a touch hold
// (docs/polish/7-mobile-ux.md; R2617000: on a touch screen a card is read by holding it, which shows
// its hover preview).
//
//   craft   ME-CRAFT (Meditative #17, R880): the block editor docked as one sheet
//   market  ME-MARKET (Meditative #42): the night market's stall
//
// At 360x780 and 390x844 each prompt is a bottom sheet across the whole width, never taller than the
// screen; nothing in it scrolls sideways, every control is a 44 px target, and the card it shows has a
// box inside the sheet. A hold on that card shows its preview and picks nothing: a lot is read, not bought.
//
// The play-time choices (R81) the set adds are no prompt kind of their own: a card's modes, picked in
// a "Choose one" before the play is sent (#617), each the same sheet under the same checks:
//
//   secret      #22 Mind Games' secret choice (ME-SECRET): Greed, Attack or Defend
//   prediction  #22.1 Fortify Mind's guess at it
//   lane        #98 Showdown's lane, the target that names a lane: Lane 1 to Lane 5
//
// The Created status (ME-CREATED, R943) of a card in hand: no mark on its face (R2647000), and a hold
// shows its preview, which says it was made during the game.
//
// The mount is mobile-ux.cy.tsx's (the narrowest box the board is ever given), with the real catalog,
// since the market draws its lots from it. The block editor's meters and face are the engine's own
// `craftPreview`, so the WebAssembly module is loaded first.

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PendingOption, PlayerView } from "@jackioh/shared";

import { INSPECT_HOVER, closeInspect } from "../../../apps/web/src/cards/index.ts";
import { testid } from "../../../apps/web/src/game/contract.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, card, emptySide, pendingFor } from "../../../apps/web/src/test/fixtures.ts";
import { loadWasm } from "../../../apps/web/src/wasm/index.ts";
import type { CraftRecipe } from "../../../apps/web/src/wire/index.ts";

/** WCAG 2.5.5 / Apple HIG, the floor docs/polish/7-mobile-ux.md sets for every touch target. */
const TOUCH_PX = 44;
/** Subpixel slack for edges the browser rounds. */
const EPSILON = 0.5;
/** Long enough for the WebAssembly module to load on a cold dev server. */
const WASM_TIMEOUT_MS = 30_000;

const PHONES = [
  { width: 360, height: 780 },
  { width: 390, height: 844 },
] as const;

const PROMPT = '[data-testid="prompt-modal"]';

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/* views */

/** ME-CRAFT's presets, as `viewFor` sends them: a Unit and a Spell the points cover at cost 2. */
const UNIT_PRESET: CraftRecipe = { cost: 2, type: "Unit", adjective: "Pure", noun: "Closure", attack: 5, health: 6, keywords: [], echo: 0, hats: [] };
const SPELL_PRESET: CraftRecipe = {
  cost: 2,
  type: "Spell",
  adjective: "Seeded",
  noun: "Reducer",
  attack: 0,
  health: 1,
  keywords: [],
  echo: 0,
  hats: [{ hat: "whenCast", effects: [{ verb: "damageEnemyHero", n: 8 }] }],
};

function craftView(): PlayerView {
  const options: PendingOption[] = [UNIT_PRESET, SPELL_PRESET].map((recipe, index) => ({
    key: `craft:preset-${String(index)}`,
    label: `${recipe.adjective} ${recipe.noun}`,
    recipe,
  }));
  return baseView({
    you: emptySide("p1", { hand: [card({ defId: "meditative-017", cost: 3 })] }),
    pending: pendingFor("craft", options, { prompt: "Craft a card", budget: 2 }),
  });
}

/** ME-MARKET's stall as `viewFor` builds it: lots with their price, a barter on the Radiant face, Leave. */
const LOTS: readonly PendingOption[] = [
  { key: "mode:core-080", label: "Lot", defId: "core-080", cost: 30 },
  { key: "mode:meditative-041", label: "Lot", defId: "meditative-041", cost: 35 },
  { key: "mode:classicplus-t-ai-01", label: "AI lot", defId: "classicplus-t-ai-01", cost: 15 },
  { key: "mode:meditative-045", label: "Lot", defId: "meditative-045", cost: 45 },
];

function marketView(): PlayerView {
  const options: PendingOption[] = [
    ...LOTS,
    { key: "instance:h1", label: "Traded", instanceId: "h1", defId: "core-005", cost: -30 },
    { key: "none", label: "Leave" },
  ];
  return baseView({
    you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-005", cost: 1 })] }),
    pending: pendingFor("market", options, { prompt: "Night market", budget: 50 }),
  });
}

type Kind = {
  kind: string;
  /** The `data-prompt-kind` the sheet carries: a play-time "Choose one" of five or fewer is a Discover pop-up. */
  shows: string;
  view: () => PlayerView;
  /** The legal actions the board is given: a play-time choice's candidate plays. */
  legal: readonly ActionBody[];
  /** A play-time choice opens from a tap on this hand card. */
  open?: string;
  /** The card face a hold reads, and the name its preview must show. */
  card: { selector: string; name: string };
};

const HAND = "h1";

function cardName(defId: string): string {
  return CATALOG[defId]?.name ?? defId;
}

/** A play-time choice: the card in hand and one candidate play per mode, as `legalActions` lists them (R81). */
function choice(kind: string, defId: string, modes: readonly string[]): Kind {
  return {
    kind,
    shows: "discover",
    view: () => baseView({ you: emptySide("p1", { hand: [card({ instanceId: HAND, defId, cost: 2 })] }) }),
    legal: modes.map((mode): ActionBody => ({ type: "play", instanceId: HAND, modes: [mode] })),
    open: ts(testid.handCard(HAND)),
    card: { selector: `${PROMPT} ${ts("prompt-title-source")}`, name: cardName(defId) },
  };
}

const KINDS: readonly Kind[] = [
  { kind: "craft", shows: "craft", view: craftView, legal: [], card: { selector: `${PROMPT} ${ts("craft-face")}`, name: `${UNIT_PRESET.adjective} ${UNIT_PRESET.noun}` } },
  { kind: "market", shows: "market", view: marketView, legal: [], card: { selector: ts("prompt-option-mode:meditative-041"), name: cardName("meditative-041") } },
  choice("secret", "meditative-022", ["greed", "attack", "defend"]),
  choice("prediction", "meditative-022-1", ["greed", "attack", "defend"]),
  choice("lane", "meditative-098", ["lane-1", "lane-2", "lane-3", "lane-4", "lane-5"]),
];

function openSheet(entry: Kind): void {
  mount(entry.view(), entry.legal);
  if (entry.open !== undefined) cy.get(entry.open).click();
  cy.get(`${PROMPT}[data-prompt-kind="${entry.shows}"]`).should("be.visible");
}

/**
 * Holds a touch on the element until its hover preview opens (R2617000), checks the preview while the
 * finger is still down, then lifts it where it was and sees the preview go.
 */
function hold(selector: string, whileHeld: (preview: HTMLElement) => void): void {
  cy.get(selector).should("be.visible");
  cy.get(selector).then(($face) => {
    const box = rectOf($face[0] as Element);
    const at = { clientX: box.left + box.width / 2, clientY: box.top + box.height / 3 };
    cy.get(selector).trigger("pointerdown", { eventConstructor: "PointerEvent", pointerType: "touch", pointerId: 7, isPrimary: true, button: 0, ...at });
  });
  // The hold opens the preview after LONG_PRESS_MS; the retry is the wait.
  cy.get(ts(INSPECT_HOVER)).should("be.visible");
  cy.get(ts(INSPECT_HOVER)).should(($preview) => {
    whileHeld($preview[0] as HTMLElement);
  });
  // The press lifts where it began: forced, since the preview may lie over the finger.
  cy.get(selector).trigger("pointerup", { eventConstructor: "PointerEvent", pointerType: "touch", pointerId: 7, isPrimary: true, button: 0, force: true });
  // The lift's lone click, as the browser sends it after a long-press: `cy.click()` would play a
  // fresh tap (a new pointerdown first) that no finger made.
  cy.get(selector).trigger("click", { force: true });
  cy.get(ts(INSPECT_HOVER)).should("not.exist");
}

/** The preview starts on the screen, inside its width and below its top. */
function expectOnScreen(preview: HTMLElement, width: number, label: string): void {
  const box = rectOf(preview);
  expect(box.left, `${label}: the preview is on the screen`).to.be.at.least(-EPSILON);
  expect(box.right, `${label}: the preview is on the screen`).to.be.at.most(width + EPSILON);
  expect(box.top, `${label}: the preview is on the screen`).to.be.at.least(-EPSILON);
}

/* helpers */

const sent: ActionBody[] = [];

function mount(view: PlayerView, legal: readonly ActionBody[] = []): void {
  sent.length = 0;
  closeInspect();
  cy.mount(
    <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[...legal]} onAction={(action) => sent.push(action)} />
      </div>
    </CatalogContext.Provider>,
  );
}

function rectOf(element: Element): DOMRect {
  return element.getBoundingClientRect();
}

function shown(element: Element): boolean {
  const style = getComputedStyle(element);
  const box = rectOf(element);
  return style.display !== "none" && style.visibility !== "hidden" && box.width > 0 && box.height > 0;
}

/** Every way the open sheet fails at this phone size, as readable lines. */
function sheetProblems(doc: Document, width: number, height: number, label: string): string[] {
  const modal = doc.querySelector<HTMLElement>(PROMPT);
  if (modal === null) return [`${label}: no prompt-modal`];
  const problems: string[] = [];
  const box = rectOf(modal);
  if (box.left > EPSILON || box.right < width - EPSILON) problems.push(`${label}: the sheet spans ${String(box.left)}..${String(box.right)}, not the width`);
  if (Math.abs(box.bottom - height) > 1) problems.push(`${label}: the sheet's bottom is ${String(box.bottom)}, not the screen's ${String(height)}`);
  if (box.top < -EPSILON) problems.push(`${label}: the sheet's top is above the screen (${String(box.top)})`);
  if (doc.documentElement.scrollWidth > width + 1) problems.push(`${label}: the page scrolls sideways (${String(doc.documentElement.scrollWidth)} px)`);
  for (const element of [modal, ...modal.querySelectorAll<HTMLElement>("*")]) {
    if (!shown(element)) continue;
    const style = getComputedStyle(element);
    const scrolls = style.overflowX === "auto" || style.overflowX === "scroll";
    if (scrolls && element.scrollWidth > element.clientWidth + 1) problems.push(`${label}: ${nameOf(element)} scrolls sideways`);
    const own = rectOf(element);
    if (own.right > width + 1 && style.position !== "fixed") problems.push(`${label}: ${nameOf(element)} runs off the right edge (${String(own.right)})`);
  }
  for (const control of modal.querySelectorAll<HTMLElement>("button, select, input")) {
    if (!shown(control)) continue;
    const own = rectOf(control);
    if (own.height < TOUCH_PX - EPSILON) problems.push(`${label}: ${nameOf(control)} is ${own.height.toFixed(1)} px tall, under ${String(TOUCH_PX)}`);
    if (own.width < TOUCH_PX - EPSILON) problems.push(`${label}: ${nameOf(control)} is ${own.width.toFixed(1)} px wide, under ${String(TOUCH_PX)}`);
  }
  return problems;
}

function nameOf(element: Element): string {
  const testid = element.getAttribute("data-testid");
  if (testid !== null) return testid;
  const name = element.getAttribute("aria-label") ?? element.textContent?.trim().slice(0, 24) ?? "";
  return `${element.tagName.toLowerCase()}.${(element.getAttribute("class") ?? "").split(/\s+/)[0] ?? ""} "${name}"`;
}

/* ------------------------------------------------------------------------------------- specs */

describe("MN09: the Meditative prompt kinds at phone width, with a touch hold", () => {
  beforeEach(() => {
    cy.wrap(loadWasm(), { timeout: WASM_TIMEOUT_MS });
  });

  for (const phone of PHONES) {
    for (const entry of KINDS) {
      const label = `${entry.kind} at ${String(phone.width)}x${String(phone.height)}`;

      it(`MN09 the ${label} is a bottom sheet across the width, nothing scrolls sideways, and every control is a 44 px target`, () => {
        cy.viewport(phone.width, phone.height);
        openSheet(entry);
        cy.document().should((doc) => {
          expect(sheetProblems(doc, phone.width, phone.height, label)).to.deep.equal([]);
          const face = doc.querySelector<HTMLElement>(entry.card.selector);
          expect(face, `${label}: the card it shows`).to.not.eq(null);
          if (face === null) return;
          const own = rectOf(face);
          expect(own.width, `${label}: the card has a box`).to.be.greaterThan(TOUCH_PX);
          expect(own.left, `${label}: the card is inside the sheet`).to.be.at.least(-EPSILON);
          expect(own.right, `${label}: the card is inside the sheet`).to.be.at.most(phone.width + EPSILON);
        });
      });

      it(`MN09 R2617000 a hold on the ${label}'s card shows its preview on the screen while the finger is down, and picks nothing`, () => {
        cy.viewport(phone.width, phone.height);
        openSheet(entry);
        hold(entry.card.selector, (preview) => {
          expect(preview.textContent ?? "", `${label}: the preview reads the card`).to.contain(entry.card.name);
          expectOnScreen(preview, phone.width, label);
        });
        cy.wrap(sent).should("deep.equal", []);
      });
    }
  }
});

/* the Created mark */

/** ME-CREATED (R943): a card made during the game, in hand beside one dealt with the deck. */
const CREATED = "h-created";
const DEALT = "h-dealt";

function createdView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: CREATED, defId: "meditative-022-1", cost: 0, created: true }), card({ instanceId: DEALT, defId: "meditative-022", cost: 2 })],
    }),
  });
}

/** R2647000: every way a hand card still wears the Created mark, as readable lines. */
function createdProblems(doc: Document, label: string): string[] {
  const root = doc.querySelector<HTMLElement>(ts(testid.handCard(CREATED)));
  if (root === null) return [`${label}: no created hand card`];
  if (root.querySelector(".cf") === null) return [`${label}: no face`];
  const problems: string[] = [];
  if (root.querySelector('.cf-state[data-state="created"]') !== null) problems.push(`${label}: the created card wears a Created mark on its face`);
  const dealt = doc.querySelector<HTMLElement>(ts(testid.handCard(DEALT)));
  if (dealt?.querySelector('.cf-state[data-state="created"]') !== null) problems.push(`${label}: a dealt card carries the Created mark`);
  return problems;
}

describe("MN09 R2647000: a created card at phone width wears no Created mark, and a hold's preview says it", () => {
  for (const phone of PHONES) {
    const label = `a created card at ${String(phone.width)}x${String(phone.height)}`;

    it(`MN09 R2647000 ${label} wears no Created mark on its face in hand`, () => {
      cy.viewport(phone.width, phone.height);
      mount(createdView());
      cy.get(ts(testid.handCard(CREATED))).should("be.visible");
      cy.document().should((doc) => {
        expect(createdProblems(doc, label)).to.deep.equal([]);
      });
    });

    it(`MN09 R2617000 a hold on the created card at ${String(phone.width)}x${String(phone.height)} shows its preview, which says it was made during the game`, () => {
      cy.viewport(phone.width, phone.height);
      mount(createdView());
      hold(ts(testid.handCard(CREATED)), (preview) => {
        expect(preview.textContent ?? "", `${label}: the preview reads the card`).to.contain(cardName("meditative-022-1"));
        expect(preview.textContent ?? "", `${label}: the preview says it was created`).to.contain("made during the game");
        expectOnScreen(preview, phone.width, label);
      });
      cy.wrap(sent).should("deep.equal", []);
    });
  }
});
