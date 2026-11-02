// R2715002: the Mythic's signature in a real browser. apps/web's vitest suite proves the signature's
// markup, its faces and its timings (cards/signature.test.tsx) and what the stylesheet declares
// (cards/cosmetics.test.tsx); what only a style engine can say is measured here, on two cards of
// different sets signed in different hands: a frame mid-signing, the animations paused there, has the
// pen tip lit on the wipe's edge inside the name and the name only partly inked, and the landed frame
// has the name whole, the pen gone and the flourish drawn; and under Reduce motion the name is whole
// from the first frame and nothing moves.
//
// The component runner serves no /fonts, so each hand is checked as the face the stylesheet asks for
// (its computed family), not as glyphs; the file and licence of each are cards/signature.test.tsx's.

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { resetCosmeticsForTests } from "../../../apps/web/src/cards/cosmetics.ts";
import { FULL_ARTS_OF_CARD, FullArtShown, OwnCopy, fullArtPickOf } from "../../../apps/web/src/cards/fullArt.ts";
import {
  SIGNATURE_LAND_MS,
  SIGNATURE_SHINE_PERIOD_MS,
  SIGNATURE_START_MS,
  SIGNATURE_WRITE_MS,
} from "../../../apps/web/src/cards/FullArtSignature.tsx";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { SIGNATURE_FACES, signatureFaceOf } from "../../../apps/web/src/cards/signatureFaces.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

type CardDef = (typeof CATALOG)[string];
type Shot = { width: number; height: number; data: Uint8ClampedArray; scale: number };
type Point = { x: number; y: number };

/** Two Mythics of different sets, each signed in its set's hand. */
const CARDS = [
  { id: "core-001", why: "Big D-fender, Core: Kaushan Script" },
  { id: "classic-090", why: "In Too Deep, Classic: Pinyon Script" },
] as const;
/** An inspect-sized face, so the signature is large enough to sample. */
const FACE_WIDTH = 300;
/** A quarter of the way through the writing, which eases out, so its pen is about two thirds along the name. */
const MID_SIGNING_MS = SIGNATURE_START_MS + SIGNATURE_WRITE_MS / 4;
/** Past the landing and the first shine's pass, which crosses in the first fifth of its period. */
const LANDED_MS = SIGNATURE_START_MS + SIGNATURE_WRITE_MS + SIGNATURE_LAND_MS + SIGNATURE_SHINE_PERIOD_MS / 5;
/** The pen tip shows at least this opaque mid-signing. */
const PEN_OPACITY_MIN = 0.9;
/** The grid the signature is sampled on, in CSS pixels; a channel must move this much to count. */
const SAMPLE_STEP_PX = 2;
const PIXEL_DELTA_MIN = 24;
/** How close a pixel's channels come to the ink's color for it to count as ink. */
const INK_DELTA_MAX = 18;
/** The fewest ink points the landed name shows past where the pen was: a name is drawn there. */
const INK_POINTS_MIN = 8;
/** Mid-signing, the part past the pen holds at most this share of the ink it holds once landed. */
const UNWRITTEN_INK_SHARE_MAX = 0.5;
/** How far before the pen's tip the ink is whole (the wipe's feathered edge lies between), in CSS pixels. */
const FEATHER_GAP_PX = 14;
/** Enough room for the faces to mount and fit. */
const SETTLE_TIMEOUT_MS = 8_000;
/** The wait under Reduce motion between the first frame and a later one. */
const STILL_GAP_MS = 1_000;

function must<T>(found: T | null | undefined, what: string): T {
  if (found === null || found === undefined) throw new Error(`no ${what}`);
  return found;
}

function Mythics() {
  return (
    <OwnCopy own>
      <div style={{ display: "flex", gap: 12, padding: 12, background: "#1b1f2b" }}>
        {CARDS.map(({ id }) => (
          <div key={id} data-face={id} style={{ width: FACE_WIDTH, height: Math.round(FACE_WIDTH / FACE_ASPECT), flex: "none" }}>
            <FullArtShown defId={id} pick={fullArtPickOf(id, must(FULL_ARTS_OF_CARD.get(id)?.get("Mythic"), `${id}'s Mythic`))}>
              <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant: false })} layout="full" />
            </FullArtShown>
          </div>
        ))}
      </div>
    </OwnCopy>
  );
}

function signatureOf(doc: Document, id: string): HTMLElement {
  return must(doc.querySelector<HTMLElement>(`[data-face="${id}"] .cf-signature`), `${id}'s signature`);
}

/** Every signature's animations paused at `ms` after they began: one frame, held. */
function holdAt(doc: Document, ms: number): void {
  for (const { id } of CARDS) {
    const animations = signatureOf(doc, id).getAnimations({ subtree: true });
    expect(animations.length, `${id}'s signing runs`).to.be.greaterThan(0);
    for (const animation of animations) {
      animation.pause();
      animation.currentTime = ms;
    }
  }
}

