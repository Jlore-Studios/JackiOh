// R3391000: every Stinkyscape card #3391's pass draws as a scene of its own (apps/web/src/cards/art/stinkyscapeRecipes.ts),
// in a real browser at desktop size and at a phone's 390x844: each Epic, Legendary and Mythic at hand size, base
// and Radiant, a screenshot per rarity, and a contact sheet of the Commons and Rares. apps/web's vitest suite proves
// each scene's parts, floors and colours (cards/art/stinkyscapeScenes.test.ts); here every face's window paints
// (an Epic or rarer face in its depth planes, R3344000, whose farthest is the window's own art), each Radiant
// picture differs from its base one pixel by pixel, no two base pictures are the same, and the screenshots are what
// the pass was tuned against by eye, as scene-proofs.cy.tsx's were.

import { CATALOG } from "@jackioh/cards";
import type { Rarity } from "@jackioh/shared";
import { STINKYSCAPE_RECIPES } from "../../../apps/web/src/cards/art/stinkyscapeRecipes.ts";
import { CardFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { proceduralCardArtUri } from "../../../apps/web/src/patches/cardArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";

/** The pass's cards, by id. */
const PASS: readonly string[] = Object.keys(STINKYSCAPE_RECIPES).sort();
/** The rarities shown at hand size, one screenshot each, and those on the contact sheet. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
const SHEET: readonly Rarity[] = ["Common", "Rare"];

type Viewport = {
  readonly label: string;
  readonly width: number;
  readonly height: number;
  /** A hand-size face's width, 5:7. */
  readonly face: number;
  /** A contact sheet face's box. */
  readonly thumb: { readonly width: number; readonly height: number };
};

const VIEWPORTS: readonly Viewport[] = [
  { label: "desktop", width: 1280, height: 720, face: 220, thumb: { width: 100, height: 140 } },
  { label: "phone", width: 390, height: 844, face: 150, thumb: { width: 72, height: 100 } },
];

/** A face's height for its width, 5:7. */
const FACE_RATIO = 7 / 5;
/** How long the faces may take to settle before their art must be painted. */
const SETTLE_TIMEOUT_MS = 30_000;
/** Each picture is sampled down to this square before its pixels are compared (scene-proofs.cy.tsx's). */
const SAMPLE_PX = 48;
/** Two channel values within this distance count as the same. */
const CHANNEL_SLACK = 24;
/** At least this share of a Radiant picture's pixels differs from its base face's. */
const MIN_CHANGED_SHARE = 0.25;

function rarityOf(id: string): Rarity | undefined {
  return CATALOG[id]?.rarity;
}

function idsOf(rarities: readonly Rarity[]): string[] {
  return PASS.filter((id) => {
    const rarity = rarityOf(id);
    return rarity !== undefined && rarities.includes(rarity);
  });
}

/** The faces of `ids`, each base then Radiant, wrapped, each in a `box`. */
function Sheet({ ids, box }: { ids: readonly string[]; box: { width: number; height: number } }) {
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8, background: "#1b1f2b" }}>
      {ids.flatMap((id) =>
        [false, true].map((radiant) => {
          const name = `${id}-${radiant ? "radiant" : "base"}`;
          return (
            <div key={name} data-face={name} style={{ ...box, flex: "none", position: "relative" }}>
              <CardFace face={faceModel({ defId: id, def: CATALOG[id], radiant })} layout="full" />
            </div>
          );
        }),
      )}
    </div>
  );
}

/** A face's window art: the picture, or under depth planes the farthest of them (CardFace's own `.cf-art`). */
const WINDOW_ART = ".cf-art-frame > .cf-art";

/** The painted art URI of the face with this name, or "" when it has none yet. */
function pictureOf(doc: Document, name: string): string {
  const art = doc.querySelector(`[data-face="${name}"] ${WINDOW_ART}`);
  if (art === null || !(art instanceof HTMLElement)) return "";
  return /^url\("(data:image\/svg\+xml,[^"]*)"\)$/.exec(getComputedStyle(art).backgroundImage)?.[1] ?? "";
}

/** The card's whole picture on a face, as its painter draws it (the window's, unsplit). */
function wholePicture(id: string, radiant: boolean): string {
  const card = CATALOG[id];
  if (card === undefined) throw new Error(`the catalog has no ${id}`);
  return proceduralCardArtUri(card, radiant);
}

