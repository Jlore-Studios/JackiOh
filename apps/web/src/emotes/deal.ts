// R1341: both seats' emote hands for a practice game, dealt from the game's seed by the engine's own
// `deal_emote_hand` over WASM, so a seat holds the eight it would hold in a match on that seed, the
// same on a reload or a resumed game. (The hotseat deals through its engine port, `dealEmoteHand`,
// which is the same binding.) Kept apart from `./hand.ts` because it needs the WebAssembly module
// loaded, which the board and its tests do not.

import type { EmoteId } from "@jackioh/shared";
import { dealEmoteHand } from "@jackioh/engine";

export function dealEmoteHands(seed: string): { p1: readonly EmoteId[]; p2: readonly EmoteId[] } {
  return { p1: dealEmoteHand(seed, "p1"), p2: dealEmoteHand(seed, "p2") };
}
