// Procedural sound effects (docs/polish/2-sound.md, "sfx.ts"; B14, B15, B16): one sfxr-style recipe
// per `SfxId`. It stays within its returned span, connects only into `out` below peak 1, uses positive
// exponential ramps, and relies only on Web Audio that the test fake implements.
// Cypress imports this file alone, so it imports only small leaf modules; local recipe values need no
// constants (CLAUDE.md rule 9).
// PITCH (R655) preserves timings; LEVELS (B57) are tested.

import { CHAOS_REVEAL_MAX, IMPACT_AMOUNT_CAP, IMPACT_HEADROOM } from "./constants.ts";
import type { SfxId, SfxParams, SfxTimbre } from "./types.ts";
import { isAnomalyCode, type AnomalyCode } from "../cards/anomalies.ts";
import { damageTier, PLAY_IMPACT_TIERS, SLAM_PITCH_SPREAD, UNIT_SLAM } from "../game/damageFeel.ts";
import { createRng } from "../fx/rng.ts";

export const SFX_IDS: readonly SfxId[] = [
  "draw", "play", "summon", "attack", "impact", "shieldShatter", "heal", "buff", "debuff",
  "death", "burn", "trapSet", "trapSting", "spell", "mana", "turnStart", "victory",
  "defeat", "uiClick", "uiHover", "whoosh", "radiant", "lock", "poof", "sand", "endTurn", "notify", "drain",
  "cancel", "entrance", "fatigue", "refuse",
  "manaCrack", "bloodDrain", "goldBurst", "castOnDraw", "chaosRoll", "brand", "heartbeat", "clockTick",
  "emoteSob", "emoteYawn", "emoteLaugh", "emoteAngry", "emoteWahWah",
  "sting",
  "armorClank", "armorRing", "overkill", "crumble", "unlock", "steal", "give", "counterspell", "bleat", "fuse",
  "degrade", "upgrade",
  "emoteWave", "emoteClap", "emoteThumbsUp", "emoteFacepalm", "emoteShrug", "emoteThinking", "emoteHeart",
  "emoteFire", "emoteSkull", "emoteSweat", "emoteCool", "emoteGasp", "emoteSalute", "emoteParty",
  "tapWood", "tapFelt", "tapStone", "tapNeon", "tapParchment", "tapAstral",
  "pickUp", "dragOver", "drop", "returnToHand", "deal", "riffle",
  "horn", "domeFlare", "bayDoor", "reactorPulse", "deckShear",
  "organ",
  "anomaly",
  "playImpact", "trapSnap",
  "auraTouch",
  "starshipDock", "starshipIgnite", "starshipRoar", "starshipBeam", "starshipRepeater", "starshipCannon",
  "starshipFlak", "starshipLauncher", "starshipRam", "starshipWreck",
  "inkSummit", "plowHeave", "cursorTear", "seanceChant", "slateRap", "diceGong", "timberLash", "pencilArpeggio",
  "ballotBeep", "lullabySigh", "compactTut", "glitchDrop", "harpGliss", "markerThunder", "stampTrill", "tillRattle",
  "gavelSlam", "dockingChime", "keyboardPing", "synthSnap", "blobDrum", "warpSparkle", "tearChoir", "cackleThunder",
  "kazooRatchet", "thrusterSniff", "marbleGong", "sanctumBell", "cashWhistle", "whisperClink", "crystalRise",
  "bowlBreath", "peakFlute", "ticketHorn", "forgeQuench", "brushGuzheng", "wellGush", "minePick", "jadeZither",
  "windChime", "marketGong", "chalkHush", "porcelainRoar", "bronzeGulp", "modemGuqin", "serverChoir", "dialupStutter",
  "holoBow", "synthPops", "skullMutter", "keyJangle", "caneCrack", "caneThunder", "rootCreak", "chalkGasp",
  "frostPages", "coinCheer", "totemDrums", "templeDie", "golemStomp", "klaxonSlam", "cellSlam", "hangarRev",
  "vaultHiss", "safeChirp", "stoneChime", "magiChord", "lectureRumble", "pressStamp", "corvetteRoar", "popGasp",
  "geigerHum", "wireRatchet", "beamWhine", "rattleSteam", "breechBoom", "flakWind", "crunchJackpot", "choirHulls",
  "hornEcho", "jetSnap", "whiteRing", "hullDrills", "klaxonSputter", "moltenSnarl", "shutterPurr", "venomDrip",
];

/** Every card family a summon or spell may be given (types.ts SfxTimbre), for the tests. */
export const SFX_TIMBRES: readonly SfxTimbre[] = [
  "human", "felinor", "ky", "cn", "fruit", "chaos", "quickdraw", "token", "field",
  "book", "pancake", "ai",
];

/** Schedules one sound starting at `at` (context seconds) into `out`; returns its length in seconds. */
export type SfxRecipe = (ctx: BaseAudioContext, out: AudioNode, at: number, params: SfxParams) => number;
export type SfxSpec = { recipe: SfxRecipe; /** upper bound over all params */ durationMs: number; gain: number };

// Noise

const NOISE_SEED = 0x4a41434b; // "JACK"
const noiseCache = new WeakMap<BaseAudioContext, AudioBuffer>();

/** 1 s of mono white noise, cached per context (WeakMap), filled from a fixed-seed mulberry32 so every run is identical. */
export function noiseBuffer(ctx: BaseAudioContext): AudioBuffer {
  const cached = noiseCache.get(ctx);
  if (cached !== undefined) return cached;
  const length = Math.round(ctx.sampleRate);
  const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
  const random = createRng(NOISE_SEED);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i += 1) data[i] = random() * 2 - 1;
  noiseCache.set(ctx, buffer);
  return buffer;
}

// The bit-crushed wavetable (the AI family, R506)

/** Samples in one cycle of the crushed wave; a looped buffer source plays it at any pitch. */
const CRUSH_CYCLE = 32;
/** Each value is held this many samples: the sample-rate reduction half of a bit crusher. */
const CRUSH_HOLD = 4;
/** Amplitude steps on each side of 0 (about three bits): the bit-depth half. */
const CRUSH_LEVELS = 3;
const crushCache = new WeakMap<BaseAudioContext, AudioBuffer>();

/** A sine cycle held in coarse steps and rounded to a few levels, cached per context (the permitted subset has no wave shaper). */
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

// Building blocks

/** The quietest level an exponential ramp aims at: -80 dB, silent in practice and never 0. */
const FLOOR = 0.0001;
/** An offset past any recipe's end; `time()` clamps it to the end. */
const UNTIL_END = Number.POSITIVE_INFINITY;

/** One recipe run: its context, its output, its span [at, end] and its pitch shift in cents (R655). */
type Kit = { ctx: BaseAudioContext; out: AudioNode; at: number; end: number; cents: number };

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

function level(k: Kit, value: number, start = 0): GainNode {
  const gain = k.ctx.createGain();
  gain.gain.setValueAtTime(value, time(k, start));
  return gain;
}

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

/** A gain that an LFO swings between `base - depth` and `base + depth` for the whole recipe. */
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

/** Short clicks on one filtered noise band at `times`: one gain whose envelope strikes and falls at each. */
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

// Recipes

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
 * The family's voice on a summon thud, quiet and ended by 0.24 s (inside the shortest thud): the thud
 * carries the unit's size, the accent only says what kind landed.
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

/** A unit lands: a thud sized by `amount` (attack plus health), plus its family's accent when the viewer can name it. */
const summon: SfxRecipe = (ctx, out, at, params) => {
  // A landing Unit's tier weighs its thud at a pitch within SLAM_PITCH_SPREAD; without one, its stats do.
  const t = params.slamTier === undefined ? amountT(params) : UNIT_SLAM[params.slamTier].thud;
  const pitch = params.slamTier === undefined ? 1 : 1 + (Math.min(1, Math.max(0, params.variation ?? 0.5)) * 2 - 1) * SLAM_PITCH_SPREAD;
  const len = 0.25 + 0.12 * t;
  const peak = 0.45 + 0.45 * t;
  const k = kit(ctx, out, at, len);
  const fromHz = (170 - 60 * t) * pitch;
  const thud = tone(k, out, "sine", fromHz, 0, 0.005, peak, len);
  glide(k, thud.frequency, 60 - 25 * t, len * 0.7);
  const noise = noiseSource(k);
  const dust = 0.05 + 0.05 * t;
  chain(noise, biquad(k, "lowpass", 900 - 300 * t, 0), envelope(k, 0, 0.005, peak, dust), out);
  run(k, noise, 0, dust);
  summonAccent(k, params.timbre);
  return len;
};

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
  // A caller with no tier (a preview, a recipe test) is sized by its amount, capped (B16).
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

/**
 * R3346000: the impact under a play, by the mana it cost (`playTier`, its weight `t` from
 * PLAY_IMPACT_TIERS) and coloured by the card's type (`playType`): a Unit is a body landing, a Spell a
 * charge and release, a Field Spell the ground settling. Each grows with `t` in length and in voices
 * (a boom under a heavy Unit or Spell, a sub under a massive Field Spell). Its parts peak together
 * under IMPACT_HEADROOM, as `impact` does (B15), and it ends within 0.5 s.
 */
const playImpact: SfxRecipe = (ctx, out, at, params) => {
  const t = PLAY_IMPACT_TIERS[params.playTier ?? "light"].weight;
  const heavy = t >= PLAY_IMPACT_TIERS.heavy.weight;
  const massive = t >= PLAY_IMPACT_TIERS.massive.weight;
  const peak = (0.4 + 0.4 * t) * IMPACT_HEADROOM;
  switch (params.playType ?? "unit") {
    case "unit": {
      const len = 0.14 + 0.3 * t;
      const k = kit(ctx, out, at, len);
      const body = tone(k, out, "triangle", 200 - 90 * t, 0, 0.004, 0.5 * peak, len);
      glide(k, body.frequency, (200 - 90 * t) * 0.45, len);
      const thwack = noiseSource(k);
      chain(thwack, biquad(k, "lowpass", 2400 - 1600 * t, 0), envelope(k, 0, 0.003, 0.35 * peak, 0.05), out);
      run(k, thwack, 0, 0.05);
      if (heavy) {
        const boom = tone(k, out, "sine", 55, 0, 0.006, 0.45 * peak, len);
        glide(k, boom.frequency, 35, len);
      }
      return len;
    }
    case "spell": {
      const charge = 0.03 + 0.07 * t;
      const len = charge + 0.1 + 0.28 * t;
      const k = kit(ctx, out, at, len);
      const soften = biquad(k, "lowpass", 3000, 0);
      soften.connect(out);
      const rise = tone(k, soften, "sawtooth", 300, 0, charge * 0.9, 0.3 * peak, charge + 0.02);
      glide(k, rise.frequency, 1400, charge);
      const crack = noiseSource(k);
      const crackEnd = charge + 0.06 + 0.1 * t;
      chain(crack, biquad(k, "highpass", 1500 - 800 * t, 0), envelope(k, charge, 0.003, 0.6 * peak, crackEnd), out);
      run(k, crack, charge, crackEnd);
      if (heavy) {
        const boom = tone(k, out, "sine", 90, charge, 0.004, 0.55 * peak, len);
        glide(k, boom.frequency, 45, len);
      } else if (t <= PLAY_IMPACT_TIERS.light.weight) {
        fmBell(k, out, 1760, 2, 200, charge, 0.003, 0.3 * peak, len);
      }
      return len;
    }
    case "field": {
      const len = 0.18 + 0.3 * t;
      const k = kit(ctx, out, at, len);
      tone(k, out, "sine", 110 - 30 * t, 0, 0.06 + 0.06 * t, 0.4 * peak, len);
      const rumble = noiseSource(k);
      chain(rumble, biquad(k, "lowpass", 600 - 300 * t, 0), envelope(k, 0, 0.04, 0.4 * peak, len), out);
      run(k, rumble, 0, len);
      if (massive) tone(k, out, "sine", 40, 0, 0.05, 0.35 * peak, len);
      return len;
    }
  }
};

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

const heal: SfxRecipe = (ctx, out, at) => {
  const len = 0.68;
  const k = kit(ctx, out, at, len);
  [1047, 1319, 1568].forEach((hz, i) => {
    const start = 0.09 * i;
    fmBell(k, out, hz, 2, 300, start, 0.005, 0.3, start + 0.5);
  });
  return len;
};

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

const trapSet: SfxRecipe = (ctx, out, at) => {
  const len = 0.15;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "bandpass", 1200, 3), envelope(k, 0, 0.003, 1.45, 0.09), out);
  run(k, noise, 0, 0.09);
  tone(k, out, "sine", 220, 0, 0.003, 0.58, len);
  return len;
};

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

/** R3346000: a trap firing: a sharp click and a short twang, the counterpart of `trapSet`'s soft click. */
const trapSnap: SfxRecipe = (ctx, out, at) => {
  const len = 0.2;
  const k = kit(ctx, out, at, len);
  const click = noiseSource(k);
  chain(click, biquad(k, "highpass", 2500, 0), envelope(k, 0, 0.002, 0.6, 0.05), out);
  run(k, click, 0, 0.05);
  const twang = tone(k, out, "triangle", 880, 0, 0.003, 0.35, len);
  glide(k, twang.frequency, 440, len);
  return len;
};

/** Each family's four chimes, FM ratio and spacing, at the plain spell's peak and span: a family changes color, never level. */
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

/** What a family lays quietly under its chimes (R506). */
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

/** A mana crystal fills, higher on your own side. */
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

/** Victory: a brass arpeggio onto a held C major chord, bigger than anything the game played before it. */
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

const uiClick: SfxRecipe = (ctx, out, at) => {
  const len = 0.045;
  const k = kit(ctx, out, at, len);
  tone(k, out, "sine", 1800, 0, 0.004, 0.6, 0.044);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 3000, 0), envelope(k, 0, 0.001, 0.25, 0.01), out);
  run(k, noise, 0, 0.01);
  return len;
};

const uiHover: SfxRecipe = (ctx, out, at) => {
  const len = 0.035;
  const k = kit(ctx, out, at, len);
  tone(k, out, "sine", 2600, 0, 0.003, 0.5, len);
  return len;
};

/** Something moves past. */
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

/** A zone locks. */
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

/** A card vanishes (exile, transform, fuse), or sand compresses. */
const poof: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.43;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  // Sand's 0..1 `variation` sways the pitch ±8%; without one the recipe is unshifted.
  const pitch = 0.92 + Math.min(1, Math.max(0, params.variation ?? 0.5)) * 0.16;
  const filter = biquad(k, "lowpass", 900 * pitch, 0);
  glide(k, filter.frequency, 300 * pitch, len);
  chain(noise, filter, envelope(k, 0, 0.04, 1.43, len), out);
  run(k, noise, 0, len);
  return len;
};

/** A dry sand crunch, an ±8% pitch sway and a fuller transient as a pile builds. */
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

/** #1954: a tap's variety, read as `sand` reads it: a variant of four, a ±8% pitch and the build. */
function tapFeel(params: SfxParams): { variant: number; pitch: number; build: number } {
  return {
    variant: Math.abs(Math.trunc(params.sandVariant ?? 0)) % 4,
    pitch: 0.92 + Math.min(1, Math.max(0, params.variation ?? 0.5)) * 0.16,
    build: Math.max(1, Math.min(6, params.sandBuild ?? 1)),
  };
}

const tapWood: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.16;
  const k = kit(ctx, out, at, len);
  const { variant, pitch, build } = tapFeel(params);
  const hz = ([190, 215, 240, 265][variant] ?? 190) * pitch;
  const body = tone(k, out, "triangle", hz, 0, 0.002, 0.4 + build * 0.03, len);
  glide(k, body.frequency, hz * 0.82, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "bandpass", 2600 * pitch, 2.2), envelope(k, 0, 0.0008, 0.32, 0.018), out);
  run(k, noise, 0, 0.018);
  return len;
};

const tapFelt: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.14;
  const k = kit(ctx, out, at, len);
  const { variant, pitch, build } = tapFeel(params);
  tone(k, out, "sine", ([92, 104, 116, 128][variant] ?? 92) * pitch, 0, 0.006, 0.38 + build * 0.03, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "lowpass", 380 * pitch, 0.7), envelope(k, 0, 0.005, 0.3, 0.09), out);
  run(k, noise, 0, 0.09);
  return len;
};

const tapStone: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.18;
  const k = kit(ctx, out, at, len);
  const { variant, pitch, build } = tapFeel(params);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 3200 * pitch, 0.7), envelope(k, 0, 0.0005, 0.36 + build * 0.03, 0.012), out);
  run(k, noise, 0, 0.012);
  tone(k, out, "sine", ([2960, 3320, 3730, 4180][variant] ?? 2960) * pitch, 0, 0.001, 0.2, len);
  return len;
};

const tapNeon: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.15;
  const k = kit(ctx, out, at, len);
  const { variant, pitch, build } = tapFeel(params);
  const hz = ([660, 740, 830, 990][variant] ?? 660) * pitch;
  tone(k, out, "square", hz, 0, 0.002, 0.14 + build * 0.01, 0.06);
  const zap = tone(k, out, "sawtooth", hz * 2, 0.02, 0.002, 0.12, len);
  glide(k, zap.frequency, hz * 0.5, len);
  return len;
};

const tapParchment: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.16;
  const k = kit(ctx, out, at, len);
  const { variant, pitch, build } = tapFeel(params);
  tone(k, out, "sine", ([150, 165, 180, 195][variant] ?? 150) * pitch, 0, 0.004, 0.28, 0.09);
  const paper = biquad(k, "highpass", 3500 * pitch, 0.7);
  chain(paper, out);
  clickTrain(k, paper, [0, 0.016, 0.03, 0.047], 5200 * pitch, 0.8, 0.3 + build * 0.03, 0.012);
  return len;
};

const tapAstral: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.24;
  const k = kit(ctx, out, at, len);
  const { variant, pitch, build } = tapFeel(params);
  const hz = ([1568, 1760, 1976, 2093][variant] ?? 1568) * pitch;
  fmBell(k, out, hz, 3.5, 180, 0, 0.003, 0.2 + build * 0.02, len);
  tone(k, out, "sine", hz * 2, 0.05, 0.002, 0.06, len);
  tone(k, out, "sine", hz * 3, 0.1, 0.002, 0.04, len);
  return len;
};

const AURA_NOTES_HZ = [523.25, 587.33, 659.25, 783.99, 880, 1046.5] as const;
const auraTouch: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.2;
  const k = kit(ctx, out, at, len);
  const hue = (((params.hue ?? 0) % 360) + 360) % 360;
  const hz = AURA_NOTES_HZ[Math.floor(hue / 60) % AURA_NOTES_HZ.length] ?? 523.25;
  fmBell(k, out, hz, 3.5, 160, 0, 0.003, 0.16, len);
  tone(k, out, "sine", hz * 2, 0.04, 0.002, 0.04, len);
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
 * A notice: two rising blips. An `urgent` one (a question the viewer has to answer, the other seat's
 * draw offer) is a doorbell instead, so it is never taken for the routine blips.
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

/** Life drains away, growing with the amount like impact. */
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
 * A Legendary or Mythic card enters (cues.ts, with the effects layer's rays), rebuilt by R2722000 to
 * stand beside its orchestral intro: a band-passed swell rising into the downbeat at a quarter
 * second, where a sub boom, a brass chord opening through a low-pass and a cymbal's wash land
 * together under high glints; a Mythic's glints are more and faster and shimmer, over a choir of
 * detuned saws. Its loudest moment, Mythic or not, is no louder than the Legendary gong and brass it
 * replaced (B15, B57).
 */
const entrance: SfxRecipe = (ctx, out, at, params) => {
  const len = 1.3;
  const hit = 0.25;
  const k = kit(ctx, out, at, len);
  const mythic = params.mythic === true;
  // The swell: noise through a band that rises with it, cut off at the downbeat.
  const swell = noiseSource(k);
  const band = biquad(k, "bandpass", 700, 1.2);
  glide(k, band.frequency, 3200, hit);
  const rise = k.ctx.createGain();
  rise.gain.setValueAtTime(FLOOR, time(k, 0));
  rise.gain.exponentialRampToValueAtTime(0.2, time(k, hit));
  rise.gain.exponentialRampToValueAtTime(FLOOR, time(k, hit + 0.06));
  chain(swell, band, rise, out);
  run(k, swell, 0, hit + 0.08);
  // The downbeat: a sub boom, the brass chord opening, the cymbal.
  fmBell(k, out, 49, 1.5, 90, hit, 0.008, 0.33, len);
  const brass = biquad(k, "lowpass", 400, 0);
  glide(k, brass.frequency, 3200, hit + 0.45);
  const voices = level(k, 0.5);
  chain(voices, brass, heldEnvelope(k, hit, 0.04, 0.3, 0.95, 0.2, len), out);
  for (const hz of [98, 147, 196, 247]) {
    const note = oscillator(k, "sawtooth", hz, hit);
    chain(note, voices);
    run(k, note, hit, len);
  }
  const wash = noiseSource(k);
  chain(wash, biquad(k, "highpass", 6000, 0), envelope(k, hit, 0.005, 0.12, len), out);
  run(k, wash, hit, len);
  if (mythic) {
    // The voices start with their envelope: a gain is 1 until its first event, so a voice that
    // sounded before the envelope's start would ring at full level for that gap.
    const sing = hit + 0.05;
    const choir = biquad(k, "lowpass", 2200, 0);
    const swing = tremolo(k, 5.5, 0.25);
    chain(choir, heldEnvelope(k, sing, 0.3, 0.12, 1, 0.08, len), swing, out);
    for (const hz of [392, 494, 587]) {
      for (const cents of [-9, 9]) {
        const voice = oscillator(k, "sawtooth", hz, sing);
        voice.detune.setValueAtTime(k.cents + cents, time(k, sing));
        chain(voice, choir);
        run(k, voice, sing, len);
      }
    }
  }
  const glints = mythic ? [2093, 2637, 3136, 3520, 4186, 5274] : [1568, 2093, 2637];
  const shimmer = mythic ? tremolo(k, 9, 0.2) : level(k, 1);
  chain(shimmer, out);
  glints.forEach((hz, i) => {
    const start = hit + 0.05 + (mythic ? 0.06 : 0.08) * i;
    tone(k, shimmer, "sine", hz, start, 0.003, mythic ? 0.08 : 0.1, start + 0.45);
  });
  return len;
};

/** R319: a draw finds the library empty (`fatigue`, before the hit's own impact). */
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

/** R319: a full library turns a card away (`libraryOverflow`). */
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

// Card moments (R506), Call to Chaos's roll (R436), a mark (R437), the clock (R439)

/** #21 Hinder lands on the victim's next refresh: a mana crystal cracks. */
const manaCrack: SfxRecipe = (ctx, out, at) => {
  const len = 0.88;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 1800, 0), envelope(k, 0, 0.002, 0.22, 0.06), out);
  chain(noise, biquad(k, "bandpass", 4200, 2), envelope(k, 0.028, 0.002, 0.5, 0.09), out);
  run(k, noise, 0, 0.09);
  tone(k, out, "sine", 3730, 0, 0.002, 0.16, 0.25);
  const shards: readonly (readonly [number, number])[] = [
    [0.05, 5274], [0.09, 4435], [0.12, 6272], [0.17, 3951], [0.23, 4978], [0.3, 3322], [0.38, 4186],
  ];
  for (const [start, hz] of shards) tone(k, out, "sine", hz, start, 0.002, 0.14, start + 0.12);
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

/** #27's blood price. */
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

/** #27's reward. */
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

/** A card cast as it is drawn: one quick sting. */
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

/** Call to Chaos's reveal dings, one per effect named. */
const CHAOS_DINGS: readonly number[] = [1568, 2093, 2637];

/** Call to Chaos rolls (R436): reels ratchet to a stop, then one ding per effect named (`amount`, up to CHAOS_REVEAL_MAX). */
const chaosRoll: SfxRecipe = (ctx, out, at, params) => {
  const reveals = Math.min(CHAOS_REVEAL_MAX, Math.max(0, Math.round(params.amount ?? 1)));
  const landAt = 0.78;
  const dingGap = 0.2;
  const ring = 0.42;
  const len = reveals === 0 ? landAt + 0.1 : landAt + dingGap * (reveals - 1) + ring;
  const k = kit(ctx, out, at, len);
  const clicks: number[] = [];
  for (let t = 0, gap = 0.035; t < landAt - 0.04; gap *= 1.16) {
    clicks.push(t);
    t += gap;
  }
  clickTrain(k, out, clicks, 2600, 5, 2.4, 0.025);
  const notes = [1047, 1319, 1568, 2093];
  const jingle = oscillator(k, "square", notes[0] ?? 1047);
  for (let i = 1; 0.05 * i < landAt; i += 1) {
    jingle.frequency.setValueAtTime(notes[i % notes.length] ?? 1047, time(k, 0.05 * i));
  }
  chain(jingle, biquad(k, "lowpass", 2400, 0), heldEnvelope(k, 0, 0.02, 0.14, 0.5, 0.1, landAt), out);
  run(k, jingle, 0, landAt);
  tone(k, out, "sine", 150, landAt - 0.03, 0.003, 0.6, landAt + 0.1);
  CHAOS_DINGS.slice(0, reveals).forEach((hz, i) => {
    const start = landAt + dingGap * i;
    fmBell(k, out, hz, 3.5, 350, start, 0.002, 0.5, start + ring);
  });
  return len;
};

