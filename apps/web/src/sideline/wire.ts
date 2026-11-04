// The watcher's emote frames as the web reads them (R2990000, R2990001): the hand a watch socket is
// dealt (`spectatorHand`) and a relayed watcher's emote (`spectatorEmote`), checked shallowly as every
// frame is (`game/net.ts`). The server decides both; the client only refuses what it cannot draw.

import { SPECTATOR_EMOTE_HAND_SIZE } from "@jackioh/server-config";
import { isEmoteId, isVoiceEmote, type EmojiEmoteId } from "@jackioh/shared";

/** One of the pool's emoji, never a voice line: a watcher has no portrait to speak with (R2990001). */
export function isEmojiEmoteId(value: unknown): value is EmojiEmoteId {
  return isEmoteId(value) && !isVoiceEmote(value);
}

/** R2990001: a watcher's hand, 1 to `SPECTATOR_EMOTE_HAND_SIZE` distinct emoji ids. */
export function isSpectatorHand(value: unknown): value is readonly EmojiEmoteId[] {
  return (
    Array.isArray(value) &&
    value.length >= 1 &&
    value.length <= SPECTATOR_EMOTE_HAND_SIZE &&
    value.every(isEmojiEmoteId) &&
    new Set(value).size === value.length
  );
}

/** R2990000: a relayed watcher's emote: the watcher's name, never empty, and an emoji id. */
export function spectatorEmoteOf(frame: Record<string, unknown>): { watcher: string; emote: EmojiEmoteId } | null {
  const { watcher, emote } = frame;
  if (typeof watcher !== "string" || watcher.length === 0 || !isEmojiEmoteId(emote)) return null;
  return { watcher, emote };
}
