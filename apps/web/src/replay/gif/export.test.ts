// Exporting a clip as a GIF (#513, R513000): the tries a clip over the byte cap goes through, what
// the raster is given to draw, cancellation, and that the export, the raster and the encoder stay out
// of the main bundle.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG } from "@jackioh/cards";
import type { ReplayStep } from "@jackioh/engine";
import { PORTRAIT_IDS } from "@jackioh/shared";
import ts from "typescript";
import { describe, expect, it } from "vitest";

import { lookupFromDefs } from "../../game/catalog.ts";
import { baseView, fullBoardView } from "../../test/fixtures.ts";
import { foldedReplaySteps } from "../../test/replay.ts";
import { REPLAY_GIF_FALLBACK_FPS, REPLAY_GIF_FPS, REPLAY_GIF_MAX_BYTES, REPLAY_GIF_SIZES } from "../config.ts";
import { assetsFor, attemptsFor, exportGif, renderGif } from "./export.ts";
import type { GifProgress, GifRender } from "./export.ts";
import { faceKey, planFrames } from "./plan.ts";
import type { GifPlan } from "./plan.ts";

const lookup = lookupFromDefs(CATALOG);
const clip: ReplayStep[] = foldedReplaySteps().slice(8, 14);
const FIRST = clip[0]?.step ?? 0;
const LAST = clip.at(-1)?.step ?? 0;
const GIF = Uint8Array.from([0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 1, 2, 3]);

/** What a render was asked: the size and rate of each plan, in order. */
type Asked = { width: number; height: number; fps: number; maxBytes: number };

function recorder(answers: (Uint8Array<ArrayBuffer> | null)[]): { render: GifRender; asked: Asked[] } {
  const asked: Asked[] = [];
  const render: GifRender = (plan, _assets, options) => {
    asked.push({ width: plan.width, height: plan.height, fps: plan.fps, maxBytes: options.maxBytes });
    options.onFrame(1);
    return Promise.resolve(answers[asked.length - 1] ?? null);
  };
  return { render, asked };
}

const hooks = (): { signal: AbortSignal; onProgress: (progress: GifProgress) => void } => ({ signal: new AbortController().signal, onProgress: () => undefined });