/** The viewport as Chrome painted it, decoded to RGBA, with the device pixels per CSS pixel. */
function screenshot(name: string): Cypress.Chainable<Shot> {
  let path = "";
  return cy
    .screenshot(name, {
      capture: "viewport",
      overwrite: true,
      disableTimersAndAnimations: false,
      onAfterScreenshot: (_el, props) => {
        path = props.path;
      },
    })
    .then(() => cy.readFile(path, null))
    .then((bytes: Uint8Array) => cy.window().then((win) => ({ bytes, innerWidth: win.innerWidth })))
    .then(({ bytes, innerWidth }) =>
      cy.wrap(
        (async (): Promise<Shot> => {
          const bitmap = await createImageBitmap(new Blob([new Uint8Array(bytes)], { type: "image/png" }));
          const canvas = document.createElement("canvas");
          canvas.width = bitmap.width;
          canvas.height = bitmap.height;
          const ctx = canvas.getContext("2d");
          if (ctx === null) throw new Error("this browser has no 2D canvas");
          ctx.drawImage(bitmap, 0, 0);
          return { width: bitmap.width, height: bitmap.height, data: ctx.getImageData(0, 0, bitmap.width, bitmap.height).data, scale: bitmap.width / innerWidth };
        })(),
        { log: false },
      ),
    );
}

function pixelAt(shot: Shot, { x, y }: Point): [number, number, number] {
  const at = (Math.round(y * shot.scale) * shot.width + Math.round(x * shot.scale)) * 4;
  return [shot.data[at] ?? -1, shot.data[at + 1] ?? -1, shot.data[at + 2] ?? -1];
}

/** The points of `box`'s part between `from` and `to` along x, on the sampling grid. */
function pointsIn(box: DOMRect, from: number, to: number): Point[] {
  const points: Point[] = [];
  for (let y = box.top; y < box.bottom; y += SAMPLE_STEP_PX) {
    for (let x = Math.max(box.left, from); x < Math.min(box.right, to); x += SAMPLE_STEP_PX) points.push({ x, y });
  }
  return points;
}

/** The ink's color, as the browser computes the name's fill. */
function inkOf(text: Element): [number, number, number] {
  const [r = -1, g = -1, b = -1] = (getComputedStyle(text).fill.match(/\d+/gu) ?? []).map(Number);
  return [r, g, b];
}

/** How many of `points` are drawn in `ink`: the moving art behind the name is never so close to it. */
function inkCount(shot: Shot, points: readonly Point[], ink: readonly [number, number, number]): number {
  return points.filter((point) => pixelAt(shot, point).every((channel, i) => Math.abs(channel - (ink[i] ?? -1)) <= INK_DELTA_MAX)).length;
}

function changedShare(before: Shot, after: Shot, points: readonly Point[]): number {
  if (points.length === 0) return 0;
  const changed = points.filter((point) => {
    const a = pixelAt(before, point);
    const b = pixelAt(after, point);
    return a.some((channel, i) => Math.abs(channel - (b[i] ?? -1)) >= PIXEL_DELTA_MIN);
  });
  return changed.length / points.length;
}

/** An element's computed transform as the identity or not. */
function atRest(element: Element): boolean {
  return ["none", "matrix(1, 0, 0, 1, 0, 0)"].includes(getComputedStyle(element).transform);
}

