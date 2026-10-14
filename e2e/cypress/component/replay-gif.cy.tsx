// Sharing a replay as a GIF (#513, R513000): the raster draws a frame plan's known view and the
// encoder's GIF decodes back, in a browser, to the colours the plan names. jsdom draws nothing, so
// the unit tests (apps/web/src/replay/gif/) hold the plan and the bytes, and this holds the pixels.
//
// The golden is the plan itself: each filled rectangle's colour is sampled where nothing drawn after
// it lies over it and compared with the palette entry the encoder quantises that fill to, within
// COLOUR_SLACK per channel (the specs compare colours exactly; 1 is rounding in the browser's GIF
// decoder). No stored image is compared, since text depends on the machine's fonts. Each card's art
// window must also show several colours, so a blank or missing picture fails.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/replay-gif.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import { REPLAY_GIF_FPS, REPLAY_GIF_MAX_BYTES, REPLAY_GIF_SIZES } from "../../../apps/web/src/replay/config.ts";
import type { ReplayGifPreset } from "../../../apps/web/src/replay/config.ts";
import { GIF_PALETTE, paletteIndex } from "../../../apps/web/src/replay/gif/encoder.ts";
import { assetsFor, renderGif } from "../../../apps/web/src/replay/gif/export.ts";
import { faceKey, planFrames } from "../../../apps/web/src/replay/gif/plan.ts";
import type { GifFrame, GifItem } from "../../../apps/web/src/replay/gif/plan.ts";
import { baseView, fullBoardView } from "../../../apps/web/src/test/fixtures.ts";

/** A sample this near a rectangle's edge, plus its stroke, is anti-aliasing's, not the fill's. */
const EDGE_SLACK_PX = 1;
/** Fewer samples than this and the golden proves nothing. */
const MIN_SAMPLES = 5;
/** Per channel; the palette's own colours decode exactly, so this is only rounding. */
const COLOUR_SLACK = 1;
/** A card's art window shows at least this many palette colours on the grid below. */
const MIN_ART_COLOURS = 3;
/** The art window is sampled on a grid this many points across. */
const ART_GRID = 5;
/** Where in a rectangle a sample may be taken, as fractions of its width and height: its middle first, then outwards. */
const CANDIDATE_FRACTIONS = [0.5, 0.25, 0.75, 0.125, 0.875, 0.375, 0.625] as const;
/** A big rectangle (the background, a seat's band) is sampled at up to this many of its candidates. */
const SAMPLES_PER_RECT = 3;
/** A glyph is at most this wide in ems, and the box around text is padded by this much. */
const TEXT_EM_WIDTH = 0.7;
const TEXT_PAD_PX = 2;
const RENDER_TIMEOUT_MS = 60_000;

const PORTRAITS = { p1: "vanilla", p2: "gary" } as const;
const lookup = lookupFromDefs(CATALOG);

type Box = { x: number; y: number; w: number; h: number };
type Sample = { x: number; y: number; expected: readonly [number, number, number]; label: string };
type Decoded = { width: number; height: number; data: Uint8ClampedArray };

/** The region an item covers, text over-estimated by the widest glyph and padded. */
function boxOf(item: GifItem): Box {
  if (item.kind !== "text") return item;
  const w = item.size * TEXT_EM_WIDTH * item.text.length;
  const left = item.align === "left" ? item.x : item.align === "center" ? item.x - w / 2 : item.x - w;
  return { x: left - TEXT_PAD_PX, y: item.y - item.size / 2 - TEXT_PAD_PX, w: w + 2 * TEXT_PAD_PX, h: item.size + 2 * TEXT_PAD_PX };
}

function inside(box: Box, x: number, y: number, inset: number): boolean {
  return x > box.x + inset && x < box.x + box.w - inset && y > box.y + inset && y < box.y + box.h - inset;
}

function rgbOf(hex: string): [number, number, number] {
  return [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16)) as [number, number, number];
}

/** The palette's colour for a fill. */
function quantised(hex: string): [number, number, number] {
  const [r, g, b] = rgbOf(hex);
  const at = paletteIndex(r, g, b) * 3;
  return [GIF_PALETTE[at] ?? -1, GIF_PALETTE[at + 1] ?? -1, GIF_PALETTE[at + 2] ?? -1];
}

