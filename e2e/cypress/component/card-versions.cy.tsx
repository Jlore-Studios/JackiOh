// R2725003, R2725004, R2949000: a card's versions and augments in a real browser. apps/web's vitest suite proves the
// attributes each look wears (cards/fullArtTiers.test.tsx) and the stylesheet's colors (cards/cosmetics.test.tsx);
// what only a layout and style engine can say is measured here. Part 1's two versions, the normal face and Full art,
// and the Signature look (Full art with Animated and Signature, R2949000), on one Unit and one Spell, base and
// Radiant, the normal face bare and with Animated, the Unit with a foil: every part keeps clear of every other, only
// the Signature look signs and it draws a picture of its own, a Radiant face keeps its gold frame and its art's
// Radiant paint under every look, and under Reduce motion nothing moves.
//
// Each face is the viewer's own copy (`worn`, below an `OwnCopy`) shown in a look by `FullArtShown`, as the Collection's preview
// shows one, so the looks stand side by side with no pick written. No `FullArtOwned` is mounted, so every cosmetic
// counts as owned where it is offered, as in practice.

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { resetCosmeticsForTests, writeCardPick } from "../../../apps/web/src/cards/cosmetics.ts";
import { FullArtShown, OwnCopy } from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID, VERSION_FULL_ART_ID } from "../../../apps/web/src/cards/versions.ts";
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

function Faces({ width }: { width: number }) {
  return (
    <OwnCopy own>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8, background: "#1b1f2b" }}>
        {CARDS.flatMap(({ id }) =>
          LOOKS.flatMap((look) =>
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
});
