// The raster of a shared GIF (#513, R513000): a frame plan drawn, frame by frame, onto a 2D canvas
// and handed to the encoder. It runs in `raster.worker.ts` on an `OffscreenCanvas`; where a worker
// cannot draw, `export.ts` runs it on the main thread, one frame per macrotask. It knows no React,
// no catalog and no `t()`: the faces and art it draws arrive in `GifAssets`, made from the same
// data the board's faces use (`cards/art`), so the same plan and assets give the same pixels' bytes.
// It loads on demand with the encoder, never in the main bundle.

import type { CardType, PortraitId, Rarity } from "@jackioh/shared";

import { EMBLEMS, EMBLEM_BOX } from "../../cards/art/emblems.ts";
import { ART_BOX, type ArtSpec } from "../../cards/art/procedural.ts";
import {
  ACCENT_OPACITY,
  DROP_TAIL,
  EMBLEM_STROKE_WIDTH,
  GLOW_MID_STOP,
  RAY_LENGTH,
  VIGNETTE_OPACITY,
  VIGNETTE_RADIUS,
  VIGNETTE_START,
} from "../../cards/art/svg.ts";
import { RADIANT_PALETTE } from "../../cards/art/themes.ts";
import { createGifEncoder } from "./encoder.ts";
import { faceKey } from "./face.ts";
import type { GifFrame, GifItem, GifPlan } from "./plan.ts";

/** The slice of the 2D context the raster draws with: both a canvas element's and an `OffscreenCanvas`'s. */
export type Ctx2D = CanvasState &
  CanvasTransform &
  CanvasCompositing &
  CanvasFillStrokeStyles &
  CanvasRect &
  CanvasDrawPath &
  CanvasPath &
  CanvasText &
  CanvasTextDrawingStyles &
  CanvasPathDrawingStyles &
  CanvasImageData;

/** What a card face needs to be drawn: its name, type, rarity and procedural art (null for a Glitch). */
export type GifFace = { name: string; type: CardType; rarity: Rarity | null; art: ArtSpec | null };

export type GifAssets = {
  /** By `faceKey`. */
  faces: Record<string, GifFace>;
  portraits: Partial<Record<PortraitId, { art: ArtSpec; rim: string }>>;
};

/** What the worker is asked: the plan, its assets and the byte cap. */
export type RasterRequest = { plan: GifPlan; assets: GifAssets; maxBytes: number };

/** What the worker answers, in order: a `frame` per frame drawn, then one of the other four. */
export type RasterAnswer =
  | { type: "frame"; frame: number }
  | { type: "done"; bytes: Uint8Array<ArrayBuffer> }
  | { type: "over" }
  | { type: "unsupported" }
  | { type: "failed"; message: string };

// The faces' colours are cards.css's, copied: the raster has no DOM to read a custom property from.
const TYPE_FRAME: Readonly<Record<CardType, string>> = {
  Unit: "#7a6446",
  Spell: "#3e5f9a",
  "Field Spell": "#3f7d5c",
  Trap: "#8e3a4c",
  "Field Trap": "#6e3a8e",
};
const RARITY_EDGE: Readonly<Record<Rarity, string>> = {
  Common: "#d9dde6",
  Rare: "#3d8bfd",
  Epic: "#a45cf0",
  Legendary: "#ff9f1c",
  Mythic: "#ff5fd2",
  Token: "#8a90a6",
};
const COST_FILL = "#3d8bfd";
const ATTACK_FILL = "#e0a526";
const HEALTH_FILL = "#c0392b";
const UNKNOWN_FILL = "#555555";
const BACK_FILL = "#1b2036";
const BACK_EDGE = "#d4af37";
const BADGE_TEXT = "#ffffff";
const FONT = "sans-serif";

const EDGE_MIN = 2;
const EDGE_SHARE = 0.06;
const ART_HEIGHT_SHARE = 0.5;
const NAME_LINE_SHARE = 0.66;
const NAME_SIZE_SHARE = 0.1;
const NAME_SIZE_MIN = 6;
const BADGE_RADIUS_SHARE = 0.2;
const BADGE_RADIUS_MIN = 4;
const BADGE_TEXT_SHARE = 1.3;
const BACK_LINE = 2;
const RIM_SHARE = 0.06;
const RIM_MIN = 2;
const DEGREES = Math.PI / 180;
const FULL_CIRCLE = Math.PI * 2;

