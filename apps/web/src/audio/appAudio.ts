// The page-wide half of sound (SPEC §10.11; B52): the gesture unlock and the UI click and hover
// ticks, installed once on the document for as long as anything holds them, and the main menu
// theme on every screen without a board (R631, musicScene.ts).
//
// `main.tsx` holds it for the page's lifetime, so every screen ticks and the first tap unlocks the
// context before a game starts; every mounted `Game` holds it too (`useGameAudio`), so a Game
// rendered on its own still does. However many hold it, there is one set of listeners.
//
// The listeners call `getAudioEngine()` on every event, so a test that swaps the singleton
// (`setAudioEngineForTests`) is heard.

import { getAudioEngine } from "./engine.ts";
import { holdMenuMusic } from "./musicScene.ts";
import type { SfxId, SfxParams } from "./types.ts";
import { installUiSounds } from "./uiSounds.ts";
import { installAudioUnlock } from "./unlock.ts";

let holders = 0;
let remove: (() => void) | null = null;

function install(): () => void {
  const removeUnlock = installAudioUnlock({
    unlock: () => getAudioEngine().unlock(),
    state: () => getAudioEngine().state(),
  });
  const removeTicks = installUiSounds(
    { playSfx: (id: SfxId, params?: SfxParams, delayMs?: number) => getAudioEngine().playSfx(id, params, delayMs) },
    document,
  );
  const releaseMenu = holdMenuMusic();
  return () => {
    removeUnlock();
    removeTicks();
    releaseMenu();
  };
}

/** Holds the page-wide listeners, installing them for the first holder; returns the release. */
export function retainAppAudio(): () => void {
  holders += 1;
  remove ??= install();
  let released = false;
  return () => {
    if (released) return;
    released = true;
    holders -= 1;
    if (holders === 0 && remove !== null) {
      remove();
      remove = null;
    }
  };
}

/** How many holders there are (tests). */
export function appAudioHolders(): number {
  return holders;
}
