// R3388000: every Classic card offered Alt art, its normal face beside its Alt art face, base and Radiant, in a real
// browser, a page of eight cards at a time. Each Alt art window paints its alt recipe's picture (apps/web/src/cards/art/
// classicAltRecipes1.ts to classicAltRecipes3.ts) and changes at least ALT_PICTURE_CHANGE_MIN of the normal window's
// sampled points, measured on card-versions.cy.tsx's grid with its threshold for a point that differs, and the
// screenshots are what the pass was tuned against by eye. apps/web's vitest suite proves each recipe's structure, every
// Alt art level and the Radiant marks (cards/art/classicAltScenes.test.tsx).
//
// Each face is the viewer's own copy (`worn`, below an `OwnCopy`) shown in a look by `FullArtShown`, as card-versions.cy.tsx
// shows them: the normal frame itself (`FullArtAllowed` false, so a Legendary's or a Mythic's default look does not stand
// in for it) and Alt art's first level.
//
//   E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/classic-alt-scenes.cy.tsx

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { ALT_PICTURE_CHANGE_MIN, ALT_SCENE_RECIPES } from "../../../apps/web/src/cards/art/sceneRecipes.ts";
import { FullArtAllowed, FullArtShown, OwnCopy } from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { VERSION_ALT_ART_ID } from "../../../apps/web/src/cards/versions.ts";

type CardDef = (typeof CATALOG)[string];
type Point = { x: number; y: number };
type Shot = { width: number; height: number; data: Uint8ClampedArray; scale: number };

/** Every Classic card offered Alt art, in id order (R3396000's Classic+ cards are classicplus-alt-scenes.cy.tsx's). */
const IDS: readonly string[] = Object.keys(ALT_SCENE_RECIPES)
  .filter((id) => id.startsWith("classic-"))
  .sort();
/** The cards a page shows: each four faces, all inside the viewport. */
const CARDS_PER_PAGE = 8;
const PAGES: readonly (readonly string[])[] = Array.from({ length: Math.ceil(IDS.length / CARDS_PER_PAGE) }, (_, n) => IDS.slice(n * CARDS_PER_PAGE, (n + 1) * CARDS_PER_PAGE));
/** The screen, and a face small enough that a page's 32 faces fit inside it: four rows of ten. */
const VIEWPORT = { width: 1280, height: 720 } as const;
const FACE_WIDTH = 110;
/** The space round a page and between its faces. */
const GAP_PX = 8;
/** Enough room for the faces to mount and the art to draw. */
const SETTLE_TIMEOUT_MS = 20_000;

/** The two looks each card shows: the normal frame itself, and Alt art's first level. */
const LOOKS = [
  { name: "normal", pick: false, plain: true },
  { name: "alt", pick: VERSION_ALT_ART_ID, plain: false },
] as const;

/** The grid the art windows are sampled on, in CSS pixels, and the inset from their edges (card-versions.cy.tsx's). */
const SAMPLE_STEP_PX = 6;
const SAMPLE_INSET_PX = 3;
/** A channel must differ this much for a point to count as different: more than compression noise (card-versions.cy.tsx's). */
const PIXEL_DELTA_MIN = 4;

function heightFor(width: number): number {
  return Math.round(width / FACE_ASPECT);
}

function Faces({ ids }: { ids: readonly string[] }) {
  return (
    <OwnCopy own>
      <div style={{ display: "flex", flexWrap: "wrap", gap: GAP_PX, padding: GAP_PX, background: "#1b1f2b" }}>
        {ids.flatMap((id) =>
          LOOKS.flatMap((look) =>
            [false, true].map((radiant) => (
              <div key={`${id}-${look.name}-${String(radiant)}`} data-face={`${id}-${look.name}-${radiant ? "radiant" : "base"}`} style={{ width: FACE_WIDTH, height: heightFor(FACE_WIDTH), flex: "none" }}>
                <FullArtAllowed allowed={!look.plain}>
                  <FullArtShown defId={id} pick={look.pick} augments={[]}>
                    <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant })} layout="full" worn />
                  </FullArtShown>
                </FullArtAllowed>
              </div>
            )),
          ),
        )}
      </div>
    </OwnCopy>
  );
}

/** The art window of the face showing `id` in `look`, on its Radiant face or its base one. */
function windowOf(doc: Document, id: string, look: string, radiant: boolean): DOMRect {
  const art = doc.querySelector<HTMLElement>(`[data-face="${id}-${look}-${radiant ? "radiant" : "base"}"] .cf-art-frame > .cf-art`);
  if (art === null) throw new Error(`no art window for ${id} ${look}`);
  return art.getBoundingClientRect();
}