/** A 2D context over a canvas of the given size: an `OffscreenCanvas` where there is one, else a canvas element. */
export function canvasContext(width: number, height: number): Ctx2D | null {
  if (typeof OffscreenCanvas === "function") {
    return new OffscreenCanvas(width, height).getContext("2d", { willReadFrequently: true });
  }
  if (typeof document !== "object") return null;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  return canvas.getContext("2d", { willReadFrequently: true });
}

const HEX = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i;

/** `hex` with an alpha, as a CSS colour; anything that is not `#rrggbb` is returned as it came. */
function rgba(hex: string, alpha: number): string {
  const match = HEX.exec(hex);
  if (match === null) return hex;
  const [, r = "00", g = "00", b = "00"] = match;
  return `rgba(${String(parseInt(r, 16))},${String(parseInt(g, 16))},${String(parseInt(b, 16))},${String(alpha)})`;
}

function glyph(ctx: Ctx2D, placed: NonNullable<ArtSpec["accent"]>, opacity: number): void {
  const path = EMBLEMS[placed.glyph];
  ctx.save();
  ctx.globalAlpha = opacity;
  ctx.translate(placed.x, placed.y);
  ctx.rotate(placed.rotate * DEGREES);
  const scale = placed.size / EMBLEM_BOX;
  ctx.scale(scale, scale);
  ctx.translate(-EMBLEM_BOX / 2, -EMBLEM_BOX / 2);
  const shape = new Path2D(path.d);
  ctx.fillStyle = placed.fill;
  ctx.fill(shape, path.rule);
  ctx.strokeStyle = placed.stroke;
  ctx.lineWidth = EMBLEM_STROKE_WIDTH;
  ctx.lineJoin = "round";
  ctx.stroke(shape);
  ctx.restore();
}

function mote(ctx: Ctx2D, m: ArtSpec["motes"][number], shape: ArtSpec["moteShape"]): void {
  ctx.globalAlpha = m.opacity;
  ctx.fillStyle = m.fill;
  if (shape === "pixel") {
    ctx.fillRect(m.x - m.r, m.y - m.r, m.r * 2, m.r * 2);
    return;
  }
  ctx.beginPath();
  if (shape === "dot") {
    ctx.arc(m.x, m.y, m.r, 0, FULL_CIRCLE);
  } else {
    ctx.moveTo(m.x, m.y - m.r * DROP_TAIL);
    ctx.lineTo(m.x + m.r, m.y);
    ctx.arc(m.x, m.y, m.r, 0, Math.PI, false);
  }
  ctx.closePath();
  ctx.fill();
}

/**
 * A card's procedural art in the window `(x, y, w, h)`, in the order `artSvg` (cards/art/svg.ts)
 * lays its picture out: the sky, the glow, a Radiant card's rays, the ridges, the motif's glyphs,
 * the emblem and its accent, the motes and the vignette, the box scaled to cover the window.
 */
