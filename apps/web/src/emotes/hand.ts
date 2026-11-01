// The emote hand on the client (R1341–R1343): the eight emotes a seat may send this game, from the
// server's portraits frame (R1342) or, in a game on this device, WASM (`./deal.ts`). This module
// never touches the engine, so the board, the menus and the socket client import it without WASM.

import type { EmoteId, PlayerId } from "@jackioh/shared";
import { EMOJI_EMOTE_IDS, EMOTE_HAND_SIZE, EMOTE_HAND_VOICE, VOICE_EMOTE_IDS, isEmoteId } from "@jackioh/shared";

/** Each seat's hand, as far as this client knows it: a match tells it only its own (R1342). */
export type EmoteHands = Readonly<Partial<Record<PlayerId, readonly EmoteId[]>>>;

/**
 * R1343: the hand a seat shows before a deal reaches it: a dealt hand's shape, in the pool's order,
 * and only ids of R643's ten, which any server relays.
 */
export const DEFAULT_EMOTE_HAND: readonly EmoteId[] = [
  ...VOICE_EMOTE_IDS.slice(0, EMOTE_HAND_VOICE),
  ...EMOJI_EMOTE_IDS.slice(0, EMOTE_HAND_SIZE - EMOTE_HAND_VOICE),
];

/**
 * R1342: a hand as the portraits frame carries it — EMOTE_HAND_SIZE distinct ids of the pool.
 * Anything else is not a hand, and the client keeps showing the one it had.
 */
export function isEmoteHand(value: unknown): value is EmoteId[] {
  if (!Array.isArray(value) || value.length !== EMOTE_HAND_SIZE) return false;
  if (!value.every((id) => isEmoteId(id))) return false;
  return new Set(value).size === value.length;
}

export function handFor(hands: EmoteHands | null | undefined, player: PlayerId): readonly EmoteId[] {
  return hands?.[player] ?? DEFAULT_EMOTE_HAND;
}
