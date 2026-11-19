// Scene data loader (#4222).
//
// The scene kits (`sceneKit.ts`, `figureKit.ts`) and both recipe tables (`sceneRecipes.ts`) load
// lazily beside the engine module, keeping ~150 KB gzipped of recipes and the kits out of the first
// load: `main.tsx` starts this load next to `loadWasm()`, and `EngineGate` and `useEngineReady()` wait
// for both before a face is drawn. `scene.ts` keeps its API and reads the tables from the slot this
// fills; before it does, `sceneRecipeOf` throws, so a face drawn too early fails a test instead of
// caching the painter's picture. A failed download is not cached; the next call tries again.

import { clearSceneDataForTests, installSceneData, sceneDataReady, type SceneTables } from "./scene.ts";

export { sceneDataReady };

let pendingLoad: Promise<void> | null = null;

/** Fill the tables a scene is drawn from once, from the one data module. Idempotent. */
export function loadSceneData(): Promise<void> {
  if (sceneDataReady()) return Promise.resolve();
  if (pendingLoad !== null) return pendingLoad;
  pendingLoad = import("./sceneData.ts")
    .then((mod) => {
      const tables: SceneTables = {
        SCENE_KIT: mod.SCENE_KIT,
        drawFigure: mod.drawFigure,
        SCENE_RECIPES: mod.SCENE_RECIPES,
        ALT_SCENE_RECIPES: mod.ALT_SCENE_RECIPES,
      };
      installSceneData(tables);
      pendingLoad = null;
    })
    .catch((cause: unknown) => {
      pendingLoad = null;
      throw cause;
    });
  return pendingLoad;
}

/** Empty the slot, back to before `loadSceneData()`. Tests only. */
export function resetSceneDataForTests(): void {
  pendingLoad = null;
  clearSceneDataForTests();
}

/** A held scene-data load, settled by the test. */
export type DeferredSceneData = {
  /** Install the real tables and let the held load resolve. */
  resolve(): Promise<void>;
  reject(err: unknown): void;
};

/** Hold `loadSceneData()` back until the test settles it. Tests only. */
export function deferSceneDataForTests(): DeferredSceneData {
  pendingLoad = null;
  clearSceneDataForTests();
  let res!: () => void;
  let rej!: (err: unknown) => void;
  pendingLoad = new Promise<void>((resolve, reject) => {
    res = resolve;
    rej = reject;
  });
  return {
    async resolve(): Promise<void> {
      const mod = await import("./sceneData.ts");
      installSceneData({
        SCENE_KIT: mod.SCENE_KIT,
        drawFigure: mod.drawFigure,
        SCENE_RECIPES: mod.SCENE_RECIPES,
        ALT_SCENE_RECIPES: mod.ALT_SCENE_RECIPES,
      });
      pendingLoad = null;
      res();
    },
    reject(err: unknown) {
      pendingLoad = null;
      rej(err);
    },
  };
}