export function drawArt(ctx: Ctx2D, spec: ArtSpec, x: number, y: number, w: number, h: number): void {
  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, w, h);
  ctx.clip();
  const scale = Math.max(w, h) / ART_BOX;
  ctx.translate(x + (w - ART_BOX * scale) / 2, y + (h - ART_BOX * scale) / 2);
  ctx.scale(scale, scale);

  const { sky, glow } = spec;
  const half = ART_BOX / 2;
  const angle = sky.angle * DEGREES;
  const dx = Math.sin(angle) * half;
  const dy = -Math.cos(angle) * half;
  const skyGradient = ctx.createLinearGradient(half - dx, half - dy, half + dx, half + dy);
  skyGradient.addColorStop(0, sky.from);
  skyGradient.addColorStop(1, sky.to);
  ctx.fillStyle = skyGradient;
  ctx.fillRect(0, 0, ART_BOX, ART_BOX);

  const glowGradient = ctx.createRadialGradient(glow.cx, glow.cy, 0, glow.cx, glow.cy, glow.r);
  glowGradient.addColorStop(0, rgba(glow.color, glow.opacity));
  glowGradient.addColorStop(GLOW_MID_STOP, rgba(glow.color, glow.opacity * GLOW_MID_STOP));
  glowGradient.addColorStop(1, rgba(glow.color, 0));
  ctx.fillStyle = glowGradient;
  ctx.beginPath();
  ctx.arc(glow.cx, glow.cy, glow.r, 0, FULL_CIRCLE);
  ctx.fill();

  ctx.fillStyle = RADIANT_PALETTE.ray;
  for (const ray of spec.rays) {
    const a = (ray.angle - ray.width / 2) * DEGREES;
    const b = (ray.angle + ray.width / 2) * DEGREES;
    ctx.globalAlpha = ray.opacity;
    ctx.beginPath();
    ctx.moveTo(glow.cx, glow.cy);
    ctx.lineTo(glow.cx + RAY_LENGTH * Math.cos(a), glow.cy + RAY_LENGTH * Math.sin(a));
    ctx.lineTo(glow.cx + RAY_LENGTH * Math.cos(b), glow.cy + RAY_LENGTH * Math.sin(b));
    ctx.closePath();
    ctx.fill();
  }

  for (const ridge of spec.ridges) {
    ctx.globalAlpha = ridge.opacity;
    ctx.fillStyle = ridge.fill;
    ctx.fill(new Path2D(ridge.d), "evenodd");
  }
  ctx.globalAlpha = 1;

  for (const placed of spec.motifGlyphs) glyph(ctx, placed, placed.opacity);
  glyph(ctx, spec.emblem, 1);
  if (spec.accent !== null) glyph(ctx, spec.accent, ACCENT_OPACITY);

  for (const m of spec.motes) mote(ctx, m, spec.moteShape);
  ctx.globalAlpha = 1;

  const vignette = ctx.createRadialGradient(half, half, 0, half, half, VIGNETTE_RADIUS);
  vignette.addColorStop(VIGNETTE_START, "rgba(0,0,0,0)");
  vignette.addColorStop(1, `rgba(0,0,0,${String(VIGNETTE_OPACITY)})`);
  ctx.fillStyle = vignette;
  ctx.fillRect(0, 0, ART_BOX, ART_BOX);
  ctx.restore();
}

function font(ctx: Ctx2D, size: number, bold: boolean): void {
  ctx.font = `${bold ? "bold " : ""}${String(size)}px ${FONT}`;
}

function badge(ctx: Ctx2D, cx: number, cy: number, radius: number, fill: string, value: number): void {
  ctx.fillStyle = fill;
  ctx.beginPath();
  ctx.arc(cx, cy, radius, 0, FULL_CIRCLE);
  ctx.fill();
  ctx.fillStyle = BADGE_TEXT;
  ctx.textAlign = "center";
  font(ctx, Math.round(radius * BADGE_TEXT_SHARE), true);
  ctx.fillText(String(value), cx, cy);
}

function drawCard(ctx: Ctx2D, item: Extract<GifItem, { kind: "card" }>, assets: GifAssets): void {
  const { x, y, w, h } = item;
  const face = assets.faces[faceKey(item.defId, item.radiant)];
  if (face === undefined) {
    ctx.fillStyle = UNKNOWN_FILL;
    ctx.fillRect(x, y, w, h);
    ctx.fillStyle = BADGE_TEXT;
    ctx.textAlign = "center";
    font(ctx, Math.round(h * NAME_LINE_SHARE * ART_HEIGHT_SHARE), true);
    ctx.fillText("?", x + w / 2, y + h / 2);
    return;
  }
  const edge = Math.max(EDGE_MIN, Math.round(h * EDGE_SHARE));
  ctx.fillStyle = TYPE_FRAME[face.type];
  ctx.fillRect(x, y, w, h);
  ctx.strokeStyle = face.rarity === null ? BACK_EDGE : RARITY_EDGE[face.rarity];
  ctx.lineWidth = edge;
  ctx.strokeRect(x + edge / 2, y + edge / 2, w - edge, h - edge);
  const artBox = { x: x + edge, y: y + edge, w: w - 2 * edge, h: h * ART_HEIGHT_SHARE };
  if (face.art === null) {
    ctx.fillStyle = BACK_FILL;
    ctx.fillRect(artBox.x, artBox.y, artBox.w, artBox.h);
  } else {
    drawArt(ctx, face.art, artBox.x, artBox.y, artBox.w, artBox.h);
  }
  ctx.fillStyle = BADGE_TEXT;
  ctx.textAlign = "center";
  font(ctx, Math.max(NAME_SIZE_MIN, Math.round(h * NAME_SIZE_SHARE)), true);
  ctx.fillText(face.name, x + w / 2, y + h * NAME_LINE_SHARE, w - 2 * edge);
  const radius = Math.max(BADGE_RADIUS_MIN, Math.round(w * BADGE_RADIUS_SHARE));
  badge(ctx, x + radius, y + radius, radius, COST_FILL, item.cost);
  if (item.attack !== null) badge(ctx, x + radius, y + h - radius, radius, ATTACK_FILL, item.attack);
  if (item.health !== null) badge(ctx, x + w - radius, y + h - radius, radius, HEALTH_FILL, item.health);
}

