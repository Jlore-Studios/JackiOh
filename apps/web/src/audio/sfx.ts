// Procedural sound effects (docs/polish/2-sound.md, "sfx.ts"; B14, B15, B16).
//
// One recipe per `SfxId`, in the sfxr tradition: an oscillator or a noise buffer, a filter and a
// gain envelope, plus two-operator FM for bells and chimes. No audio files ship for effects.
//
// The recipe contract every entry keeps:
//   - it schedules nothing before `at` and stops every source it starts by `at + returned`, and
//     `returned <= durationMs / 1000` (every time goes through `time()`, which clamps into the span);
//   - it connects only into `out`, never `ctx.destination`, and its peak output stays at or under 1;
//   - exponential ramps target a positive value (`FLOOR`), never 0;
//   - it uses only the permitted Web Audio subset (gain, oscillator, biquad, buffer source, buffer,
//     connect, start/stop and AudioParam automation), which is all the test fake implements.
//
// This file imports only `./types.ts` and `./constants.ts`, because the Cypress component spec
// imports it on its own and renders each recipe into an OfflineAudioContext.
//
// The frequencies and times below are each recipe's data, like keyframes in `animations.css`, and
// stay local to it (CLAUDE.md rule 9 names only the numbers another module reads).
//
// PITCH (R655). A card's effect plays a recipe shifted in pitch (`renderSfx`): every oscillator and
// filter the run builds is detuned by the same cents, and the crushed wavetable plays that much
// faster or slower. The times stay as written, so a pitched recipe keeps its contract.
//
// LEVELS (B57). The recipes use their headroom (each peaks well inside 1 on its own), and the gains
// in the SFX table set the mix against the voice lines, which sit at about -20 dBFS active RMS at
// the default settings: a maximum hit and the big moments (death, a trap springing, turn start,
// victory, defeat) within a few dB of a line, routine card and board sounds 5 to 9 dB under it, and
// the UI ticks quieter still but plainly audible. The component spec (audio-recipes.cy.tsx)
// renders every recipe through the real mix and holds these bands, so a retune cannot drift.

import { CHAOS_REVEAL_MAX, IMPACT_AMOUNT_CAP, IMPACT_HEADROOM } from "./constants.ts";
import type { SfxId, SfxParams, SfxTimbre } from "./types.ts";
import { damageTier } from "../game/damageFeel.ts";

export const SFX_IDS: readonly SfxId[] = [
  "draw", "play", "summon", "attack", "impact", "shieldShatter", "heal", "buff", "debuff",
  "death", "burn", "trapSet", "trapSting", "spell", "mana", "turnStart", "victory",
  "defeat", "uiClick", "uiHover", "whoosh", "radiant", "lock", "poof", "sand", "endTurn", "notify", "drain",
  "cancel", "entrance", "fatigue", "refuse",
  "manaCrack", "bloodDrain", "goldBurst", "castOnDraw", "chaosRoll", "brand", "heartbeat", "clockTick",
  "emoteSob", "emoteYawn", "emoteLaugh", "emoteAngry", "emoteWahWah",
  "sting",
];

/** Every card family a summon or spell may be given (types.ts SfxTimbre), for the tests. */
export const SFX_TIMBRES: readonly SfxTimbre[] = [
  "human", "felinor", "ky", "cn", "fruit", "chaos", "quickdraw", "token", "field",
  "book", "pancake", "ai",
];

/** Schedules one sound starting at `at` (context seconds) into `out`; returns its length in seconds. */
export type SfxRecipe = (ctx: BaseAudioContext, out: AudioNode, at: number, params: SfxParams) => number;
export type SfxSpec = { recipe: SfxRecipe; /** upper bound over all params */ durationMs: number; gain: number };

/* ------------------------------------------------------------------------------------------- *
 * Noise
 * ------------------------------------------------------------------------------------------- */

const NOISE_SEED = 0x4a41434b; // "JACK"
const noiseCache = new WeakMap<BaseAudioContext, AudioBuffer>();

function mulberry32(seed: number): () => number {
  let a = seed | 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** 1 s of mono white noise, cached per context (WeakMap), filled from a fixed-seed mulberry32 so every run is identical. */
export function noiseBuffer(ctx: BaseAudioContext): AudioBuffer {
  const cached = noiseCache.get(ctx);
  if (cached !== undefined) return cached;
  const length = Math.round(ctx.sampleRate);
  const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
  const random = mulberry32(NOISE_SEED);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i += 1) data[i] = random() * 2 - 1;
  noiseCache.set(ctx, buffer);
  return buffer;
}

/* ------------------------------------------------------------------------------------------- *
 * The bit-crushed wavetable (the AI family, R506)
 * ------------------------------------------------------------------------------------------- */

/** Samples in one cycle of the crushed wave; a looped buffer source plays it at any pitch. */
const CRUSH_CYCLE = 32;
/** Each value is held this many samples: the sample-rate reduction half of a bit crusher. */
const CRUSH_HOLD = 4;
/** Amplitude steps on each side of 0 (about three bits): the bit-depth half. */
const CRUSH_LEVELS = 3;
const crushCache = new WeakMap<BaseAudioContext, AudioBuffer>();

/**
 * One cycle of a sine held in coarse steps and rounded to a few levels, cached per context: the
 * gritty, aliased blip of an old sound chip, built from a buffer since the permitted subset has no
 * wave shaper.
 */
function crushedBuffer(ctx: BaseAudioContext): AudioBuffer {
  const cached = crushCache.get(ctx);
  if (cached !== undefined) return cached;
  const buffer = ctx.createBuffer(1, CRUSH_CYCLE, ctx.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < CRUSH_CYCLE; i += 1) {
    const held = i - (i % CRUSH_HOLD);
    data[i] = Math.round(Math.sin((2 * Math.PI * held) / CRUSH_CYCLE) * CRUSH_LEVELS) / CRUSH_LEVELS;
  }
  crushCache.set(ctx, buffer);
  return buffer;
}

/* ------------------------------------------------------------------------------------------- *
 * Building blocks
 * ------------------------------------------------------------------------------------------- */

/** The quietest level an exponential ramp aims at: -80 dB, silent in practice and never 0. */
const FLOOR = 0.0001;
/** An offset past any recipe's end; `time()` clamps it to the end. */
const UNTIL_END = Number.POSITIVE_INFINITY;

/** One recipe run: its context, its output, its span [at, end] and its pitch shift in cents (R655). */
type Kit = { ctx: BaseAudioContext; out: AudioNode; at: number; end: number; cents: number };

/** A detune of this many cents is an octave. */
const CENTS_PER_OCTAVE = 1200;
/** The pitch shift of the recipe `renderSfx` is running, in cents; 0 outside it. Recipes run synchronously. */
let pitchNow = 0;

function kit(ctx: BaseAudioContext, out: AudioNode, at: number, lengthS: number): Kit {
  return { ctx, out, at, end: at + lengthS, cents: pitchNow };
}

/** `hz` shifted by the run's pitch, for a node with no detune of its own (the crushed wavetable). */
function pitched(k: Kit, hz: number): number {
  return hz * 2 ** (k.cents / CENTS_PER_OCTAVE);
}

/** `at + offset`, clamped into the recipe's span, so nothing lands before `at` or after the end. */
function time(k: Kit, offset: number): number {
  return Math.min(k.end, k.at + Math.max(0, offset));
}

function chain(first: AudioNode, ...rest: AudioNode[]): void {
  let prev = first;
  for (const node of rest) {
    prev.connect(node);
    prev = node;
  }
}

function run(k: Kit, source: AudioScheduledSourceNode, start: number, stop: number): void {
  source.start(time(k, start));
  source.stop(time(k, stop));
}

function oscillator(k: Kit, type: OscillatorType, hz: number, start = 0): OscillatorNode {
  const osc = k.ctx.createOscillator();
  osc.type = type;
  osc.frequency.setValueAtTime(hz, time(k, start));
  if (k.cents !== 0) osc.detune.setValueAtTime(k.cents, time(k, start));
  return osc;
}

