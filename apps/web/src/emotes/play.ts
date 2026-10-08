// What one emote looks and sounds like once it is shown (issue §3, §4): the voice line's bubble
// text and how long it stays, the emoji's sticker and synth sound, and the call that plays either
// on the right channel. Pure mapping — the session (session.ts) decides whether and when an emote
// shows at all.

import type { EmojiEmoteId, EmoteId, PortraitId, VoiceEmoteId } from "@jackioh/shared";
import { isVoiceEmote } from "@jackioh/shared";

import type { SfxId, SoundSink } from "../audio/types.ts";
import { VOICE_PRIORITY } from "../audio/constants.ts";
import { CARD_AUDIO, emoteLineFor, emoteVoiceDef } from "../audio/voiceData.ts";
import {
  EMOTE_BUBBLE_MAX_MS,
  EMOTE_BUBBLE_MIN_MS,
  EMOTE_EMOJI_MS,
  FELINORS_ECHO_TAIL_MS,
  SAPI_BASELINE_WPM,
} from "./config.ts";

/** Every emoji id's sound on the effects channel: the issue's five (§4), then MN03's fourteen (R1345). */
export const EMOJI_SFX: Record<EmojiEmoteId, SfxId> = {
  sob: "emoteSob",
  yawn: "emoteYawn",
  laugh: "emoteLaugh",
  angry: "emoteAngry",
  wahWah: "emoteWahWah",
  wave: "emoteWave",
  clap: "emoteClap",
  thumbsUp: "emoteThumbsUp",
  facepalm: "emoteFacepalm",
  shrug: "emoteShrug",
  thinking: "emoteThinking",
  heart: "emoteHeart",
  fire: "emoteFire",
  skull: "emoteSkull",
  sweat: "emoteSweat",
  cool: "emoteCool",
  gasp: "emoteGasp",
  salute: "emoteSalute",
  party: "emoteParty",
};

/**
 * How long a voice line's bubble holds (issue §3): its audible length, clamped to 2–4s. Rendered
 * files' real spans aren't in the client, so the estimate is the line's words at its persona's
 * rate — say personas give wpm directly, sapi ones a percent of SAPI_BASELINE_WPM — plus
 * Felinors' echoed tail. When the voice table can't resolve a line the bubble holds the minimum.
 */
export function voiceBubbleMs(portrait: PortraitId, emote: VoiceEmoteId): number {
  const line = emoteLineFor(CARD_AUDIO, portrait, emote);
  if (line === null) return EMOTE_BUBBLE_MIN_MS;
  const words = line.text.split(/\s+/).filter((word) => word.length > 0).length;
  const wpm =
    line.persona.backend === "sapi"
      ? SAPI_BASELINE_WPM * (line.persona.rate / 100)
      : line.persona.rate;
  const spokenMs = wpm > 0 ? (words / wpm) * 60_000 : EMOTE_BUBBLE_MIN_MS;
  const echoMs = portrait === "felinors" ? FELINORS_ECHO_TAIL_MS : 0;
  return Math.min(EMOTE_BUBBLE_MAX_MS, Math.max(EMOTE_BUBBLE_MIN_MS, Math.round(spokenMs + echoMs)));
}

/** The bubble text for a voice emote, or null when the voice table can't resolve it. */
export function voiceBubbleText(portrait: PortraitId, emote: VoiceEmoteId): string | null {
  return emoteLineFor(CARD_AUDIO, portrait, emote)?.text ?? null;
}

/** What an emote shows: voice lines' bubble, emoji's sticker and their spans. */
export function emoteShowInfo(
  portrait: PortraitId,
  emote: EmoteId,
): { text: string | null; holdMs: number } {
  if (!isVoiceEmote(emote)) return { text: null, holdMs: EMOTE_EMOJI_MS };
  return {
    text: voiceBubbleText(portrait, emote),
    holdMs: voiceBubbleMs(portrait, emote),
  };
}

/**
 * Plays an already-shown emote (issue §3, §4): a voice line on the voice channel at react
 * priority — it cuts in on a card line the way R204's death line does — or an emoji's synth on
 * the effects channel. Callers gate muting before this; a muted emote never reaches it.
 */
export function playEmote(engine: SoundSink, portrait: PortraitId, emote: EmoteId): void {
  if (isVoiceEmote(emote)) {
    engine.playVoice(emoteVoiceDef(portrait), emote, 0, VOICE_PRIORITY.react);
  } else {
    engine.playSfx(EMOJI_SFX[emote]);
  }
}
