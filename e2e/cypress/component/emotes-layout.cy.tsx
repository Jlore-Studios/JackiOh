// Issue #219: the emote picker, the Mute emotes menu and a voice line's speech bubble, measured in a
// real browser.
//
// Only a layout engine can say what these measure (jsdom has none; ui.test.tsx reads the
// stylesheet's text instead):
//   - every emoji in the picker is drawn at least EMOJI_MIN_PX square,
//   - every voice label is at least LABEL_MIN_PX, and on a touch-sized screen every picker item is
//     at least TOUCH_PX square (B29's floor),
//   - both menus stay on the screen, also where the seat puts the portrait at its left edge
//     (narrower than 761 px, or shorter than 501 px),
//   - a voice line's bubble is sized by its line rather than wrapped word by word, and stays on the
//     screen.
//
// The mount is mobile-ux.cy.tsx's: `Game` inside `.app-shell.app-shell--wide`, the narrowest
// container the board is given, here with the routes' own `useEmotes` (no audio engine), so a pick
// shows what it shows in a match. No portraits frame is passed, so both seats wear the default
// portrait, vanilla, whose Well Played line is "Well played. Plainly.".
//
// Every measurement is taken inside a `should` callback so it is retried until the layout has
// settled after `cy.viewport()`, and until the bubble's pop-in has finished.

import { StrictMode, type ReactElement } from "react";

import { useEmotes } from "../../../apps/web/src/emotes/useEmotes.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView } from "../../../apps/web/src/test/fixtures.ts";

type GameProps = Parameters<typeof Game>[0];
type View = GameProps["view"];
type Legal = GameProps["legal"];

/** WCAG 2.5.5 / Apple HIG, the floor B29 sets for every touch target. */
const TOUCH_PX = 44;
/** An emoji's drawing in the picker; before #219 it was squeezed to a few px. */
const EMOJI_MIN_PX = 36;
/** A voice item's label; before #219 it was under 8 px on a phone. */
const LABEL_MIN_PX = 11;
/** "Well played. Plainly." on one line is about 140 px; wrapped word by word it was about 60. */
const BUBBLE_MIN_PX = 120;
/** Subpixel slack for edges the browser rounds; never enough to hide a real overflow. */
const EPSILON = 0.5;

/** The seat centres the portrait from 761x501 up and puts it at the seat's left edge below that. */
const VIEWPORTS = [
  { width: 1280, height: 720, touch: false },
  { width: 768, height: 1024, touch: true },
  { width: 700, height: 1000, touch: true },
  { width: 390, height: 844, touch: true },
  { width: 360, height: 740, touch: true },
  { width: 844, height: 390, touch: true },
] as const;

/** Nothing targets either hero, so a click on a portrait opens its emote menu (SPEC §10.10). */
const END_TURN_ONLY: Legal = [{ type: "endTurn" }];

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

function EmoteGame({ view, legal }: { view: View; legal: Legal }): ReactElement {
  const emotes = useEmotes({ engine: null, you: view.viewer });
  return <Game view={view} legal={legal} onAction={() => undefined} emotes={emotes} />;
}

function mountGame(): void {
  // StrictMode, as main.tsx mounts the app: dev replays every layout effect, and the menu's
  // keep-on-screen measure has to hold its answer across the replay, not only the first run.
  cy.mount(
    <StrictMode>
      <div className="app-shell app-shell--wide">
        <EmoteGame view={fullBoardView()} legal={END_TURN_ONLY} />
      </div>
    </StrictMode>,
  );
}

function fontSizePx(element: Element): number {
  const win = element.ownerDocument.defaultView;
  return win === null ? 0 : Number.parseFloat(win.getComputedStyle(element).fontSize);
}

/** Opens your portrait's picker: the portrait, not the hero's centre, where a phone puts the power. */
function openPicker(): void {
  cy.get(`${ts("hero-you")} .hero-portrait`).click();
  cy.get(ts("emote-menu")).should("be.visible");
}

