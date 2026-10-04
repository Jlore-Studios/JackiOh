// What a shown emote sounds and reads like (play.ts, R644): a voice-line emote is its portrait's
// own line — played on the voice channel as the `emote-<portrait>` defId at react priority, the
// way R204's death line cuts in — and captioned by a bubble holding that line's text for the
// line's audible span, two to four seconds. An emoji emote is one of the five synthesized effects
// on the effects channel and a two-second sticker with no words. The SoundSink is a two-method
// seam, so the engine here is a pair of spies — no Web Audio is constructed.

import { describe, expect, it, vi } from "vitest";

import {
  EMOJI_EMOTE_IDS,
  PORTRAIT_IDS,
  VOICE_EMOTE_IDS,
  type EmojiEmoteId,
  type PortraitId,
} from "@jackioh/shared";

import { VOICE_PRIORITY } from "../audio/constants.ts";
import type { SfxId, SoundSink } from "../audio/types.ts";
import { CARD_AUDIO, emoteVoiceDef } from "../audio/voiceData.ts";
import {
  EMOTE_BUBBLE_MAX_MS,
  EMOTE_BUBBLE_MIN_MS,
  EMOTE_EMOJI_MS,
} from "./config.ts";
import {
  EMOJI_SFX,
  emoteShowInfo,
  playEmote,
  voiceBubbleMs,
  voiceBubbleText,
} from "./play.ts";

/** A SoundSink that records: exactly what engine.ts implements and a test can spy on. */
function sink(): SoundSink & {
  playSfx: ReturnType<typeof vi.fn>;
  playVoice: ReturnType<typeof vi.fn>;
  playEffect: ReturnType<typeof vi.fn>;
} {
  return {
    playSfx: vi.fn(() => true),
    playVoice: vi.fn(() => true),
    playEffect: vi.fn(() => true),
  };
}

/** The issue §4 mapping: each emoji's own recipe on the effects channel. */
const EMOJI_RECIPE: Record<EmojiEmoteId, SfxId> = {
  sob: "emoteSob",
  yawn: "emoteYawn",
  laugh: "emoteLaugh",
  angry: "emoteAngry",
  wahWah: "emoteWahWah",
};

describe("R644 the voice emotes", () => {
  it("R644 every VOICE_EMOTE_IDS id plays on the voice channel as emote-<portrait>, at react priority", () => {
    const engine = sink();

    for (const portrait of PORTRAIT_IDS) {
      for (const emote of VOICE_EMOTE_IDS) {
        playEmote(engine, portrait, emote);
      }
    }

    expect(engine.playVoice).toHaveBeenCalledTimes(PORTRAIT_IDS.length * VOICE_EMOTE_IDS.length);
    for (const portrait of PORTRAIT_IDS) {
      for (const emote of VOICE_EMOTE_IDS) {
        expect(engine.playVoice).toHaveBeenCalledWith(
          `emote-${portrait}`,
          emote,
          0,
          VOICE_PRIORITY.react,
        );
      }
    }
    // A voice emote never reaches the effects channel.
    expect(engine.playSfx).not.toHaveBeenCalled();
  });

  it("R644 a voice emote's bubble carries its portrait's own line text and holds its audible span", () => {
    for (const portrait of PORTRAIT_IDS) {
      const entry = CARD_AUDIO.emotes[portrait];
      if (entry === undefined) throw new Error(`card-audio.json5 has no emotes.${portrait}`);
      for (const emote of VOICE_EMOTE_IDS) {
        const text = voiceBubbleText(portrait, emote);
        expect(text, `${portrait}.${emote}`).toBe(entry[emote]);
        const info = emoteShowInfo(portrait, emote);
        expect(info.text, `${portrait}.${emote}`).toBe(entry[emote]);
        // §3's 2–4s: the line's words at its persona's rate, clamped.
        expect(info.holdMs, `${portrait}.${emote}`).toBe(voiceBubbleMs(portrait, emote));
        expect(info.holdMs, `${portrait}.${emote}`).toBeGreaterThanOrEqual(EMOTE_BUBBLE_MIN_MS);
        expect(info.holdMs, `${portrait}.${emote}`).toBeLessThanOrEqual(EMOTE_BUBBLE_MAX_MS);
      }
    }
  });

  it("R644 a line the voice table cannot resolve shows no text and holds the bubble minimum", () => {
    const nobody = "nobody" as PortraitId;
    expect(emoteLineCheck(nobody)).toBeUndefined();
    expect(voiceBubbleText(nobody, "oops")).toBeNull();
    expect(voiceBubbleMs(nobody, "oops")).toBe(EMOTE_BUBBLE_MIN_MS);
    expect(emoteShowInfo(nobody, "oops")).toEqual({ text: null, holdMs: EMOTE_BUBBLE_MIN_MS });
  });
});

/** Read back what `play.ts` reads, so the fallback test states its premise. */
function emoteLineCheck(portrait: string): unknown {
  return CARD_AUDIO.emotes[portrait];
}

describe("R644 the emoji emotes", () => {
  it("R644 every EMOJI_EMOTE_IDS id plays its own synthesized effect on the effects channel", () => {
    const engine = sink();

    for (const emote of EMOJI_EMOTE_IDS) {
      playEmote(engine, "gary", emote);
    }

    expect(engine.playSfx).toHaveBeenCalledTimes(EMOJI_EMOTE_IDS.length);
    for (const emote of EMOJI_EMOTE_IDS) {
      expect(engine.playSfx).toHaveBeenCalledWith(EMOJI_RECIPE[emote]);
      expect(EMOJI_SFX[emote]).toBe(EMOJI_RECIPE[emote]);
    }
    // An emoji has no line: the voice channel is never touched, whichever portrait sent it.
    expect(engine.playVoice).not.toHaveBeenCalled();
  });

  it("R644 an emoji's show is the sticker's two seconds and no text — the same for every portrait", () => {
    for (const portrait of PORTRAIT_IDS) {
      for (const emote of EMOJI_EMOTE_IDS) {
        expect(emoteShowInfo(portrait, emote), `${portrait}.${emote}`).toEqual({
          text: null,
          holdMs: EMOTE_EMOJI_MS,
        });
      }
    }
    expect(EMOTE_EMOJI_MS).toBe(2000);
  });

  it("R644 playVoice is handed the emote-<portrait> defId the voice table resolves", () => {
    // The SoundSink receives the defId verbatim; voiceData's lineFor splits it back off the end.
    for (const portrait of PORTRAIT_IDS) {
      expect(emoteVoiceDef(portrait)).toBe(`emote-${portrait}`);
    }
  });
});
