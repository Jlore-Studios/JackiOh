// Exporting a clip as a GIF (#513, R513000, decisions D5 and D6): the clip's frame plan, drawn and
// encoded in a worker (or on the main thread where a worker cannot draw), tried at a smaller size
// and then a slower frame rate when it comes out over the byte cap, and refused when nothing fits.
// `ShareGif.tsx` loads this file with `import()`, so neither it, the raster nor the encoder is in the
// main bundle. Nothing here is uploaded: the result is a `File` in memory.

import { CATALOG_VERSION } from "@jackioh/cards";
import type { ReplayStep } from "@jackioh/engine";
import { DEFAULT_PORTRAIT, PORTRAIT_IDS } from "@jackioh/shared";
import type { PlayerId, PortraitId } from "@jackioh/shared";

import { compositionFor, themeFor } from "../../cards/art/themes.ts";
import { artSpec } from "../../cards/art/procedural.ts";
import type { ArtSpec } from "../../cards/art/procedural.ts";
import { motifFor } from "../../cards/art/motifs.ts";
import { isGlitch } from "../../cards/glitch.ts";
import { PORTRAIT_LOOKS } from "../../emotes/portraitLook.ts";
import { PORTRAIT_DEFS } from "../../emotes/portraits.ts";
import { withMatchDefs } from "../../game/catalog.ts";
import type { CardLookup } from "../../game/catalog.ts";
import {
  REPLAY_GIF_FALLBACK_FPS,
  REPLAY_GIF_FPS,
  REPLAY_GIF_MAX_BYTES,
  REPLAY_GIF_SIZES,
} from "../config.ts";
import type { ReplayGifPreset } from "../config.ts";
import { faceKey } from "./face.ts";
import { planFrames } from "./plan.ts";
import type { GifPlan } from "./plan.ts";
import { gifFile } from "./share.ts";
import type { GifAssets, GifFace, RasterAnswer, RasterRequest } from "./raster.ts";

export { canShareFile, shareGif } from "./share.ts";

/** One try at making the GIF: a size and a frame rate. */
export type GifAttempt = { preset: ReplayGifPreset; fps: number };

const PRESETS = Object.keys(REPLAY_GIF_SIZES) as ReplayGifPreset[];

function pixels(preset: ReplayGifPreset): number {
  return REPLAY_GIF_SIZES[preset].width * REPLAY_GIF_SIZES[preset].height;
}

/**
 * The tries, in order: the picked size at the normal rate, then every smaller size (the larger of
 * them first) at the normal rate, then the smallest at the slower rate.
 */
export function attemptsFor(preset: ReplayGifPreset): GifAttempt[] {
  const smaller = PRESETS.filter((other) => pixels(other) < pixels(preset)).sort((a, b) => pixels(b) - pixels(a));
  const smallest = smaller.at(-1) ?? preset;
  return [preset, ...smaller].map((size) => ({ preset: size, fps: REPLAY_GIF_FPS })).concat({ preset: smallest, fps: REPLAY_GIF_FALLBACK_FPS });
}

function artFor(defId: string, tags: Parameters<typeof themeFor>[0], type: Parameters<typeof compositionFor>[0], name: string, radiant: boolean): ArtSpec {
  const theme = themeFor(tags, type);
  return artSpec(defId, theme, compositionFor(type), radiant, motifFor(name, theme));
}

/**
 * What the raster needs to draw `plan`: each card a frame names with its name, type, rarity and
 * procedural art, read from the catalog and the match's own definitions of the step it is drawn
 * from (R243), and each portrait's art. A card the lookup does not know is drawn as a blank face.
 */
export function assetsFor(plan: GifPlan, clip: readonly ReplayStep[], lookup: CardLookup | null): GifAssets {
  const faces: Record<string, GifFace> = {};
  const portraits = new Set<PortraitId>();
  for (const frame of plan.frames) {
    const step = clip.find((candidate) => candidate.step === frame.step);
    const find = withMatchDefs(lookup, step?.view.defs);
    for (const item of frame.items) {
      if (item.kind === "portrait") portraits.add(item.portrait);
      if (item.kind !== "card") continue;
      const key = faceKey(item.defId, item.radiant);
      if (Object.hasOwn(faces, key)) continue;
      const info = find?.(item.defId, item.radiant);
      faces[key] =
        info === undefined
          ? { name: item.defId, type: "Unit", rarity: null, art: null }
          : {
              name: info.name,
              type: info.type,
              rarity: info.rarity ?? null,
              art: isGlitch(item.defId) ? null : artFor(item.defId, info.tags, info.type, info.name, item.radiant),
            };
    }
  }
  const looks: GifAssets["portraits"] = {};
  for (const id of PORTRAIT_IDS) {
    if (!portraits.has(id)) continue;
    const { defId, def } = PORTRAIT_DEFS[id];
    looks[id] = { art: artFor(defId, def.tags, def.type, def.name, false), rim: PORTRAIT_LOOKS[id].rim };
  }
  return { faces, portraits: looks };
}

export type RenderOptions = {
  signal: AbortSignal;
  maxBytes: number;
  onFrame: (frames: number) => void;
};

function cancelled(): DOMException {
  return new DOMException("cancelled", "AbortError");
}