/** A mark settles on a card (R437; #50 K-Pop Fanatic's pending steal): a seared brand; with `release` it lifts. */
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

/** R439: the last ten seconds of the viewer's turn clock: each tick (`amount` 1 at ten left) higher, louder, shorter. */
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

// R644's emoji emotes (issue §4): five synthesized sticker sounds on the effects channel

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
  // The grind has its own envelope: a raw oscillator stopped at full level clips against the growl
  // (B15's 1.0 ceiling) and rings the filter past durationMs (B15's silence floor).
  chain(rattle, envelope(k, 0, 0.01, 0.35, len), muffle);
  const grind = oscillator(k, "sawtooth", 190, 0);
  chain(grind, rattle);
  run(k, grind, 0, len);
  return len;
};

/** Wah Wah: the sad trombone. */
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

// R1345, MN03's fourteen emoji emotes: one short, light sticker sound each, never a card moment

const emoteWave: SfxRecipe = (ctx, out, at) => {
  const len = 0.5;
  const k = kit(ctx, out, at, len);
  const soft = biquad(k, "lowpass", 2600, 0.7);
  chain(soft, out);
  tone(k, soft, "triangle", 660, 0, 0.006, 0.32, 0.15);
  const ya = tone(k, soft, "triangle", 880, 0.16, 0.01, 0.34, len);
  glide(k, ya.frequency, 990, 0.3);
  vibrato(k, ya.frequency, 9, 14, 0.2, len);
  return len;
};

const emoteClap: SfxRecipe = (ctx, out, at) => {
  const len = 0.62;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.09, 0.17, 0.27, 0.34, 0.44, 0.52], 1500, 0.9, 0.42, 0.07);
  clickTrain(k, out, [0.04, 0.13, 0.22, 0.31, 0.39, 0.48], 900, 0.8, 0.3, 0.06);
  return len;
};

const emoteThumbsUp: SfxRecipe = (ctx, out, at) => {
  const len = 0.4;
  const k = kit(ctx, out, at, len);
  const warm = biquad(k, "lowpass", 1500, 0.7);
  chain(warm, out);
  tone(k, warm, "square", 392, 0, 0.006, 0.24, 0.12);
  tone(k, warm, "square", 523, 0.14, 0.006, 0.26, len);
  return len;
};

const emoteFacepalm: SfxRecipe = (ctx, out, at) => {
  const len = 0.95;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "lowpass", 1200, 0.7), envelope(k, 0, 0.002, 0.45, 0.09), out);
  run(k, noise, 0, 0.09);
  const muffle = biquad(k, "lowpass", 900, 0.8);
  chain(muffle, out);
  const groan = oscillator(k, "sawtooth", 250, 0.1);
  glide(k, groan.frequency, 150, len);
  vibrato(k, groan.frequency, 5, 6, 0.3, len);
  chain(groan, heldEnvelope(k, 0.1, 0.08, 0.3, 0.6, 0.18, len), muffle);
  run(k, groan, 0.1, len);
  return len;
};

const emoteShrug: SfxRecipe = (ctx, out, at) => {
  const len = 0.6;
  const k = kit(ctx, out, at, len);
  const soft = biquad(k, "lowpass", 1800, 0.7);
  chain(soft, out);
  tone(k, soft, "triangle", 466, 0, 0.01, 0.3, 0.2);
  const meh = tone(k, soft, "triangle", 392, 0.22, 0.01, 0.3, len);
  glide(k, meh.frequency, 440, len);
  return len;
};

const emoteThinking: SfxRecipe = (ctx, out, at) => {
  const len = 1;
  const k = kit(ctx, out, at, len);
  const hum = oscillator(k, "sine", 220, 0);
  glide(k, hum.frequency, 262, 0.85);
  vibrato(k, hum.frequency, 5, 5, 0.2, len);
  chain(hum, heldEnvelope(k, 0, 0.12, 0.36, 0.75, 0.24, len), out);
  run(k, hum, 0, len);
  const octave = oscillator(k, "sine", 440, 0);
  glide(k, octave.frequency, 524, 0.85);
  chain(octave, heldEnvelope(k, 0, 0.15, 0.08, 0.75, 0.05, len), out);
  run(k, octave, 0, len);
  return len;
};

const emoteHeart: SfxRecipe = (ctx, out, at) => {
  const len = 0.9;
  const k = kit(ctx, out, at, len);
  for (const start of [0, 0.18]) {
    const thump = tone(k, out, "sine", 95, start, 0.008, 0.42, start + 0.14);
    glide(k, thump.frequency, 55, start + 0.14);
  }
  fmBell(k, out, 1047, 2, 260, 0.36, 0.004, 0.24, len);
  return len;
};

const emoteFire: SfxRecipe = (ctx, out, at) => {
  const len = 0.9;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const roar = biquad(k, "bandpass", 450, 0.7);
  chain(noise, roar, heldEnvelope(k, 0, 0.25, 0.4, 0.6, 0.26, len), out);
  glide(k, roar.frequency, 1300, 0.6);
  run(k, noise, 0, len);
  clickTrain(k, out, [0.12, 0.21, 0.33, 0.4, 0.52, 0.63, 0.71], 3200, 1.2, 0.26, 0.03);
  return len;
};

const emoteSkull: SfxRecipe = (ctx, out, at) => {
  const len = 0.45;
  const k = kit(ctx, out, at, len);
  const hollow = biquad(k, "bandpass", 1100, 1.2);
  chain(hollow, out);
  [880, 784, 698, 622, 554].forEach((hz, i) => {
    const start = 0.07 * i;
    tone(k, hollow, "triangle", hz, start, 0.002, 0.42, start + 0.07);
  });
  clickTrain(k, out, [0, 0.07, 0.14, 0.21, 0.28], 1900, 1.4, 0.16, 0.025);
  return len;
};

const emoteSweat: SfxRecipe = (ctx, out, at) => {
  const len = 0.45;
  const k = kit(ctx, out, at, len);
  const up = tone(k, out, "sine", 260, 0, 0.006, 0.38, 0.17);
  glide(k, up.frequency, 620, 0.1);
  const down = tone(k, out, "sine", 520, 0.2, 0.006, 0.32, len);
  glide(k, down.frequency, 300, len);
  return len;
};

const emoteCool: SfxRecipe = (ctx, out, at) => {
  const len = 0.9;
  const k = kit(ctx, out, at, len);
  const smooth = biquad(k, "lowpass", 1300, 0.8);
  chain(smooth, out);
  const yeah = oscillator(k, "sawtooth", 196, 0);
  glide(k, yeah.frequency, 294, 0.3);
  vibrato(k, yeah.frequency, 5, 4, 0.35, len);
  chain(yeah, heldEnvelope(k, 0, 0.05, 0.34, 0.6, 0.22, len), smooth);
  run(k, yeah, 0, len);
  fmBell(k, out, 1175, 3, 200, 0.3, 0.004, 0.16, len);
  return len;
};

const emoteGasp: SfxRecipe = (ctx, out, at) => {
  const len = 0.5;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const breath = biquad(k, "bandpass", 600, 0.9);
  chain(noise, breath, envelope(k, 0, 0.18, 0.5, len), out);
  glide(k, breath.frequency, 2400, 0.3);
  run(k, noise, 0, len);
  return len;
};

const emoteSalute: SfxRecipe = (ctx, out, at) => {
  const len = 0.75;
  const k = kit(ctx, out, at, len);
  const brass = biquad(k, "lowpass", 2000, 0.9);
  chain(brass, out);
  tone(k, brass, "sawtooth", 392, 0, 0.02, 0.3, 0.18);
  const call = oscillator(k, "sawtooth", 523, 0.2);
  vibrato(k, call.frequency, 6, 7, 0.4, len);
  chain(call, heldEnvelope(k, 0.2, 0.03, 0.34, 0.6, 0.24, len), brass);
  run(k, call, 0.2, len);
  return len;
};

const emoteParty: SfxRecipe = (ctx, out, at) => {
  const len = 0.85;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 1800, 0.7), envelope(k, 0, 0.001, 0.4, 0.05), out);
  run(k, noise, 0, 0.05);
  const pop = tone(k, out, "sine", 160, 0.004, 0.003, 0.36, 0.12);
  glide(k, pop.frequency, 60, 0.12);
  [1568, 2093, 1760, 2349, 1976].forEach((hz, i) => {
    const start = 0.08 + 0.09 * i;
    fmBell(k, out, hz, 2, 180, start, 0.002, 0.14, Math.min(len, start + 0.3));
  });
  return len;
};

// The play sting (R669)

/** R669: a card the viewer can read is played (cues.ts): a rarity sting; Legendary and Mythic have the entrance. */
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

// Armor (R1363) and the niche moments (R1364–R1366), docs/meditative-set.md M8, MN05

/** R1363: Armor takes half or more of a hit (`damage.absorbed`), under the hit's own impact. */
const armorClank: SfxRecipe = (ctx, out, at) => {
  const len = 0.32;
  const k = kit(ctx, out, at, len);
  const ring = level(k, 0);
  const carrier = oscillator(k, "square", 140);
  const modulator = oscillator(k, "square", 213);
  chain(carrier, ring);
  modulator.connect(ring.gain);
  chain(ring, biquad(k, "lowpass", 900, 2), envelope(k, 0, 0.003, 0.3, 0.22), out);
  run(k, carrier, 0, 0.22);
  run(k, modulator, 0, 0.22);
  const thud = tone(k, out, "sine", 120, 0, 0.003, 0.3, len);
  glide(k, thud.frequency, 70, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "bandpass", 1800, 2), envelope(k, 0, 0.001, 0.6, 0.04), out);
  run(k, noise, 0, 0.04);
  return len;
};

/** R1363: Armor takes the whole hit (`damageAbsorbed`). */
const armorRing: SfxRecipe = (ctx, out, at) => {
  const len = 0.8;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 3000, 0), envelope(k, 0, 0.001, 0.6, 0.03), out);
  run(k, noise, 0, 0.03);
  fmBell(k, out, 1568, 1.41, 600, 0, 0.002, 0.32, len);
  fmBell(k, out, 2349, 2.76, 400, 0.004, 0.002, 0.18, 0.64);
  const shimmer = tremolo(k, 14, 0.2);
  chain(shimmer, out);
  tone(k, shimmer, "sine", 3951, 0.01, 0.002, 0.08, 0.5);
  return len;
};

/** R1364: a hit that does far more than the health left (overkill), on top of its impact. */
const overkill: SfxRecipe = (ctx, out, at) => {
  const len = 0.5;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const crunch = modulatedGain(k, "square", 37, 0.5, 0.5);
  chain(noise, biquad(k, "bandpass", 700, 1.2), crunch, envelope(k, 0, 0.003, 1.3, 0.22), out);
  run(k, noise, 0, 0.22);
  const sub = tone(k, out, "sine", 80, 0.01, 0.005, 0.6, len);
  glide(k, sub.frequency, 38, len);
  return len;
};

/** R1365's crumble: the grains of rubble. */
const CRUMBLE_GRAINS: readonly number[] = [0.04, 0.07, 0.09, 0.13, 0.16, 0.2, 0.25, 0.29, 0.34, 0.4, 0.47];

/** R1365: a Brittle count runs out (`crumbled`): a dry snap and grit: stone, not glass, and nothing killed it. */
const crumble: SfxRecipe = (ctx, out, at) => {
  const len = 0.6;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 2500, 0), envelope(k, 0, 0.001, 0.5, 0.05), out);
  clickTrain(k, out, CRUMBLE_GRAINS, 1400, 3, 1.6, 0.04);
  const rubble = biquad(k, "lowpass", 1200, 0);
  glide(k, rubble.frequency, 300, len);
  chain(noise, rubble, envelope(k, 0.03, 0.04, 0.5, len), out);
  run(k, noise, 0, len);
  return len;
};

/** R1365: a Locked zone opens (`unlocked`), which `lock` closed. */
const unlock: SfxRecipe = (ctx, out, at) => {
  const len = 0.34;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 2500, 0), envelope(k, 0, 0.001, 0.8, 0.02), out);
  chain(noise, biquad(k, "bandpass", 3200, 4), envelope(k, 0.06, 0.001, 0.9, 0.09), out);
  run(k, noise, 0, 0.09);
  const shackle = tone(k, out, "triangle", 520, 0.05, 0.004, 0.35, len);
  glide(k, shackle.frequency, 1040, 0.2);
  tone(k, out, "sine", 1568, 0.08, 0.003, 0.18, len);
  return len;
};

/** R1366: a card taken (`stolen`, and a `controlChanged` toward the player who acts). */
const steal: SfxRecipe = (ctx, out, at) => {
  const len = 0.36;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 700, 1.6);
  glide(k, band.frequency, 4200, 0.1);
  chain(noise, band, envelope(k, 0, 0.02, 1.8, 0.13), out);
  run(k, noise, 0, 0.13);
  const grab = tone(k, out, "sine", 260, 0.11, 0.003, 0.6, 0.24);
  glide(k, grab.frequency, 130, 0.24);
  const sly = tone(k, out, "triangle", 880, 0.15, 0.004, 0.22, len);
  glide(k, sly.frequency, 587, len);
  return len;
};

/** R1366: a card handed over (a `controlChanged` away from the player who acts, R1423's give). */
const give: SfxRecipe = (ctx, out, at) => {
  const len = 0.5;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 2400, 1.2);
  glide(k, band.frequency, 900, 0.25);
  chain(noise, band, envelope(k, 0, 0.06, 1.1, 0.28), out);
  run(k, noise, 0, 0.28);
  fmBell(k, out, 784, 2, 160, 0.12, 0.005, 0.36, 0.42);
  fmBell(k, out, 659, 2, 160, 0.22, 0.005, 0.34, len);
  return len;
};

/** R1365: a Counter cancels a play (`countered`). */
const counterspell: SfxRecipe = (ctx, out, at) => {
  const len = 0.55;
  const k = kit(ctx, out, at, len);
  const shut = biquad(k, "lowpass", 2600, 2);
  glide(k, shut.frequency, 400, 0.3);
  chain(shut, envelope(k, 0, 0.004, 0.32, 0.32), out);
  for (const hz of [622, 659]) {
    const sq = oscillator(k, "square", hz);
    glide(k, sq.frequency, hz / 2, 0.3);
    chain(sq, shut);
    run(k, sq, 0, 0.32);
  }
  const noise = noiseSource(k);
  const hiss = biquad(k, "highpass", 3000, 0);
  glide(k, hiss.frequency, 1200, len);
  chain(noise, hiss, envelope(k, 0.18, 0.02, 0.35, len), out);
  run(k, noise, 0.18, len);
  return len;
};

/** R1365: a card transformed into a Sheep (`transformed`, a Sheep the viewer can read) bleats. */
const bleat: SfxRecipe = (ctx, out, at) => {
  const len = 0.62;
  const k = kit(ctx, out, at, len);
  const voice = oscillator(k, "sawtooth", 560);
  glide(k, voice.frequency, 470, len);
  vibrato(k, voice.frequency, 9, 34, 0, len);
  chain(voice, biquad(k, "bandpass", 1300, 2.5), biquad(k, "lowpass", 2600, 0), heldEnvelope(k, 0, 0.05, 0.9, 0.38, 0.6, len), out);
  run(k, voice, 0, len);
  return len;
};

/** R1365: cards fuse into one (`fused`). */
const fuse: SfxRecipe = (ctx, out, at) => {
  const len = 0.6;
  const k = kit(ctx, out, at, len);
  const rising = tone(k, out, "triangle", 330, 0, 0.06, 0.22, 0.4);
  glide(k, rising.frequency, 523, 0.32);
  const falling = tone(k, out, "triangle", 784, 0, 0.06, 0.22, 0.4);
  glide(k, falling.frequency, 523, 0.32);
  const noise = noiseSource(k);
  const swirl = biquad(k, "bandpass", 600, 3);
  glide(k, swirl.frequency, 2600, 0.32);
  chain(noise, swirl, envelope(k, 0, 0.12, 0.7, 0.36), out);
  run(k, noise, 0, 0.36);
  fmBell(k, out, 523, 2, 220, 0.32, 0.004, 0.3, len);
  tone(k, out, "sine", 110, 0.32, 0.004, 0.4, 0.5);
  return len;
};

/** R1365: a Nerf (`degraded`, one change): the card goes out of tune. */
const degrade: SfxRecipe = (ctx, out, at) => {
  const len = 0.48;
  const k = kit(ctx, out, at, len);
  const muffle = biquad(k, "lowpass", 1800, 1);
  glide(k, muffle.frequency, 500, len);
  chain(muffle, envelope(k, 0, 0.01, 0.17, len), out);
  const pairs: readonly (readonly [number, number])[] = [
    [523, 349],
    [554, 330],
  ];
  for (const [from, to] of pairs) {
    const sq = oscillator(k, "square", from);
    glide(k, sq.frequency, to, 0.4);
    vibrato(k, sq.frequency, 7, 12, 0.1, len);
    chain(sq, muffle);
    run(k, sq, 0, len);
  }
  return len;
};

/** R1365: a Buff (`upgraded`, one change): the card tunes up. */
const upgrade: SfxRecipe = (ctx, out, at) => {
  const len = 0.46;
  const k = kit(ctx, out, at, len);
  const bright = biquad(k, "lowpass", 3200, 0);
  chain(bright, out);
  [523, 659, 784].forEach((hz, i) => {
    const start = 0.055 * i;
    tone(k, bright, "square", hz, start, 0.003, 0.22, start + 0.09);
  });
  fmBell(k, out, 1047, 3, 300, 0.17, 0.003, 0.4, len);
  return len;
};

// #3347: the card-motion sounds. A card lifted, carried over a tile, put down, sent home, dealt, and a
// deck riffled: each quiet and over in a fraction of a second, so none of them talks over the play.

/** A card lifted off the fan: a short breath of paper rising in pitch. */
const pickUp: SfxRecipe = (ctx, out, at) => {
  const len = 0.08;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 1800, 1.4);
  glide(k, band.frequency, 2600, len);
  chain(noise, band, envelope(k, 0, 0.008, 1.2, len), out);
  run(k, noise, 0, len);
  return len;
};

/** The dragged card over a tile it could land on: one soft high tick. */
const dragOver: SfxRecipe = (ctx, out, at) => {
  const len = 0.05;
  const k = kit(ctx, out, at, len);
  tone(k, out, "sine", 880, 0, 0.004, 0.35, len);
  return len;
};

/** A card put down: a felt pat with a low body under it. */
const drop: SfxRecipe = (ctx, out, at) => {
  const len = 0.13;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "bandpass", 500, 1.5), envelope(k, 0, 0.006, 1, 0.1), out);
  run(k, noise, 0, 0.1);
  tone(k, out, "sine", 140, 0, 0.005, 0.5, len);
  return len;
};

/** A card let go over nothing and sent home: the pick-up's breath, falling. */
const returnToHand: SfxRecipe = (ctx, out, at) => {
  const len = 0.19;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 3000, 1.2);
  glide(k, band.frequency, 1200, len);
  chain(noise, band, envelope(k, 0, 0.01, 1, len), out);
  run(k, noise, 0, len);
  return len;
};

/** One card flicked off the deck: a single crisp snap. */
const deal: SfxRecipe = (ctx, out, at) => {
  const len = 0.07;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "bandpass", 3800, 1.4), envelope(k, 0, 0.004, 1.2, len), out);
  run(k, noise, 0, len);
  return len;
};

/** Twelve clicks about 35 ms apart on one band: the leaves of a deck's two halves riffling together. */
const riffle: SfxRecipe = (ctx, out, at) => {
  const len = 0.46;
  const k = kit(ctx, out, at, len);
  const clicks = Array.from({ length: 12 }, (_, i) => i * 0.035);
  clickTrain(k, out, clicks, 2200, 1.2, 1, 0.03);
  return len;
};

// The Drone Battleship (R3336000): the great ship's horn, its dome, its bay doors, its reactor and its decks

/** R3336000: a layer of the Battleship arrives (`summoned`, the Meltdown layer): two detuned low saws, a long horn blast. */
const horn: SfxRecipe = (ctx, out, at) => {
  const len = 0.9;
  const k = kit(ctx, out, at, len);
  const throat = biquad(k, "lowpass", 220, 1.2);
  glide(k, throat.frequency, 1400, 0.32);
  glide(k, throat.frequency, 320, len);
  const voices = level(k, 0.5);
  chain(voices, throat, heldEnvelope(k, 0, 0.06, 0.5, 0.6, 0.4, len), out);
  for (const cents of [-14, 14]) {
    const saw = oscillator(k, "sawtooth", 73);
    saw.detune.setValueAtTime(k.cents + cents, time(k, 0));
    chain(saw, voices);
    run(k, saw, 0, len);
  }
  return len;
};

/** R3336000: a hit is redirected onto the Shield layer (`redirected`): noise rising through a band, then the dome rings. */
const domeFlare: SfxRecipe = (ctx, out, at) => {
  const len = 0.78;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 500, 1.4);
  glide(k, band.frequency, 4200, 0.3);
  chain(noise, band, envelope(k, 0, 0.28, 0.5, 0.62), out);
  run(k, noise, 0, 0.62);
  fmBell(k, out, 1397, 1.41, 520, 0.22, 0.004, 0.3, len);
  fmBell(k, out, 2093, 2.76, 360, 0.26, 0.004, 0.16, 0.66);
  return len;
};

/** R3336000: the Propagation layer opens its bays (`activated`): a door sliding on its rail, then a low clank. */
const bayDoor: SfxRecipe = (ctx, out, at) => {
  const len = 0.62;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const rail = biquad(k, "bandpass", 260, 2.2);
  glide(k, rail.frequency, 1100, 0.42);
  chain(noise, rail, heldEnvelope(k, 0, 0.08, 0.5, 0.36, 0.35, 0.46), out);
  chain(noise, biquad(k, "bandpass", 1900, 4), envelope(k, 0.44, 0.001, 0.7, 0.5), out);
  run(k, noise, 0, 0.56);
  const clank = tone(k, out, "sine", 82, 0.44, 0.004, 0.55, len);
  glide(k, clank.frequency, 48, len);
  return len;
};

/**
 * R3336000: the Meltdown layer's core thumps (`damage` to itself, `destroyed`): `amount` 1 (a tick) to 10
 * (the blast) makes it lower in the chest, longer, louder and rougher, so a ship near its end beats hardest.
 */
const reactorPulse: SfxRecipe = (ctx, out, at, params) => {
  const t = amountT(params);
  const len = 0.3 + 0.35 * t;
  const k = kit(ctx, out, at, len);
  const hz = 52 + 36 * t;
  const thump = tone(k, out, "sine", hz, 0, 0.006, 0.3 + 0.35 * t, len);
  glide(k, thump.frequency, hz * 0.55, len);
  tone(k, out, "sine", hz * 2, 0, 0.004, 0.03 + 0.12 * t, 0.7 * len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "lowpass", 400 + 600 * t, 0), envelope(k, 0, 0.004, 0.1 + 0.3 * t, 0.8 * len), out);
  run(k, noise, 0, 0.8 * len);
  return len;
};

/** R3336000: a layer with decks beneath it breaks (`destroyed`): the plates tear off with a falling screech. */
const deckShear: SfxRecipe = (ctx, out, at) => {
  const len = 0.5;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const screech = biquad(k, "bandpass", 4800, 5);
  glide(k, screech.frequency, 800, 0.42);
  const tear = modulatedGain(k, "square", 46, 0.5, 0.5);
  chain(noise, screech, tear, envelope(k, 0, 0.01, 1, len), out);
  run(k, noise, 0, len);
  const metal = oscillator(k, "sawtooth", 1900);
  glide(k, metal.frequency, 520, 0.42);
  chain(metal, biquad(k, "lowpass", 3000, 0), envelope(k, 0, 0.01, 0.2, len), out);
  run(k, metal, 0, len);
  return len;
};

// The Terminus organ (R3338000)

