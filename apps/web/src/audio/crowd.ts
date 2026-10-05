// A procedural venue bed and public-event crowd reactions. It owns no game data: callers pass
// damage amounts already present in the rendered event stream.

import { CROWD_FEEL, damageFeel, UNIT_SLAM, type SlamTier } from "../game/damageFeel.ts";
import { noiseBuffer } from "./sfx.ts";
import type { AudioEngine } from "./types.ts";

type CrowdOutput = { context: AudioContext; ambience: AudioNode; crowd: AudioNode; ambienceDuck: GainNode };
type AmbienceCapableEngine = AudioEngine & { ambienceOutput(): CrowdOutput | null };
type Cancel = () => void;

export type CrowdDirector = {
  start(): void;
  observeDamage(amount: number): void;
  /** #185: a landing Unit's tier; a Large one murmurs, a Huge one stirs, a MASSIVE one excites. */
  observeSlam(tier: SlamTier): void;
  end(): void;
  dispose(): void;
};

export type CrowdDirectorOptions = {
  engine: AudioEngine;
  later?: (ms: number, run: () => void) => Cancel;
};

const DB_TO_GAIN = 10 ** (-CROWD_FEEL.ambientDuckDb / 20);
const PATRON_VARIANTS = [0.82, 0.94, 1.06, 1.18] as const;
const BED_SEAM_SAMPLES = 128;
type ReactionKind = keyof typeof CROWD_FEEL.reactionMs;
/** The reactions an event can start; cheer and applause only ever follow one. */
type CrowdReaction = "ooh" | "gasp" | "roar";
const REACTION_RANK: Readonly<Record<CrowdReaction, number>> = { ooh: 1, gasp: 2, roar: 3 };
/** #185: a landing's murmur, stirring and excitement, in the reactions the venue has. */
const SLAM_CROWD: Readonly<Record<"none" | "murmur" | "anticipation" | "excited", CrowdReaction | null>> = {
  none: null,
  murmur: "ooh",
  anticipation: "gasp",
  excited: "roar",
};
const venueBuffers = new WeakMap<BaseAudioContext, Map<number, AudioBuffer>>();

function defaultLater(ms: number, run: () => void): Cancel {
  const timer = window.setTimeout(run, ms);
  return () => window.clearTimeout(timer);
}

/** Small deterministic stream; the soundscape varies without affecting gameplay RNG. */
function sequence(seed: number): () => number {
  let value = seed;
  return () => {
    value = (value * 1664525 + 1013904223) >>> 0;
    return value / 0x1_0000_0000;
  };
}

function safeStop(node: AudioScheduledSourceNode): void {
  try {
    node.stop();
  } catch {
    // A one-shot may have ended first.
  }
}

/**
 * A genuinely long loop, not a short noise file whose loud seam repeats under an LFO. The final
 * samples blend into the opening samples, so the 47 s and 73 s beds wrap without a click.
 */
function venueBuffer(context: BaseAudioContext, cycleSeconds: number, colour: number): AudioBuffer {
  const cached = venueBuffers.get(context)?.get(cycleSeconds);
  if (cached !== undefined) return cached;
  const length = Math.round(context.sampleRate * cycleSeconds);
  const buffer = context.createBuffer(1, length, context.sampleRate);
  const data = buffer.getChannelData(0);
  const next = sequence(Math.round(cycleSeconds * colour));
  for (let index = 0; index < length; index += 1) data[index] = next() * 2 - 1;
  for (let offset = 0; offset < BED_SEAM_SAMPLES; offset += 1) {
    const at = length - BED_SEAM_SAMPLES + offset;
    const ratio = (offset + 1) / BED_SEAM_SAMPLES;
    data[at] = (data[at] ?? 0) * (1 - ratio) + (data[offset] ?? 0) * ratio;
  }
  const cache = venueBuffers.get(context) ?? new Map<number, AudioBuffer>();
  cache.set(cycleSeconds, buffer);
  venueBuffers.set(context, cache);
  return buffer;
}

