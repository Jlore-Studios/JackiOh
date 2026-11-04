// R2725003, R2725004, R2949000: a card's versions and augments in a real browser. apps/web's vitest suite proves the
// attributes each look wears (cards/fullArtTiers.test.tsx) and the stylesheet's colors (cards/cosmetics.test.tsx);
// what only a layout and style engine can say is measured here. Part 1's two versions, the normal face and Full art,
// and the Signature look (Full art with Animated and Signature, R2949000), on one Unit and one Spell, base and
// Radiant, the normal face bare and with Animated, the Unit with a foil: every part keeps clear of every other, only
// the Signature look signs and it draws a picture of its own, a Radiant face keeps its gold frame and its art's
// Radiant paint under every look, and under Reduce motion nothing moves.
// R2940001, R2940002: part 2's versions, Alt art, the Sketch and Alt full art, each bare and with Animated (the Unit
// with a foil under it), on a Unit and a Spell offered all three, base and Radiant: every part shows and none signs,
// each draws its own picture (the Sketch in pencil on paper), and a Radiant face keeps its gold frame.
// R2942001, R2942002: part 3's versions, Extended art, Pixel and Comic, each bare and with Animated (the Unit with a
// foil under it), on a Unit and a Spell offered all three, base and Radiant: every part shows and none signs, Extended
// art's picture runs past its window, Pixel's and Comic's are their own (a base Pixel in its palette's levels), and a
// Radiant face keeps its gold frame.
// R2943001, R2943002: part 4's versions, Set showcase and Out of frame, each bare and with Animated (the first Unit with
// a foil under it), on two Units offered both and a Spell offered Set showcase, base and Radiant: every part shows and
// none signs, Set showcase's picture is its set's own, Out of frame's subject rises out of its window inside the face
// and leaves the cost gem and the stats as they were, and a Radiant face keeps its gold frame.
//
// Each face is the viewer's own copy (`worn`, below an `OwnCopy`) shown in a look by `FullArtShown`, as the Collection's preview
// shows one, so the looks stand side by side with no pick written. No `FullArtOwned` is mounted, so every cosmetic
// counts as owned where it is offered, as in practice.

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { resetCosmeticsForTests, writeCardPick } from "../../../apps/web/src/cards/cosmetics.ts";
import { FullArtShown, OwnCopy } from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_COMIC_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_FULL_ART_ID,
  VERSION_OUT_OF_FRAME_ID,
  VERSION_PIXEL_ID,
  VERSION_SET_SHOWCASE_ID,
  VERSION_SKETCH_ID,
  artPictureOf,
  isFullBleed,
  offeredOn,
} from "../../../apps/web/src/cards/versions.ts";
import { showcaseFamilyOf } from "../../../apps/web/src/cards/art/pictures.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

type CardDef = (typeof CATALOG)[string];

/** One Unit and one Spell offered both versions and both augments (R2725001). */
const CARDS = [
  { id: "classicplus-012", why: "a Unit (The Mother Pancake: stats in the bottom corners, a Legendary crest)" },
  { id: "stinkyscape-154", why: "a Spell (Dice of Doom: no stats, a Mythic crest)" },
] as const;

/** The Unit wears a foil under its augment stack. */
const FOIL = "foil-rainbow";
/** R2949000: what Signature needs beside it, and itself. */
const STACK = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID] as const;

/** The normal face bare and with Animated, Full art bare, and the Signature look. */
const LOOKS = [
  { name: "normal", pick: false, augments: [] },
  { name: "normal-animated", pick: false, augments: [AUGMENT_ANIMATED_ID] },
  { name: "full", pick: VERSION_FULL_ART_ID, augments: [] },
  { name: "signature", pick: VERSION_FULL_ART_ID, augments: STACK },
] as const;

/** R2940001: one Unit and one Spell offered each of part 2's versions and Animated. */
const NEW_CARDS = [
  { id: "classicplus-012", why: "a Unit (The Mother Pancake: stats in the bottom corners, a Legendary crest)" },
  { id: "core-083", why: "a Spell (Transmogulate: no stats, a Legendary crest)" },
] as const;

/** R2940002: the normal face to compare with, then each of part 2's versions bare and with Animated, Signature on none. */
const NEW_LOOKS = [
  { name: "normal", pick: false, augments: [] },
  { name: "alt", pick: VERSION_ALT_ART_ID, augments: [] },
  { name: "alt-stack", pick: VERSION_ALT_ART_ID, augments: [AUGMENT_ANIMATED_ID] },
  { name: "sketch", pick: VERSION_SKETCH_ID, augments: [] },
  { name: "sketch-stack", pick: VERSION_SKETCH_ID, augments: [AUGMENT_ANIMATED_ID] },
  { name: "alt-full", pick: VERSION_ALT_FULL_ART_ID, augments: [] },
  { name: "alt-full-stack", pick: VERSION_ALT_FULL_ART_ID, augments: [AUGMENT_ANIMATED_ID] },
] as const;

/** R2940001: the looks the screenshot compares, the Sketch with its stack too (Animated + the Unit's foil). */
const SHOT_LOOKS = NEW_LOOKS.filter((look) => ["normal", "alt", "sketch", "sketch-stack", "alt-full"].includes(look.name));
/** Small enough that every face of the screenshot lies inside the viewport it captures: three rows of nine. */
const SHOT_WIDTH = 120;

/** R2940001: the Sketch's base bezel, palette.css's `--pal-slate-700`. */
const SKETCH_BEZEL = "#3f4964";
/** R2940001: the most a base Sketch's sampled pixels may hold of colour on average (max channel less min, of 255): graphite on paper. */
const SKETCH_CHROMA_MAX = 48;

/** R2942001: one Unit and one Spell offered each of part 3's versions and Animated. */
const PART_THREE_CARDS = [
  { id: "classic-056", why: "a Unit (Spell Tyrant: stats in the bottom corners, a Legendary crest)" },
  { id: "meditative-042", why: "a Spell (CN Flea Market: no stats, a Legendary crest)" },
] as const;

