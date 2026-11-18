// Sound tables loader (docs/polish/2-sound.md; #4130).
//
// The sound tables (`card-audio.json5`, `voice-manifest.json`, `voice-text.json`, `music-manifest.json`,
// `music-cards.json`) and the JSON5 parser load lazily on first sound, first unlock or route intent,
// keeping ~100 KB gzipped off the entry bundle. #4180: the tables parse against the module's catalog,
// so the loader awaits it before the chunk.

import { useSyncExternalStore } from "react";

import { loadWasm } from "../wasm/index.ts";
import type { CardAudioTable, MusicCardEntry, MusicManifest, VoiceManifest } from "./types.ts";

export type SoundTables = {
  readonly cardAudio: CardAudioTable;
  readonly voiceManifest: VoiceManifest;
  readonly musicManifest: MusicManifest;
  readonly musicCards: Readonly<Record<string, MusicCardEntry>>;
};

let cachedTables: SoundTables | null = null;
let pendingLoad: Promise<SoundTables> | null = null;
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

export function soundTablesNow(): SoundTables | null {
  return cachedTables;
}

export function loadSoundTables(): Promise<SoundTables> {
  // #4180: the chunk parses at its top level against `currentCatalog()`, so the module comes
  // first. Every call starts (or reuses) the module load, so a route-intent prefetch counts one
  // `loadWasm()` per engine route through here alone (`routes/chunks.tsx` starts no second one).
  const wasm = loadWasm();
  if (cachedTables !== null) {
    const cached = cachedTables;
    return wasm.then(() => cached);
  }
  if (pendingLoad !== null) {
    const pending = pendingLoad;
    return wasm.then(() => pending);
  }
  // Both were already fetched together in `routes/chunks.tsx`, so only a cold first sound waits.
  pendingLoad = wasm
    .then(() => import("./soundTables.ts"))
    .then((mod) => {
      cachedTables = mod.SOUND_TABLES ?? mod.default;
      pendingLoad = null;
      notify();
      return cachedTables;
    })
    .catch((err) => {
      pendingLoad = null;
      throw err;
    });
  return pendingLoad;
}

function subscribe(callback: () => void): () => void {
  listeners.add(callback);
  return () => {
    listeners.delete(callback);
  };
}

export function onSoundTablesLoaded(callback: (tables: SoundTables) => void): () => void {
  if (cachedTables !== null) {
    try {
      callback(cachedTables);
    } catch {
      // Ignored
    }
  }
  const listener = () => {
    if (cachedTables !== null) {
      try {
        callback(cachedTables);
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

export function useSoundTables(): SoundTables | null {
  if (cachedTables === null) {
    void loadSoundTables().catch(() => {});
  }
  return useSyncExternalStore(subscribe, soundTablesNow, () => null);
}

export function setSoundTablesForTests(tables: SoundTables | null): void {
  cachedTables = tables;
  pendingLoad = tables !== null ? Promise.resolve(tables) : null;
  notify();
}

export type DeferredSoundTables = {
  resolve(tables: SoundTables): void;
  reject(err: unknown): void;
};

export function deferSoundTablesForTests(): DeferredSoundTables {
  cachedTables = null;
  let res!: (tables: SoundTables) => void;
  let rej!: (err: unknown) => void;
  pendingLoad = new Promise<SoundTables>((resolve, reject) => {
    res = resolve;
    rej = reject;
  });
  return {
    resolve(tables: SoundTables) {
      cachedTables = tables;
      notify();
      res(tables);
    },
    reject(err: unknown) {
      pendingLoad = null;
      rej(err);
    },
  };
}