/** For each filled rectangle, the first few of its candidate points (CANDIDATE_FRACTIONS) that nothing is drawn over. */
function samplesOf(frame: GifFrame): Sample[] {
  const samples: Sample[] = [];
  frame.items.forEach((item, index) => {
    if (item.kind !== "rect" || item.fill === null) return;
    const later = frame.items.slice(index + 1).map(boxOf);
    const points = CANDIDATE_FRACTIONS.flatMap((fy) => CANDIDATE_FRACTIONS.map((fx) => [item.x + item.w * fx, item.y + item.h * fy] as const));
    const free = points
      .filter(([x, y]) => inside(item, x, y, EDGE_SLACK_PX + item.line) && !later.some((box) => inside(box, x, y, -EDGE_SLACK_PX)))
      .slice(0, SAMPLES_PER_RECT);
    for (const [x, y] of free) {
      samples.push({ x: Math.round(x), y: Math.round(y), expected: quantised(item.fill), label: `${item.fill} rect at ${String(item.x)},${String(item.y)}` });
    }
  });
  return samples;
}

function pixel(image: Decoded, x: number, y: number): [number, number, number] {
  const at = (y * image.width + x) * 4;
  return [image.data[at] ?? -1, image.data[at + 1] ?? -1, image.data[at + 2] ?? -1];
}

/** The GIF's first frame as the browser decodes it. */
async function decode(bytes: Uint8Array<ArrayBuffer>): Promise<Decoded> {
  const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/gif" }));
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const ctx = canvas.getContext("2d");
  if (ctx === null) throw new Error("this browser has no 2D canvas");
  ctx.drawImage(bitmap, 0, 0);
  return { width: bitmap.width, height: bitmap.height, data: ctx.getImageData(0, 0, bitmap.width, bitmap.height).data };
}

const VIEWS = [
  { label: "a full board", view: fullBoardView },
  { label: "an empty board", view: baseView },
] as const;

describe("a replay clip drawn as a GIF (R513000)", () => {
  for (const { label, view } of VIEWS) {
    for (const preset of Object.keys(REPLAY_GIF_SIZES) as ReplayGifPreset[]) {
      for (const via of ["worker", "thread"] as const) {
        it(`${label} in the ${preset} shape, drawn in a ${via}, decodes to the plan's colours`, () => {
          const shown = view();
          const clip = [{ step: 0, turn: shown.turn, view: shown }];
          const plan = planFrames(clip, { ...REPLAY_GIF_SIZES[preset], fps: REPLAY_GIF_FPS, catalogVersion: "v0.0.0", portraits: PORTRAITS });
          const assets = assetsFor(plan, clip, lookup);
          const frame = plan.frames[0];
          if (frame === undefined) throw new Error("the plan has no frame");
          const samples = samplesOf(frame);
          expect(samples.length, "samples").to.be.at.least(MIN_SAMPLES);

          const drawn = renderGif(plan, assets, { signal: new AbortController().signal, maxBytes: REPLAY_GIF_MAX_BYTES, onFrame: () => undefined }, via).then(async (bytes) => {
            if (bytes === null) throw new Error("the GIF came out over the byte cap");
            return decode(bytes);
          });
          cy.wrap(drawn, { timeout: RENDER_TIMEOUT_MS }).then((value) => {
            const image = value as unknown as Decoded;
            expect([image.width, image.height]).to.deep.equal([plan.width, plan.height]);
            const off = samples.flatMap((sample) => {
              const got = pixel(image, sample.x, sample.y);
              return got.every((channel, i) => Math.abs(channel - (sample.expected[i] ?? 0)) <= COLOUR_SLACK) ? [] : [`${sample.label}: ${got.join(",")} is not ${sample.expected.join(",")}`];
            });
            expect(off, "samples off the plan's colours").to.deep.equal([]);

            // Every card's art window shows a picture, not one flat colour.
            for (const item of frame.items) {
              if (item.kind !== "card" || assets.faces[faceKey(item.defId, item.radiant)]?.art === null) continue;
              const edge = Math.max(2, Math.round(item.h * 0.06));
              const window = { x: item.x + edge, y: item.y + edge, w: item.w - 2 * edge, h: item.h * 0.5 };
              const colours = new Set<string>();
              for (let row = 0; row < ART_GRID; row += 1) {
                for (let col = 0; col < ART_GRID; col += 1) {
                  const x = Math.round(window.x + ((col + 0.5) * window.w) / ART_GRID);
                  const y = Math.round(window.y + ((row + 0.5) * window.h) / ART_GRID);
                  colours.add(pixel(image, x, y).join(","));
                }
              }
              expect(colours.size, `${item.defId}'s art colours`).to.be.at.least(MIN_ART_COLOURS);
            }
          });
        });
      }
    }
  }
});
