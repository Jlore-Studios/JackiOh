// The audio numbers' contract (audio/constants.ts, CLAUDE.md rule 9): every number another
// module or a test reads is named here, so a silent change to one is a contract change. Each value
// below is the shipped one; a mutant moving any of them fails here by name.

import { describe, expect, it } from "vitest";

import {
  AUDIO_SETTINGS_KEY,
  BANNED_RULES_WORDS,
  BANNED_WORDS_MAX_SHARE,
  BLOOD_BEAN_DEF_ID,
  CARD_EFFECT_DELAY_MS,
  CARD_HOOK_NAMES,
  CARD_HOOKS,
  CHAOS_REVEAL_MAX,
  CLOCK_ALARM_BEAT_MS,
  CLOCK_ALARM_FROM_MS,
  CLOCK_ALARM_LATE_MS,
  CLOCK_ALARM_LOOKAHEAD_MS,
  CLOCK_ALARM_SHARP_FROM_MS,
  DEATH_VOICE_DELAY_MS,
  EFFECT_PITCH_JITTER,
  FLUSH_GAP_MS,
  FLUSH_MAX_SFX,
  GAIN_SMOOTHING_S,
  GOLD_BURST_DELAY_MS,
  HIDDEN_DEF_ID,
  HINDER_DEF_ID,
  IMPACT_AMOUNT_CAP,
  IMPACT_HEADROOM,
  LANE_PAN_MAX,
  LOG_LIMIT,
  MUSIC_BAR_WAIT_MAX_S,
  MUSIC_BUDGET_BYTES,
  MUSIC_BYTES_MAX,
  MUSIC_DECODED_MAX,
  MUSIC_DUCK_ATTACK_TC_S,
  MUSIC_DUCK_GAIN,
  MUSIC_DUCK_RELEASE_TC_S,
  MUSIC_DUCK_SFX,
  MUSIC_FADE_S,
  MUSIC_FOCUS_TC_S,
  MUSIC_HANDOFF_FADE_S,
  MUSIC_LEAD_S,
  MUSIC_LOW_HEALTH_FRACTION,
  MUSIC_OPEN_LOWPASS_HZ,
  MUSIC_OPPONENT_GAIN,
  MUSIC_OPPONENT_LOWPASS_HZ,
  MUSIC_ROTATION_KEY,
  MUSIC_STATIONS,
  MUSIC_TURN_TC_S,
  NEXT_REFRESH_MODIFIER_ID,
  PAIR_OFFSET_MS,
  PICKUP_MIN_GAP_MS,
  PLAY_IMPACT_DELAY_MS,
  PLAY_IMPACT_OPPONENT_GAIN,
  PLAY_STACK_MAX,
  QUIET_SFX_GAIN,
  RAPID_SFX,
  REVERB_CHANNELS,
  REVERB_DECAY_POWER,
  REVERB_SECONDS,
  REVERB_SEED,
  REVERB_SFX_SEND,
  REVERB_VOICE_SEND,
  RIFFLE_SOUND_DELAY_MS,
  SCHEDULED_SFX,
  SFX_MAX_VOICES,
  SFX_RETRIGGER_MS,
  SFX_VOICE_DUCK_GAIN,
  STING_DELAY_MS,
  UI_HOVER_THROTTLE_MS,
  VOICE_BUDGET_BYTES,
  VOICE_DECODED_MAX,
  VOICE_DELAY_MS,
  VOICE_FADE_S,
  VOICE_FILE_MAX_MS,
  VOICE_LATE_MS,
  VOICE_MAX_WORDS,
  VOICE_PREVIEW_DEF_ID,
  VOICE_PREFETCH_CONCURRENCY,
  VOICE_PREFETCH_DELAY_MS,
  VOICE_PRELOAD_MAX,
  VOICE_PRIORITY,
  VOICE_QUEUE_MAX,
  VOICE_QUEUE_WAIT_MS,
  VOICE_SPEECH_MAX_MS,
  VOICE_TRIM_LEAD_S,
  VOICE_TRIM_TAIL_S,
  VOICE_TRIM_THRESHOLD,
} from "./constants.ts";