function makeBed(output: CrowdOutput, cycleSeconds: number, colour: number): Cancel {
  const { context, ambience } = output;
  const source = context.createBufferSource();
  const filter = context.createBiquadFilter();
  const gain = context.createGain();
  const lfo = context.createOscillator();
  const depth = context.createGain();
  source.buffer = venueBuffer(context, cycleSeconds, colour);
  source.loop = true;
  filter.type = "bandpass";
  filter.frequency.value = colour;
  filter.Q.value = 0.45;
  gain.gain.value = 0.28;
  lfo.type = "sine";
  lfo.frequency.value = 1 / cycleSeconds;
  depth.gain.value = 0.1;
  source.connect(filter);
  filter.connect(gain);
  gain.connect(ambience);
  lfo.connect(depth);
  depth.connect(gain.gain);
  source.start();
  lfo.start();
  return () => {
    safeStop(source);
    safeStop(lfo);
    source.disconnect();
    lfo.disconnect();
    filter.disconnect();
    gain.disconnect();
    depth.disconnect();
  };
}

function spatialGain(output: CrowdOutput, gain: GainNode, pan: number): StereoPannerNode {
  const panner = output.context.createStereoPanner();
  panner.pan.value = pan;
  gain.connect(panner);
  panner.connect(output.crowd);
  return panner;
}

function patron(output: CrowdOutput, variant: number, pan: number): void {
  const { context } = output;
  const length = 0.34;
  const source = context.createBufferSource();
  const filter = context.createBiquadFilter();
  const gain = context.createGain();
  source.buffer = noiseBuffer(context);
  filter.type = "bandpass";
  filter.frequency.value = 800 * variant;
  filter.Q.value = 1.5;
  gain.gain.setValueAtTime(0.0001, context.currentTime);
  gain.gain.linearRampToValueAtTime(0.08, context.currentTime + 0.03);
  gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + length);
  source.connect(filter);
  filter.connect(gain);
  const panner = spatialGain(output, gain, pan);
  source.start();
  source.stop(context.currentTime + length);
  source.onended = () => {
    source.disconnect();
    filter.disconnect();
    gain.disconnect();
    panner.disconnect();
  };
}

function reaction(output: CrowdOutput, kind: ReactionKind, pan: number): number {
  const { context } = output;
  const durationMs = kind === "applause" ? CROWD_FEEL.applauseTailMs : CROWD_FEEL.reactionMs[kind];
  const seconds = durationMs / 1000;
  const source = context.createBufferSource();
  const filter = context.createBiquadFilter();
  const gain = context.createGain();
  const baseHz = kind === "ooh" ? 620 : kind === "gasp" ? 430 : kind === "cheer" ? 510 : kind === "roar" ? 270 : 360;
  source.buffer = noiseBuffer(context);
  filter.type = "bandpass";
  filter.frequency.value = baseHz;
  filter.Q.value = 0.8;
  gain.gain.setValueAtTime(0.0001, context.currentTime);
  gain.gain.linearRampToValueAtTime(kind === "roar" ? 0.42 : kind === "applause" ? 0.3 : 0.24, context.currentTime + 0.06);
  gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + seconds);
  source.connect(filter);
  filter.connect(gain);
  const panner = spatialGain(output, gain, pan);
  source.start();
  source.stop(context.currentTime + seconds);
  source.onended = () => {
    source.disconnect();
    filter.disconnect();
    gain.disconnect();
    panner.disconnect();
  };
  return durationMs;
}

function duckAmbience(output: CrowdOutput, durationMs: number): void {
  const now = output.context.currentTime;
  const seconds = durationMs / 1000;
  output.ambienceDuck.gain.cancelScheduledValues(now);
  output.ambienceDuck.gain.setTargetAtTime(DB_TO_GAIN, now, 0.01);
  output.ambienceDuck.gain.setTargetAtTime(1, now + seconds, 0.08);
}

