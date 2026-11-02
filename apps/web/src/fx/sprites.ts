/**
 * Procedural particle sprites (docs/polish/1-animations.md, S5). Each sprite is a radial-gradient disc
 * painted once into a small offscreen canvas per (preset, color, dpr) and reused for every particle
 * after that, the cheap way to draw soft glowing dots (MDN, "Optimizing canvas"). No image assets.
 * Paper, leaf, circuit, crumb and star are flat shapes drawn with paths (#2792).
 */
import { PARTICLE_PRESETS } from "./presets.ts";
import { isHeadlessDom } from "./surface.ts";
import type { FxPreset } from "./types.ts";

export type SpriteSurface = { canvas: CanvasImageSource; ctx: CanvasRenderingContext2D };
export type SpriteCanvasFactory = (sizePx: number) => SpriteSurface | null;

export type SpriteCache = {
  /** Null without a 2D context. `colors` (#1447) stands in for the preset's palette, and the color picked is part of the cache key. */
  get(preset: FxPreset, colorIndex: number, dpr: number, colors?: readonly string[]): CanvasImageSource | null;
  size(): number;
  clear(): void;
};

/** CSS px diameter a sprite is painted at; particles scale it to their own size when drawn. */
const SPRITE_BASE_PX = 48;

type DiscStyle = "glow" | "soft" | "hard";
type ShapeStyle = "leaf" | "pixel" | "crumb" | "star";
type SpriteStyle = DiscStyle | "paper" | ShapeStyle;

/**
 * How each preset's disc falls off: glow has a hot white core, soft is a faint puff, hard a solid dot.
 * Paper is a flat rectangle with a lit edge, which the tumble squash turns into a spinning scrap.
 */
const SPRITE_STYLE: { readonly [P in FxPreset]: SpriteStyle } = {
  fire: "glow",
  ember: "hard",
  holy: "glow",
  sparkle: "glow",
  arcane: "glow",
  poison: "soft",
  smoke: "soft",
  dust: "soft",
  shard: "hard",
  spark: "glow",
  gold: "glow",
  prismatic: "glow",
  void: "soft",
  confetti: "paper",
  blood: "hard",
  frost: "hard",
  leaf: "leaf",
  circuit: "pixel",
  crumb: "crumb",
  star: "star",
  exhaust: "glow",
};

/** The paper scrap, as fractions of the sprite: its width, its height, and the lit band along its top. */
const PAPER = { width: 0.86, height: 0.5, lit: 0.35, litWhiten: 0.45 } as const;

/** Twice pi, the full turn a star's corners are spaced over (#2792). */
const TAU = Math.PI * 2;

/** The leaf's body: its half-length, its bulge, and its pale midrib (#2792). */
const LEAF = { length: 0.9, width: 0.44, rib: 0.07, ribLength: 0.7, ribWhiten: 0.5 } as const;

/** The circuit pixel: its side, and the lit square in its top-left corner (#2792). */
const PIXEL = { side: 0.64, lit: 0.34, litWhiten: 0.6 } as const;

/** The crumb's jagged outline: each corner's reach, and the lit inner copy (#2792). */
const CRUMB = { corners: [0.92, 0.64, 0.86, 0.56, 0.9, 0.7], lit: 0.5, litWhiten: 0.35 } as const;

/** The star's points: how many, its outer and inner reach, its turn, and its whitening (#2792). */
const STAR = { points: 5, outer: 0.95, inner: 0.42, turn: -Math.PI / 2, whiten: 0.3 } as const;

/** Gradient stops per disc style: [offset along the radius, alpha, how far the color is mixed toward white]. */
const GRADIENT_STOPS: { readonly [S in DiscStyle]: ReadonlyArray<readonly [number, number, number]> } = {
  glow: [
    [0, 1, 0.65],
    [0.2, 1, 0.2],
    [0.45, 0.5, 0],
    [1, 0, 0],
  ],
  soft: [
    [0, 0.72, 0.12],
    [0.4, 0.46, 0],
    [0.75, 0.16, 0],
    [1, 0, 0],
  ],
  hard: [
    [0, 1, 0.25],
    [0.55, 0.92, 0],
    [0.8, 0.35, 0],
    [1, 0, 0],
  ],
};

const HEX_RADIX = 16;
const CHANNEL_MAX = 255;
const WHITE: readonly [number, number, number] = [CHANNEL_MAX, CHANNEL_MAX, CHANNEL_MAX];

function parseHex(color: string): readonly [number, number, number] {
  const m = /^#([0-9a-f]{6})$/i.exec(color.trim());
  if (!m) return WHITE;
  const n = Number.parseInt(m[1]!, HEX_RADIX);
  return [(n >> 16) & CHANNEL_MAX, (n >> 8) & CHANNEL_MAX, n & CHANNEL_MAX];
}

function rgba(rgb: readonly [number, number, number], alpha: number, whiten: number): string {
  const mix = (c: number) => Math.round(c + (CHANNEL_MAX - c) * whiten);
  return `rgba(${mix(rgb[0])}, ${mix(rgb[1])}, ${mix(rgb[2])}, ${alpha})`;
}