/**
 * R3338000: the organ's root for each Terminus set piece, by `amount`, 1 (Ceaseless Void's fold) to 9
 * (Dice of Doom's d100) in setPieceBeds.ts: G2, then a semitone higher each, so no two share a key.
 */
const ORGAN_ROOTS_HZ = [98, 103.83, 110, 116.54, 123.47, 130.81, 138.59, 146.83, 155.56] as const;

/**
 * R3338000: a Terminus set piece's sting, after the Terminus intros' organ (R1883000): a minor triad on
 * the root `amount` picks (anything else clamped into 1 to 9), each tone a pipe of sine partials at 1, 2,
 * 3 and 4 times its pitch, swelling in over a quarter second, held 1.6 s and let go. Its bed's Mythic
 * `entrance` sings the choir over its end.
 */
const organ: SfxRecipe = (ctx, out, at, params) => {
  const swell = 0.25;
  const held = swell + 1.6;
  const len = 2;
  const k = kit(ctx, out, at, len);
  const pick = Math.min(ORGAN_ROOTS_HZ.length, Math.max(1, Math.round(params.amount ?? 1)));
  const root = ORGAN_ROOTS_HZ[pick - 1] ?? ORGAN_ROOTS_HZ[0];
  // Twelve pipes peak together at 0.72 at most, under full scale (B15).
  const wind = heldEnvelope(k, 0, swell, 0.12, held, 0.12, len);
  chain(wind, out);
  // The root, its minor third and its fifth, equal-tempered.
  for (const ratio of [1, 1.1892, 1.4983]) {
    for (const [harmonic, weight] of [[1, 1], [2, 0.5], [3, 0.3], [4, 0.2]] as const) {
      const pipe = oscillator(k, "sine", root * ratio * harmonic);
      chain(pipe, level(k, weight), wind);
      run(k, pipe, 0, len);
    }
  }
  return len;
};
// R3337003: a Stinkyscape Anomaly laid (R3337000's `boardReplaced.anomaly`), one sting per board.
// Each voice's numbers are one table: times in seconds from the sting's start, frequencies in Hz,
// peaks as gains into `out`.

/** R3337003: the Wormhole's hum: a sine swept up an octave, a soft triangle an octave above it, wobbling. */
const WORMHOLE_HUM = {
  len: 1.1,
  fromHz: 55,
  toHz: 110,
  sweepEnd: 0.85,
  attack: 0.3,
  peak: 0.6,
  holdUntil: 0.75,
  holdLevel: 0.42,
  wobbleHz: 4,
  wobbleDepthHz: 2,
  overtoneRatio: 2,
  overtoneCutHz: 700,
  overtonePeak: 0.16,
} as const;

const wormholeHum: SfxRecipe = (ctx, out, at) => {
  const s = WORMHOLE_HUM;
  const k = kit(ctx, out, at, s.len);
  const hum = oscillator(k, "sine", s.fromHz);
  glide(k, hum.frequency, s.toHz, s.sweepEnd);
  vibrato(k, hum.frequency, s.wobbleHz, s.wobbleDepthHz, 0, s.len);
  chain(hum, heldEnvelope(k, 0, s.attack, s.peak, s.holdUntil, s.holdLevel, s.len), out);
  run(k, hum, 0, s.len);
  const overtone = oscillator(k, "triangle", s.fromHz * s.overtoneRatio);
  glide(k, overtone.frequency, s.toHz * s.overtoneRatio, s.sweepEnd);
  chain(overtone, biquad(k, "lowpass", s.overtoneCutHz, 0), envelope(k, 0, s.attack, s.overtonePeak, s.len), out);
  run(k, overtone, 0, s.len);
  return s.len;
};

/** R3337003: an Asteroid Field's rumbling rock: low-passed noise tumbling as its cutoff falls, a sub thud, rocks knocking. */
type RockRumble = {
  len: number;
  fromHz: number;
  toHz: number;
  q: number;
  attack: number;
  peak: number;
  holdUntil: number;
  holdLevel: number;
  tumbleHz: number;
  thudHz: number;
  thudPeak: number;
  thudStop: number;
  knocks: readonly number[];
  knockHz: number;
  knockPeak: number;
};

/** R3337003: what every Asteroid Field's rumble shares: its tumble's swing, its thud's fall, its knocks' band and decay. */
const ROCK_SHAPE = { tumbleBase: 0.65, tumbleDepth: 0.35, thudAttack: 0.008, thudDrop: 0.6, knockQ: 3, knockDecay: 0.05 } as const;

/** R3337003: each Asteroid Field's rumble: Large's long and deep, Small's short and gritty, Narcour's and Axnit's under their chime and ring. */
const ROCK_RUMBLES: { readonly [C in Extract<AnomalyCode, "AL" | "AT" | "AN" | "AA">]: RockRumble } = {
  AL: {
    len: 1.15, fromHz: 420, toHz: 120, q: 4, attack: 0.06, peak: 1.1, holdUntil: 0.7, holdLevel: 0.8, tumbleHz: 7,
    thudHz: 62, thudPeak: 0.28, thudStop: 0.5, knocks: [0.18, 0.31, 0.47, 0.6, 0.78], knockHz: 700, knockPeak: 0.8,
  },
  AT: {
    len: 0.7, fromHz: 900, toHz: 260, q: 2, attack: 0.02, peak: 0.9, holdUntil: 0.35, holdLevel: 0.6, tumbleHz: 13,
    thudHz: 110, thudPeak: 0.2, thudStop: 0.25, knocks: [0.06, 0.12, 0.17, 0.25, 0.3, 0.38, 0.46], knockHz: 1600, knockPeak: 0.7,
  },
  AN: {
    len: 1, fromHz: 600, toHz: 180, q: 3, attack: 0.04, peak: 1, holdUntil: 0.5, holdLevel: 0.7, tumbleHz: 9,
    thudHz: 80, thudPeak: 0.24, thudStop: 0.4, knocks: [0.12, 0.27, 0.4], knockHz: 1000, knockPeak: 0.7,
  },
  AA: {
    len: 1.1, fromHz: 520, toHz: 150, q: 3, attack: 0.05, peak: 1, holdUntil: 0.55, holdLevel: 0.7, tumbleHz: 8,
    thudHz: 70, thudPeak: 0.24, thudStop: 0.45, knocks: [0.15, 0.33, 0.52], knockHz: 850, knockPeak: 0.7,
  },
};

/** R3337003: Narcour's chime over its rumble: an FM bell at 880 Hz. */
const NARCOUR_CHIME = { hz: 880, ratio: 3.5, indexHz: 240, start: 0.22, attack: 0.004, peak: 0.24, stop: 0.95 } as const;

/** R3337003: Axnit's ring over its rumble: 660 Hz and its octave, shimmering as they fade. */
const AXNIT_RING = {
  hz: 660,
  octave: 2,
  start: 0.2,
  attack: 0.004,
  peak: 0.2,
  octavePeak: 0.07,
  stop: 1.05,
  shimmerHz: 6,
  shimmerDepth: 0.2,
} as const;

/** R3337003: an Asteroid Field's sting: its rumble, and whatever rings `over` it. */
function asteroidField(r: RockRumble, over?: (k: Kit) => void): SfxRecipe {
  return (ctx, out, at) => {
    const k = kit(ctx, out, at, r.len);
    const noise = noiseSource(k);
    const rock = biquad(k, "lowpass", r.fromHz, r.q);
    glide(k, rock.frequency, r.toHz, r.len);
    const tumble = modulatedGain(k, "sine", r.tumbleHz, ROCK_SHAPE.tumbleBase, ROCK_SHAPE.tumbleDepth);
    chain(noise, rock, tumble, heldEnvelope(k, 0, r.attack, r.peak, r.holdUntil, r.holdLevel, r.len), out);
    run(k, noise, 0, r.len);
    const thud = tone(k, out, "sine", r.thudHz, 0, ROCK_SHAPE.thudAttack, r.thudPeak, r.thudStop);
    glide(k, thud.frequency, r.thudHz * ROCK_SHAPE.thudDrop, r.thudStop);
    clickTrain(k, out, r.knocks, r.knockHz, ROCK_SHAPE.knockQ, r.knockPeak, ROCK_SHAPE.knockDecay);
    over?.(k);
    return r.len;
  };
}

function narcourChime(k: Kit): void {
  const s = NARCOUR_CHIME;
  fmBell(k, k.out, s.hz, s.ratio, s.indexHz, s.start, s.attack, s.peak, s.stop);
}

function axnitRing(k: Kit): void {
  const s = AXNIT_RING;
  const shimmer = tremolo(k, s.shimmerHz, s.shimmerDepth);
  chain(shimmer, k.out);
  tone(k, shimmer, "sine", s.hz, s.start, s.attack, s.peak, s.stop);
  tone(k, shimmer, "sine", s.hz * s.octave, s.start, s.attack, s.octavePeak, s.stop);
}

/** R3337003: the Battlefields' metallic clank: two ring-modulated strikes, a scrape on each, the metal ringing after. */
const BATTLEFIELD_CLANK = {
  len: 0.8,
  hits: [0, 0.15],
  carrierHz: 233,
  modulatorHz: 351,
  bandHz: 1900,
  bandQ: 3,
  hitAttack: 0.002,
  hitPeak: 0.9,
  hitStop: 0.16,
  scrapeHz: 3800,
  scrapeQ: 2,
  scrapePeak: 0.6,
  scrapeDecay: 0.035,
  ringHz: 1245,
  ringRatio: 1.41,
  ringIndexHz: 520,
  ringPeak: 0.15,
} as const;

const battlefieldClank: SfxRecipe = (ctx, out, at) => {
  const s = BATTLEFIELD_CLANK;
  const k = kit(ctx, out, at, s.len);
  for (const start of s.hits) {
    const ring = level(k, 0, start);
    const carrier = oscillator(k, "square", s.carrierHz, start);
    const modulator = oscillator(k, "square", s.modulatorHz, start);
    chain(carrier, ring);
    modulator.connect(ring.gain);
    chain(ring, biquad(k, "bandpass", s.bandHz, s.bandQ), envelope(k, start, s.hitAttack, s.hitPeak, start + s.hitStop), out);
    run(k, carrier, start, start + s.hitStop);
    run(k, modulator, start, start + s.hitStop);
  }
  clickTrain(k, out, s.hits, s.scrapeHz, s.scrapeQ, s.scrapePeak, s.scrapeDecay);
  fmBell(k, out, s.ringHz, s.ringRatio, s.ringIndexHz, 0, s.hitAttack, s.ringPeak, s.len);
  return s.len;
};

/** R3337003: the Wrecked Spice Platform's falling hiss: noise through a band sliding down, a groan falling under it. */
const SPICE_HISS = {
  len: 0.95,
  fromHz: 7000,
  toHz: 450,
  q: 1.4,
  attack: 0.03,
  peak: 0.7,
  holdUntil: 0.55,
  holdLevel: 0.55,
  groanFromHz: 330,
  groanToHz: 70,
  groanStart: 0.12,
  groanAttack: 0.05,
  groanPeak: 0.2,
} as const;

const spiceHiss: SfxRecipe = (ctx, out, at) => {
  const s = SPICE_HISS;
  const k = kit(ctx, out, at, s.len);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", s.fromHz, s.q);
  glide(k, band.frequency, s.toHz, s.len);
  chain(noise, band, heldEnvelope(k, 0, s.attack, s.peak, s.holdUntil, s.holdLevel, s.len), out);
  run(k, noise, 0, s.len);
  const groan = tone(k, out, "sine", s.groanFromHz, s.groanStart, s.groanAttack, s.groanPeak, s.len);
  glide(k, groan.frequency, s.groanToHz, s.len);
  return s.len;
};

/** R3337003: the Monument's bell: an FM bell at 98 Hz, its octave fading sooner, a stone strike on it. */
const MONUMENT_BELL = {
  len: 1.15,
  hz: 98,
  ratio: 1.4,
  indexHz: 260,
  attack: 0.004,
  peak: 0.45,
  octave: 2,
  octaveIndexHz: 180,
  octavePeak: 0.15,
  octaveStop: 0.75,
  strikeHz: 1800,
  strikeQ: 1.5,
  strikeAttack: 0.001,
  strikePeak: 0.4,
  strikeStop: 0.03,
} as const;

const monumentBell: SfxRecipe = (ctx, out, at) => {
  const s = MONUMENT_BELL;
  const k = kit(ctx, out, at, s.len);
  fmBell(k, out, s.hz, s.ratio, s.indexHz, 0, s.attack, s.peak, s.len);
  fmBell(k, out, s.hz * s.octave, s.ratio, s.octaveIndexHz, 0, s.attack, s.octavePeak, s.octaveStop);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "bandpass", s.strikeHz, s.strikeQ), envelope(k, 0, s.strikeAttack, s.strikePeak, s.strikeStop), out);
  run(k, noise, 0, s.strikeStop);
  return s.len;
};

/** R3337003: the Drone Hideouts' buzz: two saws a hair apart, rising a little, through a low-pass, a fast tremolo on them. */
const DRONE_BUZZ = {
  len: 0.9,
  hz: [147, 150],
  rise: 1.12,
  riseEnd: 0.5,
  cutHz: 1700,
  q: 3,
  attack: 0.04,
  peak: 0.2,
  holdUntil: 0.65,
  holdLevel: 0.15,
  tremoloHz: 24,
  tremoloDepth: 0.4,
} as const;

const droneBuzz: SfxRecipe = (ctx, out, at) => {
  const s = DRONE_BUZZ;
  const k = kit(ctx, out, at, s.len);
  const buzz = biquad(k, "lowpass", s.cutHz, s.q);
  chain(buzz, tremolo(k, s.tremoloHz, s.tremoloDepth), heldEnvelope(k, 0, s.attack, s.peak, s.holdUntil, s.holdLevel, s.len), out);
  for (const hz of s.hz) {
    const saw = oscillator(k, "sawtooth", hz);
    glide(k, saw.frequency, hz * s.rise, s.riseEnd);
    chain(saw, buzz);
    run(k, saw, 0, s.len);
  }
  return s.len;
};

/** R3337003: the Frontier Outpost's chime: C then the G above it (523 and 784 Hz), warm FM bells. */
const FRONTIER_CHIME = {
  len: 1,
  notes: [
    [0, 523],
    [0.14, 784],
  ],
  ratio: 2,
  indexHz: 260,
  attack: 0.004,
  peak: 0.3,
  ring: 0.8,
} as const;

const frontierChime: SfxRecipe = (ctx, out, at) => {
  const s = FRONTIER_CHIME;
  const k = kit(ctx, out, at, s.len);
  for (const [start, hz] of s.notes) fmBell(k, out, hz, s.ratio, s.indexHz, start, s.attack, s.peak, Math.min(s.len, start + s.ring));
  return s.len;
};

/** R3337003: each Anomaly's sting, by the code its `boardReplaced` names (cards/anomalies.ts). */
const ANOMALY_STINGS: { readonly [C in AnomalyCode]: SfxRecipe } = {
  WH: wormholeHum,
  BF: battlefieldClank,
  AL: asteroidField(ROCK_RUMBLES.AL),
  AT: asteroidField(ROCK_RUMBLES.AT),
  SP: spiceHiss,
  MT: monumentBell,
  DH: droneBuzz,
  FO: frontierChime,
  AN: asteroidField(ROCK_RUMBLES.AN, narcourChime),
  AA: asteroidField(ROCK_RUMBLES.AA, axnitRing),
};

/** R3337003: an Anomaly is laid: its board's sting; with no code, or one this build does not know, the plain whoosh. */
const anomaly: SfxRecipe = (ctx, out, at, params) =>
  (isAnomalyCode(params.anomaly) ? ANOMALY_STINGS[params.anomaly] : whoosh)(ctx, out, at, params);

// The Starship's moments (R3334000–R3334003)

/**
 * R3334000: a docking module's hum, by `amount` (the ship's pieces once it has docked): G2 for one
 * piece, `stepCents` higher for each more, held from `pieces` on (fx/starship.ts STARSHIP_DOCK.humFull).
 */
const DOCK_HUM = { rootHz: 98, stepCents: 200, pieces: 8 } as const;

/**
 * R3334000: a module docks on its hardpoint: a clank and its latch as it clamps on, a weld's crackling
 * hiss along the seam, and the ship's hum under them, a step higher for each piece it holds.
 */
const starshipDock: SfxRecipe = (ctx, out, at, params) => {
  const len = 0.44;
  const weld = 0.1;
  const k = kit(ctx, out, at, len);
  // The clank: two squares ring-modulated, as Armor's is, struck, then latched.
  const ring = level(k, 0);
  const carrier = oscillator(k, "square", 180);
  const modulator = oscillator(k, "square", 267);
  chain(carrier, ring);
  modulator.connect(ring.gain);
  chain(ring, biquad(k, "lowpass", 1400, 2), envelope(k, 0, 0.002, 0.34, 0.16), out);
  run(k, carrier, 0, 0.16);
  run(k, modulator, 0, 0.16);
  clickTrain(k, out, [0, 0.06], 2400, 2, 0.9, 0.03);
  const thud = tone(k, out, "sine", 150, 0, 0.003, 0.25, 0.14);
  glide(k, thud.frequency, 80, 0.12);
  const noise = noiseSource(k);
  const crackle = modulatedGain(k, "square", 31, 0.5, 0.5);
  chain(noise, biquad(k, "highpass", 3500, 0), crackle, envelope(k, weld, 0.03, 0.45, len), out);
  run(k, noise, weld, len);
  const amount = params.amount ?? 1;
  const pieces = Number.isNaN(amount) ? 1 : Math.min(DOCK_HUM.pieces, Math.max(1, Math.round(amount)));
  const hum = oscillator(k, "triangle", DOCK_HUM.rootHz * 2 ** (((pieces - 1) * DOCK_HUM.stepCents) / CENTS_PER_OCTAVE), 0.08);
  chain(hum, heldEnvelope(k, 0.08, 0.06, 0.24, 0.32, 0.16, len), out);
  run(k, hum, 0.08, len);
  return len;
};

/**
 * R3334001: the engines ignite as the cinematic lights them: the igniters tick, the fuel catches with a
 * whump, and the flame builds over a low rumble.
 */
const starshipIgnite: SfxRecipe = (ctx, out, at) => {
  const len = 0.68;
  const catchAt = 0.14;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.05, 0.1], 3200, 4, 0.9, 0.02);
  const whump = tone(k, out, "sine", 75, catchAt, 0.01, 0.4, 0.5);
  glide(k, whump.frequency, 42, 0.45);
  const noise = noiseSource(k);
  const flame = biquad(k, "lowpass", 300, 0);
  glide(k, flame.frequency, 2400, 0.4);
  chain(noise, flame, heldEnvelope(k, catchAt, 0.04, 0.7, 0.45, 0.5, len), out);
  run(k, noise, catchAt, len);
  const rumble = oscillator(k, "sawtooth", 55, catchAt);
  glide(k, rumble.frequency, 80, len);
  chain(rumble, biquad(k, "lowpass", 240, 0), heldEnvelope(k, catchAt, 0.15, 0.22, 0.5, 0.18, len), out);
  run(k, rumble, catchAt, len);
  return len;
};

/**
 * R3334001: the launch's roar as the ship climbs out of frame: a crackling rumble swelling as it lifts,
 * its band opening with the climb and closing as it pulls away, a hiss of exhaust and a sub under it.
 */
const starshipRoar: SfxRecipe = (ctx, out, at) => {
  const len = 1.15;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const band = biquad(k, "lowpass", 400, 0);
  glide(k, band.frequency, 2000, 0.35);
  glide(k, band.frequency, 350, len);
  const crackle = modulatedGain(k, "square", 17, 0.75, 0.25);
  chain(noise, band, crackle, heldEnvelope(k, 0, 0.12, 0.6, 0.6, 0.45, len), out);
  chain(noise, biquad(k, "highpass", 4500, 0), envelope(k, 0, 0.12, 0.14, 0.7), out);
  run(k, noise, 0, len);
  const sub = oscillator(k, "sine", 46);
  glide(k, sub.frequency, 30, len);
  chain(sub, heldEnvelope(k, 0, 0.1, 0.3, 0.6, 0.22, len), out);
  run(k, sub, 0, len);
  return len;
};

/** R3334002: a beam sweeps its target: a charging whine, then a buzzing ray whose band sweeps up across it and back. */
const starshipBeam: SfxRecipe = (ctx, out, at) => {
  const len = 0.38;
  const fire = 0.04;
  const k = kit(ctx, out, at, len);
  const whine = tone(k, out, "sine", 1200, 0, 0.005, 0.14, 0.08);
  glide(k, whine.frequency, 2400, 0.06);
  const ray = biquad(k, "bandpass", 900, 2.5);
  glide(k, ray.frequency, 3600, 0.2);
  glide(k, ray.frequency, 1400, len);
  chain(ray, tremolo(k, 40, 0.3), heldEnvelope(k, fire, 0.02, 0.9, 0.3, 0.6, len), out);
  const core = oscillator(k, "sine", 660, fire);
  glide(k, core.frequency, 1320, 0.2);
  glide(k, core.frequency, 880, len);
  chain(core, heldEnvelope(k, fire, 0.02, 0.22, 0.3, 0.16, len), out);
  run(k, core, fire, len);
  for (const hz of [440, 446]) {
    const saw = oscillator(k, "sawtooth", hz, fire);
    chain(saw, ray);
    run(k, saw, fire, len);
  }
  return len;
};

/** R3334002: a repeater's bolts, one after another (fx/starship.ts STARSHIP_BROADSIDE.repeaterShots). */
const REPEATER_SHOTS: readonly number[] = [0, 0.07, 0.14, 0.21];

/** R3334002: a repeater's stutter: each bolt a crack and a falling zap. */
const starshipRepeater: SfxRecipe = (ctx, out, at) => {
  const len = 0.32;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, REPEATER_SHOTS, 1800, 1.5, 1.5, 0.04);
  const bright = biquad(k, "lowpass", 3000, 0);
  chain(bright, out);
  for (const start of REPEATER_SHOTS) {
    const zap = tone(k, bright, "square", 900, start, 0.002, 0.28, start + 0.08);
    glide(k, zap.frequency, 300, start + 0.06);
  }
  return len;
};

/** R3334002: a cannon's heavy boom: a sharp crack, a blast closing down, and a deep thump falling away. */
const starshipCannon: SfxRecipe = (ctx, out, at) => {
  const len = 0.58;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 3000, 0), envelope(k, 0, 0.001, 0.22, 0.03), out);
  const blast = biquad(k, "lowpass", 2500, 0);
  glide(k, blast.frequency, 180, len);
  chain(noise, blast, envelope(k, 0, 0.004, 0.5, len), out);
  run(k, noise, 0, len);
  const thump = tone(k, out, "sine", 90, 0, 0.004, 0.48, len);
  glide(k, thump.frequency, 32, len * 0.8);
  return len;
};

/** R3334002: flak's bursts in the air (fx/starship.ts STARSHIP_BROADSIDE.flakClouds). */
const FLAK_POPS: readonly number[] = [0.08, 0.17, 0.27];

/** R3334002: flak's pops: a dull launch, then each burst a puff of noise over a soft thump. */
const starshipFlak: SfxRecipe = (ctx, out, at) => {
  const len = 0.46;
  const k = kit(ctx, out, at, len);
  const launch = tone(k, out, "sine", 220, 0, 0.003, 0.2, 0.07);
  glide(k, launch.frequency, 110, 0.06);
  clickTrain(k, out, FLAK_POPS, 900, 0.9, 1.1, 0.12);
  for (const start of FLAK_POPS) {
    const pop = tone(k, out, "sine", 170, start, 0.003, 0.25, start + 0.12);
    glide(k, pop.frequency, 80, start + 0.1);
  }
  return len;
};

/** R3334002: a launcher's hollow thunk as it lobs its load, and the whoosh of it flying. */
const starshipLauncher: SfxRecipe = (ctx, out, at) => {
  const len = 0.42;
  const k = kit(ctx, out, at, len);
  const thunk = tone(k, out, "triangle", 150, 0, 0.002, 0.55, 0.12);
  glide(k, thunk.frequency, 70, 0.1);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "bandpass", 600, 3), envelope(k, 0, 0.002, 1.2, 0.05), out);
  // The whoosh rises from silence under the thunk (a gain is 1 until its first event, so it starts at once).
  const air = biquad(k, "bandpass", 500, 1.2);
  glide(k, air.frequency, 3000, len);
  chain(noise, air, envelope(k, 0, 0.2, 1.4, len), out);
  run(k, noise, 0, len);
  return len;
};

