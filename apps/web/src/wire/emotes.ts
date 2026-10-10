// Shared emote and portrait wire contract (docs/v0.3.0/SURFACE.md §10.4; SPEC §9.4 D5, §9.5,
// §10.10, §10.11, R641–R645, R1340–R1345). These cosmetic values never reach `ActionBody`,
// `reduce`, replays, records or `PlayerView` (R643, R1342).
// The deck-checked roster (D5, R641), dealt hand (R1341) and rate limit (R643) must match at both
// separate deployables (§9.2), so their constants live here, not `crates/server/src/config.rs`
// (CLAUDE.md rule 9). Practice personas stay in `apps/web/src/practice/personas.ts` (R645).

import type { PlayerId } from "./catalog.ts";

/** The wire's twenty-four emotes: five portrait-specific voice lines, then nineteen shared emoji (R1340, R643). */
export const VOICE_EMOTE_IDS = ["greetings", "wellPlayed", "oops", "thanks", "threaten"] as const;
export const EMOJI_EMOTE_IDS = [
  "sob",
  "yawn",
  "laugh",
  "angry",
  "wahWah",
  "wave",
  "clap",
  "thumbsUp",
  "facepalm",
  "shrug",
  "thinking",
  "heart",
  "fire",
  "skull",
  "sweat",
  "cool",
  "gasp",
  "salute",
  "party",
] as const;
export const EMOTE_IDS = [...VOICE_EMOTE_IDS, ...EMOJI_EMOTE_IDS] as const;

export type VoiceEmoteId = (typeof VOICE_EMOTE_IDS)[number];
export type EmojiEmoteId = (typeof EMOJI_EMOTE_IDS)[number];
export type EmoteId = (typeof EMOTE_IDS)[number];

export function isEmoteId(value: unknown): value is EmoteId {
  return typeof value === "string" && (EMOTE_IDS as readonly string[]).includes(value);
}

export function isVoiceEmote(emote: EmoteId): emote is VoiceEmoteId {
  return (VOICE_EMOTE_IDS as readonly string[]).includes(emote);
}

// The engine's `deal_emote_hand`, via `@jackioh/engine`'s `dealEmoteHand`, deals each seat's hand (R1341).

export const EMOTE_HAND_SIZE = 8;
export const EMOTE_HAND_VOICE = 3;

export function handHolds(hand: readonly EmoteId[], emote: EmoteId): boolean {
  return hand.includes(emote);
}

// Hero portraits (R641).

/** A saved deck's `null` portrait reads as the default `vanilla` (R641). */
export const PORTRAIT_IDS = ["vanilla", "gary", "timmy", "dfender", "felinors", "shredder"] as const;
export type PortraitId = (typeof PORTRAIT_IDS)[number];

export const DEFAULT_PORTRAIT: PortraitId = "vanilla";

export function isPortraitId(value: unknown): value is PortraitId {
  return typeof value === "string" && (PORTRAIT_IDS as readonly string[]).includes(value);
}

export function portraitOrDefault(portrait: string | null | undefined): PortraitId {
  return isPortraitId(portrait) ? portrait : DEFAULT_PORTRAIT;
}

/** Launch roster: catalog lookup is by card name, never number; `flavour` guides `card-audio.json5`'s `emote-<id>`. */
export const PORTRAITS: Record<PortraitId, { cardName: string; flavour: string }> = {
  vanilla: { cardName: "Mr. Vanilla", flavour: "Flat, polite, unbothered" },
  gary: { cardName: "Gary the Gambler", flavour: "Fast-talking card sharp" },
  timmy: { cardName: "Tempo Timmy", flavour: "Hyper, rushed" },
  dfender: { cardName: "Big D-fender", flavour: "Low, steady bodyguard" },
  felinors: { cardName: "Duplicating Felinors", flavour: "Two cats talking at once" },
  shredder: { cardName: "Jlockeed Shredder-10", flavour: "Robot, all caps" },
};

export function pickPortrait(rng: () => number): PortraitId {
  const index = Math.min(PORTRAIT_IDS.length - 1, Math.floor(rng() * PORTRAIT_IDS.length));
  return PORTRAIT_IDS[index] ?? DEFAULT_PORTRAIT;
}

function hashUnit(seed: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < seed.length; i += 1) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  // Finalize FNV-1a so distinct seat keys do not cluster in one portrait bucket; R642 requires independent draws.
  h ^= h >>> 16;
  h = Math.imul(h, 0x85ebca6b) >>> 0;
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35) >>> 0;
  h ^= h >>> 16;
  return (h >>> 0) / 0x100000000;
}

/** All Random (R642): deterministic independent per-seat draw without extra entropy. */
export function pickPortraitFromSeed(seed: string): PortraitId {
  return pickPortrait(() => hashUnit(seed));
}

// Shared client/server rate limit (R643).

export const EMOTE_COOLDOWN_MS = 1500;
export const EMOTE_WINDOW_MS = 20_000;
export const EMOTE_WINDOW_MAX = 5;

export type EmoteGate =
  | { ok: true; sentAt: readonly number[] }
  | { ok: false; retryAfterMs: number; sentAt: readonly number[] };

/** Shared pure gate (R643): returns a window-pruned history so client and server cannot disagree. */
export function emoteGate(sentAt: readonly number[], now: number): EmoteGate {
  const kept = sentAt.filter((at) => now - at < EMOTE_WINDOW_MS);
  const last = kept[kept.length - 1];
  if (last !== undefined && now - last < EMOTE_COOLDOWN_MS) {
    return { ok: false, retryAfterMs: EMOTE_COOLDOWN_MS - (now - last), sentAt: kept };
  }
  if (kept.length >= EMOTE_WINDOW_MAX) {
    return { ok: false, retryAfterMs: EMOTE_WINDOW_MS - (now - (kept[0] ?? now)), sentAt: kept };
  }
  return { ok: true, sentAt: kept };
}

export type EmoteRelay = { from: PlayerId; emote: EmoteId };
