// R2718000: the set watermarks in a real browser. apps/web's vitest suite proves which element a face
// prints and what the stylesheets declare (cards/CardFace.test.tsx); what only a layout and paint
// engine can say is measured here: a card of each set, a Radiant face and a Chinese face, each wears
// its set's watermark centred in its rules box at WATERMARK_SIZE_PCT of the box's height, under the
// words, taking no pointer and moving never; each watermark draws as one flat ink at its named
// opacity; no two sets draw alike; and with Set watermarks off no face wears one. The first and
// the last are snapshots (`cy.screenshot`), the pixel record of what the owner asked for.
//
// Every measurement sits inside `.should()`, so it retries while `useFitText` settles.

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import {
  WATERMARK_INK,
  WATERMARK_OPACITY,
  WATERMARK_SIZE_PCT,
  watermarkOf,
  type Watermark,
} from "../../../apps/web/src/cards/setMark.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";

type CardDef = (typeof CATALOG)[string];

/** One card of each set that ships, and a Radiant face and a Chinese face (R1301) besides. */
const FACES = [
  { key: "core", id: "core-002", set: "Core", radiant: false, chinese: false },
  { key: "classic", id: "classic-043", set: "Classic", radiant: false, chinese: false },
  { key: "classic-plus", id: "classicplus-043", set: "Classic+", radiant: false, chinese: false },
  { key: "meditative", id: "meditative-027", set: "Meditative", radiant: false, chinese: false },
  { key: "meditative-plus", id: "meditativeplus-001", set: "Meditative+", radiant: false, chinese: false },
  { key: "stinkyscape", id: "stinkyscape-017", set: "Stinkyscape", radiant: false, chinese: false },
  { key: "training-grounds", id: "traininggrounds-017", set: "Training Grounds", radiant: false, chinese: false },
  { key: "classic-radiant", id: "classic-043", set: "Classic", radiant: true, chinese: false },
  { key: "meditative-chinese", id: "meditative-027", set: "Meditative", radiant: false, chinese: true },
] as const;

/**
 * Nine faces in two rows of five: a headless Chrome's default window crops a `capture: "viewport"`
 * shot taller than about 633 px (cypress.config.ts), and the snapshot must hold every face.
 */
const FACE_WIDTH_PX = 200;
const FACE_HEIGHT_PX = Math.round(FACE_WIDTH_PX / FACE_ASPECT);
const VIEWPORT_WIDTH_PX = 1100;

/** Subpixel slack for edges the browser rounds. */
const SLACK_PX = 1;
/** The height share is a percentage of a box a few hundred pixels tall, rounded twice. */
const SIZE_SLACK_PX = 2;

/** Retries cover `useFitText`'s passes. */
const SETTLE_TIMEOUT_MS = 8_000;

/** The side of the canvas each watermark is drawn on to read its pixels. */
const CANVAS_PX = 96;
/** An alpha under this is no ink: the canvas's rounding, not the watermark. */
const INK_ALPHA_MIN = 8;
/** The least share of the canvas a watermark inks: a watermark has to be large. */
const INKED_SHARE_MIN = 0.02;
/** The least share of the canvas on which two sets' inked masks differ. */
const DIFFER_SHARE_MIN = 0.01;
/** How far a channel of the most opaque pixels may lie from the ink's: premultiplied rounding at a low alpha. */
const INK_CHANNEL_SLACK = 16;

function Faces() {
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8, width: VIEWPORT_WIDTH_PX, background: "#1b1f2b" }}>
      {FACES.map(({ key, id, radiant, chinese }) => (
        <div key={key} data-face={key} style={{ width: FACE_WIDTH_PX, height: FACE_HEIGHT_PX, flex: "none" }}>
          <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant, ...(chinese ? { chinese: true } : {}) })} layout="full" />
        </div>
      ))}
    </div>
  );
}

function watermarkFor(set: string): Watermark {
  const found = watermarkOf(set);
  if (found === null) throw new Error(`${set} has no watermark`);
  return found;
}

function faceIn(doc: Document, key: string): HTMLElement {
  const cf = doc.querySelector<HTMLElement>(`[data-face="${key}"] > .cf`);
  if (cf === null) throw new Error(`no face for ${key}`);
  return cf;
}

function inkOf(hex: string): [number, number, number] {
  return [parseInt(hex.slice(1, 3), 16), parseInt(hex.slice(3, 5), 16), parseInt(hex.slice(5, 7), 16)];
}

type Drawn = { set: string; inked: boolean[]; maxAlpha: number; deviation: number };

/**
 * One watermark drawn on a canvas: which pixels it inks, its highest alpha, and how far the colour of
 * the pixels at that alpha lies from the ink's, in the worst channel.
 */
async function draw(set: string): Promise<Drawn> {
  const image = new Image();
  image.width = CANVAS_PX;
  image.height = CANVAS_PX;
  image.src = watermarkFor(set).src;
  await image.decode();
  const canvas = document.createElement("canvas");
  canvas.width = CANVAS_PX;
  canvas.height = CANVAS_PX;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (context === null) throw new Error("no 2d canvas");
  context.drawImage(image, 0, 0, CANVAS_PX, CANVAS_PX);
  const { data } = context.getImageData(0, 0, CANVAS_PX, CANVAS_PX);
  const inked: boolean[] = [];
  let maxAlpha = 0;
  for (let at = 3; at < data.length; at += 4) {
    const alpha = data[at] ?? 0;
    inked.push(alpha >= INK_ALPHA_MIN);
    maxAlpha = Math.max(maxAlpha, alpha);
  }
  const ink = inkOf(WATERMARK_INK);
  let deviation = 0;
  for (let at = 3; at < data.length; at += 4) {
    if (data[at] !== maxAlpha) continue;
    for (let channel = 0; channel < 3; channel += 1) {
      deviation = Math.max(deviation, Math.abs((data[at - 3 + channel] ?? 0) - (ink[channel] ?? 0)));
    }
  }
  return { set, inked, maxAlpha, deviation };
}

