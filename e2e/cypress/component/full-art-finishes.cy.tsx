// R2720000: every finish on a full-art face in a real browser. apps/web's vitest suite proves
// which attributes a face wears (cards/cosmetics.test.tsx) and what the stylesheet declares; what
// only a layout and style engine can say is measured here: each of the nine finishes draws on a
// full-art Unit (core-004) and a full-art Spell (classicplus-019) at every tier that prints full art
// (FULL_ART_LAYOUT_TIERS, so a later one is covered; the Animated art, R2715000, prints the normal face)
// and at the inspect, hand and small-hand sizes, as a band at most
// FULL_ART_FINISH_OPACITY over the art's edge and under the cost gem, rarity gem, name banner,
// crest, set mark and stats; the three animated finishes move in their motion slot and hold still
// with Animated foil off and under Reduce motion; and a normal face and a board minion draw no band.

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { FULL_ART_FINISH_OPACITY } from "../../../apps/web/src/cards/constants.ts";
import {
  CARD_SLOT_KINDS,
  cosmeticsOf,
  resetCosmeticsForTests,
  writeCardPick,
  type CosmeticId,
} from "../../../apps/web/src/cards/cosmetics.ts";
import {
  FULL_ARTS_OF_CARD,
  FULL_ART_LAYOUT_TIERS,
  FullArtShown,
  OwnCopy,
  fullArtPickOf,
  fullArtSource,
  type FullArtTier,
} from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

type CardDef = (typeof CATALOG)[string];

/** A full-art Unit and a full-art Spell, so the stats' presence and absence are both walked. */
const CARDS = [
  { id: "core-004", why: "a Unit (Gary the Gambler)" },
  { id: "classicplus-019", why: "a Spell (League of Losers)" },
] as const;

/** Every non-default finish. */
const FINISHES = cosmeticsOf("finish")
  .filter((cosmetic) => !cosmetic.default)
  .map((cosmetic) => cosmetic.id);

/** Inspect, hand and small-hand widths. */
const WIDTHS = [270, 170, 100] as const;

/** The animated finishes and the keyframes each moves on its finish layer. */
const ANIMATED = {
  "finish-gilded-edge": "cosmetic-foil-glint",
  "finish-obsidian": "cosmetic-finish-gloss",
  "finish-starlit": "cosmetic-finish-twinkle",
} as const satisfies Partial<Record<CosmeticId, string>>;

/** The grid the worn box's points are sampled from, in CSS pixels. */
const SAMPLE_STEP_PX = 3;

/** The gap between two faces, in CSS pixels. */
const FACE_GAP_PX = 8;

/**
 * The tile Chrome rasterizes the page in, in CSS pixels. A face's twin one tile to its right is crossed by the tile
 * seams where it is, so the control measures the pick and not the seams' noise: with the twins 178px apart, the
 * Mythic's Signature look (R2949000) drew 1.1% of it under the signature's shadows at 170px. A face too wide for one
 * tile keeps the plain gap.
 */
const RASTER_TILE_PX = 256;

/** How far a channel must move for a pixel to count as drawn differently. */
const PIXEL_DELTA_MIN = 4;

/** The least share of a worn box's sampled points a finish must change to count as drawn. */
const FINISH_CHANGED_SHARE_MIN = 0.1;

/** The most share of a plain box's sampled points the control may change: the measure's noise. */
const CONTROL_CHANGED_SHARE_MAX = 0.01;

/** The parts a finish's band must sit under. */
const PARTS = [".cost-gem", ".cf-gem", ".card-name", ".cf-crest", ".cf-set", ".cf-stats", ".cf-signature"] as const;

/** The pick that shows `tier` of the full art the card `id` prints. */
function pickFor(id: string, tier: FullArtTier): boolean | string {
  const source = fullArtSource(id);
  const fullArt = FULL_ARTS_OF_CARD.get(source)?.get(tier);
  if (fullArt === undefined) throw new Error(`no ${tier} full art for ${source}`);
  return fullArtPickOf(source, fullArt);
}

/** Clears the cards' own picks, so their faces wear the defaults. */
function clearPicks(): void {
  for (const { id } of CARDS) {
    for (const kind of CARD_SLOT_KINDS) writeCardPick(id, kind, null);
  }
}