/** R3334002: the debris a ram rattles off. */
const RAM_DEBRIS: readonly number[] = [0.12, 0.17, 0.23, 0.3, 0.38];

/** R3334002: an unarmed Starship rams prow first: a crunch of hull on hull, a heavy thud, a screech of metal and debris. */
const starshipRam: SfxRecipe = (ctx, out, at) => {
  const len = 0.48;
  const scrape = 0.02;
  const k = kit(ctx, out, at, len);
  const noise = noiseSource(k);
  const crunch = modulatedGain(k, "square", 41, 0.5, 0.5);
  chain(noise, biquad(k, "bandpass", 600, 1), crunch, envelope(k, 0, 0.003, 1.2, 0.3), out);
  run(k, noise, 0, 0.3);
  const thud = tone(k, out, "sine", 100, 0, 0.004, 0.4, len);
  glide(k, thud.frequency, 40, len);
  const screech = biquad(k, "bandpass", 1500, 4);
  chain(screech, envelope(k, scrape, 0.01, 0.12, 0.35), out);
  for (const hz of [1240, 1310]) {
    const saw = oscillator(k, "sawtooth", hz, scrape);
    glide(k, saw.frequency, 900, 0.35);
    chain(saw, screech);
    run(k, saw, scrape, 0.35);
  }
  clickTrain(k, out, RAM_DEBRIS, 2600, 3, 0.8, 0.03);
  return len;
};

/** R3334003: the hull's creaks before it breaks. */
const WRECK_CREAKS: readonly number[] = [0.05, 0.11, 0.2, 0.26, 0.31];

/**
 * R3334003: a Starship's hull breaks: a long metal groan sinking under its creaks, the snap as it breaks
 * in two, a boom and the rumble of it coming apart.
 */
const starshipWreck: SfxRecipe = (ctx, out, at) => {
  const len = 1.05;
  const snap = 0.34;
  const k = kit(ctx, out, at, len);
  const groan = biquad(k, "bandpass", 320, 2.5);
  glide(k, groan.frequency, 680, 0.45);
  glide(k, groan.frequency, 260, len);
  const voices = level(k, 0.5);
  chain(voices, groan, heldEnvelope(k, 0, 0.25, 0.9, 0.8, 0.55, len), out);
  for (const hz of [72, 75.6]) {
    const saw = oscillator(k, "sawtooth", hz);
    glide(k, saw.frequency, hz * 0.6, len);
    vibrato(k, saw.frequency, 3.5, 4, 0, len);
    chain(saw, voices);
    run(k, saw, 0, len);
  }
  clickTrain(k, out, WRECK_CREAKS, 1300, 5, 0.9, 0.03);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "highpass", 2200, 0), envelope(k, snap, 0.002, 0.3, snap + 0.08), out);
  const rumble = biquad(k, "lowpass", 400, 0);
  glide(k, rumble.frequency, 150, len);
  chain(noise, rumble, envelope(k, snap, 0.02, 0.9, len), out);
  run(k, noise, snap, len);
  const boom = tone(k, out, "sine", 70, snap, 0.004, 0.35, len);
  glide(k, boom.frequency, 30, len);
  return len;
};

// R3400001: the signature sounds of Meditative+'s and Training Grounds' Legendaries and Mythics, one each
// (audio/signatureCues.ts plays them), every one layering its card's three sounds in a graph of its own.

/** Bob's Plan: a brush slapped on paper, one war-drum hit, a satisfied grunt. */
const inkSummit: SfxRecipe = (ctx, out, at) => {
  const len = 1.1;
  const k = kit(ctx, out, at, len);
  // Each noise its own source, started as its envelope opens (a gain is 1 until its envelope sets it).
  const slap = noiseSource(k);
  chain(slap, biquad(k, "bandpass", 2600, 0.9), envelope(k, 0, 0.003, 0.32, 0.12), out);
  run(k, slap, 0, 0.12);
  const drum = tone(k, out, "sine", 64, 0.1, 0.006, 0.5, 0.55);
  glide(k, drum.frequency, 42, 0.45);
  const skin = noiseSource(k);
  chain(skin, biquad(k, "lowpass", 160, 0), envelope(k, 0.1, 0.004, 0.35, 0.2), out);
  run(k, skin, 0.1, 0.2);
  const muffle = biquad(k, "lowpass", 800, 0.8);
  chain(muffle, out);
  const grunt = oscillator(k, "sawtooth", 150, 0.55);
  glide(k, grunt.frequency, 175, 0.7);
  glide(k, grunt.frequency, 120, len);
  chain(grunt, heldEnvelope(k, 0.55, 0.05, 0.24, 0.8, 0.12, len), muffle);
  run(k, grunt, 0.55, len);
  return len;
};

/** Bob: a snowplow's engine revving, a grunt of effort, a deep thud. */
const plowHeave: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  const throat = biquad(k, "lowpass", 380, 1.4);
  chain(throat, tremolo(k, 22, 0.35), out);
  const engine = oscillator(k, "sawtooth", 48, 0);
  glide(k, engine.frequency, 96, 0.45);
  glide(k, engine.frequency, 60, 0.75);
  chain(engine, heldEnvelope(k, 0, 0.08, 0.34, 0.6, 0.2, 0.8), throat);
  run(k, engine, 0, 0.8);
  const muffle = biquad(k, "lowpass", 950, 0.8);
  chain(muffle, out);
  const effort = oscillator(k, "sawtooth", 210, 0.35);
  glide(k, effort.frequency, 260, 0.55);
  glide(k, effort.frequency, 170, 0.75);
  chain(effort, envelope(k, 0.35, 0.04, 0.22, 0.78), muffle);
  run(k, effort, 0.35, 0.78);
  const thud = tone(k, out, "sine", 56, 0.78, 0.005, 0.55, len);
  glide(k, thud.frequency, 34, len);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "lowpass", 140, 0), envelope(k, 0.78, 0.004, 0.4, 1), out);
  run(k, noise, 0.78, 1);
  return len;
};

/** Scratch Pad: a soft cursor tick, keys clattering into silence, paper tearing. */
const cursorTear: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0], 3200, 6, 0.4, 0.03);
  clickTrain(k, out, [0.22, 0.27, 0.31, 0.38, 0.42, 0.5, 0.57, 0.67], 1900, 3, 0.55, 0.035);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 1800, 1.1);
  glide(k, band.frequency, 4200, len);
  chain(noise, band, tremolo(k, 34, 0.45), heldEnvelope(k, 0.85, 0.03, 0.7, 1.15, 0.4, len), out);
  run(k, noise, 0.85, len);
  return len;
};

/** Seance: a low murmured chant, candles guttering, a ghostly exhale. */
const seanceChant: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const murmur = biquad(k, "lowpass", 600, 0.9);
  chain(murmur, out);
  for (const [hz, rate] of [[110, 4], [165, 5]] as const) {
    const voice = oscillator(k, "triangle", hz, 0);
    vibrato(k, voice.frequency, rate, 3, 0, 1);
    chain(voice, heldEnvelope(k, 0, 0.2, 0.2, 0.8, 0.14, 1), murmur);
    run(k, voice, 0, 1);
  }
  const flame = noiseSource(k);
  chain(flame, biquad(k, "bandpass", 900, 2), modulatedGain(k, "square", 13, 0.5, 0.45), envelope(k, 0.1, 0.1, 0.14, 0.9), out);
  run(k, flame, 0.1, 0.9);
  const exhale = noiseSource(k);
  const breath = biquad(k, "bandpass", 1300, 0.8);
  glide(k, breath.frequency, 380, len);
  chain(exhale, breath, heldEnvelope(k, 0.85, 0.12, 0.3, 1.1, 0.1, len), out);
  run(k, exhale, 0.85, len);
  return len;
};

/** Tutorial Master: a pointer rapped on slate, a weary sigh, a chalk squeak. */
const slateRap: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.16], 1500, 4, 0.56, 0.06);
  const noise = noiseSource(k);
  const band = biquad(k, "bandpass", 650, 0.8);
  glide(k, band.frequency, 380, 0.95);
  chain(noise, band, heldEnvelope(k, 0.32, 0.18, 0.31, 0.65, 0.11, 0.95), out);
  run(k, noise, 0.32, 0.95);
  const sigh = oscillator(k, "sine", 300, 0.32);
  glide(k, sigh.frequency, 170, 0.95);
  chain(sigh, heldEnvelope(k, 0.32, 0.15, 0.17, 0.65, 0.07, 0.95), out);
  run(k, sigh, 0.32, 0.95);
  const squeak = tone(k, out, "triangle", 2300, 1.0, 0.01, 0.17, len);
  glide(k, squeak.frequency, 2750, len);
  vibrato(k, squeak.frequency, 28, 60, 1.0, len);
  return len;
};

/** Call to Chaos (Training-Grounds Edition): wooden dice clattering, a held breath, a gong for the result. */
const diceGong: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.07, 0.11, 0.2, 0.24, 0.33, 0.38, 0.44], 1100, 3, 0.34, 0.05);
  const noise = noiseSource(k);
  chain(noise, biquad(k, "lowpass", 500, 0.7), heldEnvelope(k, 0.5, 0.2, 0.06, 0.78, 0.06, 0.8), out);
  run(k, noise, 0.5, 0.8);
  fmBell(k, out, 98, 1.41, 260, 0.8, 0.008, 0.42, len);
  fmBell(k, out, 196, 2.76, 120, 0.8, 0.008, 0.14, len);
  return len;
};

/** The Training-Grounds Golem: timber creaking, ropes snapping taut, a hollow wooden thump. */
const timberLash: SfxRecipe = (ctx, out, at) => {
  const len = 1.2;
  const k = kit(ctx, out, at, len);
  const grain = biquad(k, "bandpass", 620, 6);
  chain(grain, out);
  const creak = oscillator(k, "sawtooth", 88, 0);
  glide(k, creak.frequency, 70, 0.5);
  vibrato(k, creak.frequency, 7, 9, 0, 0.55);
  chain(creak, heldEnvelope(k, 0, 0.12, 0.4, 0.4, 0.2, 0.55), grain);
  run(k, creak, 0, 0.55);
  for (const start of [0.5, 0.62]) {
    const snap = noiseSource(k);
    chain(snap, biquad(k, "highpass", 2400, 0.7), envelope(k, start, 0.002, 0.28, start + 0.05), out);
    run(k, snap, start, start + 0.05);
    const twang = tone(k, out, "triangle", 760, start, 0.002, 0.14, start + 0.12);
    glide(k, twang.frequency, 520, start + 0.12);
  }
  const thump = tone(k, out, "triangle", 150, 0.8, 0.004, 0.42, len);
  glide(k, thump.frequency, 92, len);
  const hollow = noiseSource(k);
  chain(hollow, biquad(k, "bandpass", 320, 1.5), envelope(k, 0.8, 0.003, 0.3, 1), out);
  run(k, hollow, 0.8, 1);
  return len;
};

/** Goetzmann Game Design: a pencil scratching, a napkin rustle, a bright rising arpeggio. */
const pencilArpeggio: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  const scratch = noiseSource(k);
  chain(scratch, biquad(k, "bandpass", 3400, 2.2), tremolo(k, 17, 0.45), heldEnvelope(k, 0, 0.03, 0.3, 0.45, 0.22, 0.55), out);
  run(k, scratch, 0, 0.55);
  const rustle = noiseSource(k);
  chain(rustle, biquad(k, "highpass", 2800, 0.7), envelope(k, 0.55, 0.04, 0.2, 0.75), out);
  run(k, rustle, 0.55, 0.75);
  const bright = biquad(k, "lowpass", 5200, 0.7);
  chain(bright, out);
  // Each note held until the next, the last ringing out, so the notes barely overlap.
  const notes = [523, 659, 784, 1047];
  for (const [n, hz] of notes.entries()) {
    const start = 0.62 + n * 0.13;
    const last = n === notes.length - 1;
    const stop = last ? len : start + 0.16;
    const note = oscillator(k, "triangle", hz, start);
    chain(note, heldEnvelope(k, start, 0.008, 0.28, last ? start + 0.3 : start + 0.11, 0.22, stop), bright);
    run(k, note, start, stop);
  }
  return len;
};

// R3407001: the signature sounds of Stinkyscape's signature cards, one each (audio/signatureCues.ts plays
// them), every one layering its card's sounds in a graph of its own within the Legendary's 1.6 s.

/** MELCHIOR: a sterile beep count, a ballot clacking into its box, a rising tone. */
const ballotBeep: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  for (const start of [0, 0.16, 0.32]) {
    const beep = oscillator(k, "sine", 1000, start);
    chain(beep, heldEnvelope(k, start, 0.003, 0.24, start + 0.08, 0.22, start + 0.1), out);
    run(k, beep, start, start + 0.1);
  }
  clickTrain(k, out, [0.5, 0.54], 900, 2.5, 0.6, 0.07);
  const box = tone(k, out, "triangle", 220, 0.5, 0.003, 0.3, 0.64);
  glide(k, box.frequency, 130, 0.62);
  const rise = oscillator(k, "sine", 440, 0.74);
  glide(k, rise.frequency, 880, 1.16);
  chain(rise, heldEnvelope(k, 0.74, 0.04, 0.26, 1.16, 0.22, len), out);
  run(k, rise, 0.74, len);
  return len;
};

/** BALTHASAR: a soft monitor blip, a lullaby hummed, a worried sigh. */
const lullabySigh: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  tone(k, out, "sine", 1320, 0, 0.006, 0.18, 0.14);
  const hum = biquad(k, "lowpass", 900, 0.7);
  chain(hum, out);
  for (const [start, hz, stop] of [[0.18, 392, 0.42], [0.42, 330, 0.64], [0.64, 349, 0.92]] as const) {
    const note = oscillator(k, "triangle", hz, start);
    vibrato(k, note.frequency, 5, 3, start, stop);
    chain(note, heldEnvelope(k, start, 0.05, 0.32, stop - 0.05, 0.24, stop), hum);
    run(k, note, start, stop);
  }
  const breath = noiseSource(k);
  const band = biquad(k, "bandpass", 1100, 0.8);
  band.frequency.setValueAtTime(1100, time(k, 0.95));
  glide(k, band.frequency, 420, len);
  chain(breath, band, heldEnvelope(k, 0.95, 0.1, 0.3, 1.2, 0.1, len), out);
  run(k, breath, 0.95, len);
  const sigh = oscillator(k, "sine", 280, 0.95);
  glide(k, sigh.frequency, 180, len);
  chain(sigh, heldEnvelope(k, 0.95, 0.1, 0.14, 1.2, 0.05, len), out);
  run(k, sigh, 0.95, len);
  return len;
};

/** CASPER: a compact snapping shut, a sultry chime, a tut-tut. */
const compactTut: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.03], 3400, 3, 0.64, 0.04);
  tone(k, out, "triangle", 2480, 0.03, 0.002, 0.14, 0.16);
  const sultry = tremolo(k, 5, 0.2);
  chain(sultry, out);
  fmBell(k, sultry, 523, 1.5, 180, 0.22, 0.03, 0.28, 0.95);
  fmBell(k, sultry, 622, 1.5, 140, 0.34, 0.03, 0.21, 1.0);
  clickTrain(k, out, [1.05, 1.2], 2600, 1.5, 0.58, 0.03);
  return len;
};

/** xxFreshJack: a glitch stutter, a crowd roar, a bass drop. */
const glitchDrop: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  for (const [n, hz] of [880, 1760, 660, 1320, 990].entries()) {
    const start = 0.045 * n;
    crushedBlip(k, out, hz, start, start + 0.035, 0.2);
  }
  const stutter = noiseSource(k);
  chain(stutter, biquad(k, "highpass", 2500, 0.7), modulatedGain(k, "square", 22, 0.5, 0.5), envelope(k, 0, 0.005, 0.25, 0.24), out);
  run(k, stutter, 0, 0.24);
  const crowd = noiseSource(k);
  chain(crowd, biquad(k, "bandpass", 900, 0.6), tremolo(k, 7, 0.15), heldEnvelope(k, 0.25, 0.3, 0.34, 0.8, 0.24, 1.0), out);
  run(k, crowd, 0.25, 1.0);
  const drop = oscillator(k, "sine", 140, 0.86);
  glide(k, drop.frequency, 38, 1.36);
  chain(drop, heldEnvelope(k, 0.86, 0.01, 0.5, 1.2, 0.35, len), out);
  run(k, drop, 0.86, len);
  const grit = oscillator(k, "sawtooth", 70, 0.86);
  glide(k, grit.frequency, 32, 1.36);
  chain(grit, biquad(k, "lowpass", 260, 0.8), envelope(k, 0.86, 0.01, 0.2, len), out);
  run(k, grit, 0.86, len);
  return len;
};

/** iKwzki: a harp glissando up the strings, then a warm two-note chime. */
const harpGliss: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const strings = biquad(k, "lowpass", 4200, 0.7);
  chain(strings, out);
  for (const [n, hz] of [262, 294, 330, 392, 440, 523, 587, 659, 784, 880, 1047].entries()) {
    const start = 0.04 * n;
    tone(k, strings, "triangle", hz, start, 0.003, 0.14, start + 0.4);
  }
  fmBell(k, out, 523, 2, 90, 0.6, 0.01, 0.26, 1.1);
  fmBell(k, out, 784, 2, 70, 0.85, 0.01, 0.22, len);
  return len;
};

/** Shockwave447: chatter fast-forwarded, a marker squeaking twice, an electric thunderclap. */
const markerThunder: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const chatter = oscillator(k, "sawtooth", 600, 0);
  for (const [n, hz] of [900, 520, 1100, 700, 1300, 640, 980, 760, 1200].entries()) {
    chatter.frequency.setValueAtTime(hz, time(k, 0.04 * (n + 1)));
  }
  chain(chatter, biquad(k, "bandpass", 1600, 2), modulatedGain(k, "square", 18, 0.5, 0.5), heldEnvelope(k, 0, 0.02, 0.45, 0.34, 0.35, 0.42), out);
  run(k, chatter, 0, 0.42);
  for (const start of [0.5, 0.66]) {
    const squeak = tone(k, out, "sine", 2900, start, 0.008, 0.16, start + 0.12);
    glide(k, squeak.frequency, 3400, start + 0.1);
    vibrato(k, squeak.frequency, 35, 90, start, start + 0.12);
  }
  const zap = tone(k, out, "square", 1800, 0.86, 0.002, 0.12, 1.0);
  glide(k, zap.frequency, 90, 0.98);
  const crack = noiseSource(k);
  chain(crack, biquad(k, "highpass", 1200, 0.5), modulatedGain(k, "square", 47, 0.5, 0.5), envelope(k, 0.86, 0.003, 0.6, 1.12), out);
  run(k, crack, 0.86, 1.12);
  const rumble = noiseSource(k);
  chain(rumble, biquad(k, "lowpass", 180, 0.7), heldEnvelope(k, 0.88, 0.02, 0.5, 1.1, 0.3, len), out);
  run(k, rumble, 0.88, len);
  return len;
};

/** Job Application: a pneumatic tube's thunk, a desk phone's trill, a rubber stamp pressed and peeled. */
const stampTrill: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  const tube = noiseSource(k);
  const suck = biquad(k, "bandpass", 300, 1.5);
  glide(k, suck.frequency, 1400, 0.16);
  chain(tube, suck, envelope(k, 0, 0.1, 0.45, 0.2), out);
  run(k, tube, 0, 0.2);
  const thunk = tone(k, out, "sine", 120, 0.18, 0.004, 0.55, 0.36);
  glide(k, thunk.frequency, 60, 0.32);
  // The trill: two pitches swapping every 30 ms, in two rings.
  const handset = biquad(k, "bandpass", 1400, 2.5);
  chain(handset, out);
  for (const ring of [0.42, 0.66]) {
    const trill = oscillator(k, "square", 1250, ring);
    for (const n of [1, 2, 3, 4, 5]) trill.frequency.setValueAtTime(n % 2 === 0 ? 1250 : 1560, time(k, ring + 0.03 * n));
    chain(trill, heldEnvelope(k, ring, 0.008, 0.22, ring + 0.16, 0.2, ring + 0.18), handset);
    run(k, trill, ring, ring + 0.18);
  }
  const press = tone(k, out, "triangle", 180, 1.0, 0.003, 0.5, 1.22);
  glide(k, press.frequency, 90, 1.16);
  const pad = noiseSource(k);
  chain(pad, biquad(k, "lowpass", 900, 0.7), envelope(k, 1.0, 0.002, 0.45, 1.12), out);
  run(k, pad, 1.0, 1.12);
  const peel = noiseSource(k);
  chain(peel, biquad(k, "bandpass", 2400, 1), envelope(k, 1.22, 0.02, 0.16, 1.34), out);
  run(k, peel, 1.22, 1.34);
  return len;
};

/** Paycheck: paper feeding through, a roulette ball rattling to rest, a till drawer sliding open with its ching. */
const tillRattle: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const paper = noiseSource(k);
  chain(paper, biquad(k, "bandpass", 3800, 1.5), modulatedGain(k, "square", 38, 0.5, 0.5), heldEnvelope(k, 0, 0.02, 0.34, 0.28, 0.26, 0.34), out);
  run(k, paper, 0, 0.34);
  const motor = oscillator(k, "sawtooth", 95, 0);
  chain(motor, biquad(k, "lowpass", 300, 0.7), heldEnvelope(k, 0, 0.03, 0.1, 0.28, 0.08, 0.34), out);
  run(k, motor, 0, 0.34);
  // The ball rattles round the wheel, slowing as it settles.
  const bounces: number[] = [];
  for (let t = 0.4, gap = 0.03; t < 1.0; gap *= 1.18) {
    bounces.push(t);
    t += gap;
  }
  clickTrain(k, out, bounces, 3000, 4, 0.55, 0.025);
  const slide = noiseSource(k);
  const rail = biquad(k, "bandpass", 1800, 2);
  rail.frequency.setValueAtTime(1800, time(k, 1.04));
  glide(k, rail.frequency, 700, 1.24);
  chain(slide, rail, envelope(k, 1.04, 0.04, 0.32, 1.26), out);
  run(k, slide, 1.04, 1.26);
  tone(k, out, "sine", 110, 1.24, 0.003, 0.42, 1.4);
  fmBell(k, out, 2093, 3.5, 300, 1.27, 0.002, 0.24, len);
  return len;
};

/** Scornboi: a gavel's crack, a scoff, a heavy hatch slammed shut. */
const gavelSlam: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0], 1300, 3, 0.7, 0.05);
  const knock = tone(k, out, "triangle", 420, 0, 0.002, 0.35, 0.12);
  glide(k, knock.frequency, 300, 0.1);
  const nose = biquad(k, "bandpass", 750, 2.5);
  chain(nose, out);
  const scoff = oscillator(k, "sawtooth", 240, 0.3);
  glide(k, scoff.frequency, 150, 0.48);
  chain(scoff, envelope(k, 0.3, 0.015, 0.35, 0.5), nose);
  run(k, scoff, 0.3, 0.5);
  const huff = noiseSource(k);
  chain(huff, biquad(k, "bandpass", 1600, 1), envelope(k, 0.3, 0.01, 0.2, 0.46), out);
  run(k, huff, 0.3, 0.46);
  const slam = tone(k, out, "sine", 70, 0.7, 0.004, 0.55, 1.2);
  glide(k, slam.frequency, 40, 1.1);
  fmBell(k, out, 147, 2.83, 220, 0.7, 0.003, 0.2, len);
  const clang = noiseSource(k);
  chain(clang, biquad(k, "lowpass", 600, 1), envelope(k, 0.7, 0.003, 0.4, 0.86), out);
  run(k, clang, 0.7, 0.86);
  return len;
};

/** Megabase: a deep docking clank, a lobby's three-note chime, a cascade of coins. */
const dockingChime: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  for (const start of [0, 0.14]) {
    const clamp = tone(k, out, "sine", 62, start, 0.004, 0.42, start + 0.3);
    glide(k, clamp.frequency, 44, start + 0.25);
    fmBell(k, out, 110, 3.17, 160, start, 0.003, 0.14, start + 0.32);
  }
  for (const [start, hz] of [[0.5, 784], [0.66, 988], [0.82, 1175]] as const) fmBell(k, out, hz, 2, 60, start, 0.006, 0.22, start + 0.5);
  const coins = [3520, 4186, 3951, 4699];
  for (const [n, start] of [0.96, 1.01, 1.05, 1.11, 1.14, 1.2, 1.25, 1.31, 1.37, 1.43].entries()) {
    tone(k, out, "sine", coins[n % coins.length] ?? 3520, start, 0.002, 0.1, start + 0.12);
  }
  return len;
};

