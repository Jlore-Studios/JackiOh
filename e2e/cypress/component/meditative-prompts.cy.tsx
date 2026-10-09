// MN09 (#552), mobile: the prompt kinds the Meditative set adds, at phone width with long-press inspect
// (docs/polish/7-mobile-ux.md: the prompt as a bottom sheet, 44 px touch targets, no horizontal scroll;
// docs/polish/6-cards.md: a card is read by a long-press on a touch screen).
//
//   craft   ME-CRAFT (Meditative #17, R880): the block editor docked as one sheet
//   market  ME-MARKET (Meditative #42): the night market's stall
//
// At 360x780 (the narrowest phone the client is laid out for) and 390x844 (BUILD M5-T1's phone), each
// prompt is a bottom sheet across the whole width resting on the bottom edge, never taller than the
// screen; nothing in it scrolls sideways, every control is a 44 px target, and the card it shows has
// a box inside the sheet. A touch long-press on that card opens its inspect sheet, which stays on the
// screen with its 44 px Close, and the press itself picks nothing: a lot is read, not bought.
//
// The kinds the later Meditative parts add (the secret choice, the prediction, the target that names a
// lane) are not in the engine yet; each part that adds one adds its row to KINDS here.
//
// The mount is mobile-ux.cy.tsx's (`Game` inside `.app-shell.app-shell--wide`, the narrowest box the
// board is ever given), with the real catalog, since the market draws its lots from it. The block
// editor's meters and face are the engine's own `craftPreview`, so the WebAssembly module is loaded
// first, as `main.tsx` loads it before anything renders.

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PendingOption, PlayerView } from "@jackioh/shared";

import { INSPECT_CLOSE, INSPECT_SHEET, closeInspect } from "../../../apps/web/src/cards/index.ts";
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

/* ------------------------------------------------------------------------------------- views */

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
  kind: "craft" | "market";
  view: () => PlayerView;
  /** The card face a long-press reads, and the name its sheet must show. */
  card: { selector: string; name: string };
};

const KINDS: readonly Kind[] = [
  { kind: "craft", view: craftView, card: { selector: `${PROMPT} ${ts("craft-face")}`, name: `${UNIT_PRESET.adjective} ${UNIT_PRESET.noun}` } },
  { kind: "market", view: marketView, card: { selector: ts("prompt-option-mode:meditative-041"), name: CATALOG["meditative-041"]?.name ?? "meditative-041" } },
];

/* ----------------------------------------------------------------------------------- helpers */

const sent: ActionBody[] = [];

function mount(view: PlayerView): void {
  sent.length = 0;
  closeInspect();
  cy.mount(
    <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={(action) => sent.push(action)} />
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

describe("MN09: the Meditative prompt kinds at phone width, with long-press inspect", () => {
  beforeEach(() => {
    cy.wrap(loadWasm(), { timeout: WASM_TIMEOUT_MS });
  });

  for (const phone of PHONES) {
    for (const entry of KINDS) {
      const label = `${entry.kind} at ${String(phone.width)}x${String(phone.height)}`;

      it(`MN09 the ${label} is a bottom sheet across the width, nothing scrolls sideways, and every control is a 44 px target`, () => {
        cy.viewport(phone.width, phone.height);
        mount(entry.view());
        cy.get(`${PROMPT}[data-prompt-kind="${entry.kind}"]`).should("be.visible");
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

      it(`MN09 a long-press on the ${label}'s card opens its inspect sheet on the screen, and picks nothing`, () => {
        cy.viewport(phone.width, phone.height);
        mount(entry.view());
        cy.get(entry.card.selector).should("be.visible");
        cy.get(entry.card.selector).then(($face) => {
          const box = rectOf($face[0] as Element);
          const at = { clientX: box.left + box.width / 2, clientY: box.top + box.height / 3 };
          cy.get(entry.card.selector).trigger("pointerdown", { eventConstructor: "PointerEvent", pointerType: "touch", pointerId: 7, isPrimary: true, button: 0, ...at });
        });
        // The hold opens the sheet after LONG_PRESS_MS; the retry is the wait.
        cy.get(ts(INSPECT_SHEET)).should("be.visible");
        cy.get(entry.card.selector).trigger("pointerup", { eventConstructor: "PointerEvent", pointerType: "touch", pointerId: 7, isPrimary: true, button: 0 });
        cy.get(entry.card.selector).click({ force: true });
        cy.document().should((doc) => {
          const sheet = doc.querySelector<HTMLElement>(ts(INSPECT_SHEET));
          expect(sheet, `${label}: the inspect sheet`).to.not.eq(null);
          if (sheet === null) return;
          expect(sheet.textContent ?? "", `${label}: the sheet reads the card`).to.contain(entry.card.name);
          const box = rectOf(sheet);
          expect(box.left, `${label}: the sheet is on the screen`).to.be.at.least(-EPSILON);
          expect(box.right, `${label}: the sheet is on the screen`).to.be.at.most(phone.width + EPSILON);
          expect(box.top, `${label}: the sheet is on the screen`).to.be.at.least(-EPSILON);
          const close = doc.querySelector<HTMLElement>(ts(INSPECT_CLOSE));
          expect(close, `${label}: the sheet's Close`).to.not.eq(null);
          if (close !== null) expect(rectOf(close).height, `${label}: Close is a 44 px target`).to.be.at.least(TOUCH_PX - EPSILON);
          expect(sent, `${label}: the long-press picked nothing`).to.deep.equal([]);
        });
      });
    }
  }
});
