// Shared audio types (docs/polish/2-sound.md, Surface). Every audio module and test reads these, so
// they are a cross-slice boundary: do not rename or reshape them.

import type { VoiceEmoteId } from "@jackioh/shared";

import type { CARD_HOOKS } from "./constants.ts";

export type SfxId =
  | "draw" | "play" | "summon" | "attack" | "impact" | "shieldShatter" | "heal" | "buff" | "debuff"
  | "death" | "burn" | "trapSet" | "trapSting" | "spell" | "mana" | "turnStart" | "victory"
  | "defeat" | "uiClick" | "uiHover" | "whoosh" | "radiant" | "lock" | "poof" | "sand" | "endTurn" | "notify" | "drain"
  | "cancel" | "entrance" | "fatigue" | "refuse"
  // Patch v0.2.0 (R506): card moments, Call to Chaos's roll (R436), a mark (R437), the turn clock (R439).
  | "manaCrack" | "bloodDrain" | "goldBurst" | "castOnDraw" | "chaosRoll" | "brand" | "heartbeat" | "clockTick"
  // Patch v0.2.X (R644): the five emoji emotes (issue §4), synthesized on the effects channel.
  | "emoteSob" | "emoteYawn" | "emoteLaugh" | "emoteAngry" | "emoteWahWah"
  // Patch v0.2.X (#259, R669): the play sting of a card below Legendary.
  | "sting";

/**
 * A card's sound family, from its public tags and type (cues.ts `timbreFor`, which follows the
 * card art's theme order): the summon thud gains the family's accent and the spell shimmer its
 * chimes. Absent: the plain recipe, which is all a card the viewer cannot name ever gets (R203).
 * Patch v0.2.0 adds the Book, Pancake and AI tags' families.
 */
export type SfxTimbre =
  | "human" | "felinor" | "ky" | "cn" | "fruit" | "chaos" | "quickdraw" | "token" | "field"
  | "book" | "pancake" | "ai";

export type SfxParams = {
  /**
   * damage / heal / health-loss amount, or the mana gained; recipes clamp to [1, IMPACT_AMOUNT_CAP].
   * clockTick: how far into the last ten seconds (1 at ten left, 10 at one left). chaosRoll: how
   * many effects the roll names, one ding each, clamped to [0, CHAOS_REVEAL_MAX] (default 1).
   */
  amount?: number;
  /** true when the event is the viewer's own (turnStart, mana): a brighter variant. */
  mine?: boolean;
  /** summon and spell: the card's family (see SfxTimbre). */
  timbre?: SfxTimbre;
  /** entrance: a Mythic's prismatic sting rather than a Legendary's brass. */
  mythic?: boolean;
  /**
   * notify: a notice that waits on the viewer's answer (the other seat's draw offer), which rings
   * like a doorbell rather than the routine two blips, inside the same durationMs.
   */
  urgent?: boolean;
  /** brand: the mark lifting from its card, a soft release, rather than the brand landing (R437). */
  release?: boolean;
  /** sting: the played card's rarity below Legendary (R669). Absent: Common. */
  tier?: StingTier;
  /**
   * R669: where on the board the sound comes from, -1 (the leftmost lane) to 1 (the rightmost), read
   * off the lane of the unit it is about. Absent: centred. The engine pans; no recipe reads it.
   */
  pan?: number;
  /** #185: a landing Unit's size tier; the summon thud is weighed by it rather than by `amount`. */
  slamTier?: "tiny" | "small" | "medium" | "large" | "huge" | "massive";
  /** Match-feel impact and sand variations. The caller supplies a sample from 0 through 1. */
  variation?: number;
  /** The public damage tier that selected this impact recipe. */
  impactTier?: "tiny" | "normal" | "moderate" | "big" | "giga";
  /** Sand's rolling four-way grain texture and the amount built up by sustained tapping. */
  sandVariant?: number;
  sandBuild?: number;
};

/** R669: the play sting's sizes, by the card's rarity (Common, Rare, Epic). */
export type StingTier = "common" | "rare" | "epic";

/** A card's sound family in `card-audio.json5`, from its catalog type: which hooks it may carry. */
export type CardKind = "unit" | "spell" | "trap";
/** R655: when a card's sounds play (constants.ts CARD_HOOKS, the one list of them). */
export type CardHook = keyof typeof CARD_HOOKS;
/** A voice line is a hook's line, so its kinds are the hooks. */
export type VoiceLineKind = CardHook;
/**
 * What `playVoice` accepts: a card's line, or R644's portrait emote line (`emote-<portrait>-<id>`
 * files). The split-from-the-end VoiceKey convention holds either way.
 */
export type PlayableLineKind = VoiceLineKind | VoiceEmoteId;
/** "<defId>-<line>", e.g. "core-004-play", "core-051-1-cast", "emote-gary-thanks". Parse from the END: defIds contain "-". */
export type VoiceKey = `${string}-${PlayableLineKind}`;