function noiseSource(k: Kit): AudioBufferSourceNode {
  const source = k.ctx.createBufferSource();
  source.buffer = noiseBuffer(k.ctx);
  source.loop = true;
  return source;
}

function biquad(k: Kit, type: BiquadFilterType, hz: number, q: number): BiquadFilterNode {
  const filter = k.ctx.createBiquadFilter();
  filter.type = type;
  filter.frequency.setValueAtTime(hz, k.at);
  if (k.cents !== 0) filter.detune.setValueAtTime(k.cents, k.at);
  filter.Q.setValueAtTime(q, k.at);
  return filter;
}

/** A fixed gain from `start` on. */
function level(k: Kit, value: number, start = 0): GainNode {
  const gain = k.ctx.createGain();
  gain.gain.setValueAtTime(value, time(k, start));
  return gain;
}

/** An exponential glide of `param` to `to`, landing at `offset`. */
function glide(k: Kit, param: AudioParam, to: number, offset: number): void {
  param.exponentialRampToValueAtTime(to, time(k, offset));
}

/** Silence until `start`, a linear rise to `peak` over `attack`, then an exponential fall to FLOOR at `stop`. */
function envelope(k: Kit, start: number, attack: number, peak: number, stop: number): GainNode {
  const gain = k.ctx.createGain();
  const top = time(k, start + attack);
  gain.gain.setValueAtTime(0, time(k, start));
  gain.gain.linearRampToValueAtTime(peak, top);
  gain.gain.exponentialRampToValueAtTime(FLOOR, Math.max(top, time(k, stop)));
  return gain;
}

/** Like `envelope`, but it eases from `peak` to `holdLevel` until `holdUntil` before it falls. */
function heldEnvelope(
  k: Kit,
  start: number,
  attack: number,
  peak: number,
  holdUntil: number,
  holdLevel: number,
  stop: number,
): GainNode {
  const gain = k.ctx.createGain();
  const top = time(k, start + attack);
  const hold = Math.max(top, time(k, holdUntil));
  gain.gain.setValueAtTime(0, time(k, start));
  gain.gain.linearRampToValueAtTime(peak, top);
  gain.gain.linearRampToValueAtTime(holdLevel, hold);
  gain.gain.exponentialRampToValueAtTime(FLOOR, Math.max(hold, time(k, stop)));
  return gain;
}

/**
 * A gain that an LFO swings between `base - depth` and `base + depth` for the whole recipe:
 * tremolo with a sine, flutter or crackle with a square.
 */
function modulatedGain(k: Kit, type: OscillatorType, rateHz: number, base: number, depth: number): GainNode {
  const gain = level(k, base);
  const lfo = oscillator(k, type, rateHz);
  const amount = level(k, depth);
  chain(lfo, amount);
  amount.connect(gain.gain);
  run(k, lfo, 0, UNTIL_END);
  return gain;
}

/** A sine tremolo that dips to `1 - 2 * depth` and never rises past 1. */
function tremolo(k: Kit, rateHz: number, depth: number): GainNode {
  return modulatedGain(k, "sine", rateHz, 1 - depth, depth);
}

/** A sine vibrato of ±`depthHz` on `param` between `start` and `stop`. */
function vibrato(k: Kit, param: AudioParam, rateHz: number, depthHz: number, start: number, stop: number): void {
  const lfo = oscillator(k, "sine", rateHz, start);
  const amount = level(k, depthHz, start);
  chain(lfo, amount);
  amount.connect(param);
  run(k, lfo, start, stop);
}

/** A two-operator FM bell: a sine modulator at `ratio` × the carrier, its index decaying with the note. */
function fmBell(
  k: Kit,
  into: AudioNode,
  carrierHz: number,
  ratio: number,
  indexHz: number,
  start: number,
  attack: number,
  peak: number,
  stop: number,
): void {
  const carrier = oscillator(k, "sine", carrierHz, start);
  const modulator = oscillator(k, "sine", carrierHz * ratio, start);
  const index = level(k, indexHz, start);
  glide(k, index.gain, Math.max(1, indexHz * 0.05), stop);
  chain(modulator, index);
  index.connect(carrier.frequency);
  chain(carrier, envelope(k, start, attack, peak, stop), into);
  run(k, carrier, start, stop);
  run(k, modulator, start, stop);
}

/** A plain enveloped tone into `into`. */
function tone(
  k: Kit,
  into: AudioNode,
  type: OscillatorType,
  hz: number,
  start: number,
  attack: number,
  peak: number,
  stop: number,
): OscillatorNode {
  const osc = oscillator(k, type, hz, start);
  chain(osc, envelope(k, start, attack, peak, stop), into);
  run(k, osc, start, stop);
  return osc;
}

/** A bit-crushed blip at `hz` into `into`: the crushed wavetable looped at that pitch, enveloped. */
function crushedBlip(k: Kit, into: AudioNode, hz: number, start: number, stop: number, peak: number): void {
  const source = k.ctx.createBufferSource();
  source.buffer = crushedBuffer(k.ctx);
  source.loop = true;
  source.playbackRate.setValueAtTime((pitched(k, hz) * CRUSH_CYCLE) / k.ctx.sampleRate, time(k, start));
  chain(source, envelope(k, start, 0.002, peak, stop), into);
  run(k, source, start, stop);
}

/**
 * Short clicks on one filtered noise band, at `times` (a reel's ratchet, a clock's escapement):
 * one gain whose envelope strikes and falls again at each time.
 */
function clickTrain(k: Kit, into: AudioNode, times: readonly number[], hz: number, q: number, peak: number, decay: number): void {
  const noise = noiseSource(k);
  const gain = k.ctx.createGain();
  gain.gain.setValueAtTime(0, k.at);
  for (const start of times) {
    const top = time(k, start + 0.001);
    gain.gain.setValueAtTime(0, time(k, start));
    gain.gain.linearRampToValueAtTime(peak, top);
    gain.gain.exponentialRampToValueAtTime(FLOOR, Math.max(top, time(k, start + decay)));
  }
  chain(noise, biquad(k, "bandpass", hz, q), gain, into);
  run(k, noise, 0, (times[times.length - 1] ?? 0) + decay);
}

/** amount → t in [0, 1]: 1 (or none) is 0, IMPACT_AMOUNT_CAP and above is 1. */
function amountT(params: SfxParams): number {
  const clamped = Math.min(IMPACT_AMOUNT_CAP, Math.max(1, params.amount ?? 1));
  return (clamped - 1) / (IMPACT_AMOUNT_CAP - 1);
}

/* ------------------------------------------------------------------------------------------- *
 * Recipes
 * ------------------------------------------------------------------------------------------- */

/** A card slides off the deck: a bright rising noise flick. */
const draw: SfxRecipe = (ctx, out, at) => {
  const len = 0.17;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 2500, 1.2);
  glide(k, band.frequency, 5000, len);
  chain(noise, band, envelope(k, 0, 0.01, 1.6, len), out);
  run(k, noise, 0, len);
  return len;
};

/** A card thrown onto the table: a rising whoosh, then a soft blip as it lands. */
const play: SfxRecipe = (ctx, out, at) => {
  const len = 0.25;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 600, 1.5);
  glide(k, band.frequency, 2400, 0.2);
  chain(noise, band, envelope(k, 0, 0.03, 0.9, 0.22), out);
  run(k, noise, 0, 0.22);
  tone(k, out, "sine", 180, 0.2, 0.005, 0.4, len);
  return len;
};

/** When a summon's family accent starts: after the thud's attack, once its first boom has fallen. */
const ACCENT_AT = 0.05;

/**
 * The family's voice on top of a summon thud, quiet and short (it ends by 0.24 s, inside the
 * shortest thud), so the thud still carries the size of the unit and the accent only says what kind
 * of thing landed: armour for a Human, a chirp for a Felinor, a page's bell for KY, bubbles for CN,
 * a squelch for Fruit, a warble for Call to Chaos, a zip for Quickdraw, a pop for a token.
 */
