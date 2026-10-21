// docs/polish/1-animations.md, behaviour B27: procedural sprites drawn once per (preset, colour, dpr),
// and no image asset anywhere in the effects layer.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it, vi } from "vitest";

import { PARTICLE_PRESETS } from "./presets.ts";
import { createSpriteCache, domSpriteCanvas, type SpriteCanvasFactory, type SpriteSurface } from "./sprites.ts";
import type { FxPreset } from "./types.ts";

afterEach(() => {
  vi.restoreAllMocks();
});

/**
 * A 2D context that accepts any call: gradients come back as objects with `addColorStop`, every
 * other method records itself, and properties read back what was written.
 */
function stubContext(): CanvasRenderingContext2D {
  const gradient = { addColorStop: () => {} };
  const state: Record<string | symbol, unknown> = {
    globalAlpha: 1,
    globalCompositeOperation: "source-over",
    fillStyle: "#000",
    strokeStyle: "#000",
    lineWidth: 1,
    shadowBlur: 0,
    filter: "none",
    imageSmoothingEnabled: true,
    canvas: { width: 64, height: 64 },
  };
  return new Proxy(state, {
    get(target, prop) {
      if (prop in target) return target[prop];
      if (typeof prop === "symbol") return undefined;
      return (..._args: unknown[]) => {
        if (prop === "createRadialGradient" || prop === "createLinearGradient" || prop === "createConicGradient") return gradient;
        if (prop === "getTransform") return { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
        if (prop === "measureText") return { width: 0 };
        return undefined;
      };
    },
    set(target, prop, value) {
      target[prop] = value;
      return true;
    },
  }) as unknown as CanvasRenderingContext2D;
}

/** A factory that counts its calls and hands out a distinct stub canvas each time. */
function countingFactory() {
  const made: { sizePx: number; surface: SpriteSurface }[] = [];
  const factory: SpriteCanvasFactory = vi.fn((sizePx: number) => {
    const surface: SpriteSurface = {
      canvas: { width: sizePx, height: sizePx, id: made.length } as unknown as CanvasImageSource,
      ctx: stubContext(),
    };
    made.push({ sizePx, surface });
    return surface;
  });
  return { factory, made };
}

describe("B27 the sprite cache", () => {
  it("B27 get calls the factory once per preset, colour and dpr, and returns the cached sprite afterwards", () => {
    const { factory, made } = countingFactory();
    const cache = createSpriteCache(factory);

    const first = cache.get("fire", 0, 1);
    expect(first).not.toBeNull();
    expect(cache.get("fire", 0, 1)).toBe(first);
    expect(cache.get("fire", 0, 1)).toBe(first);

    expect(factory).toHaveBeenCalledTimes(1);
    expect(first).toBe(made[0]?.surface.canvas);
    expect(cache.size()).toBe(1);
  });

  it("B27 a new preset, colour index or dpr is drawn once more, and only once", () => {
    const { factory } = countingFactory();
    const cache = createSpriteCache(factory);

    // A preset with more than one colour, so a second colour index is a real second sprite.
    const multi = (Object.keys(PARTICLE_PRESETS) as FxPreset[]).find((p) => PARTICLE_PRESETS[p].colors.length >= 2);
    const keys: (readonly [FxPreset, number, number])[] = [
      ["fire", 0, 1],
      ["fire", 0, 2],
      ["smoke", 0, 1],
      ["gold", 0, 1.5],
    ];
    if (multi !== undefined) keys.push([multi, 1, 1]);
    const sprites = keys.map(([preset, colour, dpr]) => cache.get(preset, colour, dpr));
    expect(factory).toHaveBeenCalledTimes(keys.length);
    expect(new Set(sprites).size).toBe(keys.length);

    keys.forEach(([preset, colour, dpr], index) => expect(cache.get(preset, colour, dpr)).toBe(sprites[index]));
    expect(factory).toHaveBeenCalledTimes(keys.length);
    expect(cache.size()).toBe(keys.length);
  });

  it("#1447 a supplied palette is painted and keyed separately", () => {
    // A factory whose canvases record the gradient stops each sprite was painted with.
    const painted: Array<{ canvas: CanvasImageSource; stops: string[] }> = [];
    const factory: SpriteCanvasFactory = (sizePx) => {
      const stops: string[] = [];
      const gradient = { addColorStop: (_offset: number, color: string) => void stops.push(color) };
      const ctx = {
        clearRect: () => undefined,
        createRadialGradient: () => gradient,
        beginPath: () => undefined,
        arc: () => undefined,
        fill: () => undefined,
        fillStyle: "",
      } as unknown as CanvasRenderingContext2D;
      const canvas = { width: sizePx, height: sizePx, id: painted.length } as unknown as CanvasImageSource;
      painted.push({ canvas, stops });
      return { canvas, ctx };
    };
    const cache = createSpriteCache(factory);
    const blue = ["#102030", "#112233"];
    const pink = ["#ff00aa", "#aa00ff"];
    const stopsOf = (sprite: CanvasImageSource | null) => painted.find((p) => p.canvas === sprite)?.stops ?? [];

    const own = cache.get("spark", 0, 1);
    const inBlue = cache.get("spark", 0, 1, blue);
    const inPink = cache.get("spark", 0, 1, pink);
    const secondBlue = cache.get("spark", 1, 1, blue);
    expect(painted).toHaveLength(4);
    expect(new Set([own, inBlue, inPink, secondBlue]).size).toBe(4);
    expect(cache.size()).toBe(4);

    // The glow disc's plain-colour stop (offset 0.45, no whitening) is the palette's colour, half transparent.
    expect(stopsOf(inBlue)).toContain("rgba(16, 32, 48, 0.5)");
    expect(stopsOf(secondBlue)).toContain("rgba(17, 34, 51, 0.5)");
    expect(stopsOf(inPink)).toContain("rgba(255, 0, 170, 0.5)");
    expect(PARTICLE_PRESETS.spark.colors[0]).toBe("#ffffff");
    expect(stopsOf(own)).toContain("rgba(255, 255, 255, 0.5)");
    expect(stopsOf(own)).not.toContain("rgba(16, 32, 48, 0.5)");

    // A repeat of each request reuses its own sprite, and the default palette is the one it always was.
    expect(cache.get("spark", 0, 1, blue)).toBe(inBlue);
    expect(cache.get("spark", 0, 1, [...pink])).toBe(inPink);
    expect(cache.get("spark", 0, 1)).toBe(own);
    expect(cache.get("spark", 0, 1, [])).toBe(own);
    expect(painted).toHaveLength(4);
  });

  it("B27 the factory is asked for a positive, finite size", () => {
    const { factory, made } = countingFactory();
    const cache = createSpriteCache(factory);
    cache.get("holy", 0, 1);
    cache.get("holy", 0, 2);
    for (const { sizePx } of made) {
      expect(Number.isFinite(sizePx)).toBe(true);
      expect(sizePx).toBeGreaterThan(0);
    }
  });

  it("B27 clear drops every sprite, so the next get draws again", () => {
    const { factory } = countingFactory();
    const cache = createSpriteCache(factory);
    const before = cache.get("arcane", 0, 1);
    cache.clear();
    expect(cache.size()).toBe(0);
    const after = cache.get("arcane", 0, 1);
    expect(factory).toHaveBeenCalledTimes(2);
    expect(after).not.toBe(before);
    expect(cache.size()).toBe(1);
  });

  it("B27 get returns null when the factory yields no surface", () => {
    const factory: SpriteCanvasFactory = vi.fn(() => null);
    const cache = createSpriteCache(factory);
    expect(cache.get("fire", 0, 1)).toBeNull();
    expect(cache.get("smoke", 1, 2)).toBeNull();
    expect(factory).toHaveBeenCalled();
  });

  it("B27 the DOM factory yields nothing under jsdom and never asks for a 2D context", () => {
    const getContext = vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(() => null);
    const factory = domSpriteCanvas(document);
    expect(factory(32)).toBeNull();
    const cache = createSpriteCache(domSpriteCanvas());
    expect(cache.get("fire", 0, 1)).toBeNull();
    expect(getContext).not.toHaveBeenCalled();
  });
});

/* ------------------------------------------------------------------------------------------- *
 * No image assets
 * ------------------------------------------------------------------------------------------- */

const FX_DIR = dirname(fileURLToPath(import.meta.url));

/** Every effects source file: `.ts`, `.tsx` and `.css`, tests excluded. */
function fxSources(): { name: string; text: string }[] {
  return readdirSync(FX_DIR)
    .filter((name) => /\.(ts|tsx|css)$/.test(name) && !/\.test\.tsx?$/.test(name))
    .map((name) => ({ name, text: readFileSync(join(FX_DIR, name), "utf8") }));
}

// Built from pieces so this file never matches its own patterns.
const NEW_IMAGE = new RegExp(["\\bnew", "\\s+", "Image", "\\s*\\("].join(""));
const CSS_URL = new RegExp(["url", "\\s*\\("].join(""), "i");
const IMAGE_EXT = new RegExp(["\\.", "(png|jpe?g|gif|webp|avif|svg|bmp|ico)", "\\b"].join(""), "i");

describe("B27 no image assets", () => {
  it("B27 the scan sees the effects layer's sources", () => {
    const names = fxSources().map((s) => s.name);
    for (const expected of ["sprites.ts", "particles.ts", "presets.ts", "canvasFx.ts", "fx.css"]) {
      expect(names).toContain(expected);
    }
  });

  it("B27 no file under apps/web/src/fx contains new Image, url( or an image file extension", () => {
    const offenders: string[] = [];
    for (const { name, text } of fxSources()) {
      text.split("\n").forEach((line, index) => {
        for (const [label, pattern] of [
          ["new Image", NEW_IMAGE],
          ["url(", CSS_URL],
          ["image extension", IMAGE_EXT],
        ] as const) {
          if (pattern.test(line)) offenders.push(`${name}:${index + 1} (${label}): ${line.trim()}`);
        }
      });
    }
    expect(offenders).toEqual([]);
  });
});