export function createCrowdDirector(options: CrowdDirectorOptions): CrowdDirector {
  const later = options.later ?? defaultLater;
  const next = sequence(0x43524f57); // "CROW"
  let running = false;
  let ending = false;
  let output: CrowdOutput | null = null;
  let cleanup: Cancel[] = [];
  let timer: Cancel | null = null;
  let debounce: Cancel | null = null;
  const reactionTimers = new Set<Cancel>();
  let pending: { kind: CrowdReaction; tails: boolean } | null = null;

  const clearTimers = (): void => {
    timer?.();
    timer = null;
    debounce?.();
    debounce = null;
  };

  const clearReactionTimers = (): void => {
    for (const cancel of reactionTimers) cancel();
    reactionTimers.clear();
  };

  const stopBeds = (): void => {
    cleanup.forEach((stop) => stop());
    cleanup = [];
  };

  const startBed = (): void => {
    if (!running || ending || output !== null) return;
    output = (options.engine as Partial<AmbienceCapableEngine>).ambienceOutput?.() ?? null;
    if (output === null || output.context.state !== "running") {
      output = null;
      return;
    }
    const now = output.context.currentTime;
    output.ambienceDuck.gain.cancelScheduledValues(now);
    output.ambienceDuck.gain.setValueAtTime(1, now);
    cleanup = [
      makeBed(output, CROWD_FEEL.bedLoopSeconds[0], 480),
      makeBed(output, CROWD_FEEL.bedLoopSeconds[1], 920),
    ];
    const schedulePatron = (): void => {
      if (!running || ending || output === null) return;
      const span = CROWD_FEEL.patronMaxMs - CROWD_FEEL.patronMinMs;
      const wait = CROWD_FEEL.patronMinMs + Math.round(next() * span);
      timer = later(wait, () => {
        if (output !== null) {
          patron(
            output,
            PATRON_VARIANTS[Math.floor(next() * PATRON_VARIANTS.length)] ?? PATRON_VARIANTS[0],
            next() * 2 - 1,
          );
        }
        schedulePatron();
      });
    };
    schedulePatron();
  };

  // Reactions begin one after another as the animation queue reaches them. Resetting this quiet period
  // makes one resolution settle on its strongest moment (its largest hit, its heaviest landing)
  // rather than reacting midway.
  const observe = (kind: CrowdReaction, tails: boolean): void => {
    if (!running || ending) return;
    if (pending === null || REACTION_RANK[kind] > REACTION_RANK[pending.kind] || (REACTION_RANK[kind] === REACTION_RANK[pending.kind] && tails)) {
      pending = { kind, tails };
    }
    debounce?.();
    debounce = later(CROWD_FEEL.reactionDebounceMs, () => {
      debounce = null;
      const due = pending;
      pending = null;
      if (output === null || due === null) return;
      let durationMs = reaction(output, due.kind, next() * 2 - 1);
      const addTail = (delayMs: number, tail: ReactionKind): void => {
        let cancel: Cancel = () => undefined;
        cancel = later(delayMs, () => {
          reactionTimers.delete(cancel);
          if (running && !ending && output !== null) reaction(output, tail, next() * 2 - 1);
        });
        reactionTimers.add(cancel);
        const tailDurationMs = tail === "applause" ? CROWD_FEEL.applauseTailMs : CROWD_FEEL.reactionMs[tail];
        durationMs = Math.max(durationMs, delayMs + tailDurationMs);
      };
      if (due.tails && due.kind === "gasp") addTail(CROWD_FEEL.cheerDelayMs, "cheer");
      if (due.tails && due.kind === "roar") addTail(CROWD_FEEL.applauseDelayMs, "applause");
      duckAmbience(output, durationMs);
    });
  };

  const stateOff = options.engine.subscribeState(() => startBed());

  return {
    start() {
      running = true;
      startBed();
    },
    observeDamage(amount) {
      const kind = damageFeel(amount).crowd;
      if (kind !== "none") observe(kind, true);
    },
    observeSlam(tier) {
      const kind = SLAM_CROWD[UNIT_SLAM[tier].crowd];
      // #185: the crowd is neutral, so a landing draws no cheer or applause after it.
      if (kind !== null) observe(kind, false);
    },
    end() {
      if (ending) return;
      ending = true;
      clearTimers();
      clearReactionTimers();
      if (output !== null) {
        const now = output.context.currentTime;
        output.ambienceDuck.gain.cancelScheduledValues(now);
        output.ambienceDuck.gain.setValueAtTime(output.ambienceDuck.gain.value, now);
        output.ambienceDuck.gain.linearRampToValueAtTime(0.0001, now + CROWD_FEEL.ambientFadeOutMs / 1000);
      }
      timer = later(CROWD_FEEL.ambientFadeOutMs, stopBeds);
    },
    dispose() {
      running = false;
      clearTimers();
      clearReactionTimers();
      stateOff();
      stopBeds();
      output = null;
    },
  };
}