function summonAccent(k: Kit, timbre: SfxTimbre | undefined): void {
  const t0 = ACCENT_AT;
  switch (timbre) {
    case "human":
      tone(k, k.out, "sine", 2350, t0, 0.002, 0.14, t0 + 0.15);
      tone(k, k.out, "sine", 3520, t0 + 0.01, 0.002, 0.09, t0 + 0.12);
      return;
    case "felinor": {
      const chirp = tone(k, k.out, "sine", 700, t0 + 0.01, 0.01, 0.16, t0 + 0.12);
      glide(k, chirp.frequency, 1400, t0 + 0.08);
      return;
    }
    case "ky":
      fmBell(k, k.out, 1568, 2, 150, t0, 0.004, 0.12, t0 + 0.19);
      return;
    case "cn":
      for (const start of [t0, t0 + 0.08]) {
        const bubble = tone(k, k.out, "sine", 300, start, 0.005, 0.14, start + 0.07);
        glide(k, bubble.frequency, 700, start + 0.06);
      }
      return;
    case "fruit": {
      const noise = noiseSource(k);
      const band = biquad(k, "bandpass", 1800, 4);
      glide(k, band.frequency, 400, t0 + 0.12);
      chain(noise, band, envelope(k, t0, 0.01, 0.35, t0 + 0.12), k.out);
      run(k, noise, t0, t0 + 0.12);
      return;
    }
    case "chaos":
      for (const hz of [523, 554]) tone(k, k.out, "triangle", hz, t0, 0.02, 0.08, t0 + 0.17);
      return;
    case "quickdraw": {
      const noise = noiseSource(k);
      const band = biquad(k, "bandpass", 2000, 3);
      glide(k, band.frequency, 6000, t0 + 0.08);
      chain(noise, band, envelope(k, t0, 0.005, 0.3, t0 + 0.08), k.out);
      run(k, noise, t0, t0 + 0.08);
      return;
    }
    case "token": {
      const pop = tone(k, k.out, "sine", 900, t0 - 0.02, 0.003, 0.14, t0 + 0.04);
      glide(k, pop.frequency, 500, t0 + 0.04);
      return;
    }
    // Patch v0.2.0's tags (R506): a page riffled over a low bell for a Book, a soft plop into a
    // sizzling pan for a Pancake, bit-crushed blips and a servo's whirr for an AI.
    case "book": {
      const noise = noiseSource(k);
      const riffle = modulatedGain(k, "square", 40, 0.5, 0.5);
      chain(noise, biquad(k, "bandpass", 3200, 1.2), riffle, envelope(k, t0, 0.01, 0.45, t0 + 0.09), k.out);
      run(k, noise, t0, t0 + 0.09);
      fmBell(k, k.out, 392, 1.4, 120, t0 + 0.04, 0.004, 0.13, t0 + 0.19);
      return;
    }
    case "pancake": {
      const plop = tone(k, k.out, "sine", 340, t0, 0.004, 0.16, t0 + 0.09);
      glide(k, plop.frequency, 150, t0 + 0.07);
      const noise = noiseSource(k);
      const sizzle = modulatedGain(k, "square", 29, 0.5, 0.5);
      chain(noise, biquad(k, "highpass", 5000, 0), sizzle, envelope(k, t0 + 0.05, 0.02, 0.28, t0 + 0.19), k.out);
      run(k, noise, t0 + 0.05, t0 + 0.19);
      return;
    }
    case "ai": {
      crushedBlip(k, k.out, 1760, t0, t0 + 0.03, 0.1);
      crushedBlip(k, k.out, 2349, t0 + 0.045, t0 + 0.075, 0.1);
      const servo = oscillator(k, "sawtooth", 180, t0 + 0.08);
      glide(k, servo.frequency, 420, t0 + 0.18);
      chain(servo, biquad(k, "bandpass", 900, 2), envelope(k, t0 + 0.08, 0.02, 0.14, t0 + 0.19), k.out);
      run(k, servo, t0 + 0.08, t0 + 0.19);
      return;
    }
    case "field":
    case undefined:
      return;
  }
}

/**
 * A unit lands: a falling sine thud and a puff of dust, sized by the unit (`amount` is its attack
 * plus health): a 1/1 taps the table high and short, a 7/7 lands low, long and loud. A unit the
 * viewer can name adds its family's accent (`timbre`).
 */
const summon: SfxRecipe = (ctx, out, at, params) => {
  const t = amountT(params);
  const len = 0.25 + 0.12 * t;
  const peak = 0.45 + 0.45 * t;
  const k = kit(ctx, out, at, len);
  const fromHz = 170 - 60 * t;
  const thud = tone(k, out, "sine", fromHz, 0, 0.005, peak, len);
  glide(k, thud.frequency, 60 - 25 * t, len * 0.7);
  const noise = noiseSource(k);
  const dust = 0.05 + 0.05 * t;
  chain(noise, biquad(k, "lowpass", 900 - 300 * t, 0), envelope(k, 0, 0.005, peak, dust), out);
  run(k, noise, 0, dust);
  summonAccent(k, params.timbre);
  return len;
};

/** An attack swing: a falling band of noise. */
const attack: SfxRecipe = (ctx, out, at) => {
  const len = 0.23;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 3000, 2);
  glide(k, band.frequency, 700, 0.2);
  chain(noise, band, envelope(k, 0, 0.02, 1.6, len), out);
  run(k, noise, 0, len);
  return len;
};

/** A hit whose weight is chosen by the public damage tier, from a tap to a GIGA board thump. */
const impact: SfxRecipe = (ctx, out, at, params) => {
  // A caller with no tier (a preview, a recipe test) is sized by its amount, capped as before (B16).
  const tier = params.impactTier ?? damageTier(Math.min(params.amount ?? 0, IMPACT_AMOUNT_CAP));
  const tierT = { tiny: 0.1, normal: 0.34, moderate: 0.54, big: 0.76, giga: 1 }[tier];
  const t = tierT;
  const pitch = 0.95 + Math.min(1, Math.max(0, params.variation ?? 0.5)) * 0.1;
  const len = 0.12 + 0.33 * t;
  // The noise and the thump peak together; IMPACT_HEADROOM keeps the sum under full scale (B15).
  const peak = (0.5 + 0.5 * t) * IMPACT_HEADROOM;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "lowpass", 5000 - 3800 * t, 0), envelope(k, 0, 0.004, 1.1 * peak, len), out);
  run(k, noise, 0, len);
  const thumpHz = (110 - 50 * t) * pitch;
  const thump = tone(k, out, "sine", thumpHz, 0, 0.004, 0.75 * peak, len);
  glide(k, thump.frequency, thumpHz * 0.6, len);
  return len;
};

/** A Divine Shield breaks: five glassy partials and a hiss of shards. */
const shieldShatter: SfxRecipe = (ctx, out, at) => {
  const len = 0.48;
  const k = kit(ctx, out, at, len);
  const partials = [2100, 3300, 4700, 5900, 7300];
  partials.forEach((hz, i) => {
    const start = 0.015 * i;
    tone(k, out, "sine", hz, start, 0.003, 0.18, start + 0.3 + 0.03 * i);
  });
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 4000, 0), envelope(k, 0, 0.002, 0.45, 0.08), out);
  run(k, noise, 0, 0.08);
  return len;
};

/** A heal: three FM bells rising through a major triad. */
const heal: SfxRecipe = (ctx, out, at) => {
  const len = 0.68;
  const k = kit(ctx, out, at, len);
  [1047, 1319, 1568].forEach((hz, i) => {
    const start = 0.09 * i;
    fmBell(k, out, hz, 2, 300, start, 0.005, 0.3, start + 0.5);
  });
  return len;
};

/** A buff: a sawtooth rising an octave through an opening filter. */
const buff: SfxRecipe = (ctx, out, at) => {
  const len = 0.4;
  const k = kit(ctx, out, at, len);
  const saw = oscillator(k, "sawtooth", 220);
  glide(k, saw.frequency, 440, 0.35);
  const filter = biquad(k, "lowpass", 1200, 0);
  glide(k, filter.frequency, 3000, 0.35);
  chain(saw, filter, envelope(k, 0, 0.02, 0.53, len), out);
  run(k, saw, 0, len);
  return len;
};

