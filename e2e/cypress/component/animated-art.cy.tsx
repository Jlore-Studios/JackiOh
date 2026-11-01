// R2715000, R2715001: the moving art in a real browser. apps/web's vitest suite proves which faces
// draw the moving picture and what its document holds (cards/art/motion.test.ts,
// cards/fullArtTiers.test.tsx); only a browser animates the picture inside a CSS background image, so
// it is measured here: two screenshots apart, the Animated art's, the Legendary's and the Mythic's art
// windows have changed and the still Epic's has not, and under Reduce motion and with Animated foil off
// none has.
//
// Felinor Flagbearer is printed Legendary, so no rarity foil moves over its art: what changes in its
// window is its tier's motion alone. Each face is the viewer's own copy shown at a tier by
// `FullArtShown`, as the Collection's preview shows one; no `FullArtOwned` is mounted, so every tier
// counts as owned, as in practice.

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { resetCosmeticsForTests } from "../../../apps/web/src/cards/cosmetics.ts";
import { FULL_ARTS_OF_CARD, FullArtShown, OwnCopy, fullArtPickOf, type FullArtTier } from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

type CardDef = (typeof CATALOG)[string];
type Point = { x: number; y: number };
type Shot = { width: number; height: number; data: Uint8ClampedArray; scale: number };

/** Felinor Flagbearer: a Unit printed Legendary, whose face wears no moving rarity foil. */
const CARD = "classicplus-046";
/** The still Epic, then the three tiers whose art moves (R2715001). */
const TIERS: readonly FullArtTier[] = ["Epic", "Animated", "Legendary", "Mythic"];
const MOVING_TIERS = TIERS.filter((tier) => tier !== "Epic");
/** A hand-to-inspect sized face. */
const FACE_WIDTH = 220;
/** The time between the two screenshots: shorter than every beat's loop, longer than a frame. */
const ART_SAMPLE_GAP_MS = 900;
/** The grid the art windows are sampled on, in CSS pixels, and the inset from their edges. */
const SAMPLE_STEP_PX = 6;
const SAMPLE_INSET_PX = 3;
/** A channel must move this much for a point to count as changed: more than compression noise. */
const PIXEL_DELTA_MIN = 4;
/** The share of a moving art window's points that must change between the shots. */
const MOVING_SHARE_MIN = 0.01;
/** Enough room for the faces to mount, the art to draw and the moving pictures to load. */
const SETTLE_TIMEOUT_MS = 8_000;

function must<T>(found: T | null | undefined, what: string): T {
  if (found === null || found === undefined) throw new Error(`no ${what}`);
  return found;
}

/** The pick that shows `tier` of the card's full arts. */
function pickFor(tier: FullArtTier): boolean | string {
  return fullArtPickOf(CARD, must(FULL_ARTS_OF_CARD.get(CARD)?.get(tier), `${tier} of ${CARD}`));
}

function Faces() {
  return (
    <OwnCopy own>
      <div style={{ display: "flex", gap: 12, padding: 12, background: "#1b1f2b" }}>
        {TIERS.map((tier) => (
          <div key={tier} data-face={tier} style={{ width: FACE_WIDTH, height: Math.round(FACE_WIDTH / FACE_ASPECT), flex: "none" }}>
            <FullArtShown defId={CARD} pick={pickFor(tier)}>
              <CardFace face={faceModel({ defId: CARD, def: CATALOG[CARD] as CardDef, radiant: false })} layout="full" />
            </FullArtShown>
          </div>
        ))}
      </div>
    </OwnCopy>
  );
}

/** The viewport as Chrome painted it, decoded to RGBA, with the device pixels per CSS pixel; animations left running. */
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

/** The points of each tier's art window, on the sampling grid. */
function artPoints(doc: Document): Map<FullArtTier, Point[]> {
  const out = new Map<FullArtTier, Point[]>();
  for (const tier of TIERS) {
    const box = must(doc.querySelector(`[data-face="${tier}"] .cf-art`), `${tier}'s art window`).getBoundingClientRect();
    const points: Point[] = [];
    for (let y = box.top + SAMPLE_INSET_PX; y < box.bottom - SAMPLE_INSET_PX; y += SAMPLE_STEP_PX) {
      for (let x = box.left + SAMPLE_INSET_PX; x < box.right - SAMPLE_INSET_PX; x += SAMPLE_STEP_PX) points.push({ x, y });
    }
    out.set(tier, points);
  }
  return out;
}