/** IN33DH3: a flurry of keys, a chirpy AI ping, a smug click. */
const keyboardPing: SfxRecipe = (ctx, out, at) => {
  const len = 1.2;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.05, 0.08, 0.14, 0.17, 0.22, 0.26, 0.29, 0.35, 0.39, 0.42, 0.48], 2400, 2, 0.7, 0.025);
  crushedBlip(k, out, 1568, 0.58, 0.64, 0.22);
  const chirp = tone(k, out, "sine", 1760, 0.64, 0.004, 0.34, 0.82);
  glide(k, chirp.frequency, 2637, 0.72);
  clickTrain(k, out, [1.0], 1700, 6, 0.85, 0.03);
  tone(k, out, "triangle", 880, 1.0, 0.002, 0.18, 1.1);
  return len;
};

/** IN33DH3 Prime: a low synth swell opening up, a confident finger snap, a drum tap. */
const synthSnap: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  const sweep = biquad(k, "lowpass", 180, 2);
  glide(k, sweep.frequency, 1600, 0.6);
  glide(k, sweep.frequency, 500, 0.8);
  chain(sweep, heldEnvelope(k, 0, 0.5, 0.22, 0.62, 0.16, 0.8), out);
  for (const hz of [55, 55.6]) {
    const saw = oscillator(k, "sawtooth", hz, 0);
    chain(saw, sweep);
    run(k, saw, 0, 0.8);
  }
  const snap = noiseSource(k);
  chain(snap, biquad(k, "bandpass", 2300, 1.8), envelope(k, 0.84, 0.002, 0.7, 0.92), out);
  run(k, snap, 0.84, 0.92);
  const tap = tone(k, out, "sine", 190, 1.08, 0.003, 0.5, 1.35);
  glide(k, tap.frequency, 110, 1.3);
  const skin = noiseSource(k);
  chain(skin, biquad(k, "lowpass", 1200, 0.7), envelope(k, 1.08, 0.002, 0.25, 1.14), out);
  run(k, skin, 1.08, 1.14);
  return len;
};

/** Wardrum Blob: a squishy plop, a two-beat hand drum, a happy hum. */
const blobDrum: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const plop = tone(k, out, "sine", 420, 0, 0.004, 0.45, 0.16);
  glide(k, plop.frequency, 110, 0.12);
  const squish = noiseSource(k);
  const wet = biquad(k, "bandpass", 1200, 5);
  glide(k, wet.frequency, 300, 0.14);
  chain(squish, wet, envelope(k, 0, 0.01, 0.35, 0.16), out);
  run(k, squish, 0, 0.16);
  for (const [start, hz, peak] of [[0.3, 160, 0.5], [0.48, 130, 0.42]] as const) {
    const hit = tone(k, out, "sine", hz, start, 0.003, peak, start + 0.22);
    glide(k, hit.frequency, hz * 0.62, start + 0.18);
    const slap = noiseSource(k);
    chain(slap, biquad(k, "bandpass", 900, 1.2), envelope(k, start, 0.002, peak * 0.5, start + 0.05), out);
    run(k, slap, start, start + 0.05);
  }
  const hum = biquad(k, "lowpass", 1100, 0.7);
  chain(hum, out);
  for (const [start, hz, stop] of [[0.8, 392, 1.02], [1.02, 494, 1.24], [1.24, 587, len]] as const) {
    const note = oscillator(k, "triangle", hz, start);
    vibrato(k, note.frequency, 6, 4, start, stop);
    chain(note, heldEnvelope(k, start, 0.03, 0.28, stop - 0.04, 0.22, stop), hum);
    run(k, note, start, stop);
  }
  return len;
};

/** Alteran Reach: a warp whoosh, then a sparkling chime for each node along the map. */
const warpSparkle: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const warp = noiseSource(k);
  const tunnel = biquad(k, "bandpass", 220, 2);
  glide(k, tunnel.frequency, 4200, 0.5);
  glide(k, tunnel.frequency, 900, 0.7);
  chain(warp, tunnel, heldEnvelope(k, 0, 0.35, 0.67, 0.5, 0.37, 0.7), out);
  run(k, warp, 0, 0.7);
  const shift = oscillator(k, "sine", 180, 0);
  glide(k, shift.frequency, 1200, 0.5);
  chain(shift, envelope(k, 0, 0.4, 0.12, 0.62), out);
  run(k, shift, 0, 0.62);
  const sparkle = tremolo(k, 16, 0.2);
  chain(sparkle, out);
  for (const [n, hz] of [1319, 1568, 1976, 2349, 2637].entries()) {
    const start = 0.62 + 0.14 * n;
    fmBell(k, sparkle, hz, 3.5, 200, start, 0.002, 0.22, Math.min(len, start + 0.4));
  }
  return len;
};

/** Edge of the Universe: paper tearing, a long-held choir note cut off, then silence. */
const tearChoir: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const paper = noiseSource(k);
  const rip = biquad(k, "bandpass", 1500, 1.2);
  glide(k, rip.frequency, 3800, 0.34);
  chain(paper, rip, modulatedGain(k, "square", 41, 0.55, 0.45), envelope(k, 0, 0.02, 0.8, 0.36), out);
  run(k, paper, 0, 0.36);
  // The choir sings "ah" through two formants, held, then cut: past it, silence to the end.
  const voices = heldEnvelope(k, 0.32, 0.25, 0.19, 1.15, 0.17, 1.2);
  for (const hz of [700, 1150]) chain(voices, biquad(k, "bandpass", hz, 4), out);
  for (const hz of [220, 277, 330, 440]) {
    const voice = oscillator(k, "sawtooth", hz, 0.32);
    vibrato(k, voice.frequency, 5.5, hz * 0.012, 0.32, 1.2);
    chain(voice, voices);
    run(k, voice, 0.32, 1.2);
  }
  return len;
};

/** Wildar: a cackle, a crack of thunder rolling off, two coins flicked. */
const cackleThunder: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const throat = biquad(k, "bandpass", 1400, 1.5);
  chain(throat, out);
  for (const [n, start] of [0, 0.09, 0.18, 0.27, 0.36].entries()) {
    const cackle = oscillator(k, "sawtooth", 820 - 50 * n, start);
    glide(k, cackle.frequency, 620 - 40 * n, start + 0.07);
    vibrato(k, cackle.frequency, 28, 30, start, start + 0.08);
    chain(cackle, envelope(k, start, 0.006, 0.35, start + 0.08), throat);
    run(k, cackle, start, start + 0.08);
  }
  const crack = noiseSource(k);
  chain(crack, biquad(k, "highpass", 900, 0.6), envelope(k, 0.52, 0.002, 0.6, 0.66), out);
  run(k, crack, 0.52, 0.66);
  const roll = noiseSource(k);
  chain(roll, biquad(k, "lowpass", 220, 0.8), modulatedGain(k, "sine", 6, 0.75, 0.25), heldEnvelope(k, 0.54, 0.03, 0.55, 0.8, 0.35, 1.2), out);
  run(k, roll, 0.54, 1.2);
  for (const start of [1.22, 1.38]) {
    const stop = Math.min(len, start + 0.2);
    const flick = tone(k, out, "sine", 2200, start, 0.002, 0.18, stop);
    glide(k, flick.frequency, 2900, start + 0.05);
    fmBell(k, out, 3300, 2.4, 300, start, 0.002, 0.08, stop);
  }
  return len;
};

/** Call to Chaos (Stinky Edition): a wet squelch, a ratchet spinning, a kazoo fanfare. */
const kazooRatchet: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const squelch = noiseSource(k);
  const gut = biquad(k, "lowpass", 1400, 9);
  glide(k, gut.frequency, 180, 0.22);
  chain(squelch, gut, envelope(k, 0, 0.01, 0.48, 0.24), out);
  run(k, squelch, 0, 0.24);
  const ratchet: number[] = [];
  for (let t = 0.28; t < 0.74; t += 0.028) ratchet.push(t);
  clickTrain(k, out, ratchet, 1700, 6, 0.72, 0.018);
  // The kazoo's buzz: each note's saw through one nasal band, chopped by a fast square.
  const buzz = biquad(k, "bandpass", 1100, 1.2);
  chain(buzz, modulatedGain(k, "square", 31, 0.6, 0.4), out);
  for (const [start, hz, stop] of [[0.8, 392, 0.95], [0.97, 392, 1.12], [1.14, 523, len]] as const) {
    const kazoo = oscillator(k, "sawtooth", hz, start);
    vibrato(k, kazoo.frequency, 7, 6, start, stop);
    chain(kazoo, heldEnvelope(k, start, 0.02, 0.48, stop - 0.05, 0.38, stop), buzz);
    run(k, kazoo, start, stop);
  }
  return len;
};

/** The Stinkyscape Golem: rock grinding into a thud, a thruster's roar, a loud sniff. */
const thrusterSniff: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const grind = noiseSource(k);
  chain(grind, biquad(k, "lowpass", 500, 1.2), modulatedGain(k, "sawtooth", 19, 0.6, 0.4), heldEnvelope(k, 0, 0.04, 0.5, 0.2, 0.35, 0.3), out);
  run(k, grind, 0, 0.3);
  const thud = tone(k, out, "sine", 75, 0.22, 0.004, 0.6, 0.5);
  glide(k, thud.frequency, 38, 0.45);
  const jet = noiseSource(k);
  const plume = biquad(k, "bandpass", 400, 0.7);
  plume.frequency.setValueAtTime(400, time(k, 0.45));
  glide(k, plume.frequency, 1300, 0.95);
  chain(jet, plume, heldEnvelope(k, 0.45, 0.2, 0.5, 0.95, 0.4, 1.1), out);
  run(k, jet, 0.45, 1.1);
  const burn = oscillator(k, "sawtooth", 42, 0.45);
  glide(k, burn.frequency, 58, 0.95);
  chain(burn, biquad(k, "lowpass", 200, 0.7), heldEnvelope(k, 0.45, 0.2, 0.25, 0.95, 0.2, 1.1), out);
  run(k, burn, 0.45, 1.1);
  const sniff = noiseSource(k);
  const nostril = biquad(k, "bandpass", 1800, 2.5);
  nostril.frequency.setValueAtTime(1800, time(k, 1.18));
  glide(k, nostril.frequency, 3600, 1.45);
  chain(sniff, nostril, heldEnvelope(k, 1.18, 0.06, 0.6, 1.4, 0.45, 1.5), out);
  run(k, sniff, 1.18, 1.5);
  return len;
};

/** Call to Chaos (Scape Edition): the world tilting with a deep groan, marbles rolling, a cosmic gong. */
const marbleGong: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const hull = biquad(k, "lowpass", 260, 5);
  glide(k, hull.frequency, 140, 0.5);
  chain(hull, out);
  const groan = oscillator(k, "sawtooth", 58, 0);
  glide(k, groan.frequency, 41, 0.5);
  vibrato(k, groan.frequency, 3, 2.5, 0, 0.55);
  chain(groan, heldEnvelope(k, 0, 0.12, 0.45, 0.4, 0.3, 0.55), hull);
  run(k, groan, 0, 0.55);
  clickTrain(k, out, [0.36, 0.41, 0.43, 0.5, 0.55, 0.58, 0.64, 0.7, 0.73, 0.8], 4200, 8, 0.45, 0.03);
  const roll = noiseSource(k);
  chain(roll, biquad(k, "bandpass", 2200, 1), tremolo(k, 9, 0.4), envelope(k, 0.36, 0.05, 0.12, 0.84), out);
  run(k, roll, 0.36, 0.84);
  const cosmos = tremolo(k, 3, 0.25);
  chain(cosmos, out);
  fmBell(k, cosmos, 73, 1.53, 180, 0.86, 0.01, 0.42, len);
  fmBell(k, cosmos, 219, 3.11, 140, 0.86, 0.02, 0.12, len);
  return len;
};

/** Sanctum: a hushed choir's breath, a page turned, a soft bell. */
const sanctumBell: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const breath = noiseSource(k);
  const hush = heldEnvelope(k, 0, 0.3, 0.36, 0.55, 0.24, 0.75);
  chain(breath, hush);
  for (const hz of [500, 900, 1400]) chain(hush, biquad(k, "bandpass", hz, 6), out);
  run(k, breath, 0, 0.75);
  const pad = oscillator(k, "sine", 262, 0);
  chain(pad, heldEnvelope(k, 0, 0.3, 0.1, 0.55, 0.07, 0.75), out);
  run(k, pad, 0, 0.75);
  const page = noiseSource(k);
  const leaf = biquad(k, "bandpass", 1600, 1.4);
  leaf.frequency.setValueAtTime(1600, time(k, 0.7));
  glide(k, leaf.frequency, 3600, 0.8);
  glide(k, leaf.frequency, 2000, 0.92);
  chain(page, leaf, modulatedGain(k, "square", 24, 0.6, 0.4), envelope(k, 0.7, 0.03, 0.42, 0.94), out);
  run(k, page, 0.7, 0.94);
  fmBell(k, out, 880, 2, 110, 0.98, 0.015, 0.26, len);
  fmBell(k, out, 1320, 2, 60, 0.98, 0.02, 0.07, len);
  return len;
};

/** Stimmycheck: a cash drawer's ping, then a whistle rising slowly, as prices do. */
const cashWhistle: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.04], 2000, 3, 0.4, 0.03);
  fmBell(k, out, 2637, 3.5, 320, 0.05, 0.002, 0.3, 0.55);
  fmBell(k, out, 3520, 3.5, 200, 0.07, 0.002, 0.14, 0.5);
  const whistle = oscillator(k, "sine", 880, 0.45);
  glide(k, whistle.frequency, 1760, 1.5);
  vibrato(k, whistle.frequency, 5, 12, 0.7, len);
  chain(whistle, heldEnvelope(k, 0.45, 0.25, 0.24, 1.45, 0.24, len), out);
  run(k, whistle, 0.45, len);
  const air = noiseSource(k);
  const lip = biquad(k, "bandpass", 880, 8);
  lip.frequency.setValueAtTime(880, time(k, 0.45));
  glide(k, lip.frequency, 1760, 1.5);
  chain(air, lip, heldEnvelope(k, 0.45, 0.25, 0.25, 1.45, 0.25, len), out);
  run(k, air, 0.45, len);
  return len;
};

/** Narcor: a whispered breath, a shimmer, a soft clink of glass. */
const whisperClink: SfxRecipe = (ctx, out, at) => {
  const len = 1.2;
  const k = kit(ctx, out, at, len);
  const breath = noiseSource(k);
  const whisper = biquad(k, "bandpass", 2600, 1.6);
  glide(k, whisper.frequency, 1700, 0.5);
  chain(breath, whisper, heldEnvelope(k, 0, 0.18, 0.62, 0.35, 0.39, 0.5), out);
  run(k, breath, 0, 0.5);
  const shimmer = tremolo(k, 11, 0.3);
  chain(shimmer, out);
  for (const [n, hz] of [2093, 2637, 3136].entries()) tone(k, shimmer, "sine", hz, 0.38 + 0.04 * n, 0.08, 0.11, 0.9);
  tone(k, out, "sine", 3729, 0.86, 0.002, 0.25, len);
  tone(k, out, "sine", 5588, 0.86, 0.002, 0.12, 1.05);
  return len;
};

/** Red Narcor: a deep resonant hum rising until it rings as bright crystal. */
const crystalRise: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const body = biquad(k, "lowpass", 300, 6);
  glide(k, body.frequency, 2400, 0.85);
  chain(body, out);
  const hum = oscillator(k, "sawtooth", 65, 0);
  glide(k, hum.frequency, 520, 0.85);
  chain(hum, heldEnvelope(k, 0, 0.25, 0.22, 0.75, 0.25, 0.9), body);
  run(k, hum, 0, 0.9);
  const core = oscillator(k, "sine", 65, 0);
  glide(k, core.frequency, 520, 0.85);
  chain(core, heldEnvelope(k, 0, 0.25, 0.3, 0.75, 0.2, 0.9), out);
  run(k, core, 0, 0.9);
  fmBell(k, out, 2093, 2.76, 380, 0.84, 0.002, 0.3, len);
  fmBell(k, out, 3136, 3.01, 200, 0.86, 0.002, 0.12, len);
  tone(k, out, "sine", 4186, 0.9, 0.003, 0.08, 1.4);
  return len;
};

// R3405001: the signature sounds of Meditative's Legendaries and Mythics, one each (audio/signatureCues.ts
// plays them), every one layering its card's sounds in a graph of its own, as R3400001's do.

/** R3405001: noise through one filter into `into`, its source running only while its envelope is open; returns the filter. */
function noiseHit(
  k: Kit,
  into: AudioNode,
  type: BiquadFilterType,
  hz: number,
  q: number,
  start: number,
  attack: number,
  peak: number,
  stop: number,
): BiquadFilterNode {
  const noise = noiseSource(k);
  const filter = biquad(k, type, hz, q);
  chain(noise, filter, envelope(k, start, attack, peak, stop), into);
  run(k, noise, start, stop);
  return filter;
}

/** R3405001: holds `param` at `from` until `start`, then glides it to `to` by `stop`: a filter's sweep that waits for its sound. */
function sweep(k: Kit, param: AudioParam, from: number, to: number, start: number, stop: number): void {
  glide(k, param, from, start);
  glide(k, param, to, stop);
}

/** R3405001: a plucked string at `hz`: a sawtooth whose low-pass closes from `brightHz` toward its pitch as it rings out. */
function pluck(k: Kit, into: AudioNode, hz: number, start: number, peak: number, stop: number, brightHz: number): OscillatorNode {
  const string = oscillator(k, "sawtooth", hz, start);
  const body = biquad(k, "lowpass", brightHz, 1);
  sweep(k, body.frequency, brightHz, hz * 1.5, start, stop);
  chain(string, body, envelope(k, start, 0.003, peak, stop), into);
  run(k, string, start, stop);
  return string;
}

/** Introspection: a deep inhale, a singing bowl's hum, a sharp exhale. */
const bowlBreath: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The inhale: breath through a band that opens as the lungs fill.
  const inhale = noiseSource(k);
  const chest = biquad(k, "bandpass", 450, 0.9);
  glide(k, chest.frequency, 1300, 0.45);
  chain(inhale, chest, heldEnvelope(k, 0, 0.38, 0.32, 0.42, 0.3, 0.5), out);
  run(k, inhale, 0, 0.5);
  // The bowl: two sines a few Hz apart beating as it hums, its overtone at 2.71 times fainter.
  for (const [hz, peak] of [[392, 0.16], [395, 0.16], [1062, 0.05]] as const) {
    const bowl = oscillator(k, "sine", hz, 0.42);
    chain(bowl, heldEnvelope(k, 0.42, 0.12, peak, 1.05, peak * 0.7, 1.4), out);
    run(k, bowl, 0.42, 1.4);
  }
  // The exhale: a sharp puff through a falling band.
  const exhale = noiseHit(k, out, "bandpass", 2600, 0.8, 1.1, 0.012, 0.42, 1.45);
  sweep(k, exhale.frequency, 2600, 800, 1.1, 1.45);
  return len;
};

/** Reach the Summit: wind over a peak, then three rising flute steps. */
const peakFlute: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The wind: noise through a narrow band a slow sine sways, gusting under the whole climb.
  const wind = noiseSource(k);
  const gust = biquad(k, "bandpass", 650, 2.5);
  vibrato(k, gust.frequency, 1.3, 260, 0, len);
  chain(wind, gust, heldEnvelope(k, 0, 0.25, 0.4, 1.1, 0.22, len), out);
  run(k, wind, 0, len);
  // Three steps up, each a breathy sine scooping into pitch with a player's vibrato, the last held longest.
  const steps = [[0.3, 587], [0.62, 659], [0.94, 880]] as const;
  for (const [n, [start, hz]] of steps.entries()) {
    const stop = n === steps.length - 1 ? len : start + 0.36;
    const flute = oscillator(k, "sine", hz * 0.97, start);
    glide(k, flute.frequency, hz, start + 0.04);
    vibrato(k, flute.frequency, 5.5, hz * 0.008, start + 0.1, stop);
    chain(flute, heldEnvelope(k, start, 0.05, 0.22, stop - 0.1, 0.17, stop), out);
    run(k, flute, start, stop);
    noiseHit(k, out, "bandpass", hz * 2, 4, start, 0.02, 0.12, start + 0.12);
  }
  return len;
};

/** Double Header: a stadium horn, then a ticket tearing in two. */
const ticketHorn: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  // The air horn: two squares a major third apart, scooping up into pitch, through a brassy low-pass.
  const brass = biquad(k, "lowpass", 1700, 2);
  chain(brass, heldEnvelope(k, 0, 0.03, 0.2, 0.62, 0.16, 0.75), out);
  for (const hz of [233, 294]) {
    const horn = oscillator(k, "square", hz * 0.88);
    glide(k, horn.frequency, hz, 0.06);
    chain(horn, brass);
    run(k, horn, 0, 0.75);
  }
  // The tear: a rasping band of noise chattering as the fibres part, then the last snap.
  const tear = noiseSource(k);
  const fibre = biquad(k, "bandpass", 1800, 1.4);
  sweep(k, fibre.frequency, 1800, 3400, 0.82, 1.14);
  chain(tear, fibre, modulatedGain(k, "square", 46, 0.55, 0.45), heldEnvelope(k, 0.82, 0.02, 0.5, 1.08, 0.4, 1.14), out);
  run(k, tear, 0.82, 1.14);
  clickTrain(k, out, [1.15], 4200, 2, 0.5, 0.05);
  return len;
};

/** True Craft a Card: soft keystrokes building in rhythm, then a forge's quench hiss. */
const forgeQuench: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The keys: soft thocks drawing closer together, then a brighter run as the rhythm takes hold.
  clickTrain(k, out, [0, 0.15, 0.28, 0.39, 0.48], 1500, 3, 0.3, 0.04);
  clickTrain(k, out, [0.55, 0.6, 0.65, 0.69, 0.73, 0.77], 2400, 4, 0.42, 0.03);
  // The quench: steam bursting and settling, a low boil bubbling under it as the metal cools.
  const steam = noiseSource(k);
  const vent = biquad(k, "highpass", 2600, 0.7);
  sweep(k, vent.frequency, 2600, 5200, 0.82, len);
  chain(steam, vent, heldEnvelope(k, 0.82, 0.01, 0.45, 1.05, 0.24, len), out);
  run(k, steam, 0.82, len);
  const boil = noiseSource(k);
  chain(boil, biquad(k, "bandpass", 700, 1.5), modulatedGain(k, "triangle", 19, 0.5, 0.5), envelope(k, 0.86, 0.05, 0.3, len), out);
  run(k, boil, 0.86, len);
  return len;
};

/** Aestheticize the Game: a long brush stroke on paper, a guzheng glissando. */
const brushGuzheng: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The stroke: bristles dragged across paper, the band lowering as the ink runs dry.
  const stroke = noiseSource(k);
  const bristle = biquad(k, "bandpass", 1900, 0.7);
  glide(k, bristle.frequency, 850, 0.6);
  chain(stroke, bristle, tremolo(k, 9, 0.2), heldEnvelope(k, 0, 0.06, 0.7, 0.45, 0.55, 0.62), out);
  run(k, stroke, 0, 0.62);
  // The glissando: nine strings of a pentatonic swept upward, each plucked and ringing on.
  const strings = [294, 330, 370, 440, 494, 587, 659, 740, 880];
  for (const [n, hz] of strings.entries()) {
    const start = 0.55 + n * 0.05;
    pluck(k, out, hz, start, 0.2, Math.min(len, start + 0.55), 3200);
  }
  return len;
};

/** Alternate Fate: a dry well's hollow echo, then a gush of water. */
const wellGush: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  // The echo: a pebble's knock returning from the dry shaft, each return later, quieter and lower.
  for (const [n, start] of [0, 0.17, 0.34, 0.51].entries()) {
    const knock = tone(k, out, "triangle", 260 - n * 20, start, 0.002, 0.4 / 2 ** n, start + 0.14);
    glide(k, knock.frequency, 150 - n * 10, start + 0.14);
  }
  // The gush: water breaking through, a broad roar churned by a slow wobble, bubbles rising in it.
  const water = noiseSource(k);
  const rush = biquad(k, "lowpass", 600, 0.8);
  sweep(k, rush.frequency, 600, 2400, 0.62, 0.9);
  chain(water, rush, tremolo(k, 7, 0.25), heldEnvelope(k, 0.62, 0.08, 0.6, 1.1, 0.4, len), out);
  run(k, water, 0.62, len);
  for (const start of [0.74, 0.85, 0.93, 1.04, 1.12]) {
    const bubble = tone(k, out, "sine", 520, start, 0.004, 0.12, start + 0.06);
    glide(k, bubble.frequency, 1150, start + 0.06);
  }
  return len;
};