function drawPortrait(ctx: Ctx2D, item: Extract<GifItem, { kind: "portrait" }>, assets: GifAssets): void {
  const { x, y, w, h } = item;
  const look = assets.portraits[item.portrait];
  const oval = (): void => {
    ctx.beginPath();
    ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, FULL_CIRCLE);
  };
  if (look === undefined) {
    ctx.fillStyle = UNKNOWN_FILL;
    oval();
    ctx.fill();
    return;
  }
  ctx.save();
  oval();
  ctx.clip();
  drawArt(ctx, look.art, x, y, w, h);
  ctx.restore();
  ctx.strokeStyle = look.rim;
  ctx.lineWidth = Math.max(RIM_MIN, Math.round(w * RIM_SHARE));
  oval();
  ctx.stroke();
}

/** One frame's items, in order, onto `ctx`. */
export function drawFrame(ctx: Ctx2D, frame: GifFrame, assets: GifAssets): void {
  ctx.textBaseline = "middle";
  for (const item of frame.items) {
    switch (item.kind) {
      case "rect":
        if (item.fill !== null) {
          ctx.fillStyle = item.fill;
          ctx.fillRect(item.x, item.y, item.w, item.h);
        }
        if (item.stroke !== null) {
          ctx.strokeStyle = item.stroke;
          ctx.lineWidth = item.line;
          ctx.strokeRect(item.x, item.y, item.w, item.h);
        }
        break;
      case "text":
        ctx.fillStyle = item.color;
        ctx.textAlign = item.align;
        font(ctx, item.size, item.bold);
        ctx.fillText(item.text, item.x, item.y);
        break;
      case "back":
        ctx.fillStyle = BACK_FILL;
        ctx.fillRect(item.x, item.y, item.w, item.h);
        ctx.strokeStyle = BACK_EDGE;
        ctx.lineWidth = BACK_LINE;
        ctx.strokeRect(item.x + BACK_LINE / 2, item.y + BACK_LINE / 2, item.w - BACK_LINE, item.h - BACK_LINE);
        break;
      case "portrait":
        drawPortrait(ctx, item, assets);
        break;
      case "card":
        drawCard(ctx, item, assets);
        break;
    }
  }
}

export type RasterOptions = {
  /** The file stops being worth finishing once it passes this many bytes. */
  maxBytes: number;
  /** Called with the number of frames drawn so far, after each. */
  onFrame: (frames: number) => void;
  /** Checked before each frame; true throws an `AbortError`. */
  aborted: () => boolean;
  /** Awaited after each frame, so a main-thread raster can let the page breathe. */
  between?: () => Promise<void>;
};

/** The plan drawn and encoded: the GIF's bytes, or null when they pass `maxBytes`. */
export async function rasterize(ctx: Ctx2D, plan: GifPlan, assets: GifAssets, options: RasterOptions): Promise<Uint8Array<ArrayBuffer> | null> {
  const encoder = createGifEncoder(plan.width, plan.height);
  for (const [index, frame] of plan.frames.entries()) {
    if (options.aborted()) throw new DOMException("cancelled", "AbortError");
    drawFrame(ctx, frame, assets);
    encoder.add(ctx.getImageData(0, 0, plan.width, plan.height).data, frame.delayCs);
    if (encoder.size > options.maxBytes) return null;
    options.onFrame(index + 1);
    await options.between?.();
  }
  const bytes = encoder.finish();
  return bytes.length > options.maxBytes ? null : bytes;
}