describe("the GIF export's tries", () => {
  it("attemptsFor goes to the smaller preset, then fewer frames per second", () => {
    expect(attemptsFor("landscape")).toEqual([
      { preset: "landscape", fps: REPLAY_GIF_FPS },
      { preset: "square", fps: REPLAY_GIF_FPS },
      { preset: "square", fps: REPLAY_GIF_FALLBACK_FPS },
    ]);
    expect(attemptsFor("square")).toEqual([
      { preset: "square", fps: REPLAY_GIF_FPS },
      { preset: "square", fps: REPLAY_GIF_FALLBACK_FPS },
    ]);
  });

  it("tries the picked size, then the smaller one, then fewer frames per second, then refuses", async () => {
    const { render, asked } = recorder([]);
    const result = await exportGif({ clip, preset: "landscape", lookup }, hooks(), render);
    expect(result).toEqual({ kind: "tooBig" });
    const { landscape, square } = REPLAY_GIF_SIZES;
    expect(asked).toEqual([
      { ...landscape, fps: REPLAY_GIF_FPS, maxBytes: REPLAY_GIF_MAX_BYTES },
      { ...square, fps: REPLAY_GIF_FPS, maxBytes: REPLAY_GIF_MAX_BYTES },
      { ...square, fps: REPLAY_GIF_FALLBACK_FPS, maxBytes: REPLAY_GIF_MAX_BYTES },
    ]);
  });

  it("stops at the first attempt under the byte cap", async () => {
    const { render, asked } = recorder([null, Uint8Array.from(GIF)]);
    const seen: GifProgress[] = [];
    const result = await exportGif({ clip, preset: "landscape", lookup }, { ...hooks(), onProgress: (p) => seen.push(p) }, render);
    expect(asked).toHaveLength(2);
    if (result.kind !== "ready") throw new Error(`expected a GIF, got ${result.kind}`);
    expect(result.attempt).toEqual({ preset: "square", fps: REPLAY_GIF_FPS });
    expect(result.file.type).toBe("image/gif");
    expect(result.file.name).toBe(`jackioh-replay-${String(FIRST + 1)}-${String(LAST + 1)}.gif`);
    expect(result.file.size).toBe(GIF.length);
    // Progress names the try and the plan's frames.
    const plan = planFrames(clip, { ...REPLAY_GIF_SIZES.square, fps: REPLAY_GIF_FPS, catalogVersion: "", portraits: { p1: "vanilla", p2: "vanilla" } });
    expect(seen).toEqual([
      { frame: 1, frames: planFrames(clip, { ...REPLAY_GIF_SIZES.landscape, fps: REPLAY_GIF_FPS, catalogVersion: "", portraits: { p1: "vanilla", p2: "vanilla" } }).frames.length, attempt: 1, attempts: 3 },
      { frame: 1, frames: plan.frames.length, attempt: 2, attempts: 3 },
    ]);
  });

  it("an export can be cancelled", async () => {
    const controller = new AbortController();
    let started = false;
    const waiting: GifRender = (_plan, _assets, options) => {
      started = true;
      return new Promise((_resolve, reject) => {
        options.signal.addEventListener("abort", () => {
          reject(new DOMException("cancelled", "AbortError"));
        });
      });
    };
    const running = exportGif({ clip, preset: "square", lookup }, { signal: controller.signal, onProgress: () => undefined }, waiting);
    await Promise.resolve();
    expect(started).toBe(true);
    controller.abort();
    expect(await running).toEqual({ kind: "cancelled" });

    // Already cancelled: nothing is drawn.
    const { render, asked } = recorder([]);
    expect(await exportGif({ clip, preset: "square", lookup }, { signal: controller.signal, onProgress: () => undefined }, render)).toEqual({ kind: "cancelled" });
    expect(asked).toEqual([]);

    // A failure that is not a cancellation is the caller's to report.
    const failing: GifRender = () => Promise.reject(new Error("the raster fell over"));
    await expect(exportGif({ clip, preset: "square", lookup }, hooks(), failing)).rejects.toThrow("the raster fell over");
  });

  it("refuses an empty clip", async () => {
    await expect(exportGif({ clip: [], preset: "square", lookup }, hooks(), recorder([]).render)).rejects.toThrow(RangeError);
  });

  it("renderGif refuses a worker where there is none, and an aborted signal", async () => {
    const plan: GifPlan = { width: 2, height: 2, fps: 10, frames: [] };
    const assets = { faces: {}, portraits: {} };
    const options = { signal: new AbortController().signal, maxBytes: 100, onFrame: () => undefined };
    // jsdom has neither a Worker nor an OffscreenCanvas.
    await expect(renderGif(plan, assets, options, "worker")).rejects.toThrow("worker");
    const stopped = new AbortController();
    stopped.abort();
    await expect(renderGif(plan, assets, { ...options, signal: stopped.signal }, "thread")).rejects.toMatchObject({ name: "AbortError" });
  });
});

