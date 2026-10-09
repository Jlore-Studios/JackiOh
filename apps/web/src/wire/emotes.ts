// The web client's own copy of the shared wire helper (docs/v0.3.0/SURFACE.md §10.4), kept as
// TypeScript and unchanged but for its import paths; the server's port is crates/engine/src/wire/emotes.rs.
//
// Emotes and hero portraits (patch v0.2.X, SPEC §9.4 D5, §9.5, §10.10, §10.11, R641–R645; the
// dealt hand of patch v0.3.X's MN03, #545, R1340–R1345).
//
// Everything here is cosmetic: an emote is never an `ActionBody`, never reaches `reduce`, the
// action log, the replay hash or a game record, and a portrait or an emote hand is never part of
// `PlayerView` (R643, R1342). This module holds only what BOTH sides of the wire must agree on —
// the id lists, the portrait roster the deck save checks (D5, R641), the size and shape of the hand
// each seat is dealt (R1341) and the rate limit the client and the server enforce identically
// (R643) — because `apps/web` and `crates/server` may not import each other
// (§9.2: a client and a server are separate deployables). Constants therefore live here and not
// in the server's config (`crates/server/src/config.rs`, CLAUDE.md rule 9): the rule books numbers to one named place,
// and this module is the one place both ends read.
//
// The AI's persona config is NOT here: it is presentation logic of the practice opponent and
// lives in `apps/web/src/practice/personas.ts` (R645), which the engine and the AI's move search
// never import.

import type { PlayerId } from "./catalog.ts";

/** The pool of twenty-four emotes (R1340): the five voice lines first (each portrait has its own
 * text), then the nineteen shared animated emoji — patch v0.2.X's five, then MN03's fourteen. The
 * wire spells them exactly like this (R643). */
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

// ---------------------------------------------------------------------------------------------
// The emote hand (R1341): eight of the pool, dealt to each seat each game from the match seed.
// The deal itself is the engine's (`deal_emote_hand`), reached through WASM (`@jackioh/engine`'s
// `dealEmoteHand`) so the client deals exactly what the server does.
// ---------------------------------------------------------------------------------------------

/** How many emotes a seat is dealt each game (R1341): the menu shows these and nothing else. */
export const EMOTE_HAND_SIZE = 8;
/** How many of a hand are voice lines (R1341); the rest are emoji. */
export const EMOTE_HAND_VOICE = 3;

/** R1342: whether `hand` holds `emote` — the check both ends make before a send goes out. */
export function handHolds(hand: readonly EmoteId[], emote: EmoteId): boolean {
  return hand.includes(emote);
}

// ---------------------------------------------------------------------------------------------
// Hero portraits (R641)
// ---------------------------------------------------------------------------------------------

/**
 * A portrait's id. `vanilla` is the default everywhere a deck does not name one: a saved deck's
 * `portrait` stays `null` and reads back as `vanilla` (R641).
 */
export const PORTRAIT_IDS = ["vanilla", "gary", "timmy", "dfender", "felinors", "shredder"] as const;
export type PortraitId = (typeof PORTRAIT_IDS)[number];

export const DEFAULT_PORTRAIT: PortraitId = "vanilla";

export function isPortraitId(value: unknown): value is PortraitId {
  return typeof value === "string" && (PORTRAIT_IDS as readonly string[]).includes(value);
}

/** `null` means `vanilla` (R641): the column, the deck view and imports all read through this. */
export function portraitOrDefault(portrait: string | null | undefined): PortraitId {
  return isPortraitId(portrait) ? portrait : DEFAULT_PORTRAIT;
}

/**
 * The launch roster. `cardName` is the Core card the portrait's art and voice come from, looked
 * up BY NAME in the catalog, never by number (the issue's roster table). `flavour` is the voice
 * direction the emote persona in `card-audio.json5` (`emote-<id>`) follows.
 */
export const PORTRAITS: Record<PortraitId, { cardName: string; flavour: string }> = {
  vanilla: { cardName: "Mr. Vanilla", flavour: "Flat, polite, unbothered" },
  gary: { cardName: "Gary the Gambler", flavour: "Fast-talking card sharp" },
  timmy: { cardName: "Tempo Timmy", flavour: "Hyper, rushed" },
  dfender: { cardName: "Big D-fender", flavour: "Low, steady bodyguard" },
  felinors: { cardName: "Duplicating Felinors", flavour: "Two cats talking at once" },
  shredder: { cardName: "Jlockeed Shredder-10", flavour: "Robot, all caps" },
};

/** A uniform pick over the roster (R642: All Random deals one per seat, independently). */
export function pickPortrait(rng: () => number): PortraitId {
  const index = Math.min(PORTRAIT_IDS.length - 1, Math.floor(rng() * PORTRAIT_IDS.length));
  return PORTRAIT_IDS[index] ?? DEFAULT_PORTRAIT;
}

/** A tiny deterministic hash → [0,1), for server-side portrait deals keyed on the match seed. */
function hashUnit(seed: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < seed.length; i += 1) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  // Avalanche (MurmurHash3's fmix32): FNV-1a diffuses a differing last character into a small
  // multiple of the prime, so `${seed}:portrait:p1` and `${seed}:portrait:p2` fell in the same
  // portrait bucket ~95% of the time instead of R642's independent ~1/6. The finalizer spreads
  // one-character differences over all 32 bits, so the seats draw independently.
  h ^= h >>> 16;
  h = Math.imul(h, 0x85ebca6b) >>> 0;
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35) >>> 0;
  h ^= h >>> 16;
  return (h >>> 0) / 0x100000000;
}

/**
 * All Random's deal (R642): uniform, independent per seat, decided at match creation. Seeded like
 * `dealRandomDeck`'s per-seat draws (`${seed}:portrait:<seat>`) so it needs no extra entropy and a
 * test can reproduce it exactly.
 */
export function pickPortraitFromSeed(seed: string): PortraitId {
  return pickPortrait(() => hashUnit(seed));
}

// ---------------------------------------------------------------------------------------------
// The rate limit (R643): one rule, one config, enforced at both ends.
// ---------------------------------------------------------------------------------------------

/** The pause between two emotes from one player. */
export const EMOTE_COOLDOWN_MS = 1500;
/** The rolling window the cap is counted over. */
export const EMOTE_WINDOW_MS = 20_000;
/** At most this many emotes in a window. */
export const EMOTE_WINDOW_MAX = 5;

export type EmoteGate =
  | { ok: true; sentAt: readonly number[] }
  | { ok: false; retryAfterMs: number; sentAt: readonly number[] };

/**
 * Whether `sentAt` (timestamps of this player's recent emotes, oldest first) admits one more at
 * `now`. Pure and total: the same function answers the server's drop and the client's grey-out,
 * so the two can never disagree (R643). `sentAt` is returned pruned to the window, so a caller
 * can keep one rolling array per player.
 */
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

/** The emote a seat sends, as the server relays it to the opponent (R643). */
export type EmoteRelay = { from: PlayerId; emote: EmoteId };
