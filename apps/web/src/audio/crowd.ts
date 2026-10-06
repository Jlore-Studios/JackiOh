// The crowd's reactions to public events. It owns no game data: callers pass damage amounts and
// landing tiers already present in the rendered event stream.

import { CROWD_FEEL, damageFeel, UNIT_SLAM, type SlamTier } from "../game/damageFeel.ts";
import { noiseBuffer } from "./sfx.ts";
import type { AudioEngine } from "./types.ts";

type CrowdOutput = { context: AudioContext; crowd: AudioNode };
type CrowdCapableEngine = AudioEngine & { crowdOutput(): CrowdOutput | null };
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

type ReactionKind = keyof typeof CROWD_FEEL.reactionMs;
/** The reactions an event can start; cheer and applause only ever follow one. */
type CrowdReaction = "ooh" | "gasp" | "roar";
const REACTION_RANK: Readonly<Record<CrowdReaction, number>> = { ooh: 1, gasp: 2, roar: 3 };
/** #185: a landing's murmur, stirring and excitement, in the reactions the crowd has. */
const SLAM_CROWD: Readonly<Record<"none" | "murmur" | "anticipation" | "excited", CrowdReaction | null>> = {
  none: null,
  murmur: "ooh",
  anticipation: "gasp",
  excited: "roar",
};

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

function spatialGain(output: CrowdOutput, gain: GainNode, pan: number): StereoPannerNode {
  const panner = output.context.createStereoPanner();
  panner.pan.value = pan;
  gain.connect(panner);
  panner.connect(output.crowd);
  return panner;
}

function reaction(output: CrowdOutput, kind: ReactionKind, pan: number): void {
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
}

export function createCrowdDirector(options: CrowdDirectorOptions): CrowdDirector {
  const later = options.later ?? defaultLater;
  const next = sequence(0x43524f57); // "CROW"
  let running = false;
  let ending = false;
  let output: CrowdOutput | null = null;
  let debounce: Cancel | null = null;
  const reactionTimers = new Set<Cancel>();
  let pending: { kind: CrowdReaction; tails: boolean } | null = null;

  const clearTimers = (): void => {
    debounce?.();
    debounce = null;
  };

  const clearReactionTimers = (): void => {
    for (const cancel of reactionTimers) cancel();
    reactionTimers.clear();
  };

  const connect = (): void => {
    if (!running || ending || output !== null) return;
    output = (options.engine as Partial<CrowdCapableEngine>).crowdOutput?.() ?? null;
    if (output !== null && output.context.state !== "running") output = null;
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
      reaction(output, due.kind, next() * 2 - 1);
      const addTail = (delayMs: number, tail: ReactionKind): void => {
        let cancel: Cancel = () => undefined;
        cancel = later(delayMs, () => {
          reactionTimers.delete(cancel);
          if (running && !ending && output !== null) reaction(output, tail, next() * 2 - 1);
        });
        reactionTimers.add(cancel);
      };
      if (due.tails && due.kind === "gasp") addTail(CROWD_FEEL.cheerDelayMs, "cheer");
      if (due.tails && due.kind === "roar") addTail(CROWD_FEEL.applauseDelayMs, "applause");
    });
  };

  const stateOff = options.engine.subscribeState(() => connect());

  return {
    start() {
      running = true;
      connect();
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
    },
    dispose() {
      running = false;
      clearTimers();
      clearReactionTimers();
      stateOff();
      output = null;
    },
  };
}
