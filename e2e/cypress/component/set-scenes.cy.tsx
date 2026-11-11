// R3393000, R3393001: every Meditative+ and Training Grounds entry, tokens and Legacy cards included, drawn from its
// scene recipe (apps/web/src/cards/art/sceneRecipesMeditativePlus.ts and …2.ts, sceneRecipesTrainingGrounds.ts and
// …2.ts, sceneRecipesLegacy.ts), base and Radiant, in a real browser, at desktop size and at 390x844. Every Epic, Legendary
// and Mythic entry is shown at hand size, the rest as a contact sheet at a smaller size, a page at a time, each page
// fitting the viewport. apps/web's vitest suite proves each scene's shapes, floors, geometry and colours
// (cards/art/setScenes.test.ts, artRichness.test.ts); here every picture paints, each Radiant face's picture differs
// from its base face's pixel by pixel, and the screenshots are what the pass was tuned against by eye.
//
//   E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/set-scenes.cy.tsx

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";
import { CardFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";

/** Every Meditative+ and Training Grounds entry, in id order. */
const ENTRIES: readonly CardDef[] = Object.values(CATALOG)
  .filter((card) => card.set === "Meditative+" || card.set === "Training Grounds")
  .sort((a, b) => (a.id < b.id ? -1 : 1));
/** The Epic, Legendary and Mythic entries, each shown at hand size. */
const DEEP: readonly string[] = ENTRIES.filter((card) => ["Epic", "Legendary", "Mythic"].includes(card.rarity)).map((card) => card.id);
/** Every other entry, shown on the contact sheet. */
const REST: readonly string[] = ENTRIES.filter((card) => !DEEP.includes(card.id)).map((card) => card.id);

type Box = { readonly width: number; readonly height: number };
/** A screen size, its hand-sized face and its contact sheet's face, each 5:7. */
type Screen = { readonly label: string; readonly width: number; readonly height: number; readonly hand: Box; readonly sheet: Box };

const DESKTOP: Screen = { label: "desktop", width: 1280, height: 720, hand: { width: 150, height: 210 }, sheet: { width: 75, height: 105 } };
const PHONE: Screen = { label: "phone", width: 390, height: 844, hand: { width: 110, height: 154 }, sheet: { width: 60, height: 84 } };

/** The space round a page and between its faces. */
const GAP_PX = 8;
/** How long the faces may take to settle before their art must be painted. */
const SETTLE_TIMEOUT_MS = 20_000;
/** Each picture is sampled down to this square before its pixels are compared. */
const SAMPLE_PX = 48;
/** Two channel values within this distance count as the same. */
const CHANNEL_SLACK = 24;
/** At least this share of a Radiant picture's pixels differs from its base face's (scene-proofs.cy.tsx's). */
const MIN_CHANGED_SHARE = 0.25;

/** The ids of each page: as many base and Radiant pairs as fit the screen at `box`. */
function pagesOf(ids: readonly string[], screen: Screen, box: Box): string[][] {
  const pairsPerRow = Math.max(1, Math.floor((screen.width - GAP_PX) / (2 * (box.width + GAP_PX))));
  const rows = Math.max(1, Math.floor((screen.height - GAP_PX) / (box.height + GAP_PX)));
  const perPage = pairsPerRow * rows;
  const pages: string[][] = [];
  for (let at = 0; at < ids.length; at += perPage) pages.push(ids.slice(at, at + perPage));
  return pages;
}

/** One page: each id's base face beside its Radiant face. */
function Page({ ids, box }: { ids: readonly string[]; box: Box }) {
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: GAP_PX, padding: GAP_PX / 2, background: "#1b1f2b" }}>
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

/** The painted art URI of the face with this name, or "" when it has none yet: its window's, not one of the depth planes #3383 stacks over an Epic and up. */
function pictureOf(doc: Document, name: string): string {
  const art = doc.querySelector(`[data-face="${name}"] .cf-art-frame > .cf-art`);
  if (art === null || !(art instanceof HTMLElement)) return "";
  return /^url\("(data:image\/svg\+xml,[^"]*)"\)$/.exec(getComputedStyle(art).backgroundImage)?.[1] ?? "";
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

/** Mounts each page of `ids` at `box`, checks every face paints and every Radiant picture differs, and screenshots it. */
function showPages(screen: Screen, ids: readonly string[], box: Box, kind: "deep" | "sheet"): void {
  cy.viewport(screen.width, screen.height);
  pagesOf(ids, screen, box).forEach((page, n) => {
    cy.mount(<Page ids={page} box={box} />);
    cy.get("[data-face] .cf-art-frame > .cf-art", { timeout: SETTLE_TIMEOUT_MS }).should("have.length", page.length * 2);
    cy.get("[data-face] .cf-art[data-art-pending]", { timeout: SETTLE_TIMEOUT_MS }).should("not.exist");
    cy.document().its("fonts.status").should("eq", "loaded");
    cy.document().should((doc) => {
      for (const id of page) {
        for (const face of ["base", "radiant"]) {
          const name = `${id}-${face}`;
          const shown = doc.querySelector(`[data-face="${name}"] .cf-art-frame > .cf-art`)?.getBoundingClientRect();
          expect(shown?.width ?? 0, `${name} has a window`).to.be.greaterThan(0);
          expect(shown?.bottom ?? Infinity, `${name} fits the ${screen.label} screen`).to.be.at.most(screen.height);
          expect(pictureOf(doc, name), `${name} paints`).not.to.equal("");
        }
        expect(pictureOf(doc, `${id}-radiant`), `${id}'s two faces`).not.to.equal(pictureOf(doc, `${id}-base`));
      }
    });
    cy.document()
      .then((doc) =>
        Promise.all(
          page.map((id) => Promise.all([pixelsOf(pictureOf(doc, `${id}-base`)), pixelsOf(pictureOf(doc, `${id}-radiant`))]).then(([a, b]) => ({ id, share: changedShare(a, b) }))),
        ),
      )
      .then((shares) => {
        // Every face of the page that differs too little, named at once.
        const flat = shares.filter(({ share }) => share < MIN_CHANGED_SHARE).map(({ id, share }) => `${id}: ${share.toFixed(3)}`);
        expect(flat, "the Radiant pictures that differ from their base ones by less than MIN_CHANGED_SHARE").to.deep.equal([]);
      });
    cy.screenshot(`set-scenes-${screen.label}-${kind}-${n + 1}`, { capture: "viewport", overwrite: true });
  });
}

describe("R3393000 every Meditative+ and Training Grounds entry drawn as a scene", () => {
  beforeEach(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, animatedFoil: false });
  });

  after(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  it("R3393000 R3393001 the two sets hold their 94 entries, 27 of them Epic or rarer", () => {
    expect(ENTRIES).to.have.length(94);
    expect(DEEP).to.have.length(27);
    expect(REST).to.have.length(67);
  });

  for (const screen of [DESKTOP, PHONE]) {
    it(`R3393000 every Epic, Legendary and Mythic entry at hand size, base and Radiant, on the ${screen.label} screen`, () => {
      showPages(screen, DEEP, screen.hand, "deep");
    });

    it(`R3393000 a contact sheet of every other entry, base and Radiant, on the ${screen.label} screen`, () => {
      showPages(screen, REST, screen.sheet, "sheet");
    });
  }
});
