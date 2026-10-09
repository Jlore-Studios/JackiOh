// The mix bus (docs/polish/2-sound.md, "engine.ts"; B4, B51): the graph every cue plays through.
//
//   per-cue gain ─▶ (lane pan) ─▶ sfx bus ─▶ sfx duck ──┬───────────────┐
//   persona gain ─▶ voice bus ──────────────────────────┼───────────────┤
//                                  reverb sends ◀───────┘─▶ reverb ─────┤
//   music player ─▶ music bus ─▶ music duck ────────────────────────────┴─▶ master ─▶ limiter ─▶ destination
//
// The music bus carries the music volume (R631); its duck is the engine's, which dips it under a
// voice line or an important effect while `duckMusic` is on. R669: the effects have a duck of their
// own, dipped under every voice line, and a cue about a unit is panned to its lane before the bus.
// The effects (after their duck) and the voice lines each feed a little of themselves into one
// shared reverb; the music, mixed with its own room, does not.
//
// The limiter is a DynamicsCompressor set as a peak catcher: a hard knee at LIMITER's threshold, so
// a lone sound passes at one fixed gain and only a pile-up is held back (the default 30 dB knee
// takes a few dB off every effect). Measured in Chrome (audio-recipes.cy.tsx, B57): its automatic
// makeup gain is the same for every sound, so a -6 dB threshold lifts effects and lines alike and
// keeps the densest scene under full scale. The attack stays at 3 ms: at 1 ms Chrome's detector
// clips 4 to 6 dB off short transients.
//
// Imports only ./types.ts and ./constants.ts, like sfx.ts, so the Cypress component spec can render
// the real mix in an OfflineAudioContext.

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
  /** R669: the effects' duck under a voice line, after the effects bus. */
  sfxDuck: GainNode;
  crowd: GainNode;
  voice: GainNode;
  music: GainNode;
  musicDuck: GainNode;
  limiter: DynamicsCompressorNode;
  /** R669: the shared reverb. */
  reverb: ConvolverNode;
};

/**
 * R669: the shared reverb's impulse: REVERB_SECONDS of seeded white noise on each channel (a
 * different seed each, so it is wide), falling off by REVERB_DECAY_POWER, with no early reflections.
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
export function mixLevels(s: AudioSettings): { master: number; sfx: number; crowd: number; voice: number; music: number } {
  return { master: s.muted ? 0 : s.master, sfx: s.sfx, crowd: s.crowd, voice: s.voiceOn ? s.voice : 0, music: s.music };
}

/** Builds the buses and the limiter into `ctx.destination`, at `settings`' levels. */
export function buildMix(ctx: BaseAudioContext, settings: AudioSettings): Mix {
  const master = ctx.createGain();
  const sfx = ctx.createGain();
  const sfxDuck = ctx.createGain();
  const crowd = ctx.createGain();
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
  crowd.connect(master);
  voice.connect(master);
  music.connect(musicDuck);
  musicDuck.connect(master);
  master.connect(limiter);
  limiter.connect(ctx.destination);
  const reverb = buildReverb(ctx, { sfx: sfxDuck, voice }, master);
  const levels = mixLevels(settings);
  master.gain.value = levels.master;
  sfx.gain.value = levels.sfx;
  crowd.gain.value = levels.crowd;
  voice.gain.value = levels.voice;
  music.gain.value = levels.music;
  return { master, sfx, sfxDuck, crowd, voice, music, musicDuck, limiter, reverb };
}