/** Red Jade: a pick strike, a low mountain groan, a crystal ring. */
const minePick: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The strike: iron on rock, a metallic clink over a gritty burst.
  fmBell(k, out, 1750, 2.37, 1400, 0, 0.001, 0.32, 0.2);
  noiseHit(k, out, "highpass", 2200, 0.8, 0, 0.001, 0.5, 0.08);
  // The groan: the mountain settling, a low sawtooth sagging behind a dark low-pass.
  const groan = oscillator(k, "sawtooth", 58, 0.12);
  glide(k, groan.frequency, 41, 0.95);
  vibrato(k, groan.frequency, 2.5, 2, 0.12, 0.95);
  chain(groan, biquad(k, "lowpass", 240, 2.5), heldEnvelope(k, 0.12, 0.2, 0.5, 0.75, 0.35, 0.95), out);
  run(k, groan, 0.12, 0.95);
  // The ring: a fifth of high sines glinting as they fade.
  const glint = tremolo(k, 7, 0.25);
  chain(glint, out);
  tone(k, glint, "sine", 2093, 0.78, 0.003, 0.16, len);
  tone(k, glint, "sine", 3136, 0.8, 0.003, 0.08, len);
  return len;
};

/** Jade Beauty: stone cracking, a zither glissando, a sigh. */
const jadeZither: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The crack: the jade shell splitting in uneven snaps over a dull thump.
  clickTrain(k, out, [0, 0.035, 0.08, 0.13, 0.145, 0.2], 1300, 2, 0.8, 0.03);
  noiseHit(k, out, "lowpass", 220, 0.7, 0, 0.003, 0.7, 0.18);
  // The glissando: soft triangle strings falling from high to low, one hand's sweep down the zither.
  const strings = [1175, 988, 880, 740, 659, 587, 494];
  for (const [n, hz] of strings.entries()) {
    const start = 0.28 + n * 0.055;
    const stop = Math.min(len, start + 0.5);
    const string = tone(k, out, "triangle", hz, start, 0.002, 0.24, stop);
    vibrato(k, string.frequency, 6, hz * 0.004, start, stop);
  }
  // The sigh: breath through two falling vowel bands, a soft voice under it.
  for (const [from, to] of [[850, 520], [1250, 900]] as const) {
    const vowel = noiseHit(k, out, "bandpass", from, 4, 0.88, 0.14, 0.45, len);
    sweep(k, vowel.frequency, from, to, 0.88, len);
  }
  const voice = tone(k, out, "triangle", 420, 0.9, 0.15, 0.1, len);
  glide(k, voice.frequency, 290, len);
  return len;
};

/** Feng Shui: a wind chime, a soft gong, one guqin string. */
const windChime: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  // The chime: five tubes knocked by a breeze, each a sine with its inharmonic partial.
  for (const [start, hz] of [[0, 1568], [0.07, 2093], [0.15, 1865], [0.22, 2489], [0.31, 2794]] as const) {
    tone(k, out, "sine", hz, start, 0.002, 0.12, start + 0.5);
    tone(k, out, "sine", hz * 2.76, start, 0.002, 0.03, start + 0.22);
  }
  // The gong: a padded mallet on a small gong, slow to bloom.
  fmBell(k, out, 110, 1.48, 90, 0.48, 0.035, 0.3, 1.35);
  // The guqin: one low string plucked and slid up a tone as it rings.
  const string = pluck(k, out, 147, 0.98, 0.26, len, 1600);
  sweep(k, string.frequency, 147, 165, 1.12, 1.35);
  return len;
};

/** CN Flea Market: a gong, crowd chatter, one abacus clack. */
const marketGong: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The gong: struck hard, a low inharmonic bell with a crash of shimmer on its face.
  fmBell(k, out, 82, 1.73, 320, 0, 0.004, 0.42, 1.2);
  noiseHit(k, out, "bandpass", 3200, 1.2, 0, 0.003, 0.22, 0.45);
  // The chatter: two vowel bands of noise opening and closing, each at its own syllable rate.
  for (const [hz, rate] of [[520, 7], [1450, 11]] as const) {
    const voice = noiseSource(k);
    chain(voice, biquad(k, "bandpass", hz, 3), modulatedGain(k, "square", rate, 0.5, 0.5), heldEnvelope(k, 0.25, 0.15, 0.8, 1, 0.7, 1.18), out);
    run(k, voice, 0.25, 1.18);
  }
  // The clack: two wooden beads knocked together.
  clickTrain(k, out, [1.22], 2600, 6, 1.2, 0.035);
  tone(k, out, "triangle", 1250, 1.22, 0.001, 0.22, 1.3);
  return len;
};

/** Knowledge Breaker: chalk snapping, a long hush, a cracked porcelain chime. */
const chalkHush: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The snap: a stick of chalk breaking, a dry crack and its splinter just after.
  clickTrain(k, out, [0, 0.014], 3600, 1.5, 0.75, 0.035);
  noiseHit(k, out, "highpass", 4500, 0.7, 0, 0.001, 0.3, 0.06);
  // The hush: a long "shh" through a high band, swelling and slowly let go.
  const hush = noiseSource(k);
  chain(hush, biquad(k, "highpass", 1900, 0.7), biquad(k, "bandpass", 3300, 0.9), heldEnvelope(k, 0.12, 0.16, 0.45, 0.8, 0.3, 1), out);
  run(k, hush, 0.12, 1);
  // The chime: porcelain with a crack in it, two sines a rough few Hz apart and a buzzing FM partial.
  tone(k, out, "sine", 1320, 0.98, 0.002, 0.12, len);
  tone(k, out, "sine", 1347, 0.98, 0.002, 0.1, len);
  fmBell(k, out, 1320, 1.07, 380, 0.98, 0.002, 0.08, 1.3);
  return len;
};

/** Knowledge Breaker Prime: porcelain shattering, a deep roar, a rolling boom. */
const porcelainRoar: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  // The shatter: a bright burst, shards skittering, a few tinkling as they land.
  noiseHit(k, out, "highpass", 3800, 0.8, 0, 0.002, 0.5, 0.3);
  clickTrain(k, out, [0.02, 0.05, 0.09, 0.12, 0.17, 0.21, 0.27, 0.33], 5200, 3, 0.4, 0.025);
  for (const [start, hz] of [[0.06, 2637], [0.14, 3520], [0.24, 4186]] as const) tone(k, out, "sine", hz, start, 0.001, 0.07, start + 0.18);
  // The roar: two saws a few Hz apart, growling through a low-pass that opens and shuts.
  const throat = biquad(k, "lowpass", 380, 3);
  sweep(k, throat.frequency, 380, 900, 0.3, 0.6);
  glide(k, throat.frequency, 320, 1.05);
  chain(throat, heldEnvelope(k, 0.3, 0.1, 0.32, 0.85, 0.24, 1.08), out);
  for (const hz of [68, 72]) {
    const roar = oscillator(k, "sawtooth", hz, 0.3);
    vibrato(k, roar.frequency, 27, 5, 0.3, 1.08);
    chain(roar, throat);
    run(k, roar, 0.3, 1.08);
  }
  // The boom: a sub falling away, a rumble rolling under it.
  const boom = tone(k, out, "sine", 50, 0.95, 0.008, 0.5, len);
  glide(k, boom.frequency, 30, len);
  const rumble = noiseSource(k);
  chain(rumble, biquad(k, "lowpass", 130, 0.7), tremolo(k, 6, 0.35), envelope(k, 0.95, 0.02, 0.5, len), out);
  run(k, rumble, 0.95, len);
  return len;
};

/** 饕餮: a deep bronze bell, a long inhale, a gulp. */
const bronzeGulp: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The bell: ancient bronze, a low FM bell with its hum tone above it fading sooner.
  fmBell(k, out, 65, 2.4, 170, 0, 0.006, 0.45, 0.95);
  fmBell(k, out, 156, 1.6, 90, 0, 0.006, 0.14, 0.7);
  // The inhale: the beast drawing everything in, a band of air climbing as it swells.
  const air = noiseSource(k);
  const maw = biquad(k, "bandpass", 380, 1.1);
  sweep(k, maw.frequency, 380, 1900, 0.35, 1.08);
  chain(air, maw, heldEnvelope(k, 0.35, 0.55, 0.5, 1.02, 0.45, 1.1), out);
  run(k, air, 0.35, 1.1);
  // The gulp: a wet drop of pitch, then the swallow's thump.
  const gulp = tone(k, out, "sine", 420, 1.14, 0.006, 0.42, 1.3);
  glide(k, gulp.frequency, 120, 1.3);
  const swallow = tone(k, out, "triangle", 95, 1.22, 0.004, 0.3, len);
  glide(k, swallow.frequency, 60, len);
  return len;
};

/** YileGPT: a brush stroke, a soft modem chirp, a guqin note. */
const modemGuqin: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  // The stroke: one quick flick of the brush.
  const flick = noiseHit(k, out, "bandpass", 2400, 0.9, 0, 0.02, 0.36, 0.3);
  glide(k, flick.frequency, 1100, 0.3);
  // The chirp: a sine a square keys between a modem's two tones (1200 and 2200 Hz), softened.
  const chirp = oscillator(k, "sine", 1700, 0.34);
  const key = oscillator(k, "square", 28, 0.34);
  const shift = level(k, 500, 0.34);
  chain(key, shift);
  shift.connect(chirp.frequency);
  chain(chirp, biquad(k, "lowpass", 2600, 0.7), heldEnvelope(k, 0.34, 0.02, 0.13, 0.66, 0.1, 0.74), out);
  run(k, chirp, 0.34, 0.74);
  run(k, key, 0.34, 0.74);
  // The guqin: one string plucked, bending up as the finger slides.
  const string = pluck(k, out, 196, 0.8, 0.28, len, 1900);
  sweep(k, string.frequency, 196, 220, 0.9, 1.05);
  return len;
};

/** YileGPT Prime: a thunderous brush stroke, a server hum, a choir of chirps. */
const serverChoir: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  // The stroke: a brush the size of the sky, a dark sweep of noise over a sub thump.
  const sky = noiseHit(k, out, "lowpass", 1600, 1.2, 0, 0.015, 0.6, 0.5);
  glide(k, sky.frequency, 260, 0.5);
  const thump = tone(k, out, "sine", 58, 0, 0.005, 0.45, 0.4);
  glide(k, thump.frequency, 36, 0.4);
  // The hum: a server room's mains buzz in three harmonics, its fans breathing under it.
  const mains = biquad(k, "lowpass", 420, 1);
  chain(mains, heldEnvelope(k, 0.3, 0.12, 0.2, 1.3, 0.16, 1.45), out);
  for (const [hz, weight] of [[60, 1], [120, 0.5], [180, 0.33]] as const) {
    const hum = oscillator(k, "square", hz, 0.3);
    chain(hum, level(k, weight, 0.3), mains);
    run(k, hum, 0.3, 1.45);
  }
  const fans = noiseSource(k);
  chain(fans, biquad(k, "lowpass", 900, 0.7), heldEnvelope(k, 0.3, 0.15, 0.14, 1.3, 0.12, 1.45), out);
  run(k, fans, 0.3, 1.45);
  // The choir: chirps rising in a staggered chorus, each on its own pitch.
  for (const [start, hz] of [[0.7, 1400], [0.78, 1760], [0.86, 1570], [0.95, 2100], [1.03, 1870], [1.12, 2350]] as const) {
    const chirp = tone(k, out, "sine", hz, start, 0.01, 0.09, start + 0.16);
    glide(k, chirp.frequency, hz * 1.5, start + 0.16);
  }
  return len;
};

/** Yile's Virus: a dial-up screech chopped into stutters. */
const dialupStutter: SfxRecipe = (ctx, out, at) => {
  const len = 1.2;
  const k = kit(ctx, out, at, len);
  // Everything runs through one gate a square slams open and shut: the stutter.
  const gate = modulatedGain(k, "square", 13, 0.5, 0.5);
  chain(gate, out);
  // The dial: a touch-tone pair.
  tone(k, gate, "sine", 697, 0, 0.005, 0.18, 0.22);
  tone(k, gate, "sine", 1209, 0, 0.005, 0.18, 0.22);
  // The handshake: a square screech warbling up through a band, then the carrier's hiss.
  const screech = oscillator(k, "square", 1100, 0.22);
  glide(k, screech.frequency, 2300, 0.7);
  vibrato(k, screech.frequency, 31, 260, 0.22, 0.8);
  chain(screech, biquad(k, "bandpass", 1900, 1.5), heldEnvelope(k, 0.22, 0.01, 0.3, 0.72, 0.24, 0.8), gate);
  run(k, screech, 0.22, 0.8);
  noiseHit(k, gate, "bandpass", 2800, 0.8, 0.78, 0.01, 0.4, len);
  return len;
};

/** AI Girlfriend: a hologram chime, a soft bow, palms snapping a block. */
const holoBow: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  // The hologram: two bright sines gliding up together, shimmering fast, a sparkle of air over them.
  for (const [from, to] of [[1760, 2093], [2217, 2637]] as const) {
    const shimmer = oscillator(k, "sine", from);
    glide(k, shimmer.frequency, to, 0.2);
    vibrato(k, shimmer.frequency, 11, 18, 0, 0.55);
    chain(shimmer, heldEnvelope(k, 0, 0.02, 0.12, 0.3, 0.08, 0.55), out);
    run(k, shimmer, 0, 0.55);
  }
  noiseHit(k, out, "highpass", 6000, 0.7, 0, 0.01, 0.12, 0.35);
  // The bow: a silk sleeve's soft swish as she bends.
  const swish = noiseHit(k, out, "bandpass", 1100, 1.1, 0.5, 0.12, 0.3, 0.9);
  sweep(k, swish.frequency, 1100, 480, 0.5, 0.9);
  // The block: two palms snapping together, a slap with the body's thump under it.
  clickTrain(k, out, [0.96, 0.985], 1700, 1.1, 0.8, 0.06);
  const body = tone(k, out, "triangle", 190, 0.96, 0.002, 0.3, 1.12);
  glide(k, body.frequency, 120, 1.12);
  return len;
};

/** Devin Bot: a synth hum, then a quick run of pops. */
const synthPops: SfxRecipe = (ctx, out, at) => {
  const len = 1.2;
  const k = kit(ctx, out, at, len);
  // The hum: two saws a hair apart powering up behind a low-pass that opens as it warms.
  const warm = biquad(k, "lowpass", 260, 4);
  glide(k, warm.frequency, 1400, 0.55);
  chain(warm, heldEnvelope(k, 0, 0.18, 0.24, 0.55, 0.18, 0.68), out);
  for (const hz of [110, 110.9]) {
    const saw = oscillator(k, "sawtooth", hz);
    chain(saw, warm);
    run(k, saw, 0, 0.68);
  }
  // The pops: eight bubbles bursting in a quick climbing run, each a sine dropping fast.
  for (let n = 0; n < 8; n += 1) {
    const start = 0.6 + n * 0.065;
    const pop = tone(k, out, "sine", 700 + n * 90, start, 0.002, 0.3, start + 0.05);
    glide(k, pop.frequency, 180 + n * 20, start + 0.035);
  }
  return len;
};

/** Skull of J'Nari: a rattling jaw, a ghostly cough, an interrupted mutter. */
const skullMutter: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  // The jaw: bone chattering on bone, fast and hollow.
  clickTrain(k, out, [0, 0.045, 0.085, 0.13, 0.17, 0.215, 0.25, 0.3, 0.34], 850, 5, 1.4, 0.03);
  // The cough: two hacks from far off, each a burst of breath over a voice dropping under it.
  for (const start of [0.45, 0.63]) {
    noiseHit(k, out, "bandpass", 750, 1.3, start, 0.01, 1, start + 0.16);
    const hack = tone(k, out, "sawtooth", 210, start, 0.01, 0.12, start + 0.14);
    glide(k, hack.frequency, 140, start + 0.14);
  }
  // The mutter: a voice wandering through a vowel, its syllables chopped, cut off mid-word.
  const mutter = oscillator(k, "sawtooth", 125, 0.88);
  vibrato(k, mutter.frequency, 3, 14, 0.88, 1.33);
  chain(mutter, biquad(k, "bandpass", 620, 2.5), modulatedGain(k, "square", 8, 0.55, 0.45), heldEnvelope(k, 0.88, 0.04, 1, 1.3, 0.95, 1.33), out);
  run(k, mutter, 0.88, 1.33);
  return len;
};

/** Keymaster Keenus: a key ring's jangle, then locks clicking open in turn. */
const keyJangle: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  // The jangle: keys tumbling on their ring, small metal bells over a rattle.
  for (const [start, hz] of [[0, 3150], [0.05, 4020], [0.09, 3560], [0.16, 4480], [0.22, 3330], [0.3, 3900]] as const) {
    fmBell(k, out, hz, 1.41, 1600, start, 0.001, 0.16, start + 0.16);
  }
  const rattle = noiseSource(k);
  chain(rattle, biquad(k, "highpass", 5000, 0.7), modulatedGain(k, "square", 23, 0.5, 0.5), envelope(k, 0, 0.01, 0.5, 0.42), out);
  run(k, rattle, 0, 0.42);
  // The locks: three turning in turn, each a clack, then its bolt sliding back, a step higher each.
  for (const [n, start] of [0.55, 0.8, 1.05].entries()) {
    clickTrain(k, out, [start, start + 0.06], 1100 + n * 300, 3, 1.2, 0.04);
    const bolt = tone(k, out, "triangle", 300 + n * 60, start + 0.06, 0.002, 0.45, start + 0.16);
    glide(k, bolt.frequency, 200 + n * 40, start + 0.16);
  }
  return len;
};

/** Pareto Optimality: a cane whistling through the air, a sharp crack, a tut. */
const caneCrack: SfxRecipe = (ctx, out, at) => {
  const len = 1.1;
  const k = kit(ctx, out, at, len);
  // The whistle: air torn by the swing, a narrow band rising as the cane speeds up.
  const swing = noiseSource(k);
  const air = biquad(k, "bandpass", 500, 7);
  glide(k, air.frequency, 2600, 0.34);
  chain(swing, air, heldEnvelope(k, 0, 0.26, 0.7, 0.34, 0.6, 0.38), out);
  run(k, swing, 0, 0.38);
  // The crack: the cane meeting the desk, a bright snap and its wooden knock.
  noiseHit(k, out, "highpass", 1500, 0.7, 0.35, 0.001, 0.45, 0.45);
  const knock = tone(k, out, "triangle", 720, 0.35, 0.001, 0.3, 0.55);
  glide(k, knock.frequency, 320, 0.55);
  // The tut: one disapproving click of the tongue.
  clickTrain(k, out, [0.88], 2700, 5, 0.6, 0.025);
  tone(k, out, "sine", 1150, 0.88, 0.001, 0.12, 0.93);
  return len;
};

/** The Cane: thunder rolling, a lacquer creak, a sharp rap on wood. */
const caneThunder: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  // The thunder: a crack overhead, then dark noise rolling away under a slow swell and sag.
  noiseHit(k, out, "bandpass", 1200, 0.6, 0, 0.002, 0.55, 0.12);
  const thunder = noiseSource(k);
  const sky = biquad(k, "lowpass", 420, 0.7);
  glide(k, sky.frequency, 160, 0.85);
  chain(thunder, sky, modulatedGain(k, "sine", 4.5, 0.65, 0.35), heldEnvelope(k, 0.02, 0.06, 1, 0.5, 0.7, 0.85), out);
  run(k, thunder, 0.02, 0.85);
  // The creak: lacquered wood flexing, slow pulses ringing a narrow band, quickening.
  const flex = oscillator(k, "sawtooth", 32, 0.8);
  glide(k, flex.frequency, 58, 1.2);
  chain(flex, biquad(k, "bandpass", 1350, 9), heldEnvelope(k, 0.8, 0.05, 1.6, 1.15, 1.2, 1.22), out);
  run(k, flex, 0.8, 1.22);
  // The rap: the cane's tip on the boards, a hard knock and its hollow note.
  clickTrain(k, out, [1.28], 950, 4, 1.3, 0.07);
  const rap = tone(k, out, "triangle", 440, 1.28, 0.001, 0.55, 1.5);
  glide(k, rap.frequency, 250, 1.5);
  return len;
};

/** Ever Growing Tree: creaking wood, a deep root groan, rustling leaves. */
const rootCreak: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The creak: a trunk bending, a triangle's pitch wavering through a woody band, catching as it goes.
  const trunk = oscillator(k, "triangle", 190);
  vibrato(k, trunk.frequency, 6, 28, 0, 0.5);
  glide(k, trunk.frequency, 150, 0.5);
  chain(trunk, biquad(k, "bandpass", 520, 4), modulatedGain(k, "square", 15, 0.6, 0.4), heldEnvelope(k, 0, 0.08, 0.5, 0.4, 0.4, 0.5), out);
  run(k, trunk, 0, 0.5);
  // The groan: roots pushing through earth, a deep saw climbing slowly as it grows.
  const root = oscillator(k, "sawtooth", 38, 0.3);
  glide(k, root.frequency, 54, 1.1);
  chain(root, biquad(k, "lowpass", 190, 3), heldEnvelope(k, 0.3, 0.25, 0.6, 0.95, 0.45, 1.15), out);
  run(k, root, 0.3, 1.15);
  // The leaves: a canopy stirring, high noise fluttering at two uneven rates.
  const leaves = noiseSource(k);
  chain(
    leaves,
    biquad(k, "highpass", 3600, 0.7),
    modulatedGain(k, "triangle", 17, 0.6, 0.4),
    modulatedGain(k, "sine", 5, 0.7, 0.3),
    heldEnvelope(k, 0.85, 0.12, 0.35, 1.3, 0.25, len),
    out,
  );
  run(k, leaves, 0.85, len);
  return len;
};

/** Touched by KY: a chalk squeak, a gasp, a rush of pages. */
const chalkGasp: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  // The squeak: chalk skipping on the board, a square's band stuttering as it sticks and slips.
  const chalk = oscillator(k, "square", 1950);
  glide(k, chalk.frequency, 1620, 0.3);
  chain(chalk, biquad(k, "bandpass", 2300, 5), modulatedGain(k, "square", 36, 0.5, 0.5), heldEnvelope(k, 0, 0.01, 0.6, 0.26, 0.45, 0.3), out);
  run(k, chalk, 0, 0.3);
  // The gasp: a sharp intake, air through a band jumping up, a breath of voice in it.
  const gasp = noiseHit(k, out, "bandpass", 1400, 1.3, 0.38, 0.03, 1, 0.62);
  sweep(k, gasp.frequency, 1400, 2600, 0.38, 0.5);
  const voice = tone(k, out, "sine", 620, 0.38, 0.03, 0.14, 0.6);
  glide(k, voice.frequency, 820, 0.5);
  // The pages: a book riffling past, flicks quickening then slowing under a whoosh of air.
  clickTrain(k, out, [0.66, 0.71, 0.75, 0.78, 0.81, 0.84, 0.87, 0.9, 0.94, 0.99, 1.06], 2900, 1.2, 0.9, 0.025);
  noiseHit(k, out, "bandpass", 1600, 0.6, 0.64, 0.12, 0.45, 1.22);
  return len;
};

/** Aluneth: whispered pages, a frosty chime, three wingbeats. */
const frostPages: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The pages: a book turning itself, a soft high rustle with two leaves settling.
  const leaf = noiseSource(k);
  chain(leaf, biquad(k, "bandpass", 4200, 1.8), tremolo(k, 12, 0.4), heldEnvelope(k, 0, 0.08, 0.7, 0.36, 0.5, 0.45), out);
  run(k, leaf, 0, 0.45);
  clickTrain(k, out, [0.18, 0.37], 3100, 1.5, 0.6, 0.03);
  // The chime: ice struck, a bright FM bell and a glassy fifth shimmering above it.
  fmBell(k, out, 2637, 3.01, 700, 0.42, 0.002, 0.32, 1.1);
  const frost = tremolo(k, 9, 0.35);
  chain(frost, out);
  tone(k, frost, "sine", 3951, 0.44, 0.004, 0.14, 1.15);
  // The wings: three great beats, each a low push of air whose band falls as it lands.
  for (const start of [0.82, 1.03, 1.24]) {
    const beat = noiseHit(k, out, "lowpass", 900, 1.2, start, 0.06, 1, start + 0.22);
    sweep(k, beat.frequency, 900, 320, start, start + 0.22);
  }
  return len;
};

