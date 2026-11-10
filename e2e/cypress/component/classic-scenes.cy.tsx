// R3352000: Classic's scenes (apps/web/src/cards/art/sceneRecipesClassic1.ts to sceneRecipesClassic3.ts) in a real
// browser, at a desktop size (1280x720) and a phone's (390x844). apps/web's vitest suite proves each scene's recipe,
// floor, ceiling, colours and geometry (cards/art/classicScenes.test.ts); here every Epic, Legendary and Mythic Classic
// scene is drawn at hand size and every other Classic scene on a contact sheet at board size, base and Radiant, each
// picture paints, each Radiant face's differs from its base face's, and the screenshots are what the scenes were tuned
// against by eye. The Glitch's window draws no picture (glitch.css), so its scene is drawn on the contact sheet as its
// picture alone. Each set of faces is shown a page at a time, each page fitting the viewport (as core-scenes.cy.tsx's
// are), and the faces draw flat (Reduce motion), as scene-proofs.cy.tsx's do.
// Run: E2E_COMPONENT_PORT=5291 pnpm --dir e2e test:component --spec cypress/component/classic-scenes.cy.tsx

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import { proceduralCardArtUri } from "../../../apps/web/src/patches/cardArt.ts";
import { CardFace, FACE_ASPECT, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

/** Corpse Plantation, #3331's proof card, whose scene scene-proofs.cy.tsx shows. */
const PROOF = "classic-074";
/** Every Classic face this pass draws a scene for: all but Corpse Plantation, and the Glitch apart (its window draws none). */
const CLASSIC = Object.values(CATALOG)
  .filter((card) => card.set === "Classic" && card.id !== PROOF && card.id !== GLITCH_DEF_ID)
  .map((card) => card.id);
/** The Classic scenes with a foreground, a middle and a background: every Epic, Legendary and Mythic. */
const DEEP = CLASSIC.filter((id) => ["Epic", "Legendary", "Mythic"].includes(CATALOG[id]?.rarity ?? ""));
/** Every other Classic scene. */
const REST = CLASSIC.filter((id) => !DEEP.includes(id));
/** The Glitch's picture on the contact sheet, at a board minion's width. */
const GLITCH_PX = 81;

/** The two viewports a player holds the game at, and the width a hand card is drawn at in each (card-depth.cy.tsx's). */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720, face: 220 },
  { label: "phone", width: 390, height: 844, face: 150 },
] as const;
type Viewport = (typeof VIEWPORTS)[number];

/** A board minion's box (card-cosmetics.cy.tsx's). */
const MINION_BOX = { width: 81, height: 113 } as const;
/** A face-up backrow card's box on the board. */
const BACKROW_BOX = { width: 71, height: 100 } as const;
/** The space round a page and between its faces. */
const GAP_PX = 8;

/** How long the faces may take to settle before their art must be painted. */
const SETTLE_TIMEOUT_MS = 30_000;
/** Each picture is sampled down to this square before its pixels are compared. */
const SAMPLE_PX = 48;
/** Two channel values within this distance count as the same. */
const CHANNEL_SLACK = 24;
/** At least this share of a Radiant picture's pixels differs from its base face's. */
const MIN_CHANGED_SHARE = 0.25;

const FACES = ["base", "radiant"] as const;

function nameOf(id: string, radiant: boolean): string {
  return `${id}-${radiant ? "radiant" : "base"}`;
}

/** The ids of each page: as many base and Radiant pairs as fit the viewport, each face `width` by `height`. */
function pagesOf(ids: readonly string[], viewport: Viewport, width: number, height: number): string[][] {
  const pairsPerRow = Math.max(1, Math.floor((viewport.width - GAP_PX) / (2 * (width + GAP_PX))));
  const rows = Math.max(1, Math.floor((viewport.height - GAP_PX) / (height + GAP_PX)));
  const perPage = pairsPerRow * rows;
  const pages: string[][] = [];
  for (let at = 0; at < ids.length; at += perPage) pages.push(ids.slice(at, at + perPage));
  return pages;
}

/** A page of Epic, Legendary and Mythic scenes at hand size, each card's base face beside its Radiant one. */
function Hand({ ids, width }: { ids: readonly string[]; width: number }) {
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: GAP_PX, padding: GAP_PX / 2, background: "#1b1f2b" }}>
      {ids.flatMap((id) =>
        [false, true].map((radiant) => (
          <div key={nameOf(id, radiant)} data-face={nameOf(id, radiant)} style={{ width, height: Math.round(width / FACE_ASPECT), flex: "none", position: "relative" }}>
            <CardFace face={faceModel({ defId: id, def: CATALOG[id], radiant })} layout="full" />
          </div>
        )),
      )}
    </div>
  );
}

/** A page of the other Classic scenes at board size, as scene-proofs.cy.tsx draws its board row, and, on the first page, the Glitch's picture. */
function Sheet({ ids, glitch }: { ids: readonly string[]; glitch: boolean }) {
  const def = CATALOG[GLITCH_DEF_ID];
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: GAP_PX, padding: GAP_PX / 2, background: "#1b1f2b" }}>
      {ids.flatMap((id) =>
        [false, true].map((radiant) => {
          const face = faceModel({ defId: id, def: CATALOG[id], radiant });
          const minion = CATALOG[id]?.type === "Unit";
          return (
            <div key={nameOf(id, radiant)} data-face={nameOf(id, radiant)} style={{ ...(minion ? MINION_BOX : BACKROW_BOX), flex: "none", position: "relative" }}>
              {minion ? <MinionFace face={face} unit={unit("p1", { defId: id, radiant })} /> : <CardFace face={face} layout="compact" />}
            </div>
          );
        }),
      )}
      {glitch && def !== undefined
        ? [false, true].map((radiant) => (
            <img key={nameOf(GLITCH_DEF_ID, radiant)} data-picture={nameOf(GLITCH_DEF_ID, radiant)} alt="" width={GLITCH_PX} height={GLITCH_PX} src={proceduralCardArtUri(def, radiant)} />
          ))
        : null}
    </div>
  );
}