/** A worn full-art face beside a plain one, at every tier. */
function Twins({ id, width }: { id: (typeof CARDS)[number]["id"]; width: number }) {
  return (
    <OwnCopy own>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          rowGap: FACE_GAP_PX,
          columnGap: width + FACE_GAP_PX <= RASTER_TILE_PX ? RASTER_TILE_PX - width : FACE_GAP_PX,
          padding: FACE_GAP_PX,
          background: "#1b1f2b",
        }}
      >
        {FULL_ART_LAYOUT_TIERS.flatMap((tier) =>
          (["worn", "plain"] as const).map((which) => (
            <div
              key={`${tier}-${which}`}
              data-face={`${tier}-${which}`}
              style={{ width, height: Math.round(width / FACE_ASPECT), flex: "none" }}
            >
              <FullArtShown defId={id} pick={pickFor(id, tier)}>
                <CardFace
                  face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant: false })}
                  layout="full"
                  worn={which === "worn"}
                />
              </FullArtShown>
            </div>
          )),
        )}
      </div>
    </OwnCopy>
  );
}

/** One worn full-art face, at the Full-art cards setting's Epic. */
function Single({ width }: { width: number }) {
  return (
    <OwnCopy own>
      <div style={{ display: "flex", gap: 8, padding: 8, background: "#1b1f2b" }}>
        <div data-face="single" style={{ width, height: Math.round(width / FACE_ASPECT), flex: "none" }}>
          <CardFace face={faceModel({ defId: "core-004", def: CATALOG["core-004"] as CardDef, radiant: false })} layout="full" worn />
        </div>
      </div>
    </OwnCopy>
  );
}

/** Two worn normal faces and a worn board minion. */
function NormalAndMinion() {
  const face = faceModel({ defId: "core-004", def: CATALOG["core-004"] as CardDef, radiant: false });
  const spell = faceModel({ defId: "classicplus-019", def: CATALOG["classicplus-019"] as CardDef, radiant: false });
  const u = unit("p1", { defId: "core-004" });
  return (
    <OwnCopy own>
      <div style={{ display: "flex", gap: 12, padding: 12, background: "#1b1f2b" }}>
        <div data-which="unit" style={{ width: 170, height: Math.round(170 / FACE_ASPECT), flex: "none" }}>
          <CardFace face={face} layout="full" worn />
        </div>
        <div data-which="spell" style={{ width: 170, height: Math.round(170 / FACE_ASPECT), flex: "none" }}>
          <CardFace face={spell} layout="full" worn />
        </div>
        <div data-which="minion" style={{ width: 81, height: 113, flex: "none" }}>
          <MinionFace face={face} unit={u} worn />
        </div>
      </div>
    </OwnCopy>
  );
}

/** The `tier` face's `which` copy in a Twins mount. */
function faceIn(doc: Document, tier: FullArtTier, which: "worn" | "plain"): HTMLElement {
  const cf = doc.querySelector<HTMLElement>(`[data-face="${tier}-${which}"] > .cf`);
  if (cf === null) throw new Error(`no ${tier} ${which} face`);
  return cf;
}

/** The panel of `cf`. */
function scaleOf(cf: HTMLElement): HTMLElement {
  const scale = cf.querySelector<HTMLElement>(".cf-scale");
  if (scale === null) throw new Error("no panel");
  return scale;
}

/** Whether anything under `root` runs an animation. */
function running(root: Element): boolean {
  return root.getAnimations({ subtree: true }).some((animation) => animation.playState === "running");
}

type Shot = { width: number; height: number; data: Uint8ClampedArray; scale: number };

