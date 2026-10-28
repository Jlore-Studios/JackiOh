// The audio module's public face (SPEC §10.11): the board's hooks and the engine. The settings
// panel imports `AudioControls` and the settings store from their own modules.

export { useGameAudio } from "./useGameAudio.ts";
export { useVoiceSpeaking } from "./useVoiceSpeaking.ts";
export { usePickupSound } from "./usePickupSound.ts";
export { useTurnClockAlarm } from "./useTurnClockAlarm.ts";
export { getAudioEngine } from "./engine.ts";
export type { SfxId } from "./types.ts";