/** What every voice (once called a persona) carries, whichever synthesizer rendered its files. */
type PersonaCommon = {
  /** For the speechSynthesis fallback: SpeechSynthesisUtterance pitch (0–2) and rate (0.1–10). */
  web: { pitch: number; rate: number };
  /** Output trim applied at runtime to this persona's files, 0–2. Default 1. */
  gain?: number;
};

/** A persona rendered by macOS `say` (gen-voice.mjs's default backend). */
export type SayPersona = PersonaCommon & {
  backend?: "say";
  /** A `say -v` voice name, verbatim, e.g. "Reed (English (US))". */
  say: string;
  /** `[[rate]]` words per minute, 90–360. */
  rate: number;
  /** `[[pbas]]` pitch base, 0–127. */
  pbas: number;
  /** `[[pmod]]` pitch modulation, 0–127. */
  pmod: number;
};

/** R501: a persona rendered by Windows SAPI and shaped by ffmpeg (gen-voice.mjs's second backend). */
export type SapiPersona = PersonaCommon & {
  backend: "sapi";
  /** An installed SAPI voice name, e.g. "Microsoft Zira Desktop". */
  voice: string;
  /** SSML prosody rate in percent, -50 to 100. */
  rate: number;
  /** ffmpeg pitch shift in semitones, -12 to 12, tempo kept. */
  semitones: number;
  /** An ffmpeg audio filter chain that colours the voice ("" for none). */
  filter: string;
};

export type Persona = SayPersona | SapiPersona;

/**
 * R655: a sound effect in the effects bank: one of the procedural recipes, shifted by `pitch` (a
 * frequency ratio, 1 as the recipe is written), trimmed by `gain` and given the recipe's own params.
 */
export type CardEffect = { sfx: SfxId; pitch: number; gain: number; params?: SfxParams };

/** R655: one hook's sounds, as the file writes them: a voice line, an effect, or both (never neither). */
export type HookAssignment =
  | { voice: string; text: string; effect?: string }
  | { effect: string; voice?: undefined; text?: undefined };

/** A card's hooks, with the kind its catalog type gives it (the file never states it). */
export type CardAudioEntry = { kind: CardKind } & { [H in CardHook]?: HookAssignment };

type Overrides = { rate?: number; pbas?: number; pmod?: number };

/** R644: one portrait's five issue-§3 lines. Keyed by portrait id in `CardAudioTable.emotes`. */
export type EmoteLineEntry = {
  persona: string;
  greetings: string;
  wellPlayed: string;
  oops: string;
  thanks: string;
  threaten: string;
} & Overrides;

/** `card-audio.json5` as `voiceData.ts` parses it (R655). */
export type CardAudioTable = {
  /** The voices bank: what each voice's lines are rendered and spoken with. */
  voices: Record<string, Persona>;
  /** The effects bank, by the name a hook gives. */
  effects: Record<string, CardEffect>;
  /** Keyed by catalog id: exactly the ids of packages/cards/catalog.json, tokens included. */
  cards: Record<string, CardAudioEntry>;
  /** Keyed by portrait id (shared `PORTRAIT_IDS`): the six portraits' emote lines. */
  emotes: Record<string, EmoteLineEntry>;
};

export type VoiceManifest = {
  version: 1;
  /** Informative: "m4af aac@22050 mono 32000". */
  format: string;
  /** Keyed by VoiceKey. `hash` = voiceHash(...) below; `bytes` = the file's size on disk. */
  files: Record<string, { hash: string; bytes: number }>;
};

/**
 * How much a line matters when another is already speaking (constants.ts VOICE_PRIORITY): a line
 * cuts in on one of lower priority and waits briefly behind one of equal or higher priority.
 */
export type VoicePriority = number;

export type SoundCue =
  | { kind: "sfx"; id: SfxId; params?: SfxParams; delayMs: number }
  | { kind: "voice"; defId: string; line: PlayableLineKind; delayMs: number; priority: VoicePriority }
  /** R655: a card's effect for a hook, at the moment the hook's line would speak. */
  | { kind: "effect"; defId: string; hook: CardHook; delayMs: number; priority: VoicePriority };

export type AudioState = "unsupported" | "locked" | "running" | "suspended" | "closed";

/**
 * "pending" until the line starts or is given up on. "dropped": it never started, because a more
 * important line took its place in the queue or the channel, or sound was muted or voice lines
 * turned off first. "suspended": accepted while the context was not running, so nothing was
 * scheduled (a suspended context would otherwise release every queued sound at once on resume).
 */
export type VoiceOutcome = "pending" | "file" | "speech" | "late" | "failed" | "dropped" | "suspended";
export type PlayedCue =
  | {
      kind: "sfx";
      id: SfxId;
      params?: SfxParams;
      delayMs: number;
      atMs: number;
      /** Present when the cue was accepted while the context was not running, so nothing was scheduled. */
      suspended?: true;
    }
  | {
      kind: "voice";
      defId: string;
      line: PlayableLineKind;
      delayMs: number;
      atMs: number;
      outcome: VoiceOutcome;
      priority: VoicePriority;
    }
  | {
      kind: "effect";
      defId: string;
      hook: CardHook;
      /** The effects bank's name for what played. */
      effect: string;
      delayMs: number;
      atMs: number;
      /** Present when the cue was accepted while the context was not running, so nothing was scheduled. */
      suspended?: true;
    };