function expectOnScreen(element: Element, what: string): void {
  const rect = element.getBoundingClientRect();
  const width = element.ownerDocument.documentElement.clientWidth;
  expect(rect.left, `${what}'s left edge`).to.be.at.least(-EPSILON);
  expect(rect.top, `${what}'s top edge`).to.be.at.least(-EPSILON);
  expect(rect.right, `${what}'s right edge`).to.be.at.most(width + EPSILON);
}

for (const viewport of VIEWPORTS) {
  describe(`#219 the emote menus and bubble at ${viewport.width}x${viewport.height}`, () => {
    beforeEach(() => {
      cy.viewport(viewport.width, viewport.height);
      mountGame();
    });

    it("#219 every emoji in the picker draws at least EMOJI_MIN_PX square", () => {
      openPicker();
      cy.get(`${ts("emote-menu")} .emote-emoji-pick .emote-emoji-svg`)
        .should("have.length", 5)
        .each(($svg) => {
          const box = ($svg[0] as Element).getBoundingClientRect();
          expect(box.width, "an emoji's drawn width").to.be.at.least(EMOJI_MIN_PX - EPSILON);
          expect(box.height, "an emoji's drawn height").to.be.at.least(EMOJI_MIN_PX - EPSILON);
        });
    });

    it(`#219 every voice label reads at least ${LABEL_MIN_PX}px${viewport.touch ? ", every item a 44px touch target" : ""}`, () => {
      openPicker();
      cy.get(`${ts("emote-menu")} .emote-item.emote-voice`)
        .should("have.length", 5)
        .each(($item) => {
          const item = $item[0] as Element;
          expect(fontSizePx(item), `the "${item.textContent ?? ""}" label`).to.be.at.least(LABEL_MIN_PX);
          if (viewport.touch) {
            const box = item.getBoundingClientRect();
            expect(box.height, `"${item.textContent ?? ""}"'s height`).to.be.at.least(TOUCH_PX - EPSILON);
            expect(box.width, `"${item.textContent ?? ""}"'s width`).to.be.at.least(TOUCH_PX - EPSILON);
          }
        });
      if (viewport.touch) {
        cy.get(`${ts("emote-menu")} .emote-item.emote-emoji-pick`)
          .should("have.length", 5)
          .each(($item) => {
            const box = ($item[0] as Element).getBoundingClientRect();
            expect(box.width, "an emoji pick's width").to.be.at.least(TOUCH_PX - EPSILON);
            expect(box.height, "an emoji pick's height").to.be.at.least(TOUCH_PX - EPSILON);
          });
      }
    });

    it("#219 the picker stays on the screen", () => {
      openPicker();
      cy.get(ts("emote-menu")).should(($menu) => {
        expectOnScreen($menu[0] as Element, "the emote picker");
      });
    });

    it("#219 the opponent's Mute emotes menu stays on the screen", () => {
      cy.get(`${ts("hero-opponent")} .hero-portrait`).click();
      cy.get(ts("emote-mute-menu"))
        .should("be.visible")
        .and(($menu) => {
          const menu = $menu[0] as Element;
          expectOnScreen(menu, "the mute menu");
          if (viewport.touch) {
            const item = menu.querySelector(".emote-item.emote-mute");
            const box = item?.getBoundingClientRect();
            expect(box?.height ?? 0, "the mute item's height").to.be.at.least(TOUCH_PX - EPSILON);
          }
        });
    });

    it("#219 a voice line's bubble is sized by its line and stays on the screen", () => {
      openPicker();
      cy.get(ts("emote-wellPlayed")).click();
      cy.get(ts("emote-bubble"))
        .should("contain.text", "Well played. Plainly.")
        .and(($bubble) => {
          const bubble = $bubble[0] as Element;
          const box = bubble.getBoundingClientRect();
          expect(box.width, "the bubble's width").to.be.at.least(BUBBLE_MIN_PX);
          expectOnScreen(bubble, "the bubble");
        });
    });
  });
}
