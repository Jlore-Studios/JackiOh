// A procedural venue bed and public-event crowd reactions. It owns no game data: callers pass
// damage amounts already present in the rendered event stream.

import { CROWD_FEEL, damageFeel } from "../game/damageFeel.ts";
import { noiseBuffer } from "./sfx.ts";
import type { AudioEngine } from "./types.ts";

type CrowdOutput = { context: AudioContext; ambience: AudioNode; crowd: AudioNode; ambienceDuck: GainNode };
type AmbienceCapableEngine = AudioEngine & { ambienceOutput(): CrowdOutput | null };
type Cancel = () => void;

export type CrowdDirector = {
  start(): void;
  observeDamage(amount: number): void;
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

function patron(output: CrowdOutput, variant: number): void {
  const { context, crowd } = output;
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
  gain.connect(crowd);
  source.start();
  source.stop(context.currentTime + length);
  source.onended = () => {
    source.disconnect();
    filter.disconnect();
    gain.disconnect();
  };
}

function reaction(output: CrowdOutput, kind: Exclude<ReturnType<typeof damageFeel>["crowd"], "none">): void {
  const { context, crowd, ambienceDuck } = output;
  const durationMs = CROWD_FEEL.reactionMs[kind];
  const seconds = durationMs / 1000;
  const source = context.createBufferSource();
  const filter = context.createBiquadFilter();
  const gain = context.createGain();
  const baseHz = kind === "ooh" ? 620 : kind === "gasp" ? 430 : 270;
  source.buffer = noiseBuffer(context);
  filter.type = "bandpass";
  filter.frequency.value = baseHz;
  filter.Q.value = 0.8;
  gain.gain.setValueAtTime(0.0001, context.currentTime);
  gain.gain.linearRampToValueAtTime(kind === "roar" ? 0.42 : 0.24, context.currentTime + 0.06);
  gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + seconds);
  source.connect(filter);
  filter.connect(gain);
  gain.connect(crowd);
  source.start();
  source.stop(context.currentTime + seconds);
  source.onended = () => {
    source.disconnect();
    filter.disconnect();
    gain.disconnect();
  };
  const now = context.currentTime;
  ambienceDuck.gain.cancelScheduledValues(now);
  ambienceDuck.gain.setTargetAtTime(DB_TO_GAIN, now, 0.01);
  ambienceDuck.gain.setTargetAtTime(1, now + seconds, 0.08);
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
  let largest = 0;

  const clearTimers = (): void => {
    timer?.();
    timer = null;
    debounce?.();
    debounce = null;
  };

  const startBed = (): void => {
    if (!running || ending || output !== null) return;
    output = (options.engine as Partial<AmbienceCapableEngine>).ambienceOutput?.() ?? null;
    if (output === null || output.context.state !== "running") {
      output = null;
      return;
    }
    cleanup = [
      makeBed(output, CROWD_FEEL.bedLoopSeconds[0], 480),
      makeBed(output, CROWD_FEEL.bedLoopSeconds[1], 920),
    ];
    const schedulePatron = (): void => {
      if (!running || ending || output === null) return;
      const span = CROWD_FEEL.patronMaxMs - CROWD_FEEL.patronMinMs;
      const wait = CROWD_FEEL.patronMinMs + Math.round(next() * span);
      timer = later(wait, () => {
        if (output !== null) patron(output, PATRON_VARIANTS[Math.floor(next() * PATRON_VARIANTS.length)] ?? PATRON_VARIANTS[0]);
        schedulePatron();
      });
    };
    schedulePatron();
  };

  const stateOff = options.engine.subscribeState(() => startBed());

  return {
    start() {
      running = true;
      startBed();
    },
    observeDamage(amount) {
      const feel = damageFeel(amount);
      if (!running || ending || feel.crowd === "none") return;
      largest = Math.max(largest, amount);
      if (debounce !== null) return;
      debounce = later(CROWD_FEEL.reactionDebounceMs, () => {
        debounce = null;
        const reactionFeel = damageFeel(largest);
        largest = 0;
        if (output !== null && reactionFeel.crowd !== "none") reaction(output, reactionFeel.crowd);
      });
    },
    end() {
      if (ending) return;
      ending = true;
      clearTimers();
      if (output !== null) {
        const now = output.context.currentTime;
        output.ambienceDuck.gain.cancelScheduledValues(now);
        output.ambienceDuck.gain.setTargetAtTime(0.0001, now, CROWD_FEEL.ambientFadeOutMs / 1000 / 3);
      }
      const stops = cleanup;
      cleanup = [];
      timer = later(CROWD_FEEL.ambientFadeOutMs, () => stops.forEach((stop) => stop()));
    },
    dispose() {
      running = false;
      clearTimers();
      stateOff();
      cleanup.forEach((stop) => stop());
      cleanup = [];
      output = null;
    },
  };
}