/** A debuff: the buff in reverse, falling and closing. */
const debuff: SfxRecipe = (ctx, out, at) => {
  const len = 0.4;
  const k = kit(ctx, out, at, len);
  const saw = oscillator(k, "sawtooth", 440);
  glide(k, saw.frequency, 200, 0.35);
  const filter = biquad(k, "lowpass", 2000, 0);
  glide(k, filter.frequency, 700, 0.35);
  chain(saw, filter, envelope(k, 0, 0.02, 0.53, len), out);
  run(k, saw, 0, len);
  return len;
};

/** A unit crumbles: fluttering, darkening rubble over a sinking groan. */
const death: SfxRecipe = (ctx, out, at) => {
  const len = 0.62;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const filter = biquad(k, "lowpass", 1200, 0);
  glide(k, filter.frequency, 200, 0.6);
  const flutter = modulatedGain(k, "square", 18, 0.5, 0.5);
  chain(noise, filter, flutter, envelope(k, 0, 0.01, 1.08, len), out);
  run(k, noise, 0, len);
  const groan = tone(k, out, "sine", 90, 0, 0.01, 0.54, len);
  glide(k, groan.frequency, 40, 0.6);
  return len;
};

/** A card burns: gated crackle over a low hiss. */
const burn: SfxRecipe = (ctx, out, at) => {
  const len = 0.58;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const crackle = modulatedGain(k, "square", 23, 0.5, 0.5);
  chain(noise, biquad(k, "highpass", 1500, 0), crackle, envelope(k, 0, 0.01, 0.6, len), out);
  chain(noise, biquad(k, "bandpass", 800, 1), envelope(k, 0, 0.05, 0.66, len), out);
  run(k, noise, 0, len);
  return len;
};

/** A trap is set face-down: a papery slap and a small tick. */
const trapSet: SfxRecipe = (ctx, out, at) => {
  const len = 0.15;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "bandpass", 1200, 3), envelope(k, 0, 0.003, 1.45, 0.09), out);
  run(k, noise, 0, 0.09);
  tone(k, out, "sine", 220, 0, 0.003, 0.58, len);
  return len;
};

/** A trap springs: a dissonant square pair closing down, with a high ping on top. */
const trapSting: SfxRecipe = (ctx, out, at) => {
  const len = 0.66;
  const k = kit(ctx, out, at, len);
  const filter = biquad(k, "lowpass", 3000, 0);
  glide(k, filter.frequency, 600, 0.6);
  const env = envelope(k, 0, 0.01, 0.31, len);
  chain(filter, env, out);
  for (const hz of [311, 330]) {
    const sq = oscillator(k, "square", hz);
    chain(sq, filter);
    run(k, sq, 0, len);
  }
  tone(k, out, "sine", 1245, 0, 0.003, 0.43, 0.5);
  return len;
};

/**
 * Each family's four chimes, their FM ratio and their spacing (the default is the plain spell): a
 * Field Spell rings an octave lower and warmer, Call to Chaos clashes in semitones, KY climbs a
 * major arpeggio, CN sours on a tritone, a Quickdraw spell runs its notes twice as fast; a Book
 * tolls lower on bell-like partials, a Pancake rings warm and pure, an AI chirps inharmonic data
 * notes in a quick scatter. The peak and the span are the plain spell's, so a family changes the
 * colour and never the level.
 */
const SPELL_CHIMES: Readonly<Record<SfxTimbre | "plain", { hz: readonly number[]; ratio: number; step: number }>> = {
  plain: { hz: [1319, 1760, 2093, 2637], ratio: 3.5, step: 0.06 },
  field: { hz: [659, 880, 1047, 1319], ratio: 2, step: 0.06 },
  chaos: { hz: [1319, 1397, 1976, 2093], ratio: 3.5, step: 0.06 },
  ky: { hz: [1047, 1319, 1568, 2093], ratio: 2, step: 0.06 },
  cn: { hz: [1245, 1319, 1760, 1865], ratio: 3.5, step: 0.06 },
  quickdraw: { hz: [1319, 1760, 2093, 2637], ratio: 3.5, step: 0.03 },
  felinor: { hz: [1568, 2093, 2349, 3136], ratio: 3.5, step: 0.06 },
  fruit: { hz: [1175, 1480, 1760, 2349], ratio: 3.5, step: 0.06 },
  human: { hz: [1047, 1568, 2093, 2637], ratio: 2, step: 0.06 },
  token: { hz: [1319, 1760, 2093, 2637], ratio: 3.5, step: 0.06 },
  book: { hz: [784, 988, 1175, 1568], ratio: 1.4, step: 0.07 },
  pancake: { hz: [880, 1109, 1319, 1760], ratio: 1, step: 0.07 },
  ai: { hz: [1568, 2349, 1976, 3136], ratio: 4.5, step: 0.035 },
};

/**
 * What a family lays under its chimes, quietly (R506): a Book's page turning as it opens, a
 * Pancake's sizzle under a syrupy glide, an AI's bit-crushed data blips and a servo settling.
 */
function spellTexture(k: Kit, timbre: SfxTimbre | undefined): void {
  switch (timbre) {
    case "book": {
      const noise = noiseSource(k);
      const riffle = modulatedGain(k, "square", 38, 0.5, 0.5);
      chain(noise, biquad(k, "bandpass", 3200, 1.2), riffle, envelope(k, 0, 0.02, 0.5, 0.14), k.out);
      run(k, noise, 0, 0.14);
      return;
    }
    case "pancake": {
      const noise = noiseSource(k);
      const sizzle = modulatedGain(k, "square", 27, 0.5, 0.5);
      chain(noise, biquad(k, "highpass", 5000, 0), sizzle, envelope(k, 0, 0.03, 0.2, 0.5), k.out);
      run(k, noise, 0, 0.5);
      const syrup = oscillator(k, "triangle", 660, 0.1);
      glide(k, syrup.frequency, 392, 0.6);
      vibrato(k, syrup.frequency, 5, 8, 0.1, 0.62);
      chain(syrup, biquad(k, "lowpass", 1400, 0), envelope(k, 0.1, 0.08, 0.1, 0.62), k.out);
      run(k, syrup, 0.1, 0.62);
      return;
    }
    case "ai": {
      [2093, 3136, 2637].forEach((hz, i) => {
        crushedBlip(k, k.out, hz, 0.03 * i, 0.03 * i + 0.025, 0.08);
      });
      const servo = oscillator(k, "sawtooth", 220, 0.12);
      glide(k, servo.frequency, 520, 0.3);
      glide(k, servo.frequency, 330, 0.5);
      chain(servo, biquad(k, "bandpass", 1100, 2), envelope(k, 0.12, 0.04, 0.12, 0.52), k.out);
      run(k, servo, 0.12, 0.52);
      return;
    }
    default:
      return;
  }
}

/** A spell is cast: four FM chimes with a shimmering tremolo, in its family's colour. */
const spell: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.78;
  const k = kit(ctx, out, at, len);
  const shimmer = tremolo(k, 7, 0.15);
  chain(shimmer, out);
  const chimes = SPELL_CHIMES[params.timbre ?? "plain"];
  chimes.hz.forEach((hz, i) => {
    const start = chimes.step * i;
    fmBell(k, shimmer, hz, chimes.ratio, 200, start, 0.004, 0.22, 0.6 + start);
  });
  spellTexture(k, params.timbre);
  return len;
};

/** A mana crystal fills: a pluck and its octave, higher on your own side. */
const mana: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.25;
  const k = kit(ctx, out, at, len);
  const hz = params.mine === true ? 660 : 440;
  tone(k, out, "sine", hz, 0, 0.01, 0.5, len);
  tone(k, out, "sine", hz * 2, 0, 0.01, 0.2, 0.18);
  return len;
};