describe("audio constants", () => {
  it("names the card hooks in card-audio.json5 order, with the kinds each may carry", () => {
    expect(CARD_HOOK_NAMES).toEqual(["play", "attack", "death", "cast", "trigger"]);
    expect(CARD_HOOKS.play).toEqual({ kinds: ["unit"] });
    expect(CARD_HOOKS.attack).toEqual({ kinds: ["unit"] });
    expect(CARD_HOOKS.death).toEqual({ kinds: ["unit"] });
    expect(CARD_HOOKS.cast).toEqual({ kinds: ["spell", "trap"] });
    expect(CARD_HOOKS.trigger).toEqual({ kinds: ["spell", "trap", "unit"] });
  });

  it("pins the store key, the log cap and the voice concurrency guards", () => {
    expect(AUDIO_SETTINGS_KEY).toBe("jackioh.audio.v1");
    expect(LOG_LIMIT).toBe(100);
    expect(SFX_MAX_VOICES).toBe(12);
    expect(SFX_RETRIGGER_MS).toBe(40);
    expect(VOICE_LATE_MS).toBe(600);
    expect(PLAY_STACK_MAX).toBe(8);
  });

  it("R655 ranks a pickup over every other line, and caps the queue behind the channel", () => {
    expect(VOICE_PRIORITY).toEqual({ summon: 0, play: 1, react: 2, pickup: 3 });
    expect(VOICE_QUEUE_MAX).toBe(2);
    expect(VOICE_QUEUE_WAIT_MS).toBe(1500);
    expect(VOICE_FADE_S).toBe(0.04);
  });

  it("pins the trim window, the preview line and the preload caps", () => {
    expect(VOICE_TRIM_THRESHOLD).toBe(0.005);
    expect(VOICE_TRIM_LEAD_S).toBe(0.02);
    expect(VOICE_TRIM_TAIL_S).toBe(0.08);
    expect(VOICE_PREVIEW_DEF_ID).toBe("core-008");
    expect(VOICE_DECODED_MAX).toBe(32);
    expect(VOICE_PREFETCH_DELAY_MS).toBe(2000);
    expect(VOICE_PREFETCH_CONCURRENCY).toBe(2);
    expect(VOICE_SPEECH_MAX_MS).toBe(4000);
    expect(VOICE_PRELOAD_MAX).toBe(24);
  });

  it("pins the impact, delay and pairing numbers the cues read", () => {
    expect(IMPACT_AMOUNT_CAP).toBe(10);
    expect(IMPACT_HEADROOM).toBe(0.9);
    expect(GAIN_SMOOTHING_S).toBe(0.015);
    expect(VOICE_DELAY_MS).toBe(150);
    expect(DEATH_VOICE_DELAY_MS).toBe(120);
    expect(PAIR_OFFSET_MS).toBe(220);
    expect(CARD_EFFECT_DELAY_MS).toBe(200);
    expect(EFFECT_PITCH_JITTER).toBe(0.03);
    expect(PICKUP_MIN_GAP_MS).toBe(300);
  });

  it("pins the flush cap R3346000 raised for the cost impact", () => {
    expect(FLUSH_MAX_SFX).toBe(5);
    expect(FLUSH_GAP_MS).toBe(90);
    expect(UI_HOVER_THROTTLE_MS).toBe(80);
  });

  it("pins the voice budget R501 caps and the per-hook word caps", () => {
    expect(VOICE_BUDGET_BYTES).toBe(16 * 1024 * 1024);
    expect(VOICE_FILE_MAX_MS).toBe(4000);
    expect(VOICE_MAX_WORDS).toEqual({ play: 8, attack: 4, death: 6, cast: 8, trigger: 8 });
  });

  it("pins the def ids the web reaches no engine module for", () => {
    expect(HIDDEN_DEF_ID).toBe("hidden");
    expect(HINDER_DEF_ID).toBe("core-021");
    expect(BLOOD_BEAN_DEF_ID).toBe("core-027");
    expect(NEXT_REFRESH_MODIFIER_ID).toBe("nextTurnMana");
    expect(GOLD_BURST_DELAY_MS).toBe(280);
    expect(CHAOS_REVEAL_MAX).toBe(3);
  });

  it("R439 pins the turn clock alarm's schedule", () => {
    expect(CLOCK_ALARM_FROM_MS).toBe(30_000);
    expect(CLOCK_ALARM_SHARP_FROM_MS).toBe(10_000);
    expect(CLOCK_ALARM_BEAT_MS).toBe(1_000);
    expect(CLOCK_ALARM_LOOKAHEAD_MS).toBe(400);
    expect(CLOCK_ALARM_LATE_MS).toBe(250);
  });

  it("R631 pins the stations, the rotation key and the mix schedule", () => {
    expect(MUSIC_STATIONS).toEqual(["tavern", "edm", "lofi", "epic"]);
    expect(MUSIC_ROTATION_KEY).toBe("jackioh.music.rotation.v1");
    expect(MUSIC_FADE_S).toBe(1.5);
    expect(MUSIC_BAR_WAIT_MAX_S).toBe(3);
    expect(MUSIC_LEAD_S).toBe(0.05);
    expect(MUSIC_HANDOFF_FADE_S).toBe(0.05);
    expect(MUSIC_OPPONENT_LOWPASS_HZ).toBe(1400);
    expect(MUSIC_OPPONENT_GAIN).toBe(0.75);
    expect(MUSIC_OPEN_LOWPASS_HZ).toBe(20000);
    expect(MUSIC_TURN_TC_S).toBe(0.3);
    expect(MUSIC_DUCK_GAIN).toBe(0.6);
    expect(MUSIC_DUCK_ATTACK_TC_S).toBe(0.03);
    expect(MUSIC_DUCK_RELEASE_TC_S).toBe(0.25);
  });

  it("R2722000 keeps the entrance out of the duck set, and the taps in the rapid set", () => {
    expect(MUSIC_DUCK_SFX).toEqual(["trapSting", "victory", "defeat"]);
    expect(RAPID_SFX.has("sand")).toBe(true);
    expect(RAPID_SFX.has("tapWood")).toBe(true);
    expect(RAPID_SFX.has("tapNeon")).toBe(true);
    expect(RAPID_SFX.has("uiClick")).toBe(false);
    expect(SCHEDULED_SFX.has("starshipDock")).toBe(true);
    expect(SCHEDULED_SFX.has("sand")).toBe(false);
  });

  it("pins the riffle, quiet-cue, focus and low-health numbers", () => {
    expect(RIFFLE_SOUND_DELAY_MS).toBe(300);
    expect(QUIET_SFX_GAIN).toBe(0.35);
    expect(MUSIC_FOCUS_TC_S).toBe(0.15);
    expect(MUSIC_LOW_HEALTH_FRACTION).toBe(0.25);
    expect(MUSIC_DECODED_MAX).toBe(3);
    expect(MUSIC_BYTES_MAX).toBe(8);
    expect(MUSIC_BUDGET_BYTES).toBe(49 * 1024 * 1024);
  });

  it("pins the v0.2.X polish stings, pans and ducks", () => {
    expect(STING_DELAY_MS).toBe(40);
    expect(PLAY_IMPACT_DELAY_MS).toBe(0);
    expect(PLAY_IMPACT_OPPONENT_GAIN).toBe(0.6);
    expect(LANE_PAN_MAX).toBe(0.6);
    expect(SFX_VOICE_DUCK_GAIN).toBe(0.7);
  });

  it("pins the reverb recipe B57 measured", () => {
    expect(REVERB_SECONDS).toBe(1.1);
    expect(REVERB_DECAY_POWER).toBe(3);
    expect(REVERB_CHANNELS).toBe(2);
    expect(REVERB_SFX_SEND).toBe(0.3);
    expect(REVERB_VOICE_SEND).toBe(0.3);
    expect(REVERB_SEED).toBe(0x52564242);
  });

  it("bans rules vocabulary from voice lines, with the odd one standing (B34)", () => {
    expect(BANNED_RULES_WORDS).toContain("Taunt");
    expect(BANNED_RULES_WORDS).toContain("Embiggened");
    expect(BANNED_RULES_WORDS).toContain("mana");
    expect(BANNED_RULES_WORDS).not.toContain("hello");
    expect(BANNED_WORDS_MAX_SHARE).toBe(0.02);
  });
});