/** The viewport as Chrome painted it, decoded to RGBA, with the device pixels per CSS pixel (card-versions.cy.tsx's). */
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

/** The offsets of the sampling grid over the whole of an art window `box` (card-versions.cy.tsx's). */
function windowOffsets(box: DOMRect): Point[] {
  const points: Point[] = [];
  for (let y = SAMPLE_INSET_PX; y < box.height - SAMPLE_INSET_PX; y += SAMPLE_STEP_PX) {
    for (let x = SAMPLE_INSET_PX; x < box.width - SAMPLE_INSET_PX; x += SAMPLE_STEP_PX) points.push({ x, y });
  }
  return points;
}

/** The share of `offsets` at which the windows `a` and `b` of `shot` differ by `delta` or more in a channel (card-versions.cy.tsx's). */
function differingShare(shot: Shot, a: DOMRect, b: DOMRect, offsets: readonly Point[], delta: number = PIXEL_DELTA_MIN): number {
  const differing = offsets.filter((offset) => {
    const one = pixelAt(shot, { x: a.left + offset.x, y: a.top + offset.y });
    const two = pixelAt(shot, { x: b.left + offset.x, y: b.top + offset.y });
    return one.some((channel, i) => Math.abs(channel - (two[i] ?? -1)) >= delta);
  });
  return differing.length / offsets.length;
}

describe("R3388000 every Classic Alt art restaged", () => {
  beforeEach(() => {
    cy.viewport(VIEWPORT.width, VIEWPORT.height);
    // Animated foil off holds every picture still, so one screenshot compares the pictures alone.
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, animatedFoil: false });
  });

  after(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  it("R3388000 the pass restages the 50 Classic cards offered Alt art, eight a page", () => {
    expect(IDS).to.have.length(50);
    expect(PAGES).to.have.length(7);
  });

  PAGES.forEach((page, n) => {
    it(`R3388000 page ${n + 1}: each Alt art window changes at least ALT_PICTURE_CHANGE_MIN of its normal window, base and Radiant`, () => {
      cy.mount(<Faces ids={page} />);
      cy.get("[data-face] .cf-art-frame > .cf-art", { timeout: SETTLE_TIMEOUT_MS }).should("have.length", page.length * 4);
      cy.get("[data-face] .cf-art[data-art-pending]", { timeout: SETTLE_TIMEOUT_MS }).should("not.exist");
      cy.get('[data-face] .cf-art-frame > .cf-art[data-art-picture="alt"]').should("have.length", page.length * 2);
      cy.document().its("fonts.status").should("eq", "loaded");
      const taken: { windows?: Map<string, { normal: DOMRect; alt: DOMRect }> } = {};
      cy.document().then((doc) => {
        const windows = new Map<string, { normal: DOMRect; alt: DOMRect }>();
        for (const id of page) {
          for (const radiant of [false, true]) windows.set(`${id} ${radiant ? "Radiant" : "base"}`, { normal: windowOf(doc, id, "normal", radiant), alt: windowOf(doc, id, "alt", radiant) });
        }
        taken.windows = windows;
      });
      screenshot(`classic-alt-scenes-${n + 1}`).then((shot) => {
        const shares: Record<string, number> = {};
        for (const [where, { normal, alt }] of taken.windows ?? new Map<string, { normal: DOMRect; alt: DOMRect }>()) {
          // Every window sampled lies inside what the screenshot captured, and the two match.
          for (const box of [normal, alt]) expect(Math.ceil(box.bottom * shot.scale), `${where}: inside the shot`).to.be.at.most(shot.height);
          expect(Math.round(normal.width), `${where}: the windows match`).to.eq(Math.round(alt.width));
          const offsets = windowOffsets(normal);
          expect(offsets.length, `${where}: sampled points`).to.be.greaterThan(20);
          const share = differingShare(shot, normal, alt, offsets);
          shares[where] = Math.round(share * 100) / 100;
          expect(share, `${where}: the Alt art window differs`).to.be.at.least(ALT_PICTURE_CHANGE_MIN);
        }
        expect(Object.keys(shares).length, "every card and face sampled").to.eq(page.length * 2);
        cy.task("layout:report", { spec: `classic-alt-scenes-${n + 1}`, shares });
      });
    });
  });
});