/** The share of `points` whose color differs between the two shots. */
function changedShare(before: Shot, after: Shot, points: readonly Point[]): number {
  expect(points.length, "the art window holds sampled points").to.be.greaterThan(20);
  const changed = points.filter((point) => {
    const a = pixelAt(before, point);
    const b = pixelAt(after, point);
    return a.some((channel, i) => Math.abs(channel - (b[i] ?? -1)) >= PIXEL_DELTA_MIN);
  });
  return changed.length / points.length;
}

/** Two screenshots `ART_SAMPLE_GAP_MS` apart, and each tier's changed share of its art window. */
function sampleShares(name: string): Cypress.Chainable<Map<FullArtTier, number>> {
  const taken: { before?: Shot; points?: Map<FullArtTier, Point[]> } = {};
  cy.document().then((doc) => {
    taken.points = artPoints(doc);
  });
  screenshot(`${name}-before`).then((shot) => {
    taken.before = shot;
  });
  cy.wait(ART_SAMPLE_GAP_MS);
  return screenshot(`${name}-after`).then((after) => {
    const before = must(taken.before, "first screenshot");
    const points = must(taken.points, "sampled points");
    return new Map(TIERS.map((tier) => [tier, changedShare(before, after, must(points.get(tier), tier))]));
  });
}

describe("R2715001 the full-art cards' art moves", () => {
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

  it("R2715001 the Animated, Legendary and Mythic art windows change between two moments and the still Epic's does not", () => {
    cy.mount(<Faces />);
    cy.get('[data-art-moving="true"]', { timeout: SETTLE_TIMEOUT_MS }).should("have.length", MOVING_TIERS.length);
    cy.document().then((doc) => {
      // The control: each face wears its tier, the Animated art its normal face.
      for (const tier of TIERS) {
        const cf = must(doc.querySelector(`[data-face="${tier}"] > .cf`), `${tier}'s face`);
        expect(cf.getAttribute("data-full-art-tier"), tier).to.eq(tier);
        expect(cf.getAttribute("data-full-art"), tier).to.eq(tier === "Animated" ? null : "true");
        expect(cf.getAttribute("data-art-motion"), tier).to.eq(tier === "Epic" ? null : "animated");
      }
    });
    cy.document().its("fonts.status").should("eq", "loaded");
    sampleShares("moving").then((shares) => {
      cy.task("layout:report", { spec: "animated-art", shares: Object.fromEntries(shares) });
      expect(shares.get("Epic"), "the still Epic's art window").to.eq(0);
      for (const tier of MOVING_TIERS) expect(shares.get(tier), `${tier}'s art window`).to.be.greaterThan(MOVING_SHARE_MIN);
    });
  });

  it("R2715001 under Reduce motion and with Animated foil off no tier's art window changes", () => {
    const stills: [string, () => void][] = [
      ["reduce-motion", () => writeSettings({ reduceMotion: true })],
      ["animated-foil-off", () => writeCardSettings({ animatedFoil: false })],
    ];
    for (const [name, still] of stills) {
      cy.then(() => {
        __resetSettingsForTests();
        writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
        still();
      });
      cy.mount(<Faces />);
      cy.get(".cf[data-art-motion]", { timeout: SETTLE_TIMEOUT_MS }).should("have.length", MOVING_TIERS.length);
      cy.get('.cf[data-art-motion="static"]').should("have.length", MOVING_TIERS.length);
      cy.get('[data-art-moving="true"]').should("not.exist");
      cy.get("[data-face] .cf-art[data-art-pending]").should("not.exist");
      cy.document().its("fonts.status").should("eq", "loaded");
      sampleShares(name).then((shares) => {
        for (const tier of TIERS) expect(shares.get(tier), `${tier}'s art window under ${name}`).to.eq(0);
      });
    }
  });
});