describe("R2715002 the signature signs fast and lands hard, in its card's hand", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    cy.viewport(1280, 720);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  it("R2715002 mid-signing the pen tip leads the ink inside the name, and the landed name is whole, in two sets' hands", () => {
    const at: { pens?: Map<string, number>; mid?: Shot } = {};
    cy.mount(<Mythics />);
    cy.get(".cf-signature", { timeout: SETTLE_TIMEOUT_MS }).should("have.length", CARDS.length);
    cy.document().then((doc) => {
      // Two hands, each its set's.
      const families = CARDS.map(({ id, why }) => {
        const text = must(signatureOf(doc, id).querySelector(".cf-signature-ink > text"), `${id}'s name`);
        const family = SIGNATURE_FACES[signatureFaceOf(id)].family;
        expect(getComputedStyle(text).fontFamily, why).to.contain(family);
        return family;
      });
      expect(new Set(families).size, "two different hands").to.eq(CARDS.length);
      holdAt(doc, MID_SIGNING_MS);
    });
    cy.document().then((doc) => {
      const pens = new Map<string, number>();
      for (const { id, why } of CARDS) {
        const signature = signatureOf(doc, id);
        const name = must(signature.querySelector(".cf-signature-ink > text"), `${id}'s name`).getBoundingClientRect();
        const pen = must(signature.querySelector(".cf-signature-pen"), `${id}'s pen`);
        const tip = must(signature.querySelector(".cf-signature-pen-tip"), `${id}'s pen tip`).getBoundingClientRect();
        const tipX = tip.left + tip.width / 2;
        expect(Number(getComputedStyle(pen).opacity), `${why}: the pen tip is lit`).to.be.at.least(PEN_OPACITY_MIN);
        expect(tipX, `${why}: the pen is past the name's start`).to.be.greaterThan(name.left);
        expect(tipX, `${why}: and short of its end`).to.be.lessThan(name.right);
        const wipe = must(signature.querySelector(".cf-signature-wipe"), `${id}'s wipe`);
        expect(atRest(wipe), `${why}: the wipe is under way`).to.eq(false);
        pens.set(id, tipX);
      }
      at.pens = pens;
    });
    screenshot("signature-mid").then((shot) => {
      at.mid = shot;
    });
    cy.document().then((doc) => holdAt(doc, LANDED_MS));
    cy.document().then((doc) => {
      for (const { id, why } of CARDS) {
        const signature = signatureOf(doc, id);
        for (const part of [".cf-signature-wipe", ".cf-signature-pen", ".cf-signature-flourish"]) {
          expect(atRest(must(signature.querySelector(part), part)), `${why}: ${part} has landed`).to.eq(true);
        }
        expect(Number(getComputedStyle(must(signature.querySelector(".cf-signature-pen"), "pen")).opacity), `${why}: the pen has lifted`).to.eq(0);
        expect(Number(getComputedStyle(must(signature.querySelector(".cf-signature-burst"), "burst")).opacity), `${why}: the burst is over`).to.eq(0);
      }
    });
    screenshot("signature-landed").then((landed) => {
      cy.document().then((doc) => {
        const mid = must(at.mid, "mid-signing screenshot");
        for (const { id, why } of CARDS) {
          const text = must(signatureOf(doc, id).querySelector(".cf-signature-ink > text"), `${id}'s name`);
          const name = text.getBoundingClientRect();
          const ink = inkOf(text);
          const penX = must(at.pens?.get(id), `${id}'s pen`);
          // Past the pen the name was still unwritten mid-signing, and the landing inked it; before the
          // feathered edge it was already written.
          const after = pointsIn(name, penX + FEATHER_GAP_PX, name.right);
          const before = pointsIn(name, name.left, penX - FEATHER_GAP_PX);
          const counts = {
            afterMid: inkCount(mid, after, ink),
            afterLanded: inkCount(landed, after, ink),
            beforeMid: inkCount(mid, before, ink),
            beforeLanded: inkCount(landed, before, ink),
          };
          cy.task("layout:report", { spec: "signature", card: id, ...counts });
          expect(counts.afterLanded, `${why}: the landed name is drawn past the pen`).to.be.at.least(INK_POINTS_MIN);
          expect(counts.afterMid, `${why}: mid-signing, the name past the pen is unwritten`).to.be.at.most(counts.afterLanded * UNWRITTEN_INK_SHARE_MAX);
          expect(counts.beforeMid, `${why}: mid-signing, the name before the pen is written`).to.be.at.least(counts.beforeLanded * UNWRITTEN_INK_SHARE_MAX);
        }
      });
    });
  });

  it("R2715002 under Reduce motion the name is whole from the first frame and nothing moves", () => {
    writeSettings({ reduceMotion: true });
    const first: { shot?: Shot } = {};
    cy.mount(<Mythics />);
    cy.get("html").should("have.attr", "data-reduce-motion", "true");
    cy.get(".cf-signature", { timeout: SETTLE_TIMEOUT_MS }).should("have.length", CARDS.length);
    cy.document().then((doc) => {
      for (const { id, why } of CARDS) {
        const signature = signatureOf(doc, id);
        expect(signature.getAnimations({ subtree: true }).length, `${why}: nothing runs`).to.eq(0);
        for (const part of [".cf-signature-wipe", ".cf-signature-pen", ".cf-signature-flourish", ".cf-signature-shine"]) {
          const element = must(signature.querySelector(part), part);
          expect(getComputedStyle(element).animationName, `${why}: ${part}`).to.eq("none");
          expect(atRest(element), `${why}: ${part} at rest`).to.eq(true);
        }
        for (const part of [".cf-signature-pen", ".cf-signature-burst"]) {
          expect(Number(getComputedStyle(must(signature.querySelector(part), part)).opacity), `${why}: ${part} hidden`).to.eq(0);
        }
        const box = signature.getBoundingClientRect();
        const name = must(signature.querySelector(".cf-signature-ink > text"), `${id}'s name`).getBoundingClientRect();
        expect(name.width, `${why}: the name is drawn`).to.be.greaterThan(0);
        expect(name.left, `${why}: inside its box`).to.be.at.least(box.left - 0.5);
        expect(name.right, `${why}: inside its box`).to.be.at.most(box.right + 0.5);
      }
    });
    screenshot("signature-still-first").then((shot) => {
      first.shot = shot;
    });
    cy.wait(STILL_GAP_MS);
    screenshot("signature-still-later").then((later) => {
      cy.document().then((doc) => {
        for (const { id, why } of CARDS) {
          const box = signatureOf(doc, id).getBoundingClientRect();
          expect(changedShare(must(first.shot, "first frame"), later, pointsIn(box, box.left, box.right)), `${why}: the same frame`).to.eq(0);
        }
      });
    });
  });
});
