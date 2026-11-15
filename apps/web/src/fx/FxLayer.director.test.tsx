// S9, B34: what the effects layer builds its director with: the seed from its seam, else FX_DEFAULT_SEED, and
// a particle capacity by the viewport's width. `createFxDirector` is the real one, with its options kept.

import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createAnimationQueue } from "../game/animations.ts";
import { fullBoardView } from "../test/fixtures.ts";
import { FX_DEFAULT_SEED, FX_MOBILE_WIDTH, FX_PARTICLE_CAP, FX_PARTICLE_CAP_MOBILE } from "./constants.ts";
import type { FxDirectorOptions } from "./director.ts";
import FxLayer, { type FxSeams } from "./FxLayer.tsx";
import { resetFxSettingsForTests } from "./settings.ts";

const built = vi.hoisted(() => [] as FxDirectorOptions[]);

vi.mock("./director.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./director.ts")>();
  return {
    ...actual,
    createFxDirector: (options: FxDirectorOptions) => {
      built.push(options);
      return actual.createFxDirector(options);
    },
  };
});

beforeEach(() => {
  localStorage.clear();
  resetFxSettingsForTests();
  built.length = 0;
});

afterEach(() => {
  cleanup();
  resetFxSettingsForTests();
  localStorage.clear();
});

const QUIET: Partial<FxSeams> = {
  now: () => 0,
  frames: { request: () => 0, cancel: () => undefined },
  visibility: { hidden: () => false, subscribe: () => () => undefined },
  measure: () => null,
  surface: () => null,
  shakeSink: { apply: () => undefined, clear: () => undefined },
  catalog: () => undefined,
  element: () => null,
};

/** The options of the director a layer with `seams` builds at its mount. */
function director(seams: Partial<FxSeams>): FxDirectorOptions {
  act(() => {
    render(
      <div className="game">
        <FxLayer queue={createAnimationQueue({ schedule: () => undefined, reducedMotion: false })} view={fullBoardView()} seams={seams} />
      </div>,
    );
  });
  expect(built).toHaveLength(1);
  const options = built[0];
  if (options === undefined) throw new Error("no director built");
  cleanup();
  built.length = 0;
  return options;
}

describe("S9 B34 — the director the layer builds", () => {
  it("S9 its seed is the seam's, else FX_DEFAULT_SEED", () => {
    expect(director({ ...QUIET, seed: 11, viewportWidth: () => 1280 }).seed).toBe(11);
    expect(director({ ...QUIET, viewportWidth: () => 1280 }).seed).toBe(FX_DEFAULT_SEED);
  });

  it("B34 a viewport narrower than FX_MOBILE_WIDTH caps it at FX_PARTICLE_CAP_MOBILE, from that width up at FX_PARTICLE_CAP", () => {
    expect(director({ ...QUIET, viewportWidth: () => FX_MOBILE_WIDTH - 1 }).capacity).toBe(FX_PARTICLE_CAP_MOBILE);
    expect(director({ ...QUIET, viewportWidth: () => FX_MOBILE_WIDTH }).capacity).toBe(FX_PARTICLE_CAP);
  });
});
