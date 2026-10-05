// Every audio number another module or a test reads (CLAUDE.md rule 9, applied to the client the way
// `animations.ts` names its durations). The frequencies and envelope times inside one SFX recipe are
// that recipe's data and stay local to `sfx.ts`, the way keyframes are `animations.css`'s data.

import type { CardHook, CardKind, SfxId } from "./types.ts";

/**
 * R655: the hooks a card's sounds play on, in the order a card entry of `card-audio.json5` lists
 * them, each with the kinds of card that may carry it. This is the one list: the parser accepts a
 * hook name only from here, so a new hook (an Activate, a Trap's reveal, a turn's start) is a new
 * row here and a place that plays it.
 */
export const CARD_HOOKS = {
  /** A Unit played, or put onto the field by an effect (R204). */
  play: { kinds: ["unit"] },
  /** A Unit of the viewer's picked up to attack: a drag lifted, or a click chose it. */
  attack: { kinds: ["unit"] },
  /** A Unit destroyed (R204). */
  death: { kinds: ["unit"] },
  /** A Spell or Field Spell cast, a Trap or Field Trap firing (R204). */
  cast: { kinds: ["spell", "trap"] },
} as const satisfies Record<string, { kinds: readonly CardKind[] }>;
export const CARD_HOOK_NAMES = Object.keys(CARD_HOOKS) as readonly CardHook[];

export const AUDIO_SETTINGS_KEY = "jackioh.audio.v1";
export const LOG_LIMIT = 100;
export const SFX_MAX_VOICES = 12;        // concurrent sfx cues still sounding
export const SFX_RETRIGGER_MS = 40;      // same SfxId again within this window is refused
export const VOICE_LATE_MS = 600;        // a line not ready this long after it takes the channel is dropped
/**
 * A line's claim on the one voice channel. A higher number cuts in on a lower one; an equal or
 * lower one waits in the queue. A death line and a firing trap's line ("react") answer something
 * that just happened and would be meaningless a second later, so they take the channel from a play
 * or cast line; a unit an effect summons ("summon") speaks only if nothing else is talking. A Unit
 * the viewer picks up to attack ("pickup", R655) answers their own hand at once: it cuts in on any
 * line, an earlier pick-up's included.
 */
