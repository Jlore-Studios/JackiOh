// Issue #219: the emote picker, the Mute emotes menu and a voice line's speech bubble, measured in a
// real browser. Since issue #544 both menus sit in the hero's inspect view (R1330), which a click on a
// portrait opens, so each is measured there, and so is the view itself and the portraits' vivid layers
// (R1332).
//
// Only a layout engine can say what these measure (jsdom has none; ui.test.tsx reads the
// stylesheet's text instead):
//   - every emoji in the picker is drawn at least EMOJI_MIN_PX square,
//   - every voice label is at least LABEL_MIN_PX, and on a touch-sized screen every picker item is
//     at least TOUCH_PX square (B29's floor),
//   - both menus stay on the screen, also where the seat puts the portrait at its left edge
//     (narrower than 761 px, or shorter than 501 px),
//   - a voice line's bubble is sized by its line rather than wrapped word by word, and stays on the
//     screen,
//   - nothing on the board is drawn over the opponent's bubble, sticker or Mute menu, and none of
//     them is faded by its hero (#257),
//   - the hero's inspect view stays on the screen with its portrait large and Close a touch target
//     (R1330), and the board's portraits draw their vivid layers inside the 44 px oval (R1332).
//
// The mount is mobile-ux.cy.tsx's: `Game` inside `.app-shell.app-shell--wide`, the narrowest
// container the board is given, here with the routes' own `useEmotes` (no audio engine), so a pick
// shows what it shows in a match. No portraits frame is passed, so both seats wear the default
// portrait, vanilla, whose Well Played line is "Well played. Plainly.", and the picker holds the
// default hand (R1343): three voice lines, Well Played among them, and five emoji, the shape of
// every hand a game deals.
//
// Every measurement is taken inside a `should` callback so it is retried until the layout has
// settled after `cy.viewport()`, and until the bubble's pop-in has finished.

import { StrictMode, useEffect, type ReactElement } from "react";

import { PORTRAIT_MOTE_COUNT } from "../../../apps/web/src/emotes/config.ts";
import { useEmotes, type EmotesApi } from "../../../apps/web/src/emotes/useEmotes.ts";
import { EMOTE_HAND_SIZE, EMOTE_HAND_VOICE } from "../../../apps/web/src/wire/emotes.ts";
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
/** The inspect view's portrait is large, however short the screen: the board's oval is 44 px. */
const INSPECT_PORTRAIT_MIN_PX = 96;

/** The seat centres the portrait from 761x501 up and puts it at the seat's left edge below that. */
const VIEWPORTS = [
  { width: 1280, height: 720, touch: false },
  { width: 768, height: 1024, touch: true },
  { width: 700, height: 1000, touch: true },
  { width: 390, height: 844, touch: true },
  { width: 360, height: 740, touch: true },
  { width: 844, height: 390, touch: true },
] as const;

/** Nothing targets either hero, so a click on a portrait opens its inspect view (SPEC §10.10). */
const END_TURN_ONLY: Legal = [{ type: "endTurn" }];

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

function EmoteGame({
  view,
  legal,
  onEmotes,
}: {
  view: View;
  legal: Legal;
  onEmotes: (emotes: EmotesApi) => void;
}): ReactElement {
  const emotes = useEmotes({ engine: null, you: view.viewer });
  useEffect(() => {
    onEmotes(emotes);
  }, [emotes, onEmotes]);
  return <Game view={view} legal={legal} onAction={() => undefined} emotes={emotes} />;
}

/** The mounted game's emote session, so a spec can play the opponent's emote arriving. */
let mountedEmotes: EmotesApi | null = null;

function keepEmotes(emotes: EmotesApi): void {
  mountedEmotes = emotes;
}

