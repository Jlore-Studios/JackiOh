// A received ping's sound (#936 B64): one of the recipes `audio/sfx.ts` already synthesizes, chosen
// for what the ping says, so the ear tells a "careful" from a "kill this" without looking.

import { getAudioEngine, type SfxId } from "../audio/index.ts";
import type { SfxParams } from "../audio/types.ts";
import type { PingKind } from "../net/duos.ts";

const PING_SFX: Readonly<Record<PingKind, { id: SfxId; params?: SfxParams }>> = {
  thumbsUp: { id: "emoteThumbsUp" },
  // The dull two-note "no" a full Deck turns a card away with.
  thumbsDown: { id: "refuse" },
  portal: { id: "whoosh" },
  // The doorbell a question waiting on the player rings (R269's urgent notice).
  careful: { id: "notify", params: { urgent: true } },
  // A mark landing on its card (R437): this one is the target.
  killThis: { id: "brand" },
};

/** Plays `ping`'s sound; muted, hidden or locked audio plays nothing, as for every cue. */
export function playPingSound(ping: PingKind): void {
  const { id, params } = PING_SFX[ping];
  getAudioEngine().playSfx(id, params);
}
