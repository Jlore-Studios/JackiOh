// Whether a voice line holds the voice channel (the engine's `speaking()`), for `Game`'s `data-speaking`
// mark: practice holds the AI's next step while any element carries it (SPEC §9.9, routes/practice.tsx
// `useVoiceHold`), so the AI never plays over a card that is still talking.

import { useSyncExternalStore } from "react";

import { getAudioEngine } from "./engine.ts";

const never = (): boolean => false;

export function useVoiceSpeaking(): boolean {
  const engine = getAudioEngine();
  return useSyncExternalStore(engine.subscribeSpeaking, engine.speaking, never);
}