/** A turn begins: one bell, or a bell and its fifth when the turn is yours. */
const turnStart: SfxRecipe = (ctx, out, at, params) => {
  const len = 1.15;
  const k = kit(ctx, out, at, len);
  fmBell(k, out, 392, 2, 250, 0, 0.005, 0.5, len);
  if (params.mine === true) fmBell(k, out, 587, 2, 200, 0.06, 0.005, 0.35, len);
  return len;
};

/**
 * Victory: a rising brass arpeggio that lands on a full C major chord over a low root, held and
 * left to ring: the fanfare has to sound bigger than anything the game played before it.
 */
const victory: SfxRecipe = (ctx, out, at) => {
  const len = 1.55;
  const k = kit(ctx, out, at, len);
  const filter = biquad(k, "lowpass", 3200, 0);
  chain(filter, out);
  [523, 659, 784].forEach((hz, i) => {
    const start = 0.12 * i;
    tone(k, filter, "sawtooth", hz, start, 0.01, 0.18, start + 0.26);
  });
  const land = 0.36;
  for (const [hz, peak] of [[523, 0.1], [659, 0.085], [784, 0.085], [1047, 0.1]] as const) {
    const note = oscillator(k, "sawtooth", hz, land);
    chain(note, heldEnvelope(k, land, 0.02, peak, 1.1, peak * 0.75, len), filter);
    run(k, note, land, len);
  }
  const root = oscillator(k, "triangle", 262, land);
  chain(root, heldEnvelope(k, land, 0.03, 0.21, 1.1, 0.17, len), out);
  run(k, root, land, len);
  const sub = oscillator(k, "sine", 131, land);
  chain(sub, heldEnvelope(k, land, 0.04, 0.18, 1.1, 0.13, len), out);
  run(k, sub, land, len);
  return len;
};

/** Defeat: four falling triangle notes, the last one wavering. */
const defeat: SfxRecipe = (ctx, out, at) => {
  const len = 1.55;
  const k = kit(ctx, out, at, len);
  [392, 370, 349].forEach((hz, i) => {
    const start = 0.25 * i;
    tone(k, out, "triangle", hz, start, 0.02, 0.4, start + 0.28);
  });
  const last = oscillator(k, "triangle", 311, 0.75);
  vibrato(k, last.frequency, 5, 6, 0.75, len);
  chain(last, heldEnvelope(k, 0.75, 0.02, 0.45, 1.25, 0.35, len), out);
  run(k, last, 0.75, len);
  return len;
};

/** A UI click: a short high sine and a tick of noise. */
const uiClick: SfxRecipe = (ctx, out, at) => {
  const len = 0.045;
  const k = kit(ctx, out, at, len);
  tone(k, out, "sine", 1800, 0, 0.004, 0.6, 0.044);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 3000, 0), envelope(k, 0, 0.001, 0.25, 0.01), out);
  run(k, noise, 0, 0.01);
  return len;
};

/** A UI hover: the smallest blip. */
const uiHover: SfxRecipe = (ctx, out, at) => {
  const len = 0.035;
  const k = kit(ctx, out, at, len);
  tone(k, out, "sine", 2600, 0, 0.003, 0.5, len);
  return len;
};

/** Something moves past: a noise band that sweeps up and back down. */
const whoosh: SfxRecipe = (ctx, out, at) => {
  const len = 0.33;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 400, 1.2);
  glide(k, band.frequency, 2000, 0.15);
  glide(k, band.frequency, 500, len);
  chain(noise, band, envelope(k, 0, 0.14, 0.9, len), out);
  run(k, noise, 0, len);
  return len;
};

/** A card turns Radiant: six high glints under a fast shimmer. */
const radiant: SfxRecipe = (ctx, out, at) => {
  const len = 0.72;
  const k = kit(ctx, out, at, len);
  const shimmer = tremolo(k, 11, 0.2);
  chain(shimmer, out);
  [2637, 3136, 3520, 3951, 4699, 5274].forEach((hz, i) => {
    const start = 0.08 * i;
    tone(k, shimmer, "sine", hz, start, 0.003, 0.3, start + 0.32);
  });
  return len;
};

/** A zone locks: a ring-modulated clank and a click. */
const lock: SfxRecipe = (ctx, out, at) => {
  const len = 0.36;
  const k = kit(ctx, out, at, len);
  const ring = level(k, 0);
  const carrier = oscillator(k, "square", 180);
  const modulator = oscillator(k, "square", 270);
  chain(carrier, ring);
  modulator.connect(ring.gain);
  chain(ring, biquad(k, "bandpass", 1500, 1), envelope(k, 0, 0.005, 1.1, len), out);
  run(k, carrier, 0, len);
  run(k, modulator, 0, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 2000, 0), envelope(k, 0, 0.001, 0.88, 0.02), out);
  run(k, noise, 0, 0.02);
  return len;
};

/** A card vanishes (exile, transform, fuse), or sand compresses: a soft dark puff. */
const poof: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.43;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  // Sand passes a deterministic 0..1 variation, mapping to the requested ±8% crunch pitch;
  // absent callers retain the original neutral recipe exactly.
  const pitch = 0.92 + Math.min(1, Math.max(0, params.variation ?? 0.5)) * 0.16;
  const filter = biquad(k, "lowpass", 900 * pitch, 0);
  glide(k, filter.frequency, 300 * pitch, len);
  chain(noise, filter, envelope(k, 0, 0.04, 1.43, len), out);
  run(k, noise, 0, len);
  return len;
};

/** A dry sand crunch: four grain bands, an ±8% pitch sway and a fuller transient as a pile builds. */
const sand: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.24;
  const k = kit(ctx, out, at, len);
  const variant = Math.abs(Math.trunc(params.sandVariant ?? 0)) % 4;
  const baseHz = [520, 670, 820, 980][variant] ?? 520;
  const pitch = 0.92 + Math.min(1, Math.max(0, params.variation ?? 0.5)) * 0.16;
  const build = Math.max(1, Math.min(6, params.sandBuild ?? 1));
  const noise = noiseSource(k);
  const filter = biquad(k, "bandpass", baseHz * pitch, 1.1);
  chain(noise, filter, envelope(k, 0, 0.008, 0.58 + build * 0.08, len), out);
  run(k, noise, 0, len);
  return len;
};

/** A low wooden-and-metal clunk for the end-turn mechanism, distinct from the generic UI tick. */
const endTurn: SfxRecipe = (ctx, out, at) => {
  const len = 0.18;
  const k = kit(ctx, out, at, len);
  const thump = tone(k, out, "sine", 96, 0, 0.003, 0.64, len);
  glide(k, thump.frequency, 54, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "lowpass", 780, 0), envelope(k, 0, 0.002, 0.65, 0.06), out);
  run(k, noise, 0, 0.06);
  return len;
};

/**
 * A notice: two rising blips. An `urgent` one — a question the viewer has to answer, the other
 * seat's draw offer — is a doorbell instead: a bright bell struck twice, a falling major third
 * apart, so it is never taken for the routine blips.
 */
const notify: SfxRecipe = (ctx, out, at, params) => {
  if (params.urgent === true) {
    const len = 0.29;
    const k = kit(ctx, out, at, len);
    fmBell(k, out, 1319, 3.5, 260, 0, 0.004, 0.34, 0.2);
    fmBell(k, out, 1047, 3.5, 260, 0.11, 0.004, 0.38, len);
    return len;
  }
  const len = 0.28;
  const k = kit(ctx, out, at, len);
  tone(k, out, "sine", 880, 0, 0.005, 0.4, 0.14);
  tone(k, out, "sine", 1175, 0.09, 0.005, 0.4, len);
  return len;
};

/** Life drains away: a sinking, wavering sine that grows with the amount, like impact. */
const drain: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.58;
  const k = kit(ctx, out, at, len);
  const peak = (0.5 + 0.5 * amountT(params)) * 0.8;
  const osc = oscillator(k, "sine", 300);
  glide(k, osc.frequency, 120, 0.55);
  vibrato(k, osc.frequency, 6, 15, 0, len);
  chain(osc, envelope(k, 0, 0.02, peak, len), out);
  run(k, osc, 0, len);
  return len;
};