function mountGame(): void {
  mountedEmotes = null;
  // StrictMode, as main.tsx mounts the app: dev replays every layout effect, and the menu's
  // keep-on-screen measure has to hold its answer across the replay, not only the first run.
  cy.mount(
    <StrictMode>
      <div className="app-shell app-shell--wide">
        <EmoteGame view={fullBoardView()} legal={END_TURN_ONLY} onEmotes={keepEmotes} />
      </div>
    </StrictMode>,
  );
}

function fontSizePx(element: Element): number {
  const win = element.ownerDocument.defaultView;
  return win === null ? 0 : Number.parseFloat(win.getComputedStyle(element).fontSize);
}

/** Opens a hero's inspect view: the portrait, not the hero's centre, where a phone puts the power. */
function openView(side: "you" | "opponent"): void {
  cy.get(`${ts(`hero-${side}`)} .hero-portrait`).click();
  cy.get(ts("hero-inspect")).should("be.visible");
}

/** Opens your portrait's picker, which your hero's inspect view holds (R1330). */
function openPicker(): void {
  openView("you");
  cy.get(`${ts("hero-inspect")} ${ts("emote-menu")}`).should("be.visible");
}

/** fullBoardView() seats the viewer as p1, so the opponent is p2. */
const OPPONENT = "p2";

/** The opponent's emote arrives, as the match's relay hands it to `receive`. */
function opponentEmotes(emote: Parameters<EmotesApi["receive"]>[1]): void {
  cy.wrap(null).should(() => {
    expect(mountedEmotes, "the mounted emote session").not.to.equal(null);
  });
  cy.then(() => {
    expect(mountedEmotes?.receive(OPPONENT, emote), `the opponent's ${emote}`).to.equal(true);
  });
}

/**
 * #257: nothing is drawn over `element` and nothing above it fades it. Nine points across it are
 * asked what the browser draws there; a show is blind to the pointer, so it is made hittable first,
 * which changes what is hit and nothing about what is drawn. Only points on the screen are asked.
 */