/** The plan drawn in a worker: its bytes, null over the cap, or "unsupported" where a worker cannot draw. */
function viaWorker(plan: GifPlan, assets: GifAssets, options: RenderOptions): Promise<Uint8Array<ArrayBuffer> | null | "unsupported"> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./raster.worker.ts", import.meta.url), { type: "module" });
    const end = (settle: () => void): void => {
      options.signal.removeEventListener("abort", onAbort);
      worker.terminate();
      settle();
    };
    function onAbort(): void {
      end(() => {
        reject(cancelled());
      });
    }
    options.signal.addEventListener("abort", onAbort, { once: true });
    worker.onmessage = (event: MessageEvent<RasterAnswer>) => {
      const answer = event.data;
      if (answer.type === "frame") options.onFrame(answer.frame);
      else if (answer.type === "done") end(() => { resolve(answer.bytes); });
      else if (answer.type === "over") end(() => { resolve(null); });
      else if (answer.type === "unsupported") end(() => { resolve("unsupported"); });
      else end(() => { reject(new Error(answer.message)); });
    };
    worker.onerror = () => {
      end(() => {
        resolve("unsupported");
      });
    };
    worker.postMessage({ plan, assets, maxBytes: options.maxBytes } satisfies RasterRequest);
  });
}

/** The plan drawn on the main thread, a frame to a macrotask so the page stays responsive. */
async function viaThread(plan: GifPlan, assets: GifAssets, options: RenderOptions): Promise<Uint8Array<ArrayBuffer> | null> {
  const { canvasContext, rasterize } = await import("./raster.ts");
  const ctx = canvasContext(plan.width, plan.height);
  if (ctx === null) throw new Error("this browser cannot draw the GIF");
  return rasterize(ctx, plan, assets, {
    maxBytes: options.maxBytes,
    onFrame: options.onFrame,
    aborted: () => options.signal.aborted,
    between: () =>
      new Promise<void>((resolve) => {
        setTimeout(resolve, 0);
      }),
  });
}

/**
 * The GIF's bytes, or null when they pass `options.maxBytes`. In a worker where there is one that
 * can draw (`via` "worker" insists on it and throws where there is none), else on the main thread
 * (`via` "thread"). A cancelled signal rejects with an `AbortError`.
 */
export async function renderGif(
  plan: GifPlan,
  assets: GifAssets,
  options: RenderOptions,
  via?: "worker" | "thread",
): Promise<Uint8Array<ArrayBuffer> | null> {
  if (options.signal.aborted) throw cancelled();
  if (via !== "thread") {
    if (typeof Worker === "function" && typeof OffscreenCanvas === "function") {
      const drawn = await viaWorker(plan, assets, options);
      if (drawn !== "unsupported") return drawn;
    }
    if (via === "worker") throw new Error("this browser cannot draw a GIF in a worker");
  }
  return viaThread(plan, assets, options);
}

export type GifExportInput = {
  /** Consecutive steps, at most `REPLAY_GIF_MAX_STEPS` of them. */
  clip: readonly ReplayStep[];
  /** The size to try first. */
  preset: ReplayGifPreset;
  lookup: CardLookup | null;
  /** Each seat's portrait; the default one, as the player draws them, when none is given. */
  portraits?: Readonly<Record<PlayerId, PortraitId>>;
};

export type GifProgress = { frame: number; frames: number; attempt: number; attempts: number };

export type GifExportResult =
  | { kind: "ready"; file: File; attempt: GifAttempt }
  | { kind: "tooBig" }
  | { kind: "cancelled" };

/** The `render` an export draws with: `renderGif`, or a stand-in for a test. */
export type GifRender = (plan: GifPlan, assets: GifAssets, options: RenderOptions) => Promise<Uint8Array<ArrayBuffer> | null>;

function isAbort(cause: unknown): boolean {
  return cause instanceof DOMException && cause.name === "AbortError";
}

/** The clip as a GIF file, the first try that fits the byte cap; `tooBig` when none does. */
export async function exportGif(
  input: GifExportInput,
  hooks: { signal: AbortSignal; onProgress: (progress: GifProgress) => void },
  render: GifRender = renderGif,
): Promise<GifExportResult> {
  const first = input.clip[0];
  const last = input.clip.at(-1);
  if (first === undefined || last === undefined) throw new RangeError("a GIF needs at least one step");
  const portraits = input.portraits ?? { p1: DEFAULT_PORTRAIT, p2: DEFAULT_PORTRAIT };
  const attempts = attemptsFor(input.preset);
  for (const [index, attempt] of attempts.entries()) {
    if (hooks.signal.aborted) return { kind: "cancelled" };
    const size = REPLAY_GIF_SIZES[attempt.preset];
    const plan = planFrames(input.clip, { ...size, fps: attempt.fps, catalogVersion: CATALOG_VERSION, portraits });
    const assets = assetsFor(plan, input.clip, input.lookup);
    try {
      const bytes = await render(plan, assets, {
        signal: hooks.signal,
        maxBytes: REPLAY_GIF_MAX_BYTES,
        onFrame: (frame) => {
          hooks.onProgress({ frame, frames: plan.frames.length, attempt: index + 1, attempts: attempts.length });
        },
      });
      if (bytes !== null) {
        return { kind: "ready", file: gifFile(bytes, `jackioh-replay-${String(first.step + 1)}-${String(last.step + 1)}.gif`), attempt };
      }
    } catch (cause) {
      if (isAbort(cause) || hooks.signal.aborted) return { kind: "cancelled" };
      throw cause;
    }
  }
  return { kind: "tooBig" };
}