/** Mayor Medinamogger: a coin's ping, a distant crowd cheering. */
const coinCheer: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  // The ping: a flipped coin's tick and its two bright inharmonic modes ringing after.
  clickTrain(k, out, [0], 6000, 2, 0.35, 0.01);
  tone(k, out, "sine", 3520, 0, 0.001, 0.18, 0.6);
  tone(k, out, "sine", 4730, 0, 0.001, 0.1, 0.38);
  // The cheer: a far crowd swelling, broad muffled noise with a few voices whooping up inside it.
  const crowd = noiseSource(k);
  chain(crowd, biquad(k, "bandpass", 1000, 0.6), biquad(k, "lowpass", 1800, 0.7), heldEnvelope(k, 0.25, 0.35, 0.55, 1.1, 0.45, len), out);
  run(k, crowd, 0.25, len);
  const far = biquad(k, "lowpass", 1300, 0.7);
  chain(far, out);
  for (const [start, from, to] of [[0.35, 320, 470], [0.48, 410, 590], [0.6, 360, 540]] as const) {
    const woo = oscillator(k, "sawtooth", from, start);
    glide(k, woo.frequency, to, start + 0.35);
    vibrato(k, woo.frequency, 6, 9, start, start + 0.6);
    chain(woo, heldEnvelope(k, start, 0.12, 0.06, start + 0.45, 0.05, start + 0.6), far);
    run(k, woo, start, start + 0.6);
  }
  return len;
};

/** Tatches the Totem: five different drums, one after another. */
const totemDrums: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  // A frame drum: a soft low skin.
  const frame = tone(k, out, "sine", 78, 0, 0.004, 0.5, 0.28);
  glide(k, frame.frequency, 52, 0.28);
  noiseHit(k, out, "lowpass", 400, 0.7, 0, 0.002, 0.25, 0.08);
  // A tabla: a ringing head whose pitch bends up under the palm.
  const tabla = tone(k, out, "sine", 300, 0.22, 0.002, 0.35, 0.5);
  glide(k, tabla.frequency, 370, 0.32);
  clickTrain(k, out, [0.22], 2400, 2, 0.4, 0.015);
  // A snare: a taut skin with its wires hissing.
  tone(k, out, "triangle", 200, 0.44, 0.002, 0.35, 0.56);
  noiseHit(k, out, "highpass", 1800, 0.7, 0.44, 0.002, 0.45, 0.62);
  // A log drum: hollow wood, a short bright knock.
  const log = tone(k, out, "triangle", 520, 0.66, 0.001, 0.4, 0.82);
  glide(k, log.frequency, 440, 0.82);
  noiseHit(k, out, "bandpass", 1300, 3, 0.66, 0.001, 0.4, 0.7);
  // A taiko: the biggest last, a deep boom and its rumble.
  const taiko = tone(k, out, "sine", 64, 0.9, 0.005, 0.6, len);
  glide(k, taiko.frequency, 38, len);
  noiseHit(k, out, "lowpass", 160, 0.7, 0.9, 0.003, 0.5, 1.2);
  return len;
};

/** Call to Chaos (Meditative Edition): a temple bell cut short by a crash, a rattling die. */
const templeDie: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  // The bell: a bowl bell's clear note and its high partial, choked off as the crash lands.
  for (const [hz, peak] of [[880, 0.26], [2420, 0.08]] as const) {
    const bell = oscillator(k, "sine", hz);
    chain(bell, heldEnvelope(k, 0, 0.003, peak, 0.3, peak * 0.75, 0.34), out);
    run(k, bell, 0, 0.34);
  }
  // The crash: a cymbal-bright burst and a heavy body under it.
  const crash = noiseSource(k);
  chain(crash, biquad(k, "highpass", 2400, 0.7), heldEnvelope(k, 0.3, 0.002, 0.55, 0.36, 0.3, 0.8), out);
  run(k, crash, 0.3, 0.8);
  noiseHit(k, out, "lowpass", 500, 1, 0.3, 0.003, 0.6, 0.55);
  // The die: one die skittering across the table, bright corners and a hollow bounce, then still.
  clickTrain(k, out, [0.72, 0.84, 0.93, 1, 1.06, 1.11, 1.15], 2200, 5, 0.5, 0.025);
  clickTrain(k, out, [0.78, 0.89, 0.97, 1.21], 640, 4, 0.5, 0.04);
  return len;
};

/** CN Golem: stone blocks locking, a lacquer creak, a ground-shaking stomp. */
const golemStomp: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The blocks: stone grinding into place, then two heavy clunks as they lock.
  const grind = noiseSource(k);
  chain(grind, biquad(k, "lowpass", 650, 1.5), modulatedGain(k, "sawtooth", 17, 0.6, 0.4), heldEnvelope(k, 0, 0.04, 0.5, 0.2, 0.4, 0.26), out);
  run(k, grind, 0, 0.26);
  for (const start of [0.24, 0.4]) {
    const clunk = tone(k, out, "triangle", 130, start, 0.002, 0.45, start + 0.13);
    glide(k, clunk.frequency, 85, start + 0.13);
    noiseHit(k, out, "bandpass", 420, 1.5, start, 0.002, 0.5, start + 0.08);
  }
  // The creak: its lacquered joints flexing, noise through a tight band, catching and slipping.
  const joint = noiseSource(k);
  const lacquer = biquad(k, "bandpass", 880, 14);
  sweep(k, lacquer.frequency, 880, 640, 0.56, 0.9);
  chain(joint, lacquer, modulatedGain(k, "square", 21, 0.5, 0.5), heldEnvelope(k, 0.56, 0.05, 1.6, 0.85, 1.2, 0.92), out);
  run(k, joint, 0.56, 0.92);
  // The stomp: a sub that shakes the floor, its rumble, debris pattering down after.
  const stomp = tone(k, out, "sine", 44, 0.96, 0.006, 0.6, len);
  glide(k, stomp.frequency, 27, len);
  noiseHit(k, out, "lowpass", 110, 0.7, 0.96, 0.004, 0.6, 1.35);
  clickTrain(k, out, [1.08, 1.15, 1.19, 1.27, 1.34], 520, 2, 0.25, 0.03);
  return len;
};

/** Jlockeed's Evil Blueprints: paper unrolling, a klaxon, gauntlets slamming down on steel. */
const klaxonSlam: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  // The blueprint: a stiff roll unfurling, its crackle climbing as the sheet flattens.
  const paper = noiseSource(k);
  const sheet = biquad(k, "bandpass", 1300, 0.9);
  glide(k, sheet.frequency, 3200, 0.34);
  chain(paper, sheet, modulatedGain(k, "triangle", 24, 0.6, 0.4), heldEnvelope(k, 0, 0.04, 0.4, 0.28, 0.32, 0.34), out);
  run(k, paper, 0, 0.34);
  // The klaxon: one "awooga", a square scooping up a fourth through a horn's band, throbbing.
  const klaxon = oscillator(k, "square", 300, 0.36);
  glide(k, klaxon.frequency, 300, 0.42);
  glide(k, klaxon.frequency, 410, 0.7);
  chain(klaxon, biquad(k, "bandpass", 900, 1.2), tremolo(k, 9, 0.2), heldEnvelope(k, 0.36, 0.02, 0.3, 0.78, 0.25, 0.85), out);
  run(k, klaxon, 0.36, 0.85);
  // The slam: two gauntlets on a steel desk, each a ringing plate, the clank's grit and a dull thud.
  for (const start of [0.95, 1.03]) {
    fmBell(k, out, 410, 2.9, 900, start, 0.001, 0.2, start + 0.5);
    noiseHit(k, out, "bandpass", 2500, 1.2, start, 0.001, 0.5, start + 0.07);
    const thud = tone(k, out, "sine", 90, start, 0.003, 0.4, start + 0.2);
    glide(k, thud.frequency, 55, start + 0.2);
  }
  return len;
};

/** Prison: an iron clang, rattling chains, a cell door slamming. */
const cellSlam: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  // The clang: iron bars struck, a low inharmonic ring and its strike.
  fmBell(k, out, 196, 2.76, 520, 0, 0.001, 0.35, 0.65);
  noiseHit(k, out, "bandpass", 1600, 1, 0, 0.001, 0.4, 0.05);
  // The chains: links rattling over one another, two bright bands of clicks interleaved.
  clickTrain(k, out, [0.3, 0.37, 0.42, 0.5, 0.55, 0.61, 0.68, 0.73], 3300, 6, 0.45, 0.03);
  clickTrain(k, out, [0.33, 0.4, 0.46, 0.53, 0.58, 0.65, 0.71], 4700, 7, 0.35, 0.025);
  // The door: a heavy slam, the frame's low boom, the bars ringing briefly, the bolt shooting home.
  const slam = tone(k, out, "sine", 72, 0.85, 0.003, 0.55, 1.3);
  glide(k, slam.frequency, 44, 1.3);
  noiseHit(k, out, "lowpass", 600, 0.8, 0.85, 0.002, 0.6, 1.05);
  tone(k, out, "sine", 330, 0.85, 0.002, 0.12, 1.35);
  tone(k, out, "sine", 467, 0.85, 0.002, 0.08, 1.2);
  clickTrain(k, out, [1.18], 1500, 3, 0.6, 0.04);
  return len;
};

/** Jlockeed's Headquarters: hydraulic doors grinding, a radar ping, truck engines revving. */
const hangarRev: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  // The doors: hydraulics hissing as steel grinds along its track.
  noiseHit(k, out, "highpass", 2800, 0.7, 0, 0.06, 0.18, 0.55);
  const track = oscillator(k, "sawtooth", 62);
  glide(k, track.frequency, 74, 0.55);
  chain(track, biquad(k, "bandpass", 320, 3), tremolo(k, 14, 0.35), heldEnvelope(k, 0, 0.08, 0.45, 0.48, 0.38, 0.56), out);
  run(k, track, 0, 0.56);
  // The radar: one ping and its fainter echo.
  for (const [start, peak] of [[0.56, 0.22], [0.74, 0.08]] as const) tone(k, out, "sine", 1180, start, 0.003, peak, start + 0.3);
  // The engines: two diesels a few Hz apart revving up and settling, their firing throbbing through a low-pass.
  const exhaust = biquad(k, "lowpass", 280, 2);
  sweep(k, exhaust.frequency, 280, 1000, 0.85, 1.2);
  glide(k, exhaust.frequency, 420, len);
  chain(exhaust, modulatedGain(k, "square", 19, 0.65, 0.35), heldEnvelope(k, 0.85, 0.06, 0.3, 1.4, 0.22, len), out);
  for (const hz of [44, 59]) {
    const engine = oscillator(k, "square", hz, 0.85);
    glide(k, engine.frequency, hz * 2, 1.18);
    glide(k, engine.frequency, hz * 1.3, len);
    chain(engine, exhaust);
    run(k, engine, 0.85, len);
  }
  return len;
};
// R3395000: the signature sounds of Stinkyscape 001 to 090's Legendaries and Mythics, one each
// (audio/signatureCues.ts plays them), every one layering its card's sounds in a graph of its own. A noise
// that starts late opens its envelope 10 ms before it: Chrome can play a buffer source's first frame one
// frame before a gain event set for the same instant (0.55 s at 44.1 kHz), at the gain's default of 1.

/** Tylannt: vault tumblers clicking into place, a bolt drawn, then a heavy door's hiss as it swings. */
const vaultHiss: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.09, 0.17, 0.26, 0.34], 2200, 5, 0.45, 0.04);
  const bolt = tone(k, out, "triangle", 190, 0.44, 0.003, 0.3, 0.62);
  glide(k, bolt.frequency, 110, 0.6);
  const hiss = noiseSource(k);
  const seal = biquad(k, "highpass", 1600, 0.7);
  glide(k, seal.frequency, 3800, len);
  chain(hiss, seal, heldEnvelope(k, 0.54, 0.09, 0.22, 1.05, 0.14, len), out);
  run(k, hiss, 0.55, len);
  const swing = tone(k, out, "sine", 58, 0.5, 0.12, 0.2, 1.3);
  glide(k, swing.frequency, 46, 1.3);
  return len;
};

/** The Greatest Heist: a safe dial ticking round until it catches, then a soft alarm chirping. */
const safeChirp: SfxRecipe = (ctx, out, at) => {
  const len = 1.2;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.06, 0.12, 0.17, 0.22, 0.27, 0.33, 0.4, 0.48], 4200, 8, 0.5, 0.02);
  clickTrain(k, out, [0.6], 950, 3, 0.6, 0.07);
  const soft = biquad(k, "lowpass", 2600, 0.7);
  chain(soft, out);
  for (const start of [0.74, 0.9, 1.06]) {
    const chirp = oscillator(k, "sine", 1500, start);
    glide(k, chirp.frequency, 2200, start + 0.08);
    chain(chirp, envelope(k, start, 0.01, 0.24, start + 0.12), soft);
    run(k, chirp, start, start + 0.12);
  }
  return len;
};

/** Rod of Ages: a deep stone hum, and a chime as each band of the rod lights. */
const stoneChime: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const stone = biquad(k, "lowpass", 320, 1.2);
  chain(stone, tremolo(k, 3, 0.2), heldEnvelope(k, 0, 0.25, 0.2, 1.2, 0.14, len), out);
  for (const [type, hz] of [["sine", 49], ["triangle", 98]] as const) {
    const hum = oscillator(k, type, hz);
    chain(hum, stone);
    run(k, hum, 0, len);
  }
  const bands = [659, 784, 988, 1319];
  for (const [n, hz] of bands.entries()) {
    const start = 0.25 + n * 0.24;
    fmBell(k, out, hz, 3.5, 280, start, 0.004, 0.14, n === bands.length - 1 ? len : start + 0.6);
  }
  return len;
};

/** Imitated Instrumentality: three rising boot tones, then a cold electronic chord. */
const magiChord: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const boot = biquad(k, "lowpass", 2400, 0.7);
  chain(boot, out);
  for (const [n, hz] of [330, 440, 587].entries()) {
    const start = n * 0.16;
    tone(k, boot, "square", hz, start, 0.004, 0.16, start + 0.11);
  }
  const cold = biquad(k, "lowpass", 4200, 1.5);
  glide(k, cold.frequency, 900, len);
  chain(cold, heldEnvelope(k, 0.5, 0.05, 0.5, 1.1, 0.35, len), out);
  for (const hz of [220, 261.6, 329.6, 415.3]) {
    const voice = oscillator(k, "sawtooth", hz, 0.5);
    chain(voice, level(k, 0.22), cold);
    run(k, voice, 0.5, len);
  }
  return len;
};

/** ZolarKeth: a lecture-hall bell rung for class, then a rocket's distant rumble. */
const lectureRumble: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  fmBell(k, out, 1046, 2.4, 500, 0, 0.003, 0.26, 0.8);
  clickTrain(k, out, [0, 0.045, 0.09, 0.135, 0.18, 0.225, 0.27], 3100, 6, 0.4, 0.03);
  const rumble = noiseSource(k);
  const far = biquad(k, "lowpass", 260, 0.5);
  glide(k, far.frequency, 140, len);
  chain(rumble, far, modulatedGain(k, "sine", 6, 0.75, 0.25), heldEnvelope(k, 0.49, 0.36, 0.75, 1.25, 0.55, len), out);
  run(k, rumble, 0.5, len);
  const sub = tone(k, out, "sine", 40, 0.55, 0.3, 0.22, len);
  glide(k, sub.frequency, 33, len);
  return len;
};

/** Drone Core VIII: a factory press stamping, each blow coming sooner than the last. */
const pressStamp: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const k = kit(ctx, out, at, len);
  const blows = [0, 0.32, 0.56, 0.75, 0.9, 1.02, 1.12];
  clickTrain(k, out, blows, 1200, 2, 0.5, 0.06);
  for (const start of blows) {
    const ram = tone(k, out, "sine", 95, start, 0.003, 0.36, start + 0.15);
    glide(k, ram.frequency, 48, start + 0.12);
  }
  const hydraulics = biquad(k, "lowpass", 220, 1);
  chain(hydraulics, heldEnvelope(k, 0, 0.1, 0.18, 1.1, 0.24, len), out);
  const hum = oscillator(k, "sawtooth", 55);
  glide(k, hum.frequency, 82, 1.12);
  chain(hum, hydraulics);
  run(k, hum, 0, len);
  return len;
};

/** Drone Core IX: three deep engine roars, one after another, each a little higher. */
const corvetteRoar: SfxRecipe = (ctx, out, at) => {
  const len = 1.45;
  const k = kit(ctx, out, at, len);
  for (const [n, hz] of [44, 52, 62].entries()) {
    const start = n * 0.4;
    const stop = start + 0.6;
    const throat = biquad(k, "lowpass", 260, 2);
    glide(k, throat.frequency, 900, start + 0.2);
    glide(k, throat.frequency, 300, stop);
    chain(throat, envelope(k, start, 0.07, 0.3, stop), out);
    for (const detune of [1, 1.02]) {
      const engine = oscillator(k, "sawtooth", hz * detune, start);
      glide(k, engine.frequency, hz * detune * 1.5, start + 0.2);
      glide(k, engine.frequency, hz * detune, stop);
      chain(engine, throat);
      run(k, engine, start, stop);
    }
    const grit = noiseSource(k);
    chain(grit, biquad(k, "lowpass", 600, 0.7), envelope(k, start - 0.01, 0.06, 0.28, stop), out);
    run(k, grit, start, stop);
  }
  return len;
};

/** Kwzka: a crowd's gasp, then a bright pop-chorus sting. */
const popGasp: SfxRecipe = (ctx, out, at) => {
  const len = 1.2;
  const sting = 0.45;
  const k = kit(ctx, out, at, len);
  for (const [hz, to] of [[900, 1700], [1600, 2900]] as const) {
    const gasp = noiseSource(k);
    const mouth = biquad(k, "bandpass", hz, 1.2);
    glide(k, mouth.frequency, to, 0.35);
    chain(gasp, mouth, envelope(k, 0, 0.18, 0.45, 0.4), out);
    run(k, gasp, 0, 0.4);
  }
  clickTrain(k, out, [sting], 1500, 1, 0.6, 0.08);
  const shine = biquad(k, "lowpass", 5000, 0.7);
  chain(shine, heldEnvelope(k, sting, 0.01, 0.4, 0.75, 0.25, len), out);
  for (const hz of [784, 988, 1175, 1568]) {
    const voice = oscillator(k, "square", hz, sting);
    vibrato(k, voice.frequency, 6, hz * 0.008, sting, len);
    chain(voice, level(k, 0.12), shine);
    run(k, voice, sting, len);
  }
  tone(k, out, "triangle", 196, sting, 0.005, 0.3, 0.9);
  return len;
};

/** The Narcor Project: glass clinking, a Geiger counter's crackle, a low hum. */
const geigerHum: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  fmBell(k, out, 2637, 2.76, 900, 0, 0.002, 0.16, 0.3);
  fmBell(k, out, 3136, 2.76, 900, 0.13, 0.002, 0.13, 0.42);
  clickTrain(k, out, [0.32, 0.35, 0.41, 0.44, 0.5, 0.52, 0.6, 0.63, 0.66, 0.74, 0.77, 0.85, 0.9, 0.97, 1.05, 1.16], 3500, 1.5, 0.55, 0.012);
  const mains = biquad(k, "lowpass", 400, 0.7);
  chain(mains, heldEnvelope(k, 0.3, 0.2, 0.15, 1.15, 0.11, len), out);
  for (const [type, hz] of [["sine", 60], ["triangle", 120]] as const) {
    const hum = oscillator(k, type, hz, 0.3);
    chain(hum, mains);
    run(k, hum, 0.3, len);
  }
  return len;
};

/** Great Paperclip Theory: a soft metallic twang, then a ratchet clicking ever faster and higher, without end. */
const wireRatchet: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const twang = tone(k, out, "triangle", 523, 0, 0.003, 0.4, 0.6);
  glide(k, twang.frequency, 494, 0.6);
  vibrato(k, twang.frequency, 7, 6, 0, 0.6);
  // Two ratchets an octave apart, each climbing in rate and pitch, so the climb seems never to end; their
  // rates are not multiples, so their clicks rarely land together.
  for (const [rate, hz, peak] of [[14, 700, 2], [19.5, 1400, 0.9]] as const) {
    const teeth = biquad(k, "bandpass", hz, 6);
    glide(k, teeth.frequency, hz * 3.2, len);
    const pawl = oscillator(k, "square", rate, 0.35);
    glide(k, pawl.frequency, rate * 2.6, len);
    chain(pawl, teeth, heldEnvelope(k, 0.35, 0.1, peak, 1.4, peak, len), out);
    run(k, pawl, 0.35, len);
  }
  return len;
};

/** Beam v5: a rising charge whine, a deep beam hum, the emitter ticking as it cools. */
const beamWhine: SfxRecipe = (ctx, out, at) => {
  const len = 1.45;
  const k = kit(ctx, out, at, len);
  const charge = tone(k, out, "sine", 600, 0, 0.3, 0.18, 0.42);
  glide(k, charge.frequency, 3200, 0.4);
  const beam = biquad(k, "lowpass", 420, 3);
  chain(beam, tremolo(k, 30, 0.25), heldEnvelope(k, 0.38, 0.04, 0.25, 0.95, 0.18, 1.1), out);
  for (const hz of [70, 70.7]) {
    const saw = oscillator(k, "sawtooth", hz, 0.38);
    chain(saw, beam);
    run(k, saw, 0.38, 1.1);
  }
  clickTrain(k, out, [1.12, 1.24, 1.38], 5000, 6, 0.45, 0.02);
  return len;
};

/** Repeater v5: rapid rattling bursts, each higher than the last, and a steam hiss as it vents. */
const rattleSteam: SfxRecipe = (ctx, out, at) => {
  const len = 1.35;
  const k = kit(ctx, out, at, len);
  for (const [n, hz] of [1100, 1700, 2600].entries()) {
    const start = n * 0.28;
    clickTrain(k, out, [0, 0.03, 0.06, 0.09, 0.12, 0.15, 0.18].map((t) => start + t), hz, 2, 0.7, 0.025);
  }
  const steam = noiseSource(k);
  const vent = biquad(k, "highpass", 3500, 0.7);
  glide(k, vent.frequency, 6000, len);
  chain(steam, vent, heldEnvelope(k, 0.83, 0.04, 0.38, 1.05, 0.22, len), out);
  run(k, steam, 0.84, len);
  return len;
};

/** Cannon v5: the breech clanking shut, then one thunderous boom. */
const breechBoom: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const fire = 0.4;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.12], 900, 4, 0.9, 0.07);
  fmBell(k, out, 340, 1.41, 200, 0.12, 0.002, 0.14, fire);
  const blast = noiseSource(k);
  const body = biquad(k, "lowpass", 1800, 0.7);
  glide(k, body.frequency, 1800, fire);
  glide(k, body.frequency, 90, len);
  chain(blast, body, envelope(k, fire - 0.01, 0.015, 0.6, len), out);
  run(k, blast, fire, len);
  const boom = tone(k, out, "sine", 72, fire, 0.004, 0.35, len);
  glide(k, boom.frequency, 28, len);
  return len;
};

/** Flak v5: three rolls of flak, then silence and a thin wind. */
const flakWind: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const rolls = [0, 0.24, 0.48];
  clickTrain(k, out, rolls.flatMap((start) => [start, start + 0.04, start + 0.09, start + 0.13]), 750, 0.8, 0.8, 0.09);
  for (const start of rolls) {
    const thump = tone(k, out, "sine", 130, start, 0.004, 0.3, start + 0.22);
    glide(k, thump.frequency, 60, start + 0.2);
  }
  const wind = noiseSource(k);
  const thin = biquad(k, "bandpass", 1800, 6);
  glide(k, thin.frequency, 2500, len);
  chain(wind, thin, tremolo(k, 1.5, 0.3), heldEnvelope(k, 0.94, 0.26, 0.5, 1.4, 0.35, len), out);
  run(k, wind, 0.95, len);
  return len;
};

/** Miner v5: a grinding crunch through rock, then a bright jackpot chime and coins. */
const crunchJackpot: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const strike = 0.6;
  const k = kit(ctx, out, at, len);
  const rock = noiseSource(k);
  chain(rock, biquad(k, "bandpass", 520, 1), modulatedGain(k, "square", 31, 0.55, 0.45), envelope(k, 0, 0.03, 0.9, 0.58), out);
  run(k, rock, 0, 0.58);
  const drill = oscillator(k, "sawtooth", 78);
  vibrato(k, drill.frequency, 9, 6, 0, 0.58);
  chain(drill, biquad(k, "lowpass", 500, 2), envelope(k, 0, 0.05, 0.35, 0.58), out);
  run(k, drill, 0, 0.58);
  const chimes = [1319, 1568, 2093];
  for (const [n, hz] of chimes.entries()) {
    const start = strike + n * 0.08;
    fmBell(k, out, hz, 2, 350, start, 0.003, 0.28, n === chimes.length - 1 ? len : start + 0.5);
  }
  clickTrain(k, out, [0.62, 0.7, 0.78, 0.86, 0.94, 1.02], 6200, 4, 0.8, 0.03);
  return len;
};