/** An attack is called off: a square falling an octave. */
const cancel: SfxRecipe = (ctx, out, at) => {
  const len = 0.24;
  const k = kit(ctx, out, at, len);
  const sq = oscillator(k, "square", 330);
  glide(k, sq.frequency, 165, 0.2);
  chain(sq, biquad(k, "lowpass", 1500, 0), envelope(k, 0, 0.005, 0.42, len), out);
  run(k, sq, 0, len);
  return len;
};

/**
 * A Legendary or Mythic unit enters (cues.ts, with the effects layer's light rays): a low gong under
 * a brass fifth that swells open, and a run of high glints once it has risen. A Mythic's glints are
 * a longer, faster, shimmering climb.
 */
const entrance: SfxRecipe = (ctx, out, at, params) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  fmBell(k, out, 98, 1.4, 200, 0, 0.01, 0.35, len);
  const brass = biquad(k, "lowpass", 600, 0);
  glide(k, brass.frequency, 3000, 0.5);
  chain(brass, heldEnvelope(k, 0, 0.25, 0.32, 0.9, 0.24, len), out);
  for (const hz of [196, 294]) {
    const note = oscillator(k, "sawtooth", hz);
    chain(note, brass);
    run(k, note, 0, len);
  }
  const mythic = params.mythic === true;
  const glints = mythic ? [2093, 2637, 3136, 3520, 4186, 5274] : [1568, 2093, 2637];
  const shimmer = mythic ? tremolo(k, 9, 0.2) : level(k, 1);
  chain(shimmer, out);
  glints.forEach((hz, i) => {
    const start = 0.25 + (mythic ? 0.06 : 0.08) * i;
    tone(k, shimmer, "sine", hz, start, 0.003, 0.1, start + 0.45);
  });
  return len;
};

/**
 * R319: a draw finds the library empty (`fatigue`, before the hit's own impact): two hollow knocks
 * on an empty box, the second lower, and a low sigh that sinks under them.
 */
const fatigue: SfxRecipe = (ctx, out, at) => {
  const len = 0.62;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const knocks: readonly (readonly [number, number])[] = [
    [0, 520],
    [0.16, 400],
  ];
  for (const [start, hz] of knocks) {
    chain(noise, biquad(k, "bandpass", hz * 2, 4), envelope(k, start, 0.002, 1.3, start + 0.08), out);
    tone(k, out, "sine", hz / 3, start, 0.002, 0.7, start + 0.14);
  }
  run(k, noise, 0, 0.26);
  const sigh = tone(k, out, "triangle", 196, 0.22, 0.06, 0.32, len);
  glide(k, sigh.frequency, 98, len);
  return len;
};

/**
 * R319: a full library turns a card away (`libraryOverflow`): a dull two-note "no", falling a
 * fourth, muffled, with a soft thud as the card bounces off.
 */
const refuse: SfxRecipe = (ctx, out, at) => {
  const len = 0.36;
  const k = kit(ctx, out, at, len);
  const muffle = biquad(k, "lowpass", 900, 0.7);
  chain(muffle, out);
  const notes: readonly (readonly [number, number, number])[] = [
    [0, 294, 0.14],
    [0.13, 220, len],
  ];
  for (const [start, hz, stop] of notes) {
    const note = oscillator(k, "square", hz, start);
    chain(note, envelope(k, start, 0.006, 0.5, stop), muffle);
    run(k, note, start, stop);
  }
  tone(k, out, "sine", 90, 0.13, 0.004, 0.55, 0.3);
  return len;
};

/* ------------------------------------------------------------------------------------------- *
 * Patch v0.2.0 (R506): card moments, Call to Chaos's roll (R436), a mark (R437), the clock (R439)
 * ------------------------------------------------------------------------------------------- */

/**
 * #21 Hinder lands on the victim's next refresh: a mana crystal cracks, sharp and glassy, its shards
 * tinkle down, and what is left drops away hollow.
 */
const manaCrack: SfxRecipe = (ctx, out, at) => {
  const len = 0.88;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  // The crack, and a splinter right behind it.
  chain(noise, biquad(k, "highpass", 1800, 0), envelope(k, 0, 0.002, 0.22, 0.06), out);
  chain(noise, biquad(k, "bandpass", 4200, 2), envelope(k, 0.028, 0.002, 0.5, 0.09), out);
  run(k, noise, 0, 0.09);
  tone(k, out, "sine", 3730, 0, 0.002, 0.16, 0.25);
  // The shards: short glassy pings, falling and spreading out.
  const shards: readonly (readonly [number, number])[] = [
    [0.05, 5274], [0.09, 4435], [0.12, 6272], [0.17, 3951], [0.23, 4978], [0.3, 3322], [0.38, 4186],
  ];
  for (const [start, hz] of shards) tone(k, out, "sine", hz, start, 0.002, 0.14, start + 0.12);
  // The hollow drop: a tube-like tone sinking away, over a falling sub.
  const drop = oscillator(k, "triangle", 440, 0.2);
  glide(k, drop.frequency, 110, 0.75);
  const hollow = biquad(k, "bandpass", 700, 3);
  glide(k, hollow.frequency, 260, 0.75);
  chain(drop, hollow, heldEnvelope(k, 0.2, 0.03, 0.9, 0.55, 0.5, len), out);
  run(k, drop, 0.2, len);
  const sub = tone(k, out, "sine", 220, 0.2, 0.03, 0.3, len);
  glide(k, sub.frequency, 55, 0.75);
  return len;
};

/** #27's blood price: a wet, low whoosh draining downward, gurgling as it goes. */
const bloodDrain: SfxRecipe = (ctx, out, at) => {
  const len = 0.55;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const wet = biquad(k, "bandpass", 900, 4);
  glide(k, wet.frequency, 160, 0.5);
  const gurgle = modulatedGain(k, "sine", 11, 0.6, 0.4);
  chain(noise, wet, gurgle, envelope(k, 0, 0.06, 2.2, len), out);
  const body = biquad(k, "lowpass", 1400, 0);
  glide(k, body.frequency, 220, 0.5);
  chain(noise, body, envelope(k, 0, 0.08, 0.8, len), out);
  run(k, noise, 0, len);
  const down = tone(k, out, "sine", 190, 0.02, 0.05, 0.35, len);
  glide(k, down.frequency, 55, 0.5);
  return len;
};

/** #27's reward: gold bursting from the card, a chord of bright chimes struck at once, and sparkle. */
const goldBurst: SfxRecipe = (ctx, out, at) => {
  const len = 0.82;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 6000, 0), envelope(k, 0, 0.003, 0.7, 0.07), out);
  run(k, noise, 0, 0.07);
  const shimmer = tremolo(k, 13, 0.18);
  chain(shimmer, out);
  [1568, 1976, 2349, 3136].forEach((hz, i) => {
    fmBell(k, shimmer, hz, 3.5, 260, 0.01 * i, 0.003, 0.2, 0.7);
  });
  [4186, 4699, 5274].forEach((hz, i) => {
    const start = 0.18 + 0.07 * i;
    tone(k, shimmer, "sine", hz, start, 0.003, 0.12, start + 0.22);
  });
  return len;
};

/** A card cast as it is drawn: it snaps off the deck, flares and pops open, in one quick sting. */
const castOnDraw: SfxRecipe = (ctx, out, at) => {
  const len = 0.36;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 1500, 1.4);
  glide(k, band.frequency, 6500, 0.12);
  chain(noise, band, envelope(k, 0, 0.01, 1.5, 0.13), out);
  run(k, noise, 0, 0.13);
  const zing = oscillator(k, "triangle", 988, 0.06);
  glide(k, zing.frequency, 1976, 0.2);
  chain(zing, biquad(k, "lowpass", 3500, 0), envelope(k, 0.06, 0.01, 0.5, 0.26), out);
  run(k, zing, 0.06, 0.26);
  tone(k, out, "sine", 2637, 0.18, 0.003, 0.3, len);
  return len;
};