/** A picture's pixels, sampled down to SAMPLE_PX square. */
function pixelsOf(uri: string): Promise<Uint8ClampedArray> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = (): void => {
      const canvas = document.createElement("canvas");
      canvas.width = SAMPLE_PX;
      canvas.height = SAMPLE_PX;
      const context = canvas.getContext("2d");
      if (context === null) {
        reject(new Error("no 2d context"));
        return;
      }
      context.drawImage(image, 0, 0, SAMPLE_PX, SAMPLE_PX);
      resolve(context.getImageData(0, 0, SAMPLE_PX, SAMPLE_PX).data);
    };
    image.onerror = (): void => {
      reject(new Error("the art did not load"));
    };
    image.src = uri;
  });
}

/** The share of pixels whose red, green or blue differ by more than CHANNEL_SLACK. */
function changedShare(a: Uint8ClampedArray, b: Uint8ClampedArray): number {
  let changed = 0;
  const pixels = Math.floor(a.length / 4);
  for (let i = 0; i < pixels; i += 1) {
    const at = i * 4;
    const far = [0, 1, 2].some((channel) => Math.abs((a[at + channel] ?? 0) - (b[at + channel] ?? 0)) > CHANNEL_SLACK);
    if (far) changed += 1;
  }
  return pixels === 0 ? 0 : changed / pixels;
}

/** Mounts `ids`' faces and waits until each has painted its picture, its two faces apart. */
function settle(ids: readonly string[], box: { width: number; height: number }, where: string): void {
  cy.mount(<Sheet ids={ids} box={box} />);
  cy.get(`[data-face] ${WINDOW_ART}`, { timeout: SETTLE_TIMEOUT_MS }).should("have.length", ids.length * 2);
  cy.get("[data-face] .cf-art[data-art-pending]", { timeout: SETTLE_TIMEOUT_MS }).should("not.exist");
  cy.document().its("fonts.status").should("eq", "loaded");
  cy.document().should((doc) => {
    for (const id of ids) {
      for (const face of ["base", "radiant"]) {
        const name = `${id}-${face}`;
        const rect = doc.querySelector(`[data-face="${name}"] ${WINDOW_ART}`)?.getBoundingClientRect();
        expect(rect?.width ?? 0, `${name} at ${where} has a window`).to.be.greaterThan(0);
        expect(pictureOf(doc, name), `${name} at ${where} paints`).not.to.equal("");
      }
      expect(pictureOf(doc, `${id}-radiant`), `${id}'s two faces at ${where}`).not.to.equal(pictureOf(doc, `${id}-base`));
    }
  });
}

for (const viewport of VIEWPORTS) {
  const size = `${viewport.label} size (${viewport.width}x${viewport.height})`;
  const hand = { width: viewport.face, height: Math.round(viewport.face * FACE_RATIO) };

  describe(`R3391000 Stinkyscape's scenes at ${size}`, () => {
    beforeEach(() => {
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, animatedFoil: false });
      cy.viewport(viewport.width, viewport.height);
    });

    after(() => {
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    });

    it("R3391000 every Epic, Legendary and Mythic of the pass at hand size, base and Radiant", () => {
      const seen = new Set<string>();
      for (const rarity of DEEP) {
        const ids = idsOf([rarity]);
        expect(ids.length, rarity).to.be.greaterThan(0);
        settle(ids, hand, `${viewport.label} hand size`);
        cy.screenshot(`stinkyscape-scenes-${viewport.label}-${rarity.toLowerCase()}`, { capture: "fullPage", overwrite: true });
        cy.wrap(null)
          .then(() => {
            for (const id of ids) {
              const base = wholePicture(id, false);
              expect(seen.has(base), `${id}'s base picture is its own`).to.equal(false);
              seen.add(base);
            }
            return Promise.all(ids.map((id) => Promise.all([pixelsOf(wholePicture(id, false)), pixelsOf(wholePicture(id, true))]).then(([a, b]) => ({ id, share: changedShare(a, b) }))));
          })
          .then((shares) => {
            for (const { id, share } of shares) expect(share, `${id}'s Radiant picture differs from its base one`).to.be.at.least(MIN_CHANGED_SHARE);
          });
      }
    });

    it("R3391000 a contact sheet of the pass's Commons and Rares, base and Radiant", () => {
      const ids = idsOf(SHEET);
      expect(ids.length).to.be.greaterThan(0);
      settle(ids, viewport.thumb, `${viewport.label} contact sheet`);
      cy.screenshot(`stinkyscape-scenes-${viewport.label}-commons-and-rares`, { capture: "fullPage", overwrite: true });
    });
  });
}
