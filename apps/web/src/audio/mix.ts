// The mix bus (docs/polish/2-sound.md, "engine.ts"; B4, B51): the graph every cue plays through.
//
//   per-cue gain ─▶ (lane pan) ─▶ sfx bus ─▶ sfx duck ──┬───────────────┐
//   persona gain ─▶ voice bus ──────────────────────────┼───────────────┤
//                                  reverb sends ◀───────┘─▶ reverb ─────┤
//   music player ─▶ music bus ─▶ music duck ────────────────────────────┴─▶ master ─▶ limiter ─▶ destination
//
// The music bus carries the music volume (R631); the duck under it is the engine's, which dips it
// under a voice line or an important effect while `duckMusic` is on. R658 (#259): the effects have
// a duck of their own, which the engine dips under every voice line so a line is never buried under
// the board, and a cue about a unit is panned to its lane before the bus (the engine adds the
// panner). The effects (after their duck) and the voice lines each feed a little of themselves into
// one shared reverb, a small room's impulse built here from seeded noise, so every sound sits in the
// same space; the music, which is mixed with its own room, does not.
//
// The limiter is a DynamicsCompressor set as a peak catcher for the sum: a hard knee at LIMITER's
// threshold, so a lone sound passes through at one fixed gain and only a pile-up (a board wipe's
// hits under a death line) is held back. Left at the Web Audio default 30 dB knee, the same node
// squeezes everything from 36 dB under its threshold and takes a few dB off every single effect,
// which is how the mix came out quieter than its recipes.
//
// Measured in Chrome (audio-recipes.cy.tsx, B57): its compressor adds automatic makeup gain, the
// same for every sound, so a lower threshold lifts effects and lines alike and leaves their balance
// alone; a -6 dB threshold keeps the densest scene the director can play under full scale. The
// attack stays at 3 ms: at 1 ms Chrome's detector clips 4 to 6 dB off short transients (a hit, a
// card draw, a UI tick) that are nowhere near the threshold.
//
// This file imports only ./types.ts and ./constants.ts, like sfx.ts, so the Cypress component spec can render the
// real mix in an OfflineAudioContext and measure what a player hears at the default settings.

import {
  REVERB_CHANNELS,
  REVERB_DECAY_POWER,
  REVERB_SECONDS,
  REVERB_SEED,
  REVERB_SFX_SEND,
  REVERB_VOICE_SEND,
} from "./constants.ts";
import type { AudioSettings } from "./types.ts";

/** The master limiter: catches a board wipe's pile-up before it clips, and leaves a lone cue alone. */
export const LIMITER = { thresholdDb: -6, kneeDb: 0, ratio: 20, attackS: 0.003, releaseS: 0.25 } as const;

export type Mix = {
  master: GainNode;
  sfx: GainNode;
  /** R658: the effects' duck under a voice line, after the effects bus. */
  sfxDuck: GainNode;
  voice: GainNode;
  music: GainNode;
  musicDuck: GainNode;
  limiter: DynamicsCompressorNode;
  /** R658: the shared reverb. */
  reverb: ConvolverNode;
};

/**
 * R658: the shared reverb's impulse: REVERB_SECONDS of seeded white noise on each of its channels (a different seed each, so it is wide), falling
 * off by REVERB_DECAY_POWER, a small room with no early reflections to colour a lone hit.
 */
export function reverbImpulse(ctx: BaseAudioContext): AudioBuffer {
  const length = Math.max(1, Math.round(REVERB_SECONDS * ctx.sampleRate));
  const buffer = ctx.createBuffer(REVERB_CHANNELS, length, ctx.sampleRate);
  for (let c = 0; c < buffer.numberOfChannels; c += 1) {
    let seed = (REVERB_SEED + c) | 0;
    const data = buffer.getChannelData(c);
    for (let i = 0; i < length; i += 1) {
      // xorshift32: the impulse is the same on every run, and needs no Math.random.
      seed ^= seed << 13;
      seed ^= seed >>> 17;
      seed ^= seed << 5;
      const noise = (seed >>> 0) / 4294967296;
      data[i] = (noise * 2 - 1) * (1 - i / length) ** REVERB_DECAY_POWER;
    }
  }
  return buffer;
}

/** The shared reverb and its two sends, into `master`. */
function buildReverb(ctx: BaseAudioContext, from: { sfx: AudioNode; voice: AudioNode }, master: AudioNode): ConvolverNode {
  const reverb = ctx.createConvolver();
  reverb.buffer = reverbImpulse(ctx);
  const sfxSend = ctx.createGain();
  sfxSend.gain.value = REVERB_SFX_SEND;
  const voiceSend = ctx.createGain();
  voiceSend.gain.value = REVERB_VOICE_SEND;
  from.sfx.connect(sfxSend);
  from.voice.connect(voiceSend);
  sfxSend.connect(reverb);
  voiceSend.connect(reverb);
  reverb.connect(master);
  return reverb;
}

/** Bus levels for a settings value: mute silences master, and voice lines off silence the voice bus. */
export function mixLevels(s: AudioSettings): { master: number; sfx: number; voice: number; music: number } {
  return { master: s.muted ? 0 : s.master, sfx: s.sfx, voice: s.voiceOn ? s.voice : 0, music: s.music };
}

/** Builds the buses and the limiter into `ctx.destination`, at `settings`' levels. */
export function buildMix(ctx: BaseAudioContext, settings: AudioSettings): Mix {
  const master = ctx.createGain();
  const sfx = ctx.createGain();
  const sfxDuck = ctx.createGain();
  const voice = ctx.createGain();
  const music = ctx.createGain();
  const musicDuck = ctx.createGain();
  const limiter = ctx.createDynamicsCompressor();
  limiter.threshold.value = LIMITER.thresholdDb;
  limiter.knee.value = LIMITER.kneeDb;
  limiter.ratio.value = LIMITER.ratio;
  limiter.attack.value = LIMITER.attackS;
  limiter.release.value = LIMITER.releaseS;
  sfx.connect(sfxDuck);
  sfxDuck.connect(master);
  voice.connect(master);
  music.connect(musicDuck);
  musicDuck.connect(master);
  master.connect(limiter);
  limiter.connect(ctx.destination);
  const reverb = buildReverb(ctx, { sfx: sfxDuck, voice }, master);
  const levels = mixLevels(settings);
  master.gain.value = levels.master;
  sfx.gain.value = levels.sfx;
  voice.gain.value = levels.voice;
  music.gain.value = levels.music;
  return { master, sfx, sfxDuck, voice, music, musicDuck, limiter, reverb };
}