/** Call to Chaos's reveal dings, one per effect named, climbing. */
const CHAOS_DINGS: readonly number[] = [1568, 2093, 2637];

/**
 * Call to Chaos rolls (R436): a slot machine's reels spin, ratcheting and slowing under a jangling
 * jingle, and clunk to a stop; then one bright ding for each effect the roll names (`amount`, up to
 * CHAOS_REVEAL_MAX), so the player hears how many it picked as the board shows which.
 */
const chaosRoll: SfxRecipe = (ctx, out, at, params) => {
  const reveals = Math.min(CHAOS_REVEAL_MAX, Math.max(0, Math.round(params.amount ?? 1)));
  const landAt = 0.78;
  const dingGap = 0.2;
  const ring = 0.42;
  const len = reveals === 0 ? landAt + 0.1 : landAt + dingGap * (reveals - 1) + ring;
  const k = kit(ctx, out, at, len);
  // The reels: clicks whose gaps grow as they slow.
  const clicks: number[] = [];
  for (let t = 0, gap = 0.035; t < landAt - 0.04; gap *= 1.16) {
    clicks.push(t);
    t += gap;
  }
  clickTrain(k, out, clicks, 2600, 5, 2.4, 0.025);
  // The jingle, stepping round an arpeggio until the reels land.
  const notes = [1047, 1319, 1568, 2093];
  const jingle = oscillator(k, "square", notes[0] ?? 1047);
  for (let i = 1; 0.05 * i < landAt; i += 1) {
    jingle.frequency.setValueAtTime(notes[i % notes.length] ?? 1047, time(k, 0.05 * i));
  }
  chain(jingle, biquad(k, "lowpass", 2400, 0), heldEnvelope(k, 0, 0.02, 0.14, 0.5, 0.1, landAt), out);
  run(k, jingle, 0, landAt);
  // The clunk as they stop.
  tone(k, out, "sine", 150, landAt - 0.03, 0.003, 0.6, landAt + 0.1);
  CHAOS_DINGS.slice(0, reveals).forEach((hz, i) => {
    const start = landAt + dingGap * i;
    fmBell(k, out, hz, 3.5, 350, start, 0.002, 0.5, start + ring);
  });
  return len;
};

/**
 * A mark settles on a card (R437; #50 K-Pop Fanatic's pending steal): a dark brand sears in, a low
 * beating drone swelling under a hiss, with a cold shimmer of clashing partials on top. With
 * `release` the mark lifts instead: the shimmer alone, rising softly away.
 */
const brand: SfxRecipe = (ctx, out, at, params) => {
  if (params.release === true) {
    const len = 0.45;
    const k = kit(ctx, out, at, len);
    const shimmer = tremolo(k, 9, 0.2);
    chain(shimmer, out);
    const partials: readonly (readonly [number, number])[] = [
      [1245, 1661],
      [1661, 2217],
    ];
    for (const [from, to] of partials) {
      const partial = tone(k, shimmer, "sine", from, 0, 0.12, 0.1, len);
      glide(k, partial.frequency, to, len);
    }
    return len;
  }
  const len = 0.76;
  const k = kit(ctx, out, at, len);
  const drone = biquad(k, "lowpass", 300, 4);
  glide(k, drone.frequency, 1400, 0.25);
  glide(k, drone.frequency, 400, len);
  chain(drone, heldEnvelope(k, 0, 0.06, 0.26, 0.4, 0.19, len), out);
  for (const hz of [110, 116.5]) {
    const saw = oscillator(k, "sawtooth", hz);
    chain(saw, drone);
    run(k, saw, 0, len);
  }
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 3500, 0), envelope(k, 0.01, 0.01, 0.5, 0.22), out);
  run(k, noise, 0.01, 0.22);
  const shimmer = tremolo(k, 13, 0.25);
  chain(shimmer, out);
  [1661, 1760, 2489].forEach((hz, i) => {
    const start = 0.08 + 0.05 * i;
    tone(k, shimmer, "sine", hz, start, 0.02, 0.09, start + 0.5);
  });
  return len;
};

/** R439: a tense heartbeat, lub-dub, once a second while the viewer's own turn clock runs down. */
const heartbeat: SfxRecipe = (ctx, out, at) => {
  const len = 0.42;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const beats: readonly (readonly [number, number, number])[] = [
    [0, 72, 0.9],
    [0.17, 60, 0.7],
  ];
  for (const [start, hz, peak] of beats) {
    const thump = tone(k, out, "sine", hz, start, 0.006, peak, start + 0.2);
    glide(k, thump.frequency, hz * 0.6, start + 0.15);
    chain(noise, biquad(k, "lowpass", 180, 0), envelope(k, start, 0.004, peak, start + 0.05), out);
  }
  run(k, noise, 0, 0.22);
  return len;
};

/**
 * R439: the last ten seconds of the viewer's own turn clock: a sharp tick over a tight heartbeat
 * thump, each second (`amount` 1 at ten left to 10 at one left) higher, louder and shorter.
 */
const clockTick: SfxRecipe = (ctx, out, at, params) => {
  const t = amountT(params);
  const len = 0.3 - 0.08 * t;
  const k = kit(ctx, out, at, len);
  tone(k, out, "sine", 1600 + 1400 * t, 0, 0.001, 0.26 + 0.14 * t, 0.06 - 0.03 * t);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 3000, 0), envelope(k, 0, 0.001, 0.3 + 0.2 * t, 0.012), out);
  run(k, noise, 0, 0.012);
  // The thump lands just behind the click, so their peaks do not stack.
  const thump = tone(k, out, "sine", 90, 0.012, 0.004, 0.38 + 0.2 * t, len);
  glide(k, thump.frequency, 50, len);
  return len;
};

/* ------------------------------------------------------------------------------------------- *
 * R644's emoji emotes (issue §4): five animated-sticker sounds, all synthesized, on the effects
 * channel like every other SFX. Wah Wah is the sad-trombone sting.
 * ------------------------------------------------------------------------------------------- */

/** Sob: a wobbly falling whimper — two little cries, each sliding down and shaking. */
const emoteSob: SfxRecipe = (ctx, out, at) => {
  const len = 1.2;
  const k = kit(ctx, out, at, len);
  const muffle = biquad(k, "lowpass", 1400, 0.7);
  chain(muffle, out);
  for (const [start, from, to, stop] of [
    [0, 620, 380, 0.5],
    [0.55, 520, 300, len],
  ] as const) {
    const cry = oscillator(k, "triangle", from, start);
    glide(k, cry.frequency, to, stop);
    vibrato(k, cry.frequency, 11, 26, start, stop);
    chain(cry, envelope(k, start, 0.02, 0.34, stop), muffle);
    run(k, cry, start, stop);
  }
  return len;
};

/** Yawn: a long falling breath — band-passed noise swelling in and sighing out over a soft tone. */
const emoteYawn: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 700, 0.8);
  chain(noise, band, heldEnvelope(k, 0, 0.35, 0.3, 0.8, 0.12, len), out);
  glide(k, band.frequency, 420, len);
  run(k, noise, 0, len);
  const sigh = oscillator(k, "sine", 330, 0);
  glide(k, sigh.frequency, 190, len);
  chain(sigh, heldEnvelope(k, 0, 0.3, 0.16, 0.75, 0.06, len), out);
  run(k, sigh, 0, len);
  return len;
};

/** Laugh: a quick bouncing "ha-ha" — four short square blips stepping down. */
const emoteLaugh: SfxRecipe = (ctx, out, at) => {
  const len = 0.75;
  const k = kit(ctx, out, at, len);
  const muffle = biquad(k, "lowpass", 2200, 0.7);
  chain(muffle, out);
  const has: readonly (readonly [number, number])[] = [
    [0, 520],
    [0.16, 480],
    [0.32, 440],
    [0.48, 390],
  ];
  for (const [start, hz] of has) {
    tone(k, muffle, "square", hz, start, 0.004, 0.2, start + 0.11);
  }
  return len;
};