function expectDrawnOnTop(element: HTMLElement, what: string): void {
  element.style.pointerEvents = "auto";
  const doc = element.ownerDocument;
  const box = element.getBoundingClientRect();
  const width = doc.documentElement.clientWidth;
  const height = doc.documentElement.clientHeight;
  let asked = 0;
  for (const fx of [0.15, 0.5, 0.85]) {
    for (const fy of [0.2, 0.5, 0.8]) {
      const x = box.left + box.width * fx;
      const y = box.top + box.height * fy;
      if (x < 0 || y < 0 || x > width || y > height) continue;
      asked += 1;
      const top = doc.elementFromPoint(x, y);
      const cover = top?.closest("[data-testid]")?.getAttribute("data-testid") ?? top?.className ?? "nothing";
      expect(top !== null && element.contains(top), `${what} at (${fx}, ${fy}), under ${String(cover)}`).to.equal(true);
    }
  }
  expect(asked, `points of ${what} on the screen`).to.be.at.least(1);
  let opacity = 1;
  for (let el: Element | null = element.parentElement; el !== null; el = el.parentElement) {
    opacity *= Number.parseFloat(doc.defaultView?.getComputedStyle(el).opacity ?? "1");
  }
  expect(opacity, `what fades ${what}`).to.equal(1);
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
        .should("have.length", EMOTE_HAND_SIZE - EMOTE_HAND_VOICE)
        .each(($svg) => {
          const box = ($svg[0] as Element).getBoundingClientRect();
          expect(box.width, "an emoji's drawn width").to.be.at.least(EMOJI_MIN_PX - EPSILON);
          expect(box.height, "an emoji's drawn height").to.be.at.least(EMOJI_MIN_PX - EPSILON);
        });
    });

    it(`#219 every voice label reads at least ${LABEL_MIN_PX}px${viewport.touch ? ", every item a 44px touch target" : ""}`, () => {
      openPicker();
      cy.get(`${ts("emote-menu")} .emote-item.emote-voice`)
        .should("have.length", EMOTE_HAND_VOICE)
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
          .should("have.length", EMOTE_HAND_SIZE - EMOTE_HAND_VOICE)
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
      openView("opponent");
      cy.get(`${ts("hero-inspect")} ${ts("emote-mute-menu")}`)
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

    it("#257 the opponent's speech bubble is drawn over the board, at full strength", () => {
      opponentEmotes("wellPlayed");
      cy.get(`${ts("hero-opponent")} ${ts("emote-bubble")}`)
        .should("contain.text", "Well played. Plainly.")
        .and(($bubble) => {
          expectDrawnOnTop($bubble[0] as HTMLElement, "the opponent's bubble");
        });
    });

    it("#257 the opponent's emoji is drawn over the board, at full strength", () => {
      opponentEmotes("laugh");
      cy.get(`${ts("hero-opponent")} ${ts("emote-bubble")}[data-emoji="laugh"]`).should(($sticker) => {
        expectDrawnOnTop($sticker[0] as HTMLElement, "the opponent's emoji");
      });
    });

    it("#257 the opponent's Mute emotes menu is drawn over the board and takes the click", () => {
      openView("opponent");
      cy.get(`${ts("hero-inspect")} ${ts("emote-mute-menu")}`).should(($menu) => {
        expectDrawnOnTop($menu[0] as HTMLElement, "the mute menu");
      });
      // A click Cypress cannot land (the item covered) fails here: before #257 the field took it.
      cy.get(ts("emote-mute")).click();
      cy.get(ts("hero-inspect")).should("not.exist");
      cy.get(ts("emote-mute-menu")).should("not.exist");
      cy.then(() => {
        expect(mountedEmotes?.muted(OPPONENT), "the opponent muted").to.equal(true);
      });
    });

    it("R1330 the hero's inspect view stays on the screen, its portrait large and Close a touch target", () => {
      for (const side of ["you", "opponent"] as const) {
        openView(side);
        cy.get(ts("hero-inspect")).should(($view) => {
          const view = $view[0] as Element;
          expectOnScreen(view, `${side}'s inspect view`);
          const height = view.ownerDocument.documentElement.clientHeight;
          expect(view.getBoundingClientRect().bottom, `${side}'s inspect view's bottom edge`).to.be.at.most(
            height + EPSILON,
          );
          const portrait = view.querySelector(".hero-inspect-portrait");
          expect(portrait, "the view's portrait").not.to.equal(null);
          expect(portrait?.getBoundingClientRect().width ?? 0, "the view's portrait width").to.be.at.least(
            INSPECT_PORTRAIT_MIN_PX,
          );
          expect(view.querySelectorAll(".hero-inspect-portrait .portrait-art-mote")).to.have.length(
            PORTRAIT_MOTE_COUNT,
          );
        });
        cy.get(ts("hero-inspect-close")).should(($close) => {
          expect(($close[0] as Element).getBoundingClientRect().height, "Close's height").to.be.at.least(
            TOUCH_PX - EPSILON,
          );
        });
        cy.get(ts("hero-inspect-close")).click();
        cy.get(ts("hero-inspect")).should("not.exist");
      }
    });

    it("R1332 the board's portraits draw their vivid layers inside the 44px oval", () => {
      for (const side of ["you", "opponent"] as const) {
        cy.get(`${ts(`hero-${side}`)} .hero-portrait`).should(($frame) => {
          const frame = $frame[0] as Element;
          const art = frame.querySelector(".portrait-art");
          expect(art, `${side}'s portrait art`).not.to.equal(null);
          const outer = frame.getBoundingClientRect();
          const inner = (art as Element).getBoundingClientRect();
          for (const edge of ["left", "top", "width", "height"] as const) {
            expect(inner[edge], `the art's ${edge} against its oval`).to.be.closeTo(outer[edge], EPSILON);
          }
          expect(frame.querySelectorAll(".portrait-art-mote"), `${side}'s motes`).to.have.length(PORTRAIT_MOTE_COUNT);
        });
      }
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