/** R2942002: the normal face to compare with, then each of part 3's versions bare and with Animated, Signature on none. */
const PART_THREE_LOOKS = [
  { name: "normal", pick: false, augments: [] },
  { name: "extended", pick: VERSION_EXTENDED_ART_ID, augments: [] },
  { name: "extended-stack", pick: VERSION_EXTENDED_ART_ID, augments: [AUGMENT_ANIMATED_ID] },
  { name: "pixel", pick: VERSION_PIXEL_ID, augments: [] },
  { name: "pixel-stack", pick: VERSION_PIXEL_ID, augments: [AUGMENT_ANIMATED_ID] },
  { name: "comic", pick: VERSION_COMIC_ID, augments: [] },
  { name: "comic-stack", pick: VERSION_COMIC_ID, augments: [AUGMENT_ANIMATED_ID] },
] as const;

/** R2942001: the looks the screenshot compares, Pixel with its stack too (Animated + the Unit's foil). */
const PART_THREE_SHOT_LOOKS = PART_THREE_LOOKS.filter((look) => ["normal", "extended", "pixel", "pixel-stack", "comic"].includes(look.name));

/** R2942001: the base bezels of Pixel and Comic, palette.css's `--pal-slate-400` and `--pal-white`. */
const PART_THREE_BEZEL: Readonly<Record<string, string>> = { [VERSION_PIXEL_ID]: "#9aa1b8", [VERSION_COMIC_ID]: "#ffffff" };
/** R2942001: the channel values a base Pixel's palette cuts to (`PIXEL_LEVELS` of 255), and how near a sampled one must lie. */
const PIXEL_CHANNELS = [0, 128, 255] as const;
const PIXEL_CHANNEL_SLACK = 3;
/** R2942001: the least share of a base Pixel window's sampled points whose every channel is one of its palette's. */
const PIXEL_PALETTE_SHARE_MIN = 0.8;
/** The grid beside an art window, in CSS pixels: finer than the window's, the strip being narrow. */
const BESIDE_STEP_PX = 2;
/** How far the strip beside a window keeps from it and from the face's edge: clear of the bezel and the rim. */
const BESIDE_CLEARANCE_PX = 4;

/** R2943001: two Units offered both of part 4's versions and Animated, and a Spell offered Set showcase and Animated. */
const PART_FOUR_CARDS = [
  { id: "stinkyscape-080", why: "a Unit (Galaxy: Stinkyscape's painter, stats in the bottom corners, a Legendary crest)" },
  { id: "classic-080", why: "a Unit (BOOM! Big Max: the procedural painter, stats, a Legendary crest)" },
  { id: "core-083", why: "a Spell (Transmogulate: no stats, a Legendary crest; Out of frame is a Unit's)" },
] as const;

/** R2943002: the normal face to compare with, then each of part 4's versions bare and with Animated, Signature on none. */
const PART_FOUR_LOOKS = [
  { name: "normal", pick: false, augments: [] },
  { name: "showcase", pick: VERSION_SET_SHOWCASE_ID, augments: [] },
  { name: "showcase-stack", pick: VERSION_SET_SHOWCASE_ID, augments: [AUGMENT_ANIMATED_ID] },
  { name: "out", pick: VERSION_OUT_OF_FRAME_ID, augments: [] },
  { name: "out-stack", pick: VERSION_OUT_OF_FRAME_ID, augments: [AUGMENT_ANIMATED_ID] },
] as const;

/** R2943001: the looks the screenshot compares, Out of frame with its stack too (Animated + the first Unit's foil). */
const PART_FOUR_SHOT_LOOKS = PART_FOUR_LOOKS.filter((look) => ["normal", "showcase", "out", "out-stack"].includes(look.name));

type Card = { readonly id: string; readonly why: string };
type Look = { readonly name: string; readonly pick: false | string; readonly augments: readonly string[] };

/** R2943001: the looks of `looks` card `id` is offered: the normal face, and each version where it is offered. */
function offeredLooks(id: string, looks: readonly Look[]): Look[] {
  return looks.filter((look) => look.pick === false || offeredOn(look.pick, id));
}

type Point = { x: number; y: number };
type Shot = { width: number; height: number; data: Uint8ClampedArray; scale: number };

/** The grid the art windows are sampled on, in CSS pixels, and the inset from their edges (animated-art.cy.tsx's). */
const SAMPLE_STEP_PX = 6;
const SAMPLE_INSET_PX = 3;
/** A channel must differ this much for a point to count as different: more than compression noise. */
const PIXEL_DELTA_MIN = 4;
/** R2949000: the share of the upper half of the art window that must differ between Full art and the Signature look. */
const PICTURE_SHARE_MIN = 0.2;
/** Enough room for the faces to mount and the art to draw. */
const SETTLE_TIMEOUT_MS = 8_000;

/** The inspect-sized and hand-sized full faces card-faces.cy.tsx measures (B15). */
const WIDTHS = [270, 170] as const;

/** What a face's parts must keep apart from the signature: none of them may cover it, nor it them. */
const PARTS = [".cost-gem", ".cf-gem", ".cf-set", ".cf-crest", ".card-name", ".card-type", ".card-text", ".cf-atk", ".cf-hp"] as const;

function heightFor(width: number): number {
  return Math.round(width / FACE_ASPECT);
}

function overlaps(a: DOMRect, b: DOMRect): boolean {
  const SLACK = 0.5;
  return a.left < b.right - SLACK && b.left < a.right - SLACK && a.top < b.bottom - SLACK && b.top < a.bottom - SLACK;
}