export const VOICE_PRIORITY = { summon: 0, play: 1, react: 2, pickup: 3 } as const;
export const VOICE_QUEUE_MAX = 2;        // lines waiting behind the one speaking
export const VOICE_QUEUE_WAIT_MS = 1500; // a waiting line older than this when the channel frees is dropped
export const VOICE_FADE_S = 0.04;        // how fast a line that is cut in on fades out
/** A rendered line's audible span: samples at or under this magnitude at either end are silence. */
export const VOICE_TRIM_THRESHOLD = 0.005;
export const VOICE_TRIM_LEAD_S = 0.02;   // kept before the first audible sample
export const VOICE_TRIM_TAIL_S = 0.08;   // kept after the last audible sample, for the decay
/** The voice volume slider's sample line: #8 Mr. Vanilla, "Hello. I am very normal." */
export const VOICE_PREVIEW_DEF_ID = "core-008";
export const VOICE_DECODED_MAX = 32;     // decoded lines kept in memory, least recently used evicted
export const VOICE_PREFETCH_DELAY_MS = 2000; // after the first running preload, fetch every line's bytes
export const VOICE_PREFETCH_CONCURRENCY = 2;
export const VOICE_SPEECH_MAX_MS = 4000; // speech fallback holds the voice channel at most this long
export const VOICE_PRELOAD_MAX = 24;     // new keys fetched per preloadVoices call
export const IMPACT_AMOUNT_CAP = 10;
export const GAIN_SMOOTHING_S = 0.015;   // setTargetAtTime time constant for bus changes
export const VOICE_DELAY_MS = 150;       // play/cast line after the card whoosh
export const DEATH_VOICE_DELAY_MS = 120;
export const PAIR_OFFSET_MS = 220;       // cues of the 2nd event of a collapsed cardPlayed+summoned entry
export const FLUSH_MAX_SFX = 4;
export const FLUSH_GAP_MS = 90;
export const UI_HOVER_THROTTLE_MS = 80;
/** R501: §10.11's cap on the pre-rendered voice set, raised from 3 MiB for Classic and Classic+. */
export const VOICE_BUDGET_BYTES = 6 * 1024 * 1024;
export const VOICE_FILE_MAX_MS = 4000;   // longest rendered line (gen-voice.mjs MAX_SECONDS; B35)
/** An attack line can repeat while the player fiddles with the Unit (R655), so it is the shortest. */
export const VOICE_MAX_WORDS = { play: 8, attack: 4, death: 6, cast: 8 } as const satisfies Record<CardHook, number>;
/** R655: a hook with an effect and a line starts the effect first, and the line this much later. */
export const CARD_EFFECT_DELAY_MS = 200;
/** R655: each play of a card's effect shifts its pitch by up to this share either way, so repeats differ. */
export const EFFECT_PITCH_JITTER = 0.03;
/** R655: a pick-up this soon after the last one accepted plays nothing, so quick fiddling never stacks. */
export const PICKUP_MIN_GAP_MS = 300;
/** R97's sentinel as a redacted event carries it (packages/engine/src/viewFor.ts HIDDEN_ID). */
export const HIDDEN_DEF_ID = "hidden";
/** Rules vocabulary a line may not use (whole word, case-insensitive): lines are flavour, not text. */
export const BANNED_RULES_WORDS: readonly string[] = [
  "Taunt", "Divine Shield", "Reborn", "Lifesteal", "Poisonous", "First Strike", "Trample", "Cleave", "Pierce",
  "Immutable", "Indestructible", "Stack", "Echo", "Combo", "Discover", "Recruit", "Tribute",
  "Embiggen", "Radiant", "Armor", "Rush", "Charge", "Cry", "Deathrattle", "Battlecry", "mana",
  "damage", "summon", "exile", "fatigue", "backrow", "graveyard",
  // Patch v0.2.0's rules words (docs/classic-sets.md B3, B5).
  "Animated", "Activate", "Brittle", "Degrade", "Upgrade", "Spell Damage", "Immune to Spells", "Counter",
  "Flicker", "Plague Counter",
];

/**
 * B34: the share of voice lines that may restate rules vocabulary (`BANNED_RULES_WORDS`) before the
 * test fails. A line is "Taunt" for a Taunt unit only by accident; the designer allows the odd one
 * to stand (issue #115), but never as many as two in a hundred.
 */
export const BANNED_WORDS_MAX_SHARE = 0.02;

// ---- Patch v0.2.0 sound: card moments, Call to Chaos, marks and the turn clock (R506) ----
/** #21 Hinder: its rider on the victim's next refresh is heard as a mana crack (cues.ts, R506). */
export const HINDER_DEF_ID = "core-021";
/** #27 Blood Ridden Glowy Jelly Bean: each card it turns Radiant is a blood drain and a gold burst. */
export const BLOOD_BEAN_DEF_ID = "core-027";
/**
 * The id `modifierChanged` names the next refresh's rider by (packages/engine/src/mana.ts
 * NEXT_REFRESH_MODIFIER_ID, R169): the web reaches no engine module for a string, as HIDDEN_DEF_ID.
 */
export const NEXT_REFRESH_MODIFIER_ID = "nextTurnMana";
/** On #27's `radiantSet`, the gold burst lands this long after the blood drain begins. */
export const GOLD_BURST_DELAY_MS = 280;
/** Call to Chaos's roll dings once per effect it names (R436), at most this many: the Radiant face's three. */
export const CHAOS_REVEAL_MAX = 3;
/** Plays the director keeps open at once (a cast inside a play inside a play); the oldest is forgotten past it. */
export const PLAY_STACK_MAX = 8;
/** R439: the turn clock's alarm beats through the last this-many ms of the viewer's own turn clock. */
export const CLOCK_ALARM_FROM_MS = 30_000;
/** Within the last this-many ms each beat is a sharper tick (clockTick), sharper every second. */
export const CLOCK_ALARM_SHARP_FROM_MS = 10_000;
/** One beat each time the clock crosses a whole multiple of this. */
export const CLOCK_ALARM_BEAT_MS = 1_000;
/** A beat is scheduled once its moment is at most this far ahead (the Clock repaints every 200 ms). */
export const CLOCK_ALARM_LOOKAHEAD_MS = 400;
/** A beat whose moment passed at most this long ago still plays, at once; an older one is skipped. */
export const CLOCK_ALARM_LATE_MS = 250;