describe("what the raster is given", () => {
  it("assetsFor gives every card item a face and every portrait its art", () => {
    const plan = planFrames(clip, { ...REPLAY_GIF_SIZES.landscape, fps: REPLAY_GIF_FPS, catalogVersion: "v", portraits: { p1: "vanilla", p2: "gary" } });
    const assets = assetsFor(plan, clip, lookup);
    const keys = new Set<string>();
    for (const item of plan.frames.flatMap((frame) => frame.items)) {
      if (item.kind === "card") keys.add(faceKey(item.defId, item.radiant));
    }
    expect(keys.size).toBeGreaterThan(0);
    expect(Object.keys(assets.faces).sort()).toEqual([...keys].sort());
    for (const key of keys) {
      const face = assets.faces[key];
      expect(face?.art, `${key} has its art`).not.toBeNull();
      expect(face?.name).not.toBe("");
    }
    expect(Object.keys(assets.portraits).sort()).toEqual(["gary", "vanilla"]);
    for (const id of PORTRAIT_IDS) {
      const look = assets.portraits[id];
      if (look !== undefined) expect(look.rim).toMatch(/^#[0-9a-f]{6}$/iu);
    }
    // Plain data, so it crosses to a worker.
    expect(structuredClone(assets)).toEqual(assets);
  });

  it("assetsFor reads a Radiant face as its own and a card the lookup lacks as a blank one", () => {
    const view = fullBoardView();
    const one: ReplayStep[] = [{ step: 0, turn: view.turn, view }];
    const plan = planFrames(one, { ...REPLAY_GIF_SIZES.square, fps: REPLAY_GIF_FPS, catalogVersion: "v", portraits: { p1: "vanilla", p2: "vanilla" } });
    const known = assetsFor(plan, one, lookup);
    expect(known.faces[faceKey("core-011", true)]?.art?.variant).toBe("radiant");
    expect(known.faces["core-004"]?.art?.variant).toBe("base");
    const blank = assetsFor(plan, one, null);
    expect(blank.faces["core-004"]).toEqual({ name: "core-004", type: "Unit", rarity: null, art: null });
  });

  it("assetsFor reads a match-made card from its step's own definitions (R243)", () => {
    const view = fullBoardView();
    const base = baseView();
    const unit = view.you.units[0];
    const def = CATALOG["core-004"];
    if (unit === null || unit === undefined || def === undefined) throw new Error("the fixture changed");
    base.you.units[0] = { ...unit, defId: "t-77" };
    base.defs = { "t-77": { ...def, name: "Made In The Match" } };
    const one: ReplayStep[] = [{ step: 0, turn: 1, view: base }];
    const plan = planFrames(one, { ...REPLAY_GIF_SIZES.square, fps: REPLAY_GIF_FPS, catalogVersion: "v", portraits: { p1: "vanilla", p2: "vanilla" } });
    expect(assetsFor(plan, one, lookup).faces["t-77"]?.name).toBe("Made In The Match");
  });
});

describe("a GIF's code loads on demand", () => {
  const SRC = join(dirname(fileURLToPath(import.meta.url)), "../..");

  function sources(dir: string): string[] {
    return readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return name === "test" ? [] : sources(path);
      return /\.(ts|tsx)$/u.test(name) && !/\.test\.tsx?$/u.test(name) ? [path] : [];
    });
  }

  /** The modules a file loads at once, and those it loads with `import()`; a type-only import loads nothing. */
  function importsOf(path: string): { statics: string[]; dynamics: string[] } {
    const text = readFileSync(path, "utf8");
    const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
    const statics: string[] = [];
    const dynamics: string[] = [];
    const visit = (node: ts.Node): void => {
      if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier) && node.importClause?.isTypeOnly !== true) {
        const names = node.importClause?.namedBindings;
        const onlyTypes = names !== undefined && ts.isNamedImports(names) && node.importClause?.name === undefined && names.elements.every((element) => element.isTypeOnly);
        if (!onlyTypes) statics.push(node.moduleSpecifier.text);
      }
      if (ts.isExportDeclaration(node) && node.moduleSpecifier !== undefined && ts.isStringLiteral(node.moduleSpecifier) && !node.isTypeOnly) {
        statics.push(node.moduleSpecifier.text);
      }
      if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
        const [argument] = node.arguments;
        if (argument !== undefined && ts.isStringLiteralLike(argument)) dynamics.push(argument.text);
      }
      ts.forEachChild(node, visit);
    };
    visit(file);
    return { statics, dynamics };
  }

  it("the encoder and the raster load on demand", () => {
    const files = sources(SRC);
    expect(files.length).toBeGreaterThan(100);
    const outside = files.map((path) => relative(SRC, path)).filter((name) => !name.startsWith("replay/gif/"));
    const importers = outside.filter((name) => importsOf(join(SRC, name)).statics.some((specifier) => /(^|\/)gif\//u.test(specifier)));
    expect(importers, "nothing outside replay/gif/ loads it at once").toEqual([]);

    const player = importsOf(join(SRC, "replay/ShareGif.tsx"));
    expect(player.dynamics).toEqual(["./gif/export.ts"]);
    expect(readFileSync(join(SRC, "replay/ShareGif.tsx"), "utf8")).toContain('import("./gif/export.ts")');

    // The export reaches the raster by `import()` and a worker, never at once; the worker is the raster's one importer.
    const exporter = importsOf(join(SRC, "replay/gif/export.ts"));
    expect(exporter.statics.filter((specifier) => /raster\.ts$|encoder\.ts$/u.test(specifier))).toEqual([]);
    expect(exporter.dynamics).toContain("./raster.ts");
    expect(readFileSync(join(SRC, "replay/gif/export.ts"), "utf8")).toContain('new Worker(new URL("./raster.worker.ts", import.meta.url), { type: "module" })');
    expect(importsOf(join(SRC, "replay/gif/raster.worker.ts")).statics).toEqual(["./raster.ts"]);
  });
});