function Faces({ width, cards = CARDS, looks = LOOKS }: { width: number; cards?: readonly Card[]; looks?: readonly Look[] }) {
  return (
    <OwnCopy own>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8, background: "#1b1f2b" }}>
        {cards.flatMap(({ id }) =>
          looks.flatMap((look) =>
            [false, true].map((radiant) => (
              <div key={`${id}-${look.name}-${String(radiant)}`} data-face={`${id}-${look.name}-${radiant ? "radiant" : "base"}`} style={{ width, height: heightFor(width), flex: "none" }}>
                <FullArtShown defId={id} pick={look.pick} augments={look.augments}>
                  <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant })} layout="full" worn />
                </FullArtShown>
              </div>
            )),
          ),
        )}
      </div>
    </OwnCopy>
  );
}

/** R2943001: each card's faces in the looks it is offered, one row of `Faces` a card. */
function OfferedFaces({ width, cards, looks }: { width: number; cards: readonly Card[]; looks: readonly Look[] }) {
  return (
    <>
      {cards.map((card) => (
        <Faces key={card.id} width={width} cards={[card]} looks={offeredLooks(card.id, looks)} />
      ))}
    </>
  );
}

function faceIn(doc: Document, id: string, look: string, radiant: boolean): HTMLElement {
  const cf = doc.querySelector<HTMLElement>(`[data-face="${id}-${look}-${radiant ? "radiant" : "base"}"] > .cf`);
  if (cf === null) throw new Error(`no face for ${id} ${look}`);
  return cf;
}

/** The boxes of `cf`'s parts matching `selector` that draw. */
function boxes(cf: HTMLElement, selector: string): DOMRect[] {
  return [...cf.querySelectorAll<HTMLElement>(selector)]
    .filter((element) => getComputedStyle(element).display !== "none" && element.getBoundingClientRect().width > 0)
    .map((element) => element.getBoundingClientRect());
}

/** The viewport as Chrome painted it, decoded to RGBA, with the device pixels per CSS pixel (animated-art.cy.tsx's). */
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

/** The offsets of the sampling grid over the whole of an art window `box`. */
function windowOffsets(box: DOMRect): Point[] {
  const points: Point[] = [];
  for (let y = SAMPLE_INSET_PX; y < box.height - SAMPLE_INSET_PX; y += SAMPLE_STEP_PX) {
    for (let x = SAMPLE_INSET_PX; x < box.width - SAMPLE_INSET_PX; x += SAMPLE_STEP_PX) points.push({ x, y });
  }
  return points;
}

/** The share of `offsets` at which the windows `a` and `b` of `shot` differ. */
function differingShare(shot: Shot, a: DOMRect, b: DOMRect, offsets: readonly Point[]): number {
  const differing = offsets.filter((offset) => {
    const one = pixelAt(shot, { x: a.left + offset.x, y: a.top + offset.y });
    const two = pixelAt(shot, { x: b.left + offset.x, y: b.top + offset.y });
    return one.some((channel, i) => Math.abs(channel - (two[i] ?? -1)) >= PIXEL_DELTA_MIN);
  });
  return differing.length / offsets.length;
}

/** The offsets of the sampling grid over the upper half of an art window `box`, clear of the signature below its middle. */
function upperOffsets(box: DOMRect): Point[] {
  const points: Point[] = [];
  for (let y = SAMPLE_INSET_PX; y < box.height / 2; y += SAMPLE_STEP_PX) {
    for (let x = SAMPLE_INSET_PX; x < box.width - SAMPLE_INSET_PX; x += SAMPLE_STEP_PX) points.push({ x, y });
  }
  return points;
}