/** A music station: the overall flavour of the in-game music (SPEC §10.11 "Music", R631). */
export type MusicStation = "tavern" | "edm" | "lofi" | "epic";

export type AudioSettings = {
  master: number; // 0..1
  sfx: number; // 0..1
  /** The crowd's reactions to big hits and heavy landings, 0..1. */
  crowd: number;
  voice: number; // 0..1
  muted: boolean;
  voiceOn: boolean;
  /** #51's music (R631): the music bus, 0..1. */
  music: number;
  /** The station a match starts on. A card may switch it for the rest of that match. */
  station: MusicStation;
  /** Off: the station's in-game track loops, and nothing in the game changes the music. */
  dynamicMusic: boolean;
  /** The music dips under voice lines and the important effects (MUSIC_DUCK_SFX). */
  duckMusic: boolean;
  /**
   * The music keeps playing while the page is hidden or the window has lost focus. Off (the default):
   * it fades out then, and back in when the player returns. Settings saved before this key read the
   * old `pauseMusicOnBlur` inverted (`parseAudioSettings`).
   */
  playMusicInBackground: boolean;
};

/** One rendered track (scripts/gen-music.mjs writes these into music-manifest.json). */
export type MusicTrack = {
  hash: string;
  bytes: number;
  bpm: number;
  beatsPerBar: number;
  /** false: a sting that plays once. */
  loop: boolean;
  /** Seconds of intro before the loop's music begins (a result's or a Mythic's sting); a sting's whole length. */
  intro: number;
  /** The file's length in seconds. */
  duration: number;
  /** Where the loop starts and ends in the file, in seconds; null for a sting. */
  loopStart: number | null;
  loopEnd: number | null;
  /** A sting's last bar line, where the track it leads into starts; null for a loop. */
  handoff: number | null;
};

export type MusicManifest = {
  version: 1;
  /** Informative: "aac-lc 44100 stereo 80k". */
  format: string;
  /** Keyed by track id, e.g. "menu", "tavern-1", "tavern-danger", "mythic-zephyrs". */
  files: Record<string, MusicTrack>;
};

/** What a card does to the music when it is cast (music-cards.json, keyed by catalog id). */
export type MusicCardEntry = { theme?: string; station?: MusicStation };

/** What the director and the UI need from an engine. */
export type SoundSink = {
  /** true when accepted (and logged); false when refused. Never throws. */
  playSfx(id: SfxId, params?: SfxParams, delayMs?: number): boolean;
  /** `priority` defaults to VOICE_PRIORITY.play. `defId` may be an `emote-<portrait>` id (R644). */
  playVoice(defId: string, line: PlayableLineKind, delayMs?: number, priority?: VoicePriority): boolean;
  /** R655: the card's effect for `hook`, on the effects bus. false when it has none or it is refused. */
  playEffect(defId: string, hook: CardHook, delayMs?: number): boolean;
};

export type AudioEngine = SoundSink & {
  /**
   * R655: the viewer picked up their own Unit to attack (a drag lifted, or a click chose it): its
   * `attack` hook, effect then line, at once. Refused within PICKUP_MIN_GAP_MS of the last one
   * accepted; otherwise it cuts off whatever the last pick-up is still playing.
   */
  playPickup(defId: string): boolean;
  state(): AudioState;
  /** Call synchronously inside a user gesture. Idempotent. */
  unlock(): void;
  /** Held while busy (the newest call only) and skipped while muted or with voice lines off. */
  preloadVoices(keys: readonly VoiceKey[]): void;
  /**
   * true while the board animates (useGameAudio: the runner has an entry in flight). Background
   * voice work (the prefetch and preloads) waits until it is false again; a line asked to play
   * never does (B58).
   */
  setBusy(busy: boolean): void;
  /** Accepted cues, oldest first, at most LOG_LIMIT. */
  log(): readonly PlayedCue[];
  clearLog(): void;
  /** How many AudioContexts this engine has constructed (0 or 1). */
  contextsCreated(): number;
  /**
   * true while a voice line holds the one voice channel: from the moment it takes the channel
   * (loading its file included) until it ends, is cut or is given up on. Practice holds the AI's
   * next step on it through the page's `data-speaking` mark (SPEC §9.9, `useVoiceSpeaking`).
   */
  speaking(): boolean;
  /** Called after every change of `speaking()`, never for a non-change. Returns the unsubscribe. */
  subscribeSpeaking(listener: () => void): () => void;
  /** R631: the context and the music bus the music plays into, once the first unlock made them; else null. */
  musicOutput(): { context: AudioContext; input: AudioNode } | null;
  /** Called when the context is made and each time a resume settles. Returns the unsubscribe. */
  subscribeState(listener: () => void): () => void;
  dispose(): void;
};