/** Angry: a short growl — a low sawtooth and its rattle, buzzing out through a closed filter. */
const emoteAngry: SfxRecipe = (ctx, out, at) => {
  const len = 0.7;
  const k = kit(ctx, out, at, len);
  const muffle = biquad(k, "lowpass", 800, 1.5);
  chain(muffle, out);
  const growl = oscillator(k, "sawtooth", 95, 0);
  vibrato(k, growl.frequency, 25, 14, 0, len);
  chain(growl, envelope(k, 0, 0.03, 0.42, len), muffle);
  run(k, growl, 0, len);
  const rattle = modulatedGain(k, "square", 30, 0.5, 0.5);
  // The grind goes out through its own envelope like every other voice in this file: a raw
  // oscillator stopped at full level both clips against the growl (B15's 1.0 ceiling) and rings
  // the filter past durationMs (B15's silence floor).
  chain(rattle, envelope(k, 0, 0.01, 0.35, len), muffle);
  const grind = oscillator(k, "sawtooth", 190, 0);
  chain(grind, rattle);
  run(k, grind, 0, len);
  return len;
};

/**
 * Wah Wah: the sad trombone. Four notes stepping down a minor third each — wah, wah, wah — and the
 * last dropping a semitone more, held long with vibrato until it trails off.
 */
const emoteWahWah: SfxRecipe = (ctx, out, at) => {
  const len = 1.8;
  const k = kit(ctx, out, at, len);
  const brass = biquad(k, "lowpass", 1600, 1);
  chain(brass, out);
  const notes: readonly (readonly [number, number, number])[] = [
    [0, 233, 0.28],
    [0.28, 196, 0.56],
    [0.56, 165, 0.9],
    [0.9, 156, len],
  ];
  for (const [start, hz, stop] of notes) {
    const note = oscillator(k, "sawtooth", hz, start);
    if (stop === len) vibrato(k, note.frequency, 6, 9, start + 0.2, stop);
    chain(note, heldEnvelope(k, start, 0.04, 0.4, stop - 0.06, 0.3, stop), brass);
    run(k, note, start, stop);
  }
  return len;
};

/* ------------------------------------------------------------------------------------------- *
 * Patch v0.2.X (#259, R669): the play sting
 * ------------------------------------------------------------------------------------------- */

/**
 * R669: a card the viewer can read is played (cues.ts), a sting sized by its rarity under the card
 * whoosh. Common: a bright pluck and its fifth. Rare: a three-note bell arpeggio. Epic: a four-note
 * climb over a shimmering open chord. Legendary and Mythic cards have the entrance instead.
 */
const sting: SfxRecipe = (ctx, out, at, params) => {
  const tier = params.tier ?? "common";
  const len = tier === "epic" ? 0.78 : tier === "rare" ? 0.56 : 0.36;
  const k = kit(ctx, out, at, len);
  if (tier === "common") {
    tone(k, out, "triangle", 784, 0, 0.003, 0.42, 0.2);
    tone(k, out, "triangle", 1175, 0.07, 0.003, 0.34, len);
    return len;
  }
  const notes = tier === "epic" ? [523, 659, 784, 1047] : [659, 831, 988];
  const step = tier === "epic" ? 0.07 : 0.08;
  notes.forEach((hz, i) => {
    const start = step * i;
    fmBell(k, out, hz, 2, hz * 0.6, start, 0.003, tier === "epic" ? 0.19 : 0.22, Math.min(len, start + 0.32));
  });
  if (tier === "epic") {
    const shimmer = tremolo(k, 8, 0.25);
    chain(shimmer, out);
    for (const hz of [262, 392, 523]) tone(k, shimmer, "sine", hz, 0.08, 0.06, 0.1, len);
  }
  return len;
};

export const SFX: { readonly [K in SfxId]: SfxSpec } = {
  draw: { recipe: draw, durationMs: 180, gain: 1 },
  play: { recipe: play, durationMs: 260, gain: 0.82 },
  summon: { recipe: summon, durationMs: 380, gain: 1 },
  attack: { recipe: attack, durationMs: 240, gain: 1 },
  impact: { recipe: impact, durationMs: 450, gain: 1 },
  shieldShatter: { recipe: shieldShatter, durationMs: 500, gain: 1 },
  heal: { recipe: heal, durationMs: 700, gain: 0.66 },
  buff: { recipe: buff, durationMs: 420, gain: 1 },
  debuff: { recipe: debuff, durationMs: 420, gain: 1 },
  death: { recipe: death, durationMs: 650, gain: 1 },
  burn: { recipe: burn, durationMs: 600, gain: 1 },
  trapSet: { recipe: trapSet, durationMs: 160, gain: 1 },
  trapSting: { recipe: trapSting, durationMs: 700, gain: 1 },
  spell: { recipe: spell, durationMs: 800, gain: 0.89 },
  mana: { recipe: mana, durationMs: 260, gain: 0.76 },
  turnStart: { recipe: turnStart, durationMs: 1200, gain: 0.78 },
  victory: { recipe: victory, durationMs: 1600, gain: 1 },
  defeat: { recipe: defeat, durationMs: 1600, gain: 0.6 },
  uiClick: { recipe: uiClick, durationMs: 50, gain: 0.79 },
  uiHover: { recipe: uiHover, durationMs: 40, gain: 0.4 },
  whoosh: { recipe: whoosh, durationMs: 350, gain: 0.76 },
  radiant: { recipe: radiant, durationMs: 900, gain: 0.78 },
  lock: { recipe: lock, durationMs: 400, gain: 1 },
  poof: { recipe: poof, durationMs: 450, gain: 1 },
  sand: { recipe: sand, durationMs: 240, gain: 0.52 },
  endTurn: { recipe: endTurn, durationMs: 180, gain: 0.76 },
  notify: { recipe: notify, durationMs: 300, gain: 0.6 },
  drain: { recipe: drain, durationMs: 600, gain: 0.69 },
  cancel: { recipe: cancel, durationMs: 260, gain: 1 },
  entrance: { recipe: entrance, durationMs: 1400, gain: 0.6 },
  fatigue: { recipe: fatigue, durationMs: 650, gain: 0.45 },
  refuse: { recipe: refuse, durationMs: 400, gain: 0.27 },
  manaCrack: { recipe: manaCrack, durationMs: 900, gain: 0.63 },
  bloodDrain: { recipe: bloodDrain, durationMs: 600, gain: 0.46 },
  goldBurst: { recipe: goldBurst, durationMs: 850, gain: 0.95 },
  castOnDraw: { recipe: castOnDraw, durationMs: 400, gain: 0.62 },
  chaosRoll: { recipe: chaosRoll, durationMs: 1650, gain: 0.5 },
  brand: { recipe: brand, durationMs: 800, gain: 0.39 },
  heartbeat: { recipe: heartbeat, durationMs: 450, gain: 0.34 },
  clockTick: { recipe: clockTick, durationMs: 350, gain: 1 },
  emoteSob: { recipe: emoteSob, durationMs: 1200, gain: 0.8 },
  emoteYawn: { recipe: emoteYawn, durationMs: 1400, gain: 0.8 },
  emoteLaugh: { recipe: emoteLaugh, durationMs: 750, gain: 0.8 },
  emoteAngry: { recipe: emoteAngry, durationMs: 700, gain: 0.8 },
  emoteWahWah: { recipe: emoteWahWah, durationMs: 1800, gain: 0.8 },
  sting: { recipe: sting, durationMs: 800, gain: 1 },
};

/**
 * R655: runs `id`'s recipe shifted by `pitch` (a frequency ratio: 1 as written, 0.5 an octave down)
 * and returns its length in seconds, which is the unshifted recipe's.
 */
export function renderSfx(
  id: SfxId,
  ctx: BaseAudioContext,
  out: AudioNode,
  at: number,
  params: SfxParams = {},
  pitch = 1,
): number {
  pitchNow = CENTS_PER_OCTAVE * Math.log2(pitch);
  try {
    return SFX[id].recipe(ctx, out, at, params);
  } finally {
    pitchNow = 0;
  }
}