describe("R2725003 a card's versions and augments on real faces", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    writeCardPick(CARDS[0].id, "foil", FOIL);
    cy.viewport(1280, 900);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  for (const width of WIDTHS) {
    it(`R2725003 R2949000 every look keeps its parts apart, and only the Signature look signs, at ${width}px`, () => {
      cy.mount(<Faces width={width} />);
      cy.document().should((doc) => {
        for (const { id, why } of CARDS) {
          for (const look of LOOKS) {
            for (const radiant of [false, true]) {
              const where = `${id} (${why}) ${look.name} ${radiant ? "Radiant" : "base"}, ${width}px`;
              const cf = faceIn(doc, id, look.name, radiant);
              const face = cf.getBoundingClientRect();
              const full = look.pick !== false;
              expect(face.width, `${where}: the face fills its box`).to.be.greaterThan(width / 2);
              expect(cf.getAttribute("data-full-art"), where).to.eq(full ? "true" : null);
              expect(cf.getAttribute("data-augments"), where).to.eq(look.augments.length > 0 ? look.augments.join(" ") : null);
              // The foil wears under the augments on the Unit's base faces; a Radiant face wears none.
              if (id === CARDS[0].id) expect(cf.getAttribute("data-cosmetic-foil"), `${where}: the foil`).to.eq(radiant ? null : FOIL);
              // R2949000: the Signature look alone signs, and draws a picture of its own.
              const signed = look.name === "signature";
              expect(cf.querySelector(".cf-art")?.getAttribute("data-art-picture"), `${where}: the picture`).to.eq(signed ? "signature" : null);
              const signature = boxes(cf, ".cf-signature");
              expect(signature.length, `${where}: the signature`).to.eq(signed ? 1 : 0);
              for (const box of signature) {
                expect(box.left, `${where}: the signature inside`).to.be.at.least(face.left - 0.5);
                expect(box.right, `${where}: the signature inside`).to.be.at.most(face.right + 0.5);
                for (const selector of PARTS) {
                  for (const part of boxes(cf, selector)) {
                    expect(overlaps(box, part), `${where}: the signature covers ${selector}`).to.eq(false);
                  }
                }
                const text = cf.querySelector<SVGTextElement>(".cf-signature text")?.getBoundingClientRect();
                expect(text?.width ?? 0, `${where}: the name has a width`).to.be.greaterThan(width / 5);
              }
              // R2725004: the gold-foil frame and the art's Radiant paint on a Radiant face alone.
              expect(cf.getAttribute("data-radiant-face"), where).to.eq(radiant ? "true" : null);
              expect(cf.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).to.eq(radiant ? "radiant" : "base");
              expect(getComputedStyle(cf).getPropertyValue("--cf-bezel").trim() === "#f6d77a", `${where}: the gold bezel`).to.eq(radiant);
            }
          }
        }
      });
    });
  }

  it("R2725003 under Reduce motion no look moves and the signature stands whole", () => {
    writeSettings({ reduceMotion: true });
    cy.mount(<Faces width={170} />);
    cy.get("html").should("have.attr", "data-reduce-motion", "true");
    cy.document().should((doc) => {
      for (const { id } of CARDS) {
        for (const look of LOOKS) {
          for (const radiant of [false, true]) {
            const cf = faceIn(doc, id, look.name, radiant);
            const where = `${id} ${look.name} ${radiant ? "Radiant" : "base"}`;
            expect(
              cf.getAnimations({ subtree: true }).filter((animation) => animation.playState === "running").length,
              `${where}: nothing moves`,
            ).to.eq(0);
            if (look.augments.length > 0) {
              expect(cf.getAttribute("data-art-motion"), `${where}: the art holds still`).to.eq("static");
            }
            if (look.name === "signature") {
              const text = cf.querySelector(".cf-signature text")?.getBoundingClientRect();
              expect(text?.width ?? 0, `${where}: the name is drawn`).to.be.greaterThan(0);
            }
          }
        }
      }
    });
  });

  it("R2949000 the Signature look draws a picture of its own on a Unit and a Spell, base and Radiant", () => {
    // Animated foil off holds every picture still, so one screenshot compares the pictures alone.
    writeCardSettings({ animatedFoil: false });
    cy.mount(<Faces width={170} />);
    cy.get('.cf[data-art-motion="static"]', { timeout: SETTLE_TIMEOUT_MS }).should("have.length", CARDS.length * 2 * 2);
    cy.get('[data-art-moving="true"]').should("not.exist");
    cy.get("[data-face] .cf-art[data-art-pending]").should("not.exist");
    cy.get('.cf-art[data-art-picture="signature"]').should("have.length", CARDS.length * 2);
    cy.document().its("fonts.status").should("eq", "loaded");
    const taken: { windows?: Map<string, [DOMRect, DOMRect]> } = {};
    cy.document().then((doc) => {
      const windows = new Map<string, [DOMRect, DOMRect]>();
      for (const { id } of CARDS) {
        for (const radiant of [false, true]) {
          const art = (look: string): DOMRect => {
            const box = faceIn(doc, id, look, radiant).querySelector(".cf-art")?.getBoundingClientRect();
            if (box === undefined) throw new Error(`no art window for ${id} ${look}`);
            return box;
          };
          windows.set(`${id} ${radiant ? "Radiant" : "base"}`, [art("full"), art("signature")]);
        }
      }
      taken.windows = windows;
    });
    screenshot("signature-picture").then((shot) => {
      const shares: Record<string, number> = {};
      for (const [where, [full, signature]] of taken.windows ?? new Map<string, [DOMRect, DOMRect]>()) {
        expect(Math.round(full.width), `${where}: the two windows match`).to.eq(Math.round(signature.width));
        const offsets = upperOffsets(full);
        expect(offsets.length, `${where}: sampled points`).to.be.greaterThan(20);
        const differing = offsets.filter((offset) => {
          const a = pixelAt(shot, { x: full.left + offset.x, y: full.top + offset.y });
          const b = pixelAt(shot, { x: signature.left + offset.x, y: signature.top + offset.y });
          return a.some((channel, i) => Math.abs(channel - (b[i] ?? -1)) >= PIXEL_DELTA_MIN);
        });
        shares[where] = differing.length / offsets.length;
        expect(shares[where], `${where}: the Signature look's picture differs from Full art's`).to.be.at.least(PICTURE_SHARE_MIN);
      }
      expect(Object.keys(shares).length, "every card and face sampled").to.eq(CARDS.length * 2);
      cy.task("layout:report", { spec: "card-versions", shares });
    });
  });

  for (const width of WIDTHS) {
    it(`R2725003 R2940002 each of part 2's versions keeps every part, and signs nowhere, at ${width}px`, () => {
      writeCardPick(NEW_CARDS[0].id, "foil", FOIL);
      cy.mount(<Faces width={width} cards={NEW_CARDS} looks={NEW_LOOKS} />);
      cy.document().should((doc) => {
        for (const { id, why } of NEW_CARDS) {
          for (const look of NEW_LOOKS) {
            for (const radiant of [false, true]) {
              const where = `${id} (${why}) ${look.name} ${radiant ? "Radiant" : "base"}, ${width}px`;
              const cf = faceIn(doc, id, look.name, radiant);
              const face = cf.getBoundingClientRect();
              const version = look.pick === false ? null : look.pick;
              const full = isFullBleed(version);
              expect(face.width, `${where}: the face fills its box`).to.be.greaterThan(width / 2);
              expect(cf.getAttribute("data-version"), where).to.eq(version);
              expect(cf.getAttribute("data-full-art"), where).to.eq(full ? "true" : null);
              expect(cf.getAttribute("data-augments"), where).to.eq(look.augments.length > 0 ? look.augments.join(" ") : null);
              if (id === NEW_CARDS[0].id) expect(cf.getAttribute("data-cosmetic-foil"), `${where}: the foil`).to.eq(radiant ? null : FOIL);
              const picture = artPictureOf(version);
              expect(cf.querySelector(".cf-art")?.getAttribute("data-art-picture"), `${where}: the picture`).to.eq(picture === "normal" ? null : picture);
              // R2949000: Signature on none of them.
              expect(boxes(cf, ".cf-signature").length, `${where}: no signature`).to.eq(0);
              // Every version keeps the cost gem, the crest, the name and a Unit's stats; the normal frame its words.
              const kept = [".cost-gem", ".cf-crest", ".card-name", ...(full ? [] : [".card-type", ".card-text"]), ...(id === NEW_CARDS[0].id ? [".cf-atk", ".cf-hp"] : [])];
              for (const selector of kept) expect(boxes(cf, selector).length, `${where}: ${selector}`).to.be.greaterThan(0);
              // R2725004: the gold-foil frame and the art's Radiant paint on a Radiant face alone; the Sketch's base
              // bezel is graphite.
              expect(cf.getAttribute("data-radiant-face"), where).to.eq(radiant ? "true" : null);
              expect(cf.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).to.eq(radiant ? "radiant" : "base");
              const bezel = getComputedStyle(cf).getPropertyValue("--cf-bezel").trim();
              expect(bezel === "#f6d77a", `${where}: the gold bezel`).to.eq(radiant);
              if (version === VERSION_SKETCH_ID && !radiant) expect(bezel, `${where}: the graphite bezel`).to.eq(SKETCH_BEZEL);
            }
          }
        }
      });
    });
  }

  it("R2940001 each new version prints its picture, the Sketch in pencil on paper, on a Unit and a Spell, base and Radiant", () => {
    // Animated foil off holds every picture still, so one screenshot compares the pictures alone.
    writeCardSettings({ animatedFoil: false });
    writeCardPick(NEW_CARDS[0].id, "foil", FOIL);
    cy.mount(<Faces width={SHOT_WIDTH} cards={NEW_CARDS} looks={SHOT_LOOKS} />);
    cy.get('.cf[data-art-motion="static"]', { timeout: SETTLE_TIMEOUT_MS }).should("have.length", NEW_CARDS.length * 2);
    cy.get('[data-art-moving="true"]').should("not.exist");
    cy.get("[data-face] .cf-art[data-art-pending]").should("not.exist");
    cy.get('.cf-art[data-art-picture="alt"]').should("have.length", NEW_CARDS.length * 2 * 2);
    cy.get('.cf-art[data-art-picture="sketch"]').should("have.length", NEW_CARDS.length * 2 * 2);
    cy.document().its("fonts.status").should("eq", "loaded");
    const taken: { windows?: Map<string, Record<string, DOMRect>> } = {};
    cy.document().then((doc) => {
      const windows = new Map<string, Record<string, DOMRect>>();
      for (const { id } of NEW_CARDS) {
        for (const radiant of [false, true]) {
          const art = (look: string): DOMRect => {
            const box = faceIn(doc, id, look, radiant).querySelector(".cf-art")?.getBoundingClientRect();
            if (box === undefined) throw new Error(`no art window for ${id} ${look}`);
            return box;
          };
          windows.set(`${id} ${radiant ? "Radiant" : "base"}`, { normal: art("normal"), alt: art("alt"), sketch: art("sketch") });
        }
      }
      taken.windows = windows;
    });
    screenshot("card-versions-part-2").then((shot) => {
      const shares: Record<string, number> = {};
      for (const [where, { normal, alt, sketch }] of taken.windows ?? new Map<string, Record<string, DOMRect>>()) {
        if (normal === undefined || alt === undefined || sketch === undefined) throw new Error(`${where}: a window is missing`);
        // Every window sampled lies inside what the screenshot captured.
        for (const box of [normal, alt, sketch]) expect(Math.ceil(box.bottom * shot.scale), `${where}: inside the shot`).to.be.at.most(shot.height);
        expect(Math.round(normal.width), `${where}: the windows match`).to.eq(Math.round(alt.width));
        expect(Math.round(normal.width), `${where}: the windows match`).to.eq(Math.round(sketch.width));
        const offsets = windowOffsets(normal);
        expect(offsets.length, `${where}: sampled points`).to.be.greaterThan(20);
        for (const [pair, a, b] of [["alt", normal, alt], ["sketch", normal, sketch], ["alt-sketch", alt, sketch]] as const) {
          const share = differingShare(shot, a, b, offsets);
          shares[`${where} ${pair}`] = share;
          expect(share, `${where}: ${pair} differ`).to.be.at.least(PICTURE_SHARE_MIN);
        }
        // A base Sketch is graphite on paper: little colour left in it.
        if (where.endsWith("base")) {
          const chroma =
            offsets.reduce((sum, offset) => {
              const pixel = pixelAt(shot, { x: sketch.left + offset.x, y: sketch.top + offset.y });
              return sum + Math.max(...pixel) - Math.min(...pixel);
            }, 0) / offsets.length;
          shares[`${where} sketch chroma`] = chroma;
          expect(chroma, `${where}: the Sketch is graphite on paper`).to.be.at.most(SKETCH_CHROMA_MAX);
        }
      }
      expect(Object.keys(shares).length, "every card and face sampled").to.eq(NEW_CARDS.length * (2 * 3 + 1));
      cy.task("layout:report", { spec: "card-versions-part-2", shares });
    });
  });

  for (const width of WIDTHS) {
    it(`R2725003 R2942002 each of part 3's versions keeps every part, and signs nowhere, at ${width}px`, () => {
      writeCardPick(PART_THREE_CARDS[0].id, "foil", FOIL);
      cy.mount(<Faces width={width} cards={PART_THREE_CARDS} looks={PART_THREE_LOOKS} />);
      cy.document().should((doc) => {
        for (const { id, why } of PART_THREE_CARDS) {
          for (const look of PART_THREE_LOOKS) {
            for (const radiant of [false, true]) {
              const where = `${id} (${why}) ${look.name} ${radiant ? "Radiant" : "base"}, ${width}px`;
              const cf = faceIn(doc, id, look.name, radiant);
              const face = cf.getBoundingClientRect();
              const version = look.pick === false ? null : look.pick;
              expect(face.width, `${where}: the face fills its box`).to.be.greaterThan(width / 2);
              expect(cf.getAttribute("data-version"), where).to.eq(version);
              expect(cf.getAttribute("data-full-art"), where).to.eq(null);
              expect(cf.getAttribute("data-augments"), where).to.eq(look.augments.length > 0 ? look.augments.join(" ") : null);
              if (id === PART_THREE_CARDS[0].id) expect(cf.getAttribute("data-cosmetic-foil"), `${where}: the foil`).to.eq(radiant ? null : FOIL);
              const picture = artPictureOf(version);
              const art = cf.querySelector<HTMLElement>(".cf-art");
              expect(art?.getAttribute("data-art-picture"), `${where}: the picture`).to.eq(picture === "normal" ? null : picture);
              // R2949000: Signature on none of them.
              expect(boxes(cf, ".cf-signature").length, `${where}: no signature`).to.eq(0);
              // Every version keeps the cost gem, the crest, the name, the type line, the rules box and a Unit's stats.
              const kept = [".cost-gem", ".cf-crest", ".card-name", ".card-type", ".card-text", ...(id === PART_THREE_CARDS[0].id ? [".cf-atk", ".cf-hp"] : [])];
              for (const selector of kept) expect(boxes(cf, selector).length, `${where}: ${selector}`).to.be.greaterThan(0);
              // Extended art's picture covers the panel; every other look's sits in its window.
              const scale = cf.querySelector(".cf-scale")?.getBoundingClientRect();
              const drawn = art?.getBoundingClientRect();
              const covers = scale !== undefined && drawn !== undefined && drawn.width >= scale.width - 1 && drawn.height >= scale.height - 1;
              expect(covers, `${where}: the art across the face`).to.eq(version === VERSION_EXTENDED_ART_ID);
              // Pixel's name in its pixel face, the banner's words kept unseen.
              const name = cf.querySelector<HTMLElement>(".card-name");
              expect(name?.textContent, `${where}: the name's words`).to.eq(CATALOG[id]?.name);
              const pixelled = version === VERSION_PIXEL_ID;
              expect(name?.getAttribute("data-pixel-name"), `${where}: the pixel name`).to.eq(pixelled ? "true" : null);
              if (name !== null && name !== undefined) expect(getComputedStyle(name).backgroundImage.includes("data:image/svg+xml"), `${where}: the pixel face`).to.eq(pixelled);
              // R2725004: the gold-foil frame and the art's Radiant paint on a Radiant face alone; Pixel's and Comic's
              // base bezels the palette's.
              expect(cf.getAttribute("data-radiant-face"), where).to.eq(radiant ? "true" : null);
              expect(art?.getAttribute("data-art-variant"), where).to.eq(radiant ? "radiant" : "base");
              const bezel = getComputedStyle(cf).getPropertyValue("--cf-bezel").trim();
              expect(bezel === "#f6d77a", `${where}: the gold bezel`).to.eq(radiant);
              const own = version === null ? undefined : PART_THREE_BEZEL[version];
              if (own !== undefined && !radiant) expect(bezel, `${where}: the version's bezel`).to.eq(own);
            }
          }
        }
      });
    });
  }

  it("R2942001 each part 3 version prints its look, on a Unit and a Spell, base and Radiant", () => {
    // Animated foil off holds every picture still, so one screenshot compares the pictures alone.
    writeCardSettings({ animatedFoil: false });
    writeCardPick(PART_THREE_CARDS[0].id, "foil", FOIL);
    cy.mount(<Faces width={SHOT_WIDTH} cards={PART_THREE_CARDS} looks={PART_THREE_SHOT_LOOKS} />);
    cy.get('.cf[data-art-motion="static"]', { timeout: SETTLE_TIMEOUT_MS }).should("have.length", PART_THREE_CARDS.length * 2);
    cy.get('[data-art-moving="true"]').should("not.exist");
    cy.get("[data-face] .cf-art[data-art-pending]").should("not.exist");
    cy.get('.cf-art[data-art-picture="pixel"]').should("have.length", PART_THREE_CARDS.length * 2 * 2);
    cy.get('.cf-art[data-art-picture="comic"]').should("have.length", PART_THREE_CARDS.length * 2);
    cy.get(".cf-art.cf-art--full").should("have.length", PART_THREE_CARDS.length * 2);
    cy.document().its("fonts.status").should("eq", "loaded");
    const taken: { windows?: Map<string, Record<string, DOMRect>> } = {};
    cy.document().then((doc) => {
      const windows = new Map<string, Record<string, DOMRect>>();
      for (const { id } of PART_THREE_CARDS) {
        for (const radiant of [false, true]) {
          const box = (look: string, selector: string): DOMRect => {
            const found = faceIn(doc, id, look, radiant).querySelector(selector)?.getBoundingClientRect();
            if (found === undefined) throw new Error(`no ${selector} for ${id} ${look}`);
            return found;
          };
          windows.set(`${id} ${radiant ? "Radiant" : "base"}`, {
            normal: box("normal", ".cf-art"),
            pixel: box("pixel", ".cf-art"),
            comic: box("comic", ".cf-art"),
            normalFace: box("normal", ".cf-scale"),
            extendedFace: box("extended", ".cf-scale"),
          });
        }
      }
      taken.windows = windows;
    });
    screenshot("card-versions-part-3").then((shot) => {
      const shares: Record<string, number> = {};
      for (const [where, { normal, pixel, comic, normalFace, extendedFace }] of taken.windows ?? new Map<string, Record<string, DOMRect>>()) {
        if (normal === undefined || pixel === undefined || comic === undefined || normalFace === undefined || extendedFace === undefined) {
          throw new Error(`${where}: a window is missing`);
        }
        // Every box sampled lies inside what the screenshot captured.
        for (const box of [normal, pixel, comic, normalFace, extendedFace]) expect(Math.ceil(box.bottom * shot.scale), `${where}: inside the shot`).to.be.at.most(shot.height);
        expect(Math.round(normal.width), `${where}: the windows match`).to.eq(Math.round(pixel.width));
        expect(Math.round(normal.width), `${where}: the windows match`).to.eq(Math.round(comic.width));
        const offsets = windowOffsets(normal);
        expect(offsets.length, `${where}: sampled points`).to.be.greaterThan(20);
        for (const [pair, a, b] of [["pixel", normal, pixel], ["comic", normal, comic], ["pixel-comic", pixel, comic]] as const) {
          const share = differingShare(shot, a, b, offsets);
          shares[`${where} ${pair}`] = share;
          expect(share, `${where}: ${pair} differ`).to.be.at.least(PICTURE_SHARE_MIN);
        }
        // Extended art's picture runs past its window: beside where the normal face's window sits, its panel gives
        // way to the picture.
        const beside: Point[] = [];
        for (let y = normal.top - normalFace.top; y < normal.bottom - normalFace.top; y += BESIDE_STEP_PX) {
          for (let x = BESIDE_CLEARANCE_PX; x < normal.left - normalFace.left - BESIDE_CLEARANCE_PX; x += BESIDE_STEP_PX) beside.push({ x, y });
        }
        expect(beside.length, `${where}: points beside the window`).to.be.greaterThan(20);
        const besideShare = differingShare(shot, normalFace, extendedFace, beside);
        shares[`${where} extended beside the window`] = besideShare;
        expect(besideShare, `${where}: Extended art past its window`).to.be.at.least(PICTURE_SHARE_MIN);
        // A base Pixel is cut to its palette's levels: the Spell's, which wears no foil over it.
        if (where === `${PART_THREE_CARDS[1].id} base`) {
          const inPalette = offsets.filter((offset) =>
            pixelAt(shot, { x: pixel.left + offset.x, y: pixel.top + offset.y }).every((channel) => PIXEL_CHANNELS.some((level) => Math.abs(channel - level) <= PIXEL_CHANNEL_SLACK)),
          ).length;
          shares[`${where} pixel palette`] = inPalette / offsets.length;
          expect(inPalette / offsets.length, `${where}: Pixel in its palette`).to.be.at.least(PIXEL_PALETTE_SHARE_MIN);
        }
      }
      expect(Object.keys(shares).length, "every card and face sampled").to.eq(PART_THREE_CARDS.length * 2 * 4 + 1);
      cy.task("layout:report", { spec: "card-versions-part-3", shares });
    });
  });

  for (const width of WIDTHS) {
    it(`R2725003 R2943002 each of part 4's versions keeps every part, and signs nowhere, at ${width}px`, () => {
      writeCardPick(PART_FOUR_CARDS[0].id, "foil", FOIL);
      cy.mount(<OfferedFaces width={width} cards={PART_FOUR_CARDS} looks={PART_FOUR_LOOKS} />);
      cy.document().should((doc) => {
        let cutouts = 0;
        for (const { id, why } of PART_FOUR_CARDS) {
          const unit = CATALOG[id]?.type === "Unit";
          expect(offeredOn(VERSION_SET_SHOWCASE_ID, id), `${id}: Set showcase`).to.eq(true);
          expect(offeredOn(VERSION_OUT_OF_FRAME_ID, id), `${id}: Out of frame`).to.eq(unit);
          for (const look of offeredLooks(id, PART_FOUR_LOOKS)) {
            for (const radiant of [false, true]) {
              const where = `${id} (${why}) ${look.name} ${radiant ? "Radiant" : "base"}, ${width}px`;
              const cf = faceIn(doc, id, look.name, radiant);
              const face = cf.getBoundingClientRect();
              const version = look.pick === false ? null : look.pick;
              const set = CATALOG[id]?.set ?? null;
              expect(face.width, `${where}: the face fills its box`).to.be.greaterThan(width / 2);
              expect(cf.getAttribute("data-version"), where).to.eq(version);
              expect(cf.getAttribute("data-full-art"), where).to.eq(null);
              expect(cf.getAttribute("data-augments"), where).to.eq(look.augments.length > 0 ? look.augments.join(" ") : null);
              if (id === PART_FOUR_CARDS[0].id) expect(cf.getAttribute("data-cosmetic-foil"), `${where}: the foil`).to.eq(radiant ? null : FOIL);
              expect(cf.getAttribute("data-showcase"), `${where}: the set's look`).to.eq(version === VERSION_SET_SHOWCASE_ID ? showcaseFamilyOf(set) : null);
              const picture = artPictureOf(version, set);
              const art = cf.querySelector<HTMLElement>(".cf-art:not(.cf-art--cutout)");
              expect(art?.getAttribute("data-art-picture"), `${where}: the picture`).to.eq(picture === "normal" ? null : picture);
              // R2949000: Signature on neither.
              expect(boxes(cf, ".cf-signature").length, `${where}: no signature`).to.eq(0);
              // Every version keeps the cost gem, the crest, the name, the type line, the rules box and a Unit's stats.
              const kept = [".cost-gem", ".cf-crest", ".card-name", ".card-type", ".card-text", ...(unit ? [".cf-atk", ".cf-hp"] : [])];
              for (const selector of kept) expect(boxes(cf, selector).length, `${where}: ${selector}`).to.be.greaterThan(0);
              // Out of frame's subject rises above its window and past its sides, and stays inside the face.
              const cutout = cf.querySelector<HTMLElement>(".cf-art--cutout");
              expect(cutout !== null, `${where}: the cutout`).to.eq(version === VERSION_OUT_OF_FRAME_ID);
              if (cutout !== null && art !== null && art !== undefined) {
                cutouts += 1;
                const subject = cutout.getBoundingClientRect();
                const opening = art.getBoundingClientRect();
                const scale = cf.querySelector(".cf-scale")?.getBoundingClientRect();
                expect(subject.top, `${where}: the subject above its window`).to.be.lessThan(opening.top - 1);
                expect(subject.width, `${where}: the subject wider than its window`).to.be.greaterThan(opening.width + 1);
                // Its foot sinks below the window's, under the name banner.
                const banner = cf.querySelector(".card-name")?.getBoundingClientRect();
                expect(subject.bottom, `${where}: the subject's foot below its window's`).to.be.greaterThan(opening.bottom);
                expect(banner !== undefined && subject.bottom <= banner.bottom + 0.5 && subject.bottom > banner.top, `${where}: the foot under the banner`).to.eq(true);
                expect(scale, `${where}: the face`).to.not.eq(undefined);
                if (scale !== undefined) {
                  expect(subject.top, `${where}: inside the face`).to.be.at.least(scale.top - 0.5);
                  expect(subject.left, `${where}: inside the face`).to.be.at.least(scale.left - 0.5);
                  expect(subject.right, `${where}: inside the face`).to.be.at.most(scale.right + 0.5);
                }
                expect(getComputedStyle(cutout).backgroundColor, `${where}: the cutout's ground`).to.eq("rgba(0, 0, 0, 0)");
                expect(getComputedStyle(cutout, "::after").content, `${where}: no foil on the cutout`).to.eq("none");
              }
              // R2725004: the gold-foil frame and the art's Radiant paint on a Radiant face alone.
              expect(cf.getAttribute("data-radiant-face"), where).to.eq(radiant ? "true" : null);
              for (const each of cf.querySelectorAll(".cf-art")) expect(each.getAttribute("data-art-variant"), where).to.eq(radiant ? "radiant" : "base");
              const bezel = getComputedStyle(cf).getPropertyValue("--cf-bezel").trim();
              expect(bezel === "#f6d77a", `${where}: the gold bezel`).to.eq(radiant);
            }
          }
        }
        expect(cutouts, "every Out of frame face measured").to.eq(2 * 2 * 2);
      });
    });
  }

  it("R2943001 each part 4 version prints its look, on two Units and a Spell, base and Radiant", () => {
    // Animated foil off holds every picture still, so one screenshot compares the pictures alone.
    writeCardSettings({ animatedFoil: false });
    writeCardPick(PART_FOUR_CARDS[0].id, "foil", FOIL);
    cy.mount(<OfferedFaces width={SHOT_WIDTH} cards={PART_FOUR_CARDS} looks={PART_FOUR_SHOT_LOOKS} />);
    const units = PART_FOUR_CARDS.filter(({ id }) => CATALOG[id]?.type === "Unit");
    cy.get('.cf[data-art-motion="static"]', { timeout: SETTLE_TIMEOUT_MS }).should("have.length", units.length * 2);
    cy.get('[data-art-moving="true"]').should("not.exist");
    cy.get("[data-face] .cf-art[data-art-pending]").should("not.exist");
    cy.get(".cf[data-showcase]").should("have.length", PART_FOUR_CARDS.length * 2);
    cy.get('.cf-art--cutout[data-art-part="subject"]').should("have.length", units.length * 2 * 2);
    cy.document().its("fonts.status").should("eq", "loaded");
    const taken: { boxes?: Map<string, Record<string, DOMRect | undefined>> } = {};
    cy.document().then((doc) => {
      const found = new Map<string, Record<string, DOMRect | undefined>>();
      for (const { id } of PART_FOUR_CARDS) {
        const unit = CATALOG[id]?.type === "Unit";
        for (const radiant of [false, true]) {
          const box = (look: string, selector: string): DOMRect => {
            const at = faceIn(doc, id, look, radiant).querySelector(selector)?.getBoundingClientRect();
            if (at === undefined) throw new Error(`no ${selector} for ${id} ${look}`);
            return at;
          };
          found.set(`${id} ${radiant ? "Radiant" : "base"}`, {
            normal: box("normal", ".cf-art"),
            showcase: box("showcase", ".cf-art"),
            normalGem: box("normal", ".cost-gem"),
            outGem: unit ? box("out", ".cost-gem") : undefined,
            normalAtk: unit ? box("normal", ".cf-atk") : undefined,
            outAtk: unit ? box("out", ".cf-atk") : undefined,
            normalHp: unit ? box("normal", ".cf-hp") : undefined,
            outHp: unit ? box("out", ".cf-hp") : undefined,
          });
        }
      }
      taken.boxes = found;
    });
    screenshot("card-versions-part-4").then((shot) => {
      const shares: Record<string, number> = {};
      const middle = (box: DOMRect): Point => ({ x: box.left + box.width / 2, y: box.top + box.height / 2 });
      for (const [where, found] of taken.boxes ?? new Map<string, Record<string, DOMRect | undefined>>()) {
        const { normal, showcase } = found;
        if (normal === undefined || showcase === undefined) throw new Error(`${where}: a window is missing`);
        for (const box of Object.values(found)) if (box !== undefined) expect(Math.ceil(box.bottom * shot.scale), `${where}: inside the shot`).to.be.at.most(shot.height);
        // Set showcase's picture is its set's own, apart from the normal one, in the same window.
        expect(Math.round(normal.width), `${where}: the windows match`).to.eq(Math.round(showcase.width));
        const offsets = windowOffsets(normal);
        expect(offsets.length, `${where}: sampled points`).to.be.greaterThan(20);
        const share = differingShare(shot, normal, showcase, offsets);
        shares[`${where} showcase`] = share;
        expect(share, `${where}: Set showcase differs`).to.be.at.least(PICTURE_SHARE_MIN);
        // Out of frame's subject stays under the cost gem and the stats: their middles print as on the normal face.
        for (const [part, a, b] of [["gem", found.normalGem, found.outGem], ["attack", found.normalAtk, found.outAtk], ["health", found.normalHp, found.outHp]] as const) {
          if (a === undefined || b === undefined) continue;
          const one = pixelAt(shot, middle(a));
          const two = pixelAt(shot, middle(b));
          const same = one.every((channel, i) => Math.abs(channel - (two[i] ?? -1)) < PIXEL_DELTA_MIN);
          shares[`${where} out ${part} kept`] = same ? 1 : 0;
          expect(same, `${where}: Out of frame keeps the ${part}'s middle (${one.join(",")} vs ${two.join(",")})`).to.eq(true);
        }
      }
      expect(Object.keys(shares).length, "every card and face sampled").to.eq(PART_FOUR_CARDS.length * 2 + units.length * 2 * 3);
      cy.task("layout:report", { spec: "card-versions-part-4", shares });
    });
  });
});