/** Z'Zoth: a low choir humming and the creak of great hulls. */
const choirHulls: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const k = kit(ctx, out, at, len);
  const choir = biquad(k, "bandpass", 650, 1.4);
  chain(choir, heldEnvelope(k, 0, 0.35, 1, 1.2, 0.75, len), out);
  for (const [hz, rate] of [[98, 4.6], [147, 5.2], [196, 5.8], [99, 5]] as const) {
    const voice = oscillator(k, "sawtooth", hz);
    vibrato(k, voice.frequency, rate, hz * 0.012, 0, len);
    chain(voice, level(k, 0.3), choir);
    run(k, voice, 0, len);
  }
  const hull = biquad(k, "bandpass", 380, 9);
  chain(hull, envelope(k, 0.4, 0.15, 1, 1.2), out);
  const creak = oscillator(k, "sawtooth", 62, 0.4);
  glide(k, creak.frequency, 48, 1.2);
  vibrato(k, creak.frequency, 4, 7, 0.4, 1.2);
  chain(creak, hull);
  run(k, creak, 0.4, 1.2);
  clickTrain(k, out, [0.55, 0.62, 0.93, 1, 1.28], 900, 6, 0.4, 0.04);
  return len;
};

/** A'Zoth: a brassy horn blast, echoing back in fainter, darker layers. */
const hornEcho: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const echoes = [0, 1, 2, 3];
  for (const n of echoes) {
    const start = n * 0.28;
    const stop = n === echoes.length - 1 ? len : start + 0.45;
    const fade = 0.55 ** n;
    const brass = biquad(k, "lowpass", 1800 / (1 + n), 1.5);
    chain(brass, heldEnvelope(k, start, 0.04, 0.32 * fade, start + 0.25, 0.24 * fade, stop), out);
    for (const hz of [147, 148.5]) {
      const pipe = oscillator(k, "sawtooth", hz, start);
      glide(k, pipe.frequency, hz * 1.03, start + 0.06);
      chain(pipe, brass);
      run(k, pipe, start, stop);
    }
  }
  return len;
};

/** Paragon Mk2: a jet's scream, then a shield's glassy snap. */
const jetSnap: SfxRecipe = (ctx, out, at) => {
  const len = 1.3;
  const snap = 0.82;
  const k = kit(ctx, out, at, len);
  const jet = noiseSource(k);
  const scream = biquad(k, "bandpass", 1400, 2);
  glide(k, scream.frequency, 5200, 0.5);
  glide(k, scream.frequency, 2600, 0.85);
  chain(jet, scream, heldEnvelope(k, 0, 0.15, 0.7, 0.6, 0.5, 0.85), out);
  run(k, jet, 0, 0.85);
  const turbine = tone(k, out, "sawtooth", 900, 0, 0.2, 0.06, 0.8);
  glide(k, turbine.frequency, 2700, 0.6);
  const crack = noiseSource(k);
  chain(crack, biquad(k, "highpass", 5000, 0.7), envelope(k, snap - 0.01, 0.011, 0.5, snap + 0.05), out);
  run(k, crack, snap, snap + 0.05);
  fmBell(k, out, 2349, 3.17, 1200, snap, 0.002, 0.16, len);
  fmBell(k, out, 3520, 1.73, 800, snap + 0.02, 0.002, 0.1, len);
  return len;
};

/** Paragon-X: a roar of white noise, then one clean high ring. */
const whiteRing: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const ring = 0.72;
  const k = kit(ctx, out, at, len);
  const roar = noiseSource(k);
  const open = biquad(k, "lowpass", 2000, 0.5);
  glide(k, open.frequency, 12000, 0.4);
  glide(k, open.frequency, 3000, 0.8);
  chain(roar, open, heldEnvelope(k, 0, 0.2, 0.3, 0.55, 0.25, 0.8), out);
  run(k, roar, 0, 0.8);
  const floor = noiseSource(k);
  chain(floor, biquad(k, "lowpass", 180, 0.7), envelope(k, 0, 0.2, 0.9, 0.8), out);
  run(k, floor, 0, 0.8);
  for (const [hz, peak] of [[1760, 0.24], [3520, 0.07]] as const) tone(k, out, "sine", hz, ring, 0.008, peak, len);
  return len;
};

/** Galaxy: a deep hull groaning, then two drills whining up one after the other. */
const hullDrills: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const groan = biquad(k, "bandpass", 260, 3);
  glide(k, groan.frequency, 170, 0.9);
  chain(groan, heldEnvelope(k, 0, 0.2, 0.8, 0.6, 0.6, 0.9), out);
  const hull = oscillator(k, "sawtooth", 46);
  glide(k, hull.frequency, 38, 0.9);
  vibrato(k, hull.frequency, 2.2, 3, 0, 0.9);
  chain(hull, groan);
  run(k, hull, 0, 0.9);
  const drills = [0.45, 0.78];
  for (const [n, start] of drills.entries()) {
    const stop = n === drills.length - 1 ? len : start + 0.55;
    const bit = biquad(k, "bandpass", 1800, 2);
    chain(bit, tremolo(k, 48, 0.35), heldEnvelope(k, start, 0.05, 0.4, stop - 0.15, 0.32, stop), out);
    const drill = oscillator(k, "square", 280, start);
    glide(k, drill.frequency, 1300, stop - 0.1);
    chain(drill, bit);
    run(k, drill, start, stop);
  }
  return len;
};

/** Prototype-X1: a test-range klaxon, then an engine sputtering until it catches. */
const klaxonSputter: SfxRecipe = (ctx, out, at) => {
  const len = 1.6;
  const caught = 1.05;
  const k = kit(ctx, out, at, len);
  const horn = biquad(k, "bandpass", 900, 1);
  chain(horn, out);
  for (const start of [0, 0.32]) {
    const blare = oscillator(k, "square", 370, start);
    glide(k, blare.frequency, 415, start + 0.05);
    chain(blare, heldEnvelope(k, start, 0.02, 0.3, start + 0.22, 0.27, start + 0.28), horn);
    run(k, blare, start, start + 0.28);
  }
  clickTrain(k, out, [0.7, 0.82, 0.88, 0.99, 1.03], 320, 1, 0.7, 0.07);
  const engine = oscillator(k, "sawtooth", 55, caught);
  glide(k, engine.frequency, 88, 1.4);
  chain(engine, biquad(k, "lowpass", 600, 1.5), modulatedGain(k, "square", 18, 0.8, 0.2), heldEnvelope(k, caught, 0.06, 0.3, 1.4, 0.22, len), out);
  run(k, engine, caught, len);
  return len;
};

/** Prototype-X2: a molten roar, metal shrieking, and an engine's furious snarl. */
const moltenSnarl: SfxRecipe = (ctx, out, at) => {
  const len = 1.5;
  const k = kit(ctx, out, at, len);
  const magma = noiseSource(k);
  const glow = biquad(k, "lowpass", 600, 1);
  glide(k, glow.frequency, 300, 0.75);
  chain(magma, glow, modulatedGain(k, "sawtooth", 11, 0.7, 0.3), heldEnvelope(k, 0, 0.15, 1, 0.5, 0.8, 0.75), out);
  run(k, magma, 0, 0.75);
  const shriek = biquad(k, "bandpass", 2600, 6);
  glide(k, shriek.frequency, 1500, 0.8);
  chain(shriek, envelope(k, 0.35, 0.02, 0.3, 0.8), out);
  for (const hz of [1800, 1890]) {
    const metal = oscillator(k, "sawtooth", hz, 0.35);
    glide(k, metal.frequency, hz * 0.7, 0.8);
    chain(metal, shriek);
    run(k, metal, 0.35, 0.8);
  }
  const snarl = oscillator(k, "sawtooth", 70, 0.7);
  glide(k, snarl.frequency, 140, 0.95);
  glide(k, snarl.frequency, 90, len);
  chain(snarl, biquad(k, "lowpass", 900, 4), tremolo(k, 28, 0.4), heldEnvelope(k, 0.7, 0.05, 0.45, 1.2, 0.3, len), out);
  run(k, snarl, 0.7, len);
  return len;
};

/** Siva 1: camera shutters firing, then a smug purr. */
const shutterPurr: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const purrAt = 0.62;
  const k = kit(ctx, out, at, len);
  clickTrain(k, out, [0, 0.05, 0.22, 0.27, 0.44, 0.49], 2800, 2, 0.8, 0.03);
  const purr = noiseSource(k);
  chain(purr, biquad(k, "lowpass", 380, 1), modulatedGain(k, "sawtooth", 24, 0.5, 0.5), heldEnvelope(k, purrAt - 0.01, 0.13, 1, 1.1, 0.75, len), out);
  run(k, purr, purrAt, len);
  const throat = oscillator(k, "triangle", 104, purrAt);
  glide(k, throat.frequency, 124, 0.95);
  glide(k, throat.frequency, 98, len);
  chain(throat, heldEnvelope(k, purrAt, 0.12, 0.16, 1.1, 0.12, len), out);
  run(k, throat, purrAt, len);
  return len;
};

/** Siva 2: a dark reboot sting powering down, then drips that hiss where they land. */
const venomDrip: SfxRecipe = (ctx, out, at) => {
  const len = 1.4;
  const k = kit(ctx, out, at, len);
  const dark = biquad(k, "lowpass", 1400, 2);
  glide(k, dark.frequency, 320, 0.6);
  chain(dark, heldEnvelope(k, 0, 0.02, 0.5, 0.35, 0.35, 0.6), out);
  for (const hz of [220, 261.6]) {
    const voice = oscillator(k, "sawtooth", hz);
    glide(k, voice.frequency, hz / 2, 0.55);
    chain(voice, level(k, 0.4), dark);
    run(k, voice, 0, 0.6);
  }
  const sub = tone(k, out, "sine", 82, 0, 0.01, 0.3, 0.6);
  glide(k, sub.frequency, 41, 0.55);
  for (const start of [0.72, 1.04]) {
    const drop = tone(k, out, "sine", 1100, start, 0.002, 0.3, start + 0.07);
    glide(k, drop.frequency, 2600, start + 0.05);
    const hiss = noiseSource(k);
    chain(hiss, biquad(k, "highpass", 4800, 0.7), envelope(k, start + 0.03, 0.03, 0.25, start + 0.32), out);
    run(k, hiss, start + 0.04, start + 0.32);
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
  armorClank: { recipe: armorClank, durationMs: 350, gain: 0.8 },
  armorRing: { recipe: armorRing, durationMs: 850, gain: 0.8 },
  overkill: { recipe: overkill, durationMs: 550, gain: 0.8 },
  crumble: { recipe: crumble, durationMs: 650, gain: 0.8 },
  unlock: { recipe: unlock, durationMs: 400, gain: 0.8 },
  steal: { recipe: steal, durationMs: 400, gain: 0.55 },
  give: { recipe: give, durationMs: 550, gain: 0.8 },
  counterspell: { recipe: counterspell, durationMs: 600, gain: 0.8 },
  bleat: { recipe: bleat, durationMs: 650, gain: 0.46 },
  fuse: { recipe: fuse, durationMs: 650, gain: 0.53 },
  degrade: { recipe: degrade, durationMs: 520, gain: 0.8 },
  upgrade: { recipe: upgrade, durationMs: 500, gain: 0.56 },
  emoteWave: { recipe: emoteWave, durationMs: 500, gain: 0.8 },
  emoteClap: { recipe: emoteClap, durationMs: 650, gain: 0.8 },
  emoteThumbsUp: { recipe: emoteThumbsUp, durationMs: 400, gain: 0.8 },
  emoteFacepalm: { recipe: emoteFacepalm, durationMs: 950, gain: 0.8 },
  emoteShrug: { recipe: emoteShrug, durationMs: 600, gain: 0.8 },
  emoteThinking: { recipe: emoteThinking, durationMs: 1000, gain: 0.8 },
  emoteHeart: { recipe: emoteHeart, durationMs: 900, gain: 0.8 },
  emoteFire: { recipe: emoteFire, durationMs: 900, gain: 0.8 },
  emoteSkull: { recipe: emoteSkull, durationMs: 450, gain: 0.8 },
  emoteSweat: { recipe: emoteSweat, durationMs: 450, gain: 0.8 },
  emoteCool: { recipe: emoteCool, durationMs: 900, gain: 0.8 },
  emoteGasp: { recipe: emoteGasp, durationMs: 500, gain: 0.8 },
  emoteSalute: { recipe: emoteSalute, durationMs: 750, gain: 0.8 },
  emoteParty: { recipe: emoteParty, durationMs: 850, gain: 0.8 },
  // #1954: the playmat taps, as short and quiet as sand.
  tapWood: { recipe: tapWood, durationMs: 160, gain: 0.52 },
  tapFelt: { recipe: tapFelt, durationMs: 140, gain: 0.52 },
  tapStone: { recipe: tapStone, durationMs: 180, gain: 0.52 },
  tapNeon: { recipe: tapNeon, durationMs: 150, gain: 0.52 },
  tapParchment: { recipe: tapParchment, durationMs: 160, gain: 0.52 },
  tapAstral: { recipe: tapAstral, durationMs: 240, gain: 0.52 },
  // #3347: cards that move like cards, each soft and short.
  pickUp: { recipe: pickUp, durationMs: 90, gain: 0.4 },
  dragOver: { recipe: dragOver, durationMs: 60, gain: 0.3 },
  drop: { recipe: drop, durationMs: 140, gain: 0.45 },
  returnToHand: { recipe: returnToHand, durationMs: 200, gain: 0.4 },
  deal: { recipe: deal, durationMs: 80, gain: 0.4 },
  riffle: { recipe: riffle, durationMs: 500, gain: 0.45 },
  // R3336000: the Drone Battleship.
  horn: { recipe: horn, durationMs: 950, gain: 0.8 },
  domeFlare: { recipe: domeFlare, durationMs: 800, gain: 0.8 },
  bayDoor: { recipe: bayDoor, durationMs: 650, gain: 0.8 },
  reactorPulse: { recipe: reactorPulse, durationMs: 650, gain: 0.8 },
  deckShear: { recipe: deckShear, durationMs: 550, gain: 0.7 },
  // R3338000: the bed under a Terminus set piece, at the gain of the entrance that ends it.
  organ: { recipe: organ, durationMs: 2000, gain: 0.6 },
  // R3337003: an Anomaly's sting; each of the ten ends within durationMs.
  anomaly: { recipe: anomaly, durationMs: 1200, gain: 0.7 },
  // R3346000: a play's cost impact and a firing trap's snap, level with the thud and the set click.
  playImpact: { recipe: playImpact, durationMs: 500, gain: 0.8 },
  trapSnap: { recipe: trapSnap, durationMs: 200, gain: 0.8 },
  // R3342000: the tile aura bell, pitched by its hue.
  auraTouch: { recipe: auraTouch, durationMs: 200, gain: 0.4 },
  // R3334000–R3334003: the Starship's moments, its heaviest (the roar, the cannon, the ram, the wreck) loudest.
  starshipDock: { recipe: starshipDock, durationMs: 450, gain: 0.8 },
  starshipIgnite: { recipe: starshipIgnite, durationMs: 700, gain: 0.8 },
  starshipRoar: { recipe: starshipRoar, durationMs: 1200, gain: 0.9 },
  starshipBeam: { recipe: starshipBeam, durationMs: 400, gain: 0.6 },
  starshipRepeater: { recipe: starshipRepeater, durationMs: 350, gain: 0.6 },
  starshipCannon: { recipe: starshipCannon, durationMs: 600, gain: 0.85 },
  starshipFlak: { recipe: starshipFlak, durationMs: 500, gain: 0.7 },
  starshipLauncher: { recipe: starshipLauncher, durationMs: 450, gain: 0.6 },
  starshipRam: { recipe: starshipRam, durationMs: 500, gain: 0.85 },
  starshipWreck: { recipe: starshipWreck, durationMs: 1100, gain: 0.85 },
  // R3400001: Meditative+'s and Training Grounds' signatures, each at most 1.6 s and gain 0.8, at a routine card's level (B57).
  inkSummit: { recipe: inkSummit, durationMs: 1100, gain: 0.53 },
  plowHeave: { recipe: plowHeave, durationMs: 1300, gain: 0.48 },
  cursorTear: { recipe: cursorTear, durationMs: 1400, gain: 0.8 },
  seanceChant: { recipe: seanceChant, durationMs: 1500, gain: 0.44 },
  slateRap: { recipe: slateRap, durationMs: 1300, gain: 0.8 },
  diceGong: { recipe: diceGong, durationMs: 1600, gain: 0.4 },
  timberLash: { recipe: timberLash, durationMs: 1200, gain: 0.7 },
  pencilArpeggio: { recipe: pencilArpeggio, durationMs: 1400, gain: 0.47 },
  // R3407001: Stinkyscape's signatures, each at most 1.6 s and gain 0.8, trimmed to the eight's routine level (B57).
  ballotBeep: { recipe: ballotBeep, durationMs: 1300, gain: 0.37 },
  lullabySigh: { recipe: lullabySigh, durationMs: 1500, gain: 0.4 },
  compactTut: { recipe: compactTut, durationMs: 1300, gain: 0.8 },
  glitchDrop: { recipe: glitchDrop, durationMs: 1500, gain: 0.25 },
  harpGliss: { recipe: harpGliss, durationMs: 1500, gain: 0.8 },
  markerThunder: { recipe: markerThunder, durationMs: 1500, gain: 0.66 },
  stampTrill: { recipe: stampTrill, durationMs: 1400, gain: 0.4 },
  tillRattle: { recipe: tillRattle, durationMs: 1600, gain: 0.79 },
  gavelSlam: { recipe: gavelSlam, durationMs: 1400, gain: 0.3 },
  dockingChime: { recipe: dockingChime, durationMs: 1600, gain: 0.73 },
  keyboardPing: { recipe: keyboardPing, durationMs: 1200, gain: 0.8 },
  synthSnap: { recipe: synthSnap, durationMs: 1400, gain: 0.52 },
  blobDrum: { recipe: blobDrum, durationMs: 1500, gain: 0.38 },
  warpSparkle: { recipe: warpSparkle, durationMs: 1600, gain: 0.8 },
  tearChoir: { recipe: tearChoir, durationMs: 1500, gain: 0.72 },
  cackleThunder: { recipe: cackleThunder, durationMs: 1600, gain: 0.76 },
  kazooRatchet: { recipe: kazooRatchet, durationMs: 1600, gain: 0.8 },
  thrusterSniff: { recipe: thrusterSniff, durationMs: 1600, gain: 0.46 },
  marbleGong: { recipe: marbleGong, durationMs: 1600, gain: 0.32 },
  sanctumBell: { recipe: sanctumBell, durationMs: 1600, gain: 0.8 },
  cashWhistle: { recipe: cashWhistle, durationMs: 1600, gain: 0.37 },
  whisperClink: { recipe: whisperClink, durationMs: 1200, gain: 0.8 },
  crystalRise: { recipe: crystalRise, durationMs: 1600, gain: 0.23 },
  // R3405001: Meditative's signatures, under the same bounds and at the same level.
  bowlBreath: { recipe: bowlBreath, durationMs: 1500, gain: 0.53 },
  peakFlute: { recipe: peakFlute, durationMs: 1500, gain: 0.46 },
  ticketHorn: { recipe: ticketHorn, durationMs: 1300, gain: 0.31 },
  forgeQuench: { recipe: forgeQuench, durationMs: 1500, gain: 0.3 },
  brushGuzheng: { recipe: brushGuzheng, durationMs: 1500, gain: 0.6 },
  wellGush: { recipe: wellGush, durationMs: 1400, gain: 0.76 },
  minePick: { recipe: minePick, durationMs: 1500, gain: 0.29 },
  jadeZither: { recipe: jadeZither, durationMs: 1500, gain: 0.7 },
  windChime: { recipe: windChime, durationMs: 1600, gain: 0.74 },
  marketGong: { recipe: marketGong, durationMs: 1500, gain: 0.7 },
  chalkHush: { recipe: chalkHush, durationMs: 1500, gain: 0.59 },
  porcelainRoar: { recipe: porcelainRoar, durationMs: 1600, gain: 0.32 },
  bronzeGulp: { recipe: bronzeGulp, durationMs: 1500, gain: 0.68 },
  modemGuqin: { recipe: modemGuqin, durationMs: 1400, gain: 0.7 },
  serverChoir: { recipe: serverChoir, durationMs: 1600, gain: 0.37 },
  dialupStutter: { recipe: dialupStutter, durationMs: 1200, gain: 0.44 },
  holoBow: { recipe: holoBow, durationMs: 1300, gain: 0.67 },
  synthPops: { recipe: synthPops, durationMs: 1200, gain: 0.45 },
  skullMutter: { recipe: skullMutter, durationMs: 1400, gain: 0.75 },
  keyJangle: { recipe: keyJangle, durationMs: 1400, gain: 0.7 },
  caneCrack: { recipe: caneCrack, durationMs: 1100, gain: 0.75 },
  caneThunder: { recipe: caneThunder, durationMs: 1600, gain: 0.7 },
  rootCreak: { recipe: rootCreak, durationMs: 1500, gain: 0.25 },
  chalkGasp: { recipe: chalkGasp, durationMs: 1300, gain: 0.7 },
  frostPages: { recipe: frostPages, durationMs: 1500, gain: 0.7 },
  coinCheer: { recipe: coinCheer, durationMs: 1400, gain: 0.73 },
  totemDrums: { recipe: totemDrums, durationMs: 1400, gain: 0.37 },
  templeDie: { recipe: templeDie, durationMs: 1400, gain: 0.36 },
  golemStomp: { recipe: golemStomp, durationMs: 1500, gain: 0.37 },
  klaxonSlam: { recipe: klaxonSlam, durationMs: 1600, gain: 0.58 },
  cellSlam: { recipe: cellSlam, durationMs: 1500, gain: 0.38 },
  hangarRev: { recipe: hangarRev, durationMs: 1600, gain: 0.26 },
  // R3395000: Stinkyscape 001 to 090's signatures, each at most 1.6 s and gain 0.8, at a routine card's level (B57).
  vaultHiss: { recipe: vaultHiss, durationMs: 1400, gain: 0.54 },
  safeChirp: { recipe: safeChirp, durationMs: 1200, gain: 0.71 },
  stoneChime: { recipe: stoneChime, durationMs: 1600, gain: 0.41 },
  magiChord: { recipe: magiChord, durationMs: 1500, gain: 0.65 },
  lectureRumble: { recipe: lectureRumble, durationMs: 1600, gain: 0.69 },
  pressStamp: { recipe: pressStamp, durationMs: 1300, gain: 0.47 },
  corvetteRoar: { recipe: corvetteRoar, durationMs: 1450, gain: 0.43 },
  popGasp: { recipe: popGasp, durationMs: 1200, gain: 0.69 },
  geigerHum: { recipe: geigerHum, durationMs: 1400, gain: 0.53 },
  wireRatchet: { recipe: wireRatchet, durationMs: 1600, gain: 0.74 },
  beamWhine: { recipe: beamWhine, durationMs: 1450, gain: 0.47 },
  rattleSteam: { recipe: rattleSteam, durationMs: 1350, gain: 0.47 },
  breechBoom: { recipe: breechBoom, durationMs: 1500, gain: 0.42 },
  flakWind: { recipe: flakWind, durationMs: 1600, gain: 0.73 },
  crunchJackpot: { recipe: crunchJackpot, durationMs: 1500, gain: 0.57 },
  choirHulls: { recipe: choirHulls, durationMs: 1600, gain: 0.56 },
  hornEcho: { recipe: hornEcho, durationMs: 1500, gain: 0.55 },
  jetSnap: { recipe: jetSnap, durationMs: 1300, gain: 0.56 },
  whiteRing: { recipe: whiteRing, durationMs: 1600, gain: 0.57 },
  hullDrills: { recipe: hullDrills, durationMs: 1500, gain: 0.71 },
  klaxonSputter: { recipe: klaxonSputter, durationMs: 1600, gain: 0.54 },
  moltenSnarl: { recipe: moltenSnarl, durationMs: 1500, gain: 0.53 },
  shutterPurr: { recipe: shutterPurr, durationMs: 1400, gain: 0.68 },
  venomDrip: { recipe: venomDrip, durationMs: 1400, gain: 0.57 },
};

/** R655: runs `id`'s recipe shifted by `pitch` (a frequency ratio, 1 as written); returns the unshifted length in seconds. */
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