describe("R2718000 set watermarks on real faces", () => {
  beforeEach(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, fullArt: false });
    cy.viewport(VIEWPORT_WIDTH_PX, 700);
  });

  after(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  it("R2718000 each set, a Radiant and a Chinese face wear their watermark, centred in the rules box under its words", () => {
    cy.mount(<Faces />);
    cy.document().should((doc) => {
      for (const { key, set } of FACES) {
        const cf = faceIn(doc, key);
        const box = cf.querySelector<HTMLElement>(".card-text");
        expect(box, `${key} has a rules box`).not.to.equal(null);
        const marks = cf.querySelectorAll<HTMLImageElement>(".card-text > .cf-watermark");
        expect(marks.length, `${key} wears one watermark`).to.equal(1);
        const mark = marks[0] as HTMLImageElement;
        const holder = (box as HTMLElement).getBoundingClientRect();
        const found = mark.getBoundingClientRect();
        expect(found.height, `${key} has a box`).to.be.greaterThan(0);
        expect(mark.getAttribute("src"), `${key} src`).to.equal(watermarkFor(set).src);
        // Inside the rules box, and centred in it.
        expect(found.left, `${key} left`).to.be.at.least(holder.left - SLACK_PX);
        expect(found.right, `${key} right`).to.be.at.most(holder.right + SLACK_PX);
        expect(found.top, `${key} top`).to.be.at.least(holder.top - SLACK_PX);
        expect(found.bottom, `${key} bottom`).to.be.at.most(holder.bottom + SLACK_PX);
        expect(Math.abs(found.left + found.width / 2 - (holder.left + holder.width / 2)), `${key} centred across`).to.be.at.most(SLACK_PX);
        expect(Math.abs(found.top + found.height / 2 - (holder.top + holder.height / 2)), `${key} centred down`).to.be.at.most(SLACK_PX);
        // At its named share of the box's height.
        expect(Math.abs(found.height - (holder.height * WATERMARK_SIZE_PCT) / 100), `${key} height`).to.be.at.most(SIZE_SLACK_PX);
        // Under the words, taking no pointer, and still.
        const style = getComputedStyle(mark);
        expect(style.zIndex, `${key} z-index`).to.equal("-1");
        expect(style.pointerEvents, `${key} pointer-events`).to.equal("none");
        expect(mark.getAnimations().length, `${key} animations`).to.equal(0);
        const boxStyle = getComputedStyle(box as HTMLElement);
        expect(boxStyle.position, `${key} box position`).to.equal("absolute");
        expect(boxStyle.zIndex, `${key} box z-index`).to.equal("1");
        expect(boxStyle.display, `${key} box shows`).to.equal("flex");
      }
    });
    cy.screenshot("R2718000-set-watermarks", { capture: "viewport", overwrite: true });
  });

  it("R2718000 each watermark draws as one ink at its named opacity, and no two sets alike", () => {
    const sets = [...new Set(FACES.map((face) => face.set))];
    cy.wrap(Promise.all(sets.map(draw)), { timeout: SETTLE_TIMEOUT_MS }).then((result) => {
      const drawn = result as Drawn[];
      const alphaMax = Math.ceil(255 * WATERMARK_OPACITY) + 1;
      for (const { set, inked, maxAlpha, deviation } of drawn) {
        const count = inked.filter(Boolean).length;
        expect(count / inked.length, `${set} inks at least ${String(INKED_SHARE_MIN * 100)}% of the canvas`).to.be.at.least(INKED_SHARE_MIN);
        expect(maxAlpha, `${set} is never more opaque than its opacity`).to.be.at.most(alphaMax);
        expect(maxAlpha, `${set} is inked`).to.be.greaterThan(INK_ALPHA_MIN);
        expect(deviation, `${set} is in the ink's own colour`).to.be.at.most(INK_CHANNEL_SLACK);
      }
      for (const [at, one] of drawn.entries()) {
        for (const other of drawn.slice(at + 1)) {
          const differ = one.inked.filter((on, pixel) => on !== other.inked[pixel]).length;
          expect(differ / one.inked.length, `${one.set} and ${other.set} differ`).to.be.at.least(DIFFER_SHARE_MIN);
        }
      }
    });
  });

  it("R2718000 with Set watermarks off no face wears one", () => {
    writeCardSettings({ setWatermarks: false });
    cy.mount(<Faces />);
    cy.document().should((doc) => {
      expect(doc.querySelectorAll(".cf .card-text").length, "nine rules boxes").to.equal(FACES.length);
      expect(doc.querySelectorAll(".cf-watermark").length, "no watermark").to.equal(0);
    });
    cy.screenshot("R2718000-set-watermarks-off", { capture: "viewport", overwrite: true });
  });
});