// ---- Patch v0.2.7 music (SPEC §10.11 "Music", R631) ----
/** The stations, in the order the picker lists them. */
export const MUSIC_STATIONS = ["tavern", "edm", "lofi", "epic"] as const;
/** The match counter that rotates each station's in-game tracks, kept per device. */
export const MUSIC_ROTATION_KEY = "jackioh.music.rotation.v1";
/** A crossfade between two tracks. */
export const MUSIC_FADE_S = 1.5;
/** A change waits for the playing track's next bar line, unless that is further off than this. */
export const MUSIC_BAR_WAIT_MAX_S = 3;
/** Scheduling lead: nothing starts sooner than this after it is asked for. */
export const MUSIC_LEAD_S = 0.05;
/** The track a sting hands off to fades in over this, under the sting's tail. */
export const MUSIC_HANDOFF_FADE_S = 0.05;
/** The opponent's turn: the default track through a low-pass at this cutoff, at this gain. */
export const MUSIC_OPPONENT_LOWPASS_HZ = 1400;
export const MUSIC_OPPONENT_GAIN = 0.75;
/** The low-pass's cutoff on the viewer's own turn: open. */
export const MUSIC_OPEN_LOWPASS_HZ = 20000;
/** setTargetAtTime time constant for the turn mix (about three of these to settle). */
export const MUSIC_TURN_TC_S = 0.3;
/** Ducking: the music's gain under an important sound, and how fast it dips and recovers. */
export const MUSIC_DUCK_GAIN = 0.6;
export const MUSIC_DUCK_ATTACK_TC_S = 0.03;
export const MUSIC_DUCK_RELEASE_TC_S = 0.25;
/** The effects the music ducks under, besides every voice line. */
export const MUSIC_DUCK_SFX: readonly SfxId[] = ["trapSting", "entrance", "victory", "defeat"];
/** Focus: how fast the music fades out when the page is hidden or blurred, and back. */
export const MUSIC_FOCUS_TC_S = 0.15;
/** Low health: the viewer's hero at or under this fraction of HERO_HEALTH plays the urgency track. */
export const MUSIC_LOW_HEALTH_FRACTION = 0.25;
/** A Mythic theme away from low health ends after it has played through this many times. */
export const MUSIC_MYTHIC_PLAYS = 2;
/** Decoded tracks kept in memory (a two-minute track decodes to about 45 MB), least recently used evicted. */
export const MUSIC_DECODED_MAX = 3;
/** Fetched files kept, compressed, least recently used evicted. */
export const MUSIC_BYTES_MAX = 8;
/** §10.11's cap on the rendered music, counted in whole disk blocks (gen-music.mjs BUDGET_BYTES). */
export const MUSIC_BUDGET_BYTES = 24 * 1024 * 1024;

// ---- Patch v0.2.X sound polish (#259, R669): stings, the mix's panning, ducking and reverb ----
/** A played card's rarity sting starts this long after the card whoosh, so the two read as one. */
export const STING_DELAY_MS = 40;
/** The pan of a sound about a unit in the outermost lane; the middle lane is centred. */
export const LANE_PAN_MAX = 0.6;
/** The effects' gain under a voice line, and how fast they dip and recover (the music's duck, for the effects). */
export const SFX_VOICE_DUCK_GAIN = 0.7;
export const SFX_DUCK_ATTACK_TC_S = 0.03;
export const SFX_DUCK_RELEASE_TC_S = 0.2;
/** The shared reverb: a small room's impulse, this long, its noise falling off by this power. */
export const REVERB_SECONDS = 1.1;
export const REVERB_DECAY_POWER = 3;
/** The impulse is stereo: each channel its own noise, so the room is wide around a centred sound. */
export const REVERB_CHANNELS = 2;
/** How much of each bus the reverb hears: the effects a little, the voice lines less. */
export const REVERB_SFX_SEND = 0.3;
export const REVERB_VOICE_SEND = 0.3;
/** The impulse's noise is filled from this seed, so every run of the mix is identical. */
export const REVERB_SEED = 0x52564242; // "RVBB"
