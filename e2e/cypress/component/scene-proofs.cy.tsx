// R3331000: the five proof cards drawn from scene recipes (apps/web/src/cards/art/sceneRecipes.ts), at hand
// size and at board size, base and Radiant, in a real browser. apps/web's vitest suite proves each scene's
// shapes, floors and colours (cards/art/scene.test.ts, artRichness.test.ts); here every proof's picture paints,
// each Radiant face's differs from its base face's pixel by pixel, and the two screenshots are the bar each
// set's picture pass is tuned against by eye.

import { CATALOG } from "@jackioh/cards";
import { CardFace, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

/** The proof cards, the order of each row. */
const PROOFS = ["classic-074", "core-002", "core-098", "core-100", "stinkyscape-015"] as const;

/** A hand-sized full face, 5:7. */
const HAND_BOX = { width: 150, height: 210 } as const;
/** A board minion's box (card-cosmetics.cy.tsx's). */
const MINION_BOX = { width: 81, height: 113 } as const;
/** A face-up backrow card's box on the board. */
const BACKROW_BOX = { width: 71, height: 100 } as const;

/** How long the faces may take to settle before their art must be painted. */
const SETTLE_TIMEOUT_MS = 20_000;
/** Each picture is sampled down to this square before its pixels are compared. */
const SAMPLE_PX = 48;
/** Two channel values within this distance count as the same. */
const CHANNEL_SLACK = 24;
/** At least this share of a Radiant picture's pixels differs from its base face's. */
const MIN_CHANGED_SHARE = 0.25;

type Size = "hand" | "board";

function boxOf(id: string, size: Size): { width: number; height: number } {
  if (size === "hand") return HAND_BOX;
  return CATALOG[id]?.type === "Unit" ? MINION_BOX : BACKROW_BOX;
}

/** One row of the five proofs at `size`, base faces first, then a row of their Radiant faces. */
function Rows({ size }: { size: Size }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12, padding: 12, background: "#1b1f2b" }}>
      {[false, true].map((radiant) => (
        <div key={String(radiant)} style={{ display: "flex", gap: 12 }}>
          {PROOFS.map((id) => {
            const face = faceModel({ defId: id, def: CATALOG[id], radiant });
            const name = `${id}-${radiant ? "radiant" : "base"}`;
            return (
              <div key={name} data-face={name} style={{ ...boxOf(id, size), flex: "none", position: "relative" }}>
                {size === "hand" ? (
                  <CardFace face={face} layout="full" />
                ) : CATALOG[id]?.type === "Unit" ? (
                  <MinionFace face={face} unit={unit("p1", { defId: id, radiant })} />
                ) : (
                  <CardFace face={face} layout="compact" />
                )}
              </div>
            );
          })}
        </div>
      ))}
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

function settle(size: Size): void {
  cy.mount(<Rows size={size} />);
  cy.get("[data-face] .cf-art", { timeout: SETTLE_TIMEOUT_MS }).should("have.length", PROOFS.length * 2);
  cy.get("[data-face] .cf-art[data-art-pending]", { timeout: SETTLE_TIMEOUT_MS }).should("not.exist");
  cy.document().its("fonts.status").should("eq", "loaded");
  cy.document().should((doc) => {
    for (const id of PROOFS) {
      for (const face of ["base", "radiant"]) {
        const name = `${id}-${face}`;
        const box = doc.querySelector(`[data-face="${name}"] .cf-art`)?.getBoundingClientRect();
        expect(box?.width ?? 0, `${name} at ${size} size has a window`).to.be.greaterThan(0);
        expect(pictureOf(doc, name), `${name} at ${size} size paints`).not.to.equal("");
      }
      expect(pictureOf(doc, `${id}-radiant`), `${id}'s two faces`).not.to.equal(pictureOf(doc, `${id}-base`));
    }
  });
}

describe("R3331000 the proof cards' scenes", () => {
  beforeEach(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, animatedFoil: false });
    cy.viewport(900, 520);
  });

  after(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  it("R3331000 the five proof cards' scenes at hand and board size, base and Radiant", () => {
    settle("hand");
    cy.screenshot("scene-proofs-hand", { capture: "viewport", overwrite: true });
    cy.document().then((doc) =>
      Promise.all(PROOFS.map((id) => Promise.all([pixelsOf(pictureOf(doc, `${id}-base`)), pixelsOf(pictureOf(doc, `${id}-radiant`))]).then(([a, b]) => ({ id, share: changedShare(a, b) })))),
    ).then((shares) => {
      for (const { id, share } of shares) expect(share, `${id}'s Radiant picture differs from its base one`).to.be.at.least(MIN_CHANGED_SHARE);
    });
    settle("board");
    cy.screenshot("scene-proofs-board", { capture: "viewport", overwrite: true });
  });
});