/** The painted art URI of the face with this name, or "" when it has none yet. */
function pictureOf(doc: Document, name: string): string {
  const art = doc.querySelector(`[data-face="${name}"] .cf-art`);
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

/** Waits for every face of `ids` to have a window that paints inside the viewport, each Radiant picture another than its base one. */
function settle(ids: readonly string[], where: string, viewport: Viewport): void {
  cy.get("[data-face] .cf-art", { timeout: SETTLE_TIMEOUT_MS }).should("have.length", ids.length * FACES.length);
  cy.get("[data-face] .cf-art[data-art-pending]", { timeout: SETTLE_TIMEOUT_MS }).should("not.exist");
  cy.document().its("fonts.status").should("eq", "loaded");
  cy.document().should((doc) => {
    for (const id of ids) {
      for (const face of FACES) {
        const name = `${id}-${face}`;
        const box = doc.querySelector(`[data-face="${name}"] .cf-art`)?.getBoundingClientRect();
        expect(box?.width ?? 0, `${name} at ${where} has a window`).to.be.greaterThan(0);
        expect(box?.bottom ?? Infinity, `${name} at ${where} fits the ${viewport.label} screen`).to.be.at.most(viewport.height);
        expect(pictureOf(doc, name), `${name} at ${where} paints`).not.to.equal("");
      }
      expect(pictureOf(doc, nameOf(id, true)), `${id}'s two faces at ${where}`).not.to.equal(pictureOf(doc, nameOf(id, false)));
    }
  });
}

/** Each of `ids`' Radiant pictures differs from its base one pixel by pixel. */
function radiantDiffers(ids: readonly string[]): void {
  cy.document()
    .then((doc) =>
      Promise.all(
        ids.map((id) => Promise.all([pixelsOf(pictureOf(doc, nameOf(id, false))), pixelsOf(pictureOf(doc, nameOf(id, true)))]).then(([a, b]) => ({ id, share: changedShare(a, b) }))),
      ),
    )
    .then((shares) => {
      for (const { id, share } of shares) expect(share, `${id}'s Radiant picture differs from its base one`).to.be.at.least(MIN_CHANGED_SHARE);
    });
}

/** The Glitch's two pictures load and differ pixel by pixel. */
function glitchPaints(): void {
  cy.get(`[data-picture="${nameOf(GLITCH_DEF_ID, false)}"]`).should(($img) => expect(($img[0] as HTMLImageElement).naturalWidth).to.be.greaterThan(0));
  cy.get(`[data-picture="${nameOf(GLITCH_DEF_ID, true)}"]`).should(($img) => expect(($img[0] as HTMLImageElement).naturalWidth).to.be.greaterThan(0));
  cy.document()
    .then((doc) => {
      const src = (radiant: boolean): string => doc.querySelector<HTMLImageElement>(`[data-picture="${nameOf(GLITCH_DEF_ID, radiant)}"]`)?.src ?? "";
      expect(src(true), "the Glitch's two pictures").not.to.equal(src(false));
      return Promise.all([pixelsOf(src(false)), pixelsOf(src(true))]);
    })
    .then(([a, b]) => {
      expect(changedShare(a, b), "the Glitch's Radiant picture differs from its base one").to.be.at.least(MIN_CHANGED_SHARE);
    });
}

for (const viewport of VIEWPORTS) {
  describe(`R3352000 Classic's scenes at ${viewport.label} size (${String(viewport.width)}x${String(viewport.height)})`, () => {
    beforeEach(() => {
      __resetSettingsForTests();
      writeSettings({ reduceMotion: true });
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, animatedFoil: false, fullArt: false });
      cy.viewport(viewport.width, viewport.height);
    });

    after(() => {
      __resetSettingsForTests();
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    });

    it("R3352000 every Epic, Legendary and Mythic scene at hand size, base and Radiant", () => {
      expect(DEEP).to.have.length(28);
      const height = Math.round(viewport.face / FACE_ASPECT);
      pagesOf(DEEP, viewport, viewport.face, height).forEach((page, n) => {
        cy.mount(<Hand ids={page} width={viewport.face} />);
        settle(page, `${viewport.label} hand size`, viewport);
        radiantDiffers(page);
        cy.screenshot(`classic-scenes-${viewport.label}-deep-${String(n + 1)}`, { capture: "viewport", overwrite: true });
      });
    });

    it("R3352000 a contact sheet of every other Classic scene and the Glitch's picture, base and Radiant", () => {
      expect(REST).to.have.length(61);
      pagesOf([...REST, GLITCH_DEF_ID], viewport, MINION_BOX.width, MINION_BOX.height).forEach((all, n) => {
        const glitch = all.includes(GLITCH_DEF_ID);
        const page = all.filter((id) => id !== GLITCH_DEF_ID);
        cy.mount(<Sheet ids={page} glitch={glitch} />);
        settle(page, `${viewport.label} board size`, viewport);
        radiantDiffers(page);
        if (glitch) glitchPaints();
        cy.screenshot(`classic-scenes-${viewport.label}-sheet-${String(n + 1)}`, { capture: "viewport", overwrite: true });
      });
    });
  });
}