/** The viewport as Chrome painted it, decoded to RGBA, with the device pixels per CSS pixel. */
function screenshot(name: string): Cypress.Chainable<Shot> {
  let path = "";
  return cy
    .screenshot(name, { capture: "viewport", overwrite: true, onAfterScreenshot: (_el, props) => { path = props.path; } })
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

function pixelAt(shot: Shot, { x, y }: { x: number; y: number }): [number, number, number] {
  const at = (Math.round(y * shot.scale) * shot.width + Math.round(x * shot.scale)) * 4;
  return [shot.data[at] ?? -1, shot.data[at + 1] ?? -1, shot.data[at + 2] ?? -1];
}

type Point = { x: number; y: number };

/**
 * The share of the worn box's sampled points whose color differs from the point at the same offset
 * in the plain box: the finish's paint, since the two faces are alike but for the worn pick.
 */
function changedShare(shot: Shot, wornBox: DOMRect, plainBox: DOMRect): number {
  const dx = plainBox.left - wornBox.left;
  const dy = plainBox.top - wornBox.top;
  const points: Point[] = [];
  for (let y = Math.ceil(wornBox.top) + 2; y < wornBox.bottom - 2; y += SAMPLE_STEP_PX) {
    for (let x = Math.ceil(wornBox.left) + 2; x < wornBox.right - 2; x += SAMPLE_STEP_PX) {
      points.push({ x, y });
    }
  }
  expect(points.length, "the worn box holds sampled points").to.be.greaterThan(100);
  const changed = points.filter((point) => {
    const a = pixelAt(shot, point);
    const b = pixelAt(shot, { x: point.x + dx, y: point.y + dy });
    return a.some((channel, i) => Math.abs(channel - (b[i] ?? -1)) >= PIXEL_DELTA_MIN);
  });
  return changed.length / points.length;
}

describe("R2720000 every finish on a full-art face", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    clearPicks();
    cy.viewport(1280, 900);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    clearPicks();
  });

  for (const finish of FINISHES) {
    for (const width of WIDTHS) {
      it(`R2720000 ${finish} draws on a full-art Unit and Spell at every tier at ${width}px, under its parts, at most FULL_ART_FINISH_OPACITY`, () => {
        writeSettings({ reduceMotion: true });
        for (const { id, why } of CARDS) {
          cy.then(() => {
            clearPicks();
            writeCardPick(id, "finish", finish);
          });
          cy.mount(<Twins id={id} width={width} />);
          cy.document().should((doc) => {
            for (const tier of FULL_ART_LAYOUT_TIERS) {
              const where = `${id} (${why}) at ${tier}, ${width}px`;
              const worn = faceIn(doc, tier, "worn");
              const plain = faceIn(doc, tier, "plain");
              expect(worn.getAttribute("data-full-art"), `${where}: the worn face is full art`).to.eq("true");
              expect(worn.getAttribute("data-full-art-tier"), where).to.eq(tier);
              expect(plain.getAttribute("data-full-art"), `${where}: the plain face is full art`).to.eq("true");
              expect(plain.getAttribute("data-full-art-tier"), where).to.eq(tier);
              expect(worn.getAttribute("data-finish"), `${where}: the worn face wears the finish`).to.eq(finish);
              expect(plain.getAttribute("data-finish"), `${where}: the plain face wears none`).to.eq(null);
              const band = getComputedStyle(scaleOf(worn), "::before");
              expect(band.content, `${where}: the band draws`).to.not.eq("none");
              expect(Number(band.opacity), `${where}: the band covers the art at most ${FULL_ART_FINISH_OPACITY}`).to.be.at.most(
                FULL_ART_FINISH_OPACITY,
              );
              expect(band.zIndex, `${where}: the band lies under the parts`).to.eq("1");
              expect(band.maskImage, `${where}: the band fades toward the middle`).to.contain("linear-gradient");
              expect(getComputedStyle(scaleOf(plain), "::before").content, `${where}: the plain face draws no band`).to.eq("none");
              for (const selector of PARTS) {
                const required =
                  selector === ".cost-gem" ||
                  selector === ".cf-gem" ||
                  selector === ".card-name" ||
                  (selector === ".cf-stats" && id === "core-004") ||
                  (selector === ".cf-signature" && tier === "Mythic");
                const part = scaleOf(worn).querySelector<HTMLElement>(`:scope > ${selector}`);
                if (required) expect(part, `${where}: ${selector} draws`).to.not.eq(null);
                if (part !== null) {
                  expect(Number(getComputedStyle(part).zIndex), `${where}: ${selector} sits over the band`).to.be.greaterThan(1);
                }
              }
            }
          });
          screenshot(`R2720000-${finish}-${width}-${id}`).then((shot) => {
            cy.document().then((doc) => {
              for (const tier of FULL_ART_LAYOUT_TIERS) {
                const wornBox = faceIn(doc, tier, "worn").parentElement?.getBoundingClientRect();
                const plainBox = faceIn(doc, tier, "plain").parentElement?.getBoundingClientRect();
                if (wornBox === undefined || wornBox === null || plainBox === undefined || plainBox === null) {
                  throw new Error(`no boxes at ${tier}`);
                }
                expect(
                  changedShare(shot, wornBox, plainBox),
                  `${id} at ${tier}, ${width}px: the finish changes the face`,
                ).to.be.at.least(FINISH_CHANGED_SHARE_MIN);
              }
            });
          });
        }
      });
    }
  }

  for (const width of WIDTHS) {
    it(`R2720000 the control: a full-art face wearing no finish draws as the plain one beside it at ${width}px`, () => {
      writeSettings({ reduceMotion: true });
      for (const { id } of CARDS) {
        cy.then(() => {
          clearPicks();
        });
        cy.mount(<Twins id={id} width={width} />);
        screenshot(`R2720000-control-${width}-${id}`).then((shot) => {
          cy.document().then((doc) => {
            for (const tier of FULL_ART_LAYOUT_TIERS) {
              const wornBox = faceIn(doc, tier, "worn").parentElement?.getBoundingClientRect();
              const plainBox = faceIn(doc, tier, "plain").parentElement?.getBoundingClientRect();
              if (wornBox === undefined || wornBox === null || plainBox === undefined || plainBox === null) {
                throw new Error(`no boxes at ${tier}`);
              }
              expect(changedShare(shot, wornBox, plainBox), `${id} at ${tier}, ${width}px: no pick changes nothing`).to.be.at.most(
                CONTROL_CHANGED_SHARE_MAX,
              );
            }
          });
        });
      }
    });
  }

  for (const [finish, keyframes] of Object.entries(ANIMATED)) {
    it(`R2720000 ${finish} moves on a full-art face, and holds still with Animated foil off and under Reduce motion`, () => {
      cy.then(() => {
        clearPicks();
        writeCardPick("core-004", "finish", finish as CosmeticId);
      });
      cy.mount(<Single width={170} />);
      cy.document().should((doc) => {
        const cf = doc.querySelector<HTMLElement>('[data-face="single"] > .cf');
        if (cf === null) throw new Error("no face");
        expect(cf.getAttribute("data-full-art"), "the face is full art").to.eq("true");
        expect(cf.getAttribute("data-full-art-tier"), "its tier moves nothing itself").to.eq("Epic");
        expect(cf.getAttribute("data-cosmetic-motion"), "the finish moves").to.eq("animated");
        expect(getComputedStyle(cf, "::before").animationName, "on its finish layer").to.eq(keyframes);
        expect(running(cf), "running").to.eq(true);
      });
      cy.then(() => {
        writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, animatedFoil: false });
      });
      cy.mount(<Single width={170} />);
      cy.document().should((doc) => {
        const cf = doc.querySelector<HTMLElement>('[data-face="single"] > .cf');
        if (cf === null) throw new Error("no face");
        expect(cf.getAttribute("data-full-art"), "the face is full art").to.eq("true");
        expect(cf.getAttribute("data-finish")).to.eq(finish);
        expect(cf.getAttribute("data-cosmetic-motion"), "with Animated foil off it holds still").to.eq("static");
        expect(getComputedStyle(cf, "::before").animationName).to.eq("none");
        expect(running(cf)).to.eq(false);
      });
      cy.then(() => {
        writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
        writeSettings({ reduceMotion: true });
      });
      cy.get("html").should("have.attr", "data-reduce-motion", "true");
      cy.mount(<Single width={170} />);
      cy.document().should((doc) => {
        const cf = doc.querySelector<HTMLElement>('[data-face="single"] > .cf');
        if (cf === null) throw new Error("no face");
        expect(getComputedStyle(cf, "::before").animationName, "under Reduce motion it holds still").to.eq("none");
        expect(running(cf)).to.eq(false);
      });
    });
  }

  it("R2720000 a face that is not full art wears each finish with no band, and so does a board minion", () => {
    for (const finish of FINISHES) {
      cy.then(() => {
        writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, fullArt: false });
        clearPicks();
        writeCardPick("core-004", "finish", finish);
        writeCardPick("classicplus-019", "finish", finish);
      });
      cy.mount(<NormalAndMinion />);
      cy.document().should((doc) => {
        for (const which of ["unit", "spell", "minion"] as const) {
          const cf = doc.querySelector<HTMLElement>(`[data-which="${which}"] > .cf`);
          if (cf === null) throw new Error(`no ${which} face`);
          expect(cf.getAttribute("data-finish"), `${which} wears ${finish}`).to.eq(finish);
          expect(cf.getAttribute("data-full-art"), `${which} is no full art`).to.eq(null);
          expect(getComputedStyle(scaleOf(cf), "::before").content, `${which} draws no band`).to.eq("none");
        }
      });
    }
  });
});
