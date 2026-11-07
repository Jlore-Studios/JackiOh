// The sounds and buzz of a drag (#3347): pick-up, over a tile, drop and return to the hand. Each fires
// once per event, through the same engine and settings as the game's own sounds, and a failure is
// dropped: sound is never a rule (CLAUDE.md rule 7).

import { getAudioEngine } from "../../audio/engine.ts";
import type { SfxId } from "../../audio/types.ts";
import { sharedHaptics } from "../../haptics/haptics.ts";

export type DragCue = Extract<SfxId, "pickUp" | "dragOver" | "drop" | "returnToHand">;

export function dragCue(cue: DragCue): void {
  try {
    getAudioEngine().playSfx(cue);
  } catch {
    // No sound engine (jsdom) or a refused sound: the drag goes on.
  }
  if (cue !== "pickUp") return;
  try {
    sharedHaptics().moment("pickup");
  } catch {
    // Haptics must not interrupt the game.
  }
}
