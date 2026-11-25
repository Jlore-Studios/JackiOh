// Sound-effect recipes loader (docs/polish/2-sound.md; #4428).
//
// The recipes (`sfx.ts`, about 45 KB gzipped) load lazily on first sound, first unlock or route
// intent, keeping them off the entry bundle. Unlike the tables, the recipes need no catalog, so
// this loader never waits on the WebAssembly module: a sound asked for before they land is skipped
// (`engine.ts` returns false and kicks the load), never queued.

/** The recipes module, by dynamic import: `SFX`, `renderSfx`, `SFX_IDS`, `SFX_TIMBRES`. */
export type SfxRecipes = typeof import("./sfx.ts");

let cachedSfx: SfxRecipes | null = null;
let pendingLoad: Promise<SfxRecipes> | null = null;
const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of [...listeners]) {
    try {
      listener();
    } catch {
      // One listener's failure must not stop others.
    }
  }
}

export function sfxNow(): SfxRecipes | null {
  return cachedSfx;
}

export function loadSfx(): Promise<SfxRecipes> {
  if (cachedSfx !== null) {
    return Promise.resolve(cachedSfx);
  }
  if (pendingLoad !== null) {
    return pendingLoad;
  }
  pendingLoad = import("./sfx.ts")
    .then((mod) => {
      cachedSfx = mod;
      pendingLoad = null;
      notify();
      return cachedSfx;
    })
    .catch((err) => {
      pendingLoad = null;
      throw err;
    });
  return pendingLoad;
}

export function onSfxLoaded(callback: (recipes: SfxRecipes) => void): () => void {
  if (cachedSfx !== null) {
    try {
      callback(cachedSfx);
    } catch {
      // Ignored
    }
  }
  const listener = () => {
    if (cachedSfx !== null) {
      try {
        callback(cachedSfx);
      } catch {
        // Ignored
      }
    }
  };
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function setSfxForTests(recipes: SfxRecipes | null): void {
  cachedSfx = recipes;
  pendingLoad = recipes !== null ? Promise.resolve(recipes) : null;
  notify();
}

export type DeferredSfx = {
  resolve(recipes: SfxRecipes): void;
  reject(err: unknown): void;
};

export function deferSfxForTests(): DeferredSfx {
  cachedSfx = null;
  let res!: (recipes: SfxRecipes) => void;
  let rej!: (err: unknown) => void;
  pendingLoad = new Promise<SfxRecipes>((resolve, reject) => {
    res = resolve;
    rej = reject;
  });
  return {
    resolve(recipes: SfxRecipes) {
      cachedSfx = recipes;
      notify();
      res(recipes);
    },
    reject(err: unknown) {
      pendingLoad = null;
      rej(err);
    },
  };
}
