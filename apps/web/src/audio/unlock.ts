// The autoplay unlock (docs/polish/2-sound.md, "unlock.ts"; B2). Browsers keep Web Audio silent until a
// user gesture, and iOS wants the context resumed and a sound started inside it. The HTML standard's
// activation-triggering events are keydown, mousedown, pointerdown (mouse only), pointerup (non-mouse)
// and touchend, so a touch pointerdown alone does not unlock iOS. The listeners stay for the hook's
// lifetime: iOS can suspend the context again after a call or a backgrounding.

import type { AudioEngine } from "./types.ts";

export const UNLOCK_EVENTS = ["pointerdown", "pointerup", "touchend", "click", "keydown"] as const;

export function installAudioUnlock(engine: Pick<AudioEngine, "unlock" | "state">): () => void {
  const onGesture = (): void => {
    try {
      const state = engine.state();
      if (state !== "running" && state !== "unsupported") engine.unlock();
    } catch {
      // Sound must never break input handling.
    }
  };

  for (const type of UNLOCK_EVENTS) window.addEventListener(type, onGesture, { capture: true, passive: true });
  return () => {
    for (const type of UNLOCK_EVENTS) window.removeEventListener(type, onGesture, { capture: true });
  };
}