/** Real document canvases; returns a factory that yields null under jsdom (isHeadlessDom). */
export function domSpriteCanvas(doc?: Document): SpriteCanvasFactory {
  return (sizePx) => {
    const d = doc ?? document;
    const view = d.defaultView ?? undefined;
    if (isHeadlessDom(view)) return null;
    const canvas = d.createElement("canvas");
    canvas.width = sizePx;
    canvas.height = sizePx;
    const ctx = canvas.getContext("2d");
    return ctx ? { canvas, ctx } : null;
  };
}

/** A flat polygon out of `reach`'s corners around `(r, r)`, first `turn` round (#2792). */
function fillPolygon(ctx: CanvasRenderingContext2D, r: number, reach: readonly number[], turn: number): void {
  ctx.beginPath();
  reach.forEach((k, i) => {
    const angle = turn + (i / reach.length) * TAU;
    const x = r + r * k * Math.cos(angle);
    const y = r + r * k * Math.sin(angle);
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.closePath();
  ctx.fill();
}

/** A flat shape in `rgb`'s color: a leaf, a circuit pixel, a crumb or a star (#2792). */
function paintShape(ctx: CanvasRenderingContext2D, style: ShapeStyle, rgb: readonly [number, number, number], sizePx: number): void {
  const r = sizePx / 2;
  switch (style) {
    case "leaf": {
      const half = (sizePx * LEAF.length) / 2;
      const bulge = sizePx * LEAF.width;
      ctx.fillStyle = rgba(rgb, 1, 0);
      ctx.beginPath();
      ctx.moveTo(r - half, r);
      ctx.quadraticCurveTo(r, r - bulge, r + half, r);
      ctx.quadraticCurveTo(r, r + bulge, r - half, r);
      ctx.closePath();
      ctx.fill();
      ctx.fillStyle = rgba(rgb, 1, LEAF.ribWhiten);
      ctx.fillRect(r - half * LEAF.ribLength, r - (sizePx * LEAF.rib) / 2, 2 * half * LEAF.ribLength, sizePx * LEAF.rib);
      return;
    }
    case "pixel": {
      const side = sizePx * PIXEL.side;
      const x = (sizePx - side) / 2;
      const y = (sizePx - side) / 2;
      ctx.fillStyle = rgba(rgb, 1, 0);
      ctx.fillRect(x, y, side, side);
      ctx.fillStyle = rgba(rgb, 1, PIXEL.litWhiten);
      ctx.fillRect(x, y, side * PIXEL.lit, side * PIXEL.lit);
      return;
    }
    case "crumb": {
      ctx.fillStyle = rgba(rgb, 1, 0);
      fillPolygon(ctx, r, CRUMB.corners, 0);
      ctx.fillStyle = rgba(rgb, 1, CRUMB.litWhiten);
      fillPolygon(
        ctx,
        r,
        CRUMB.corners.map((k) => k * CRUMB.lit),
        0,
      );
      return;
    }
    case "star": {
      ctx.fillStyle = rgba(rgb, 1, STAR.whiten);
      fillPolygon(
        ctx,
        r,
        Array.from({ length: STAR.points * 2 }, (_, i) => (i % 2 === 0 ? STAR.outer : STAR.inner)),
        STAR.turn,
      );
      return;
    }
  }
}

function paintSprite(ctx: CanvasRenderingContext2D, preset: FxPreset, color: string, sizePx: number): void {
  const r = sizePx / 2;
  const rgb = parseHex(color);
  const style = SPRITE_STYLE[preset];
  ctx.clearRect(0, 0, sizePx, sizePx);
  if (style === "paper") {
    const w = sizePx * PAPER.width;
    const h = sizePx * PAPER.height;
    const x = (sizePx - w) / 2;
    const y = (sizePx - h) / 2;
    ctx.fillStyle = rgba(rgb, 1, 0);
    ctx.fillRect(x, y, w, h);
    ctx.fillStyle = rgba(rgb, 1, PAPER.litWhiten);
    ctx.fillRect(x, y, w, h * PAPER.lit);
    return;
  }
  if (style === "leaf" || style === "pixel" || style === "crumb" || style === "star") {
    paintShape(ctx, style, rgb, sizePx);
    return;
  }
  const gradient = ctx.createRadialGradient(r, r, 0, r, r, r);
  for (const [offset, alpha, whiten] of GRADIENT_STOPS[style]) {
    gradient.addColorStop(offset, rgba(rgb, alpha, whiten));
  }
  ctx.fillStyle = gradient;
  ctx.beginPath();
  ctx.arc(r, r, r, 0, Math.PI * 2);
  ctx.fill();
}

export function createSpriteCache(factory: SpriteCanvasFactory): SpriteCache {
  const sprites = new Map<string, CanvasImageSource>();

  return {
    get(preset, colorIndex, dpr, palette) {
      const colors = palette !== undefined && palette.length > 0 ? palette : PARTICLE_PRESETS[preset].colors;
      const count = colors.length;
      const index = ((Math.floor(colorIndex) % count) + count) % count;
      const key = `${preset}|${index}|${dpr}|${colors[index]}`;
      const hit = sprites.get(key);
      if (hit) return hit;

      const sizePx = Math.ceil(SPRITE_BASE_PX * dpr);
      const surface = factory(sizePx);
      if (!surface) return null;
      paintSprite(surface.ctx, preset, colors[index] ?? "#ffffff", sizePx);
      sprites.set(key, surface.canvas);
      return surface.canvas;
    },
    size: () => sprites.size,
    clear: () => sprites.clear(),
  };
}
