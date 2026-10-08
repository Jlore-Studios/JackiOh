// B14 (docs/polish/2-sound.md): every procedural recipe, run on the fake context with each of the
// four parameter sets, keeps the recipe contract: nothing scheduled before `at`, every source it
// starts stopped by `at + returned`, a returned length within `durationMs`, wiring only into `out`,
// and nothing outside the permitted Web Audio subset. What a recipe sounds like is B15 and B16, in
// real Chrome (tester B's component spec).

import { describe, expect, it } from "vitest";

import { CHAOS_REVEAL_MAX, EFFECT_PITCH_JITTER, IMPACT_AMOUNT_CAP } from "./constants.ts";
import { SFX, SFX_IDS, SFX_TIMBRES, noiseBuffer, renderSfx, type SfxRecipe, type SfxSpec } from "./sfx.ts";
import { FakeAudio, FakeNode, type FakeParam, type ParamEvent } from "./test/fakeAudio.ts";
import type { SfxId, SfxParams } from "./types.ts";
import { CARD_AUDIO } from "./voiceData.ts";

/** The SfxId union, in its declared order. */
const UNION_ORDER: SfxId[] = [
  "draw", "play", "summon", "attack", "impact", "shieldShatter", "heal", "buff", "debuff",
  "death", "burn", "trapSet", "trapSting", "spell", "mana", "turnStart", "victory",
  "defeat", "uiClick", "uiHover", "whoosh", "radiant", "lock", "poof", "sand", "endTurn", "notify", "drain",
  "cancel", "entrance", "fatigue", "refuse",
  "manaCrack", "bloodDrain", "goldBurst", "castOnDraw", "chaosRoll", "brand", "heartbeat", "clockTick",
  // Patch v0.2.X (R644): the five emoji emotes.
  "emoteSob", "emoteYawn", "emoteLaugh", "emoteAngry", "emoteWahWah",
  // Patch v0.2.X (R669): the play sting.
  "sting",
  // Patch v0.3.X (MN05): Armor (R1363) and the niche moments (R1364–R1366).
  "armorClank", "armorRing", "overkill", "crumble", "unlock", "steal", "give", "counterspell", "bleat", "fuse",
  "degrade", "upgrade",
];

/** The design's durationMs column: each recipe's upper bound over all params. */
const DURATION_MS: Record<SfxId, number> = {
  draw: 180,
  play: 260,
  summon: 380,
  attack: 240,
  impact: 450,
  shieldShatter: 500,
  heal: 700,
  buff: 420,
  debuff: 420,
  death: 650,
  burn: 600,
  trapSet: 160,
  trapSting: 700,
  spell: 800,
  mana: 260,
  turnStart: 1200,
  victory: 1600,
  defeat: 1600,
  uiClick: 50,
  uiHover: 40,
  whoosh: 350,
  radiant: 900,
  lock: 400,
  poof: 450,
  sand: 240,
  endTurn: 180,
  notify: 300,
  drain: 600,
  cancel: 260,
  entrance: 1400,
  fatigue: 650,
  refuse: 400,
  // Patch v0.2.0 (R506).
  manaCrack: 900,
  bloodDrain: 600,
  goldBurst: 850,
  castOnDraw: 400,
  chaosRoll: 1650,
  brand: 800,
  heartbeat: 450,
  clockTick: 350,
  emoteSob: 1200,
  emoteYawn: 1400,
  emoteLaugh: 750,
  emoteAngry: 700,
  emoteWahWah: 1800,
  sting: 800,
  // Patch v0.3.X (MN05).
  armorClank: 350,
  armorRing: 850,
  overkill: 550,
  crumble: 650,
  unlock: 400,
  steal: 400,
  give: 550,
  counterspell: 600,
  bleat: 650,
  fuse: 650,
  degrade: 520,
  upgrade: 500,
};

const PARAM_SETS: readonly SfxParams[] = [{}, { amount: 1 }, { amount: 25 }, { mine: true }, { tier: "rare" }, { tier: "epic" }];

/** The recipe is asked to start here; `currentTime` is earlier, so "start now" is detectably early. */
const NOW = 1;
const AT = 1.5;
const EPS = 1e-9;

type Run = {
  label: string;
  audio: FakeAudio;
  out: FakeNode;
  at: number;
  durationMs: number;
  returned: unknown;
  threw: unknown;
  /** Nodes the recipe created. */
  made: FakeNode[];
  /** AudioParam calls the recipe made. */
  paramEvents: { param: FakeParam; event: ParamEvent }[];
};

function runRecipe(label: string, recipe: SfxRecipe, durationMs: number, params: SfxParams): Run {
  const audio = new FakeAudio({ state: "running", currentTime: NOW });
  const outProxy = audio.context.createGain();
  const out = audio.nodeOf(outProxy);
  const firstNode = audio.nodes.length;
  const firstParam = audio.paramLog.length;
  let returned: unknown;
  let threw: unknown = null;
  try {
    returned = recipe(audio.context, outProxy, AT, params);
  } catch (error) {
    threw = error;
  }
  return {
    label,
    audio,
    out,
    at: AT,
    durationMs,
    returned,
    threw,
    made: audio.nodes.slice(firstNode),
    paramEvents: audio.paramLog.slice(firstParam),
  };
}

/** One run per id of the union and parameter set. A missing spec becomes a run that threw, not a crash. */
function everyRun(): Run[] {
  const table = SFX as Partial<Record<SfxId, SfxSpec>>;
  const runs: Run[] = [];
  for (const id of UNION_ORDER) {
    const spec = table[id];
    const recipe: SfxRecipe =
      spec?.recipe ??
      (() => {
        throw new Error(`SFX has no spec for ${id}`);
      });
    for (const params of PARAM_SETS) {
      runs.push(runRecipe(`${id} ${JSON.stringify(params)}`, recipe, spec?.durationMs ?? DURATION_MS[id], params));
    }
  }
  return runs;
}

const returnedSeconds = (run: Run): number => (typeof run.returned === "number" ? run.returned : Number.NaN);

/* ----- the contract, one clause per checker; each returns its problems ----- */

function subsetProblems(run: Run): string[] {
  const problems = run.audio.violations.map((v) => `${run.label}: ${v}`);
  if (run.threw !== null) problems.push(`${run.label}: threw ${String(run.threw)}`);
  return problems;
}

function lengthProblems(run: Run): string[] {
  const seconds = returnedSeconds(run);
  if (!Number.isFinite(seconds)) return [`${run.label}: returned ${String(run.returned)}, not a finite number of seconds`];
  if (seconds <= 0) return [`${run.label}: returned ${String(seconds)} s, not a positive length`];
  if (seconds > run.durationMs / 1000 + EPS) return [`${run.label}: returned ${String(seconds)} s > durationMs ${String(run.durationMs)}`];
  return [];
}

function scheduleProblems(run: Run): string[] {
  const problems: string[] = [];
  const started = run.made.filter((n) => n.started);
  if (started.length === 0) problems.push(`${run.label}: starts no source, so it schedules no sound`);
  for (const node of started) {
    if ((node.startTime ?? 0) < run.at - EPS) problems.push(`${run.label}: ${node.kind} starts at ${String(node.startTime)} < at ${String(run.at)}`);
    if (node.stopTime !== null && node.stopTime < run.at - EPS) {
      problems.push(`${run.label}: ${node.kind} stops at ${String(node.stopTime)} < at ${String(run.at)}`);
    }
  }
  for (const { param, event } of run.paramEvents) {
    if (event.method === "value" || event.method === "cancelScheduledValues") continue;
    if (event.time < run.at - EPS) {
      problems.push(`${run.label}: ${param.node.kind}.${param.name}.${event.method} at ${String(event.time)} < at ${String(run.at)}`);
    }
  }
  return problems;
}

function stopProblems(run: Run): string[] {
  const problems: string[] = [];
  const end = run.at + returnedSeconds(run);
  for (const node of run.made.filter((n) => n.started)) {
    if (node.stopTime === null) {
      problems.push(`${run.label}: a ${node.kind} it starts is never stopped`);
    } else if (!(node.stopTime <= end + EPS)) {
      problems.push(`${run.label}: a ${node.kind} stops at ${String(node.stopTime)}, after at + returned = ${String(end)}`);
    }
  }
  return problems;
}

function wiringProblems(run: Run): string[] {
  const problems: string[] = [];
  const own = new Set(run.made);
  for (const node of run.made) {
    for (const target of node.connections) {
      if (target instanceof FakeNode) {
        if (target === run.audio.destination) problems.push(`${run.label}: ${node.kind} connects to ctx.destination`);
        else if (target !== run.out && !own.has(target)) problems.push(`${run.label}: ${node.kind} connects to a node it did not make`);
      } else if (!own.has(target.node)) {
        problems.push(`${run.label}: ${node.kind} modulates ${target.node.kind}.${target.name}, which it did not make`);
      }
    }
  }
  if (!run.made.some((n) => n.connections.includes(run.out))) problems.push(`${run.label}: nothing connects into out`);
  if (run.out.connections.length > 0) problems.push(`${run.label}: connects out onward (out is the engine's)`);
  return problems;
}

function rampProblems(run: Run): string[] {
  return run.paramEvents
    .filter(({ event }) => event.method === "exponentialRampToValueAtTime" && !(event.value > 0))
    .map(({ param }) => `${run.label}: exponential ramp on ${param.node.kind}.${param.name} targets a value <= 0`);
}

/* --------------------------------------------------------------------------------------------- *
 * B14
 * --------------------------------------------------------------------------------------------- */

describe("B14 the SFX table", () => {
  it("B14 SFX_IDS lists every id, in the order of the SfxId union", () => {
    expect([...SFX_IDS]).toEqual(UNION_ORDER);
  });

  it("B14 SFX has exactly one spec per id, with the design's durationMs and a gain in (0, 1]", () => {
    expect(Object.keys(SFX).sort()).toEqual([...UNION_ORDER].sort());
    for (const id of UNION_ORDER) {
      const spec = SFX[id];
      expect(typeof spec.recipe, id).toBe("function");
      expect(spec.durationMs, id).toBe(DURATION_MS[id]);
      expect(spec.gain, id).toBeGreaterThan(0);
      expect(spec.gain, id).toBeLessThanOrEqual(1);
    }
  });

  it("gives Big and GIGA impacts different escalating low-end recipes", () => {
    const big = runRecipe("big impact", SFX.impact.recipe, SFX.impact.durationMs, { impactTier: "big", variation: 0.5 });
    const giga = runRecipe("giga impact", SFX.impact.recipe, SFX.impact.durationMs, { impactTier: "giga", variation: 0.5 });
    const bigThump = big.made.find((node) => node.kind === "oscillator");
    const gigaThump = giga.made.find((node) => node.kind === "oscillator");

    expect(big.returned as number).toBeLessThan(giga.returned as number);
    expect(bigThump?.param("frequency").events.at(0)).not.toEqual(gigaThump?.param("frequency").events.at(0));
  });
});

describe("B14 every recipe keeps the recipe contract on the fake context", () => {
  const runs = everyRun();

  it("B14 covers every id with {}, {amount: 1}, {amount: 25} and {mine: true}", () => {
    expect(runs).toHaveLength(UNION_ORDER.length * PARAM_SETS.length);
  });

  it("B14 no recipe throws or reaches outside the permitted Web Audio subset", () => {
    expect(runs.flatMap(subsetProblems)).toEqual([]);
  });

  it("B14 every recipe returns a length in (0, durationMs / 1000]", () => {
    expect(runs.flatMap(lengthProblems)).toEqual([]);
  });

  it("B14 every recipe starts a source, and schedules no start, stop or automation before at", () => {
    expect(runs.flatMap(scheduleProblems)).toEqual([]);
  });

  it("B14 every source a recipe starts is stopped by at + its returned length", () => {
    expect(runs.flatMap(stopProblems)).toEqual([]);
  });

  it("B14 every recipe connects into out, and only into out or its own nodes, never the destination", () => {
    expect(runs.flatMap(wiringProblems)).toEqual([]);
  });

  it("B14 exponential ramps always target a value above 0", () => {
    expect(runs.flatMap(rampProblems)).toEqual([]);
  });

  it("B14 the checks reject a recipe that breaks each clause", () => {
    const bad: SfxRecipe = (ctx, out, at) => {
      const early = ctx.createOscillator();
      early.connect(ctx.destination); // wiring: straight to the speakers
      early.start(); // schedule: "now", before at
      const unstopped = ctx.createOscillator();
      unstopped.connect(out);
      unstopped.start(at); // stop: never stopped
      const late = ctx.createOscillator();
      late.connect(out);
      late.start(at);
      late.stop(at + 30); // stop: long after at + returned
      try {
        ctx.createWaveShaper(); // subset: not permitted
      } catch {
        // The fake throws; the violation is recorded anyway.
      }
      return 9; // length: longer than its durationMs
    };
    const run = runRecipe("bad", bad, 100, {});

    expect(subsetProblems(run)).not.toEqual([]);
    expect(lengthProblems(run)).not.toEqual([]);
    expect(scheduleProblems(run)).not.toEqual([]);
    expect(stopProblems(run)).not.toEqual([]);
    expect(wiringProblems(run)).not.toEqual([]);
  });

  it("B14 the fake refuses an exponential ramp to 0, as Web Audio does", () => {
    const zeroRamp: SfxRecipe = (ctx, out, at) => {
      const gain = ctx.createGain();
      gain.connect(out);
      gain.gain.exponentialRampToValueAtTime(0, at + 0.1);
      return 0.1;
    };
    const run = runRecipe("zero-ramp", zeroRamp, 100, {});

    expect(run.threw).toBeInstanceOf(RangeError);
    expect(subsetProblems(run)).not.toEqual([]);
  });
});

// Integration (docs/polish/reference.md, audio x cards): a card the viewer can name colours its
// summon thud and its spell shimmer with its family (`timbre`), and a Legendary or Mythic unit enters
// with its own sting. Every one of those renders keeps the same contract as the plain recipe.
describe("B14 the card families and the entrance keep the recipe contract", () => {
  const runs: Run[] = [
    ...SFX_TIMBRES.flatMap((timbre) => [
      runRecipe(`summon {timbre: ${timbre}}`, SFX.summon.recipe, SFX.summon.durationMs, { timbre }),
      runRecipe(`summon {amount: 25, timbre: ${timbre}}`, SFX.summon.recipe, SFX.summon.durationMs, { amount: 25, timbre }),
      runRecipe(`spell {timbre: ${timbre}}`, SFX.spell.recipe, SFX.spell.durationMs, { timbre }),
    ]),
    runRecipe("entrance {mythic: true}", SFX.entrance.recipe, SFX.entrance.durationMs, { mythic: true }),
    runRecipe("notify {urgent: true}", SFX.notify.recipe, SFX.notify.durationMs, { urgent: true }),
  ];

  it("covers every family on summon and spell, the Mythic entrance and the urgent notify", () => {
    expect(runs).toHaveLength(SFX_TIMBRES.length * 3 + 2);
  });

  it("no family breaks a clause of the contract", () => {
    expect([
      ...runs.flatMap(subsetProblems),
      ...runs.flatMap(lengthProblems),
      ...runs.flatMap(scheduleProblems),
      ...runs.flatMap(stopProblems),
      ...runs.flatMap(wiringProblems),
      ...runs.flatMap(rampProblems),
    ]).toEqual([]);
  });

  it("a family changes the sound: a summon with an accent builds more than the plain thud", () => {
    const plain = runRecipe("summon {}", SFX.summon.recipe, SFX.summon.durationMs, {});
    for (const timbre of SFX_TIMBRES.filter((t) => t !== "field")) {
      const coloured = runs.find((run) => run.label === `summon {timbre: ${timbre}}`);
      expect(coloured?.made.length ?? 0, timbre).toBeGreaterThan(plain.made.length);
    }
  });

  it("an urgent notify (a draw offer to answer) is not the routine one", () => {
    const plain = runRecipe("notify {}", SFX.notify.recipe, SFX.notify.durationMs, {});
    const urgent = runs.find((run) => run.label === "notify {urgent: true}");
    expect(urgent?.made.length ?? 0).toBeGreaterThan(plain.made.length);
  });

  it("a Mythic entrance is not a Legendary one", () => {
    const legendary = runRecipe("entrance {}", SFX.entrance.recipe, SFX.entrance.durationMs, {});
    const mythic = runs.find((run) => run.label === "entrance {mythic: true}");
    expect(mythic?.made.length ?? 0).toBeGreaterThan(legendary.made.length);
  });
});

describe("B14 the shared noise buffer", () => {
  it("B14 noiseBuffer is one second of mono noise, cached per context", () => {
    for (const sampleRate of [44_100, 48_000]) {
      const audio = new FakeAudio({ sampleRate });
      const noise = noiseBuffer(audio.context);

      expect(noiseBuffer(audio.context), "the same buffer for the same context").toBe(noise);
      expect(noise.numberOfChannels).toBe(1);
      expect(noise.length).toBe(sampleRate);
      const data = audio.bufferOf(noise).channel(0);
      expect(data.every((x) => Number.isFinite(x) && x >= -1 && x <= 1)).toBe(true);
      expect(data.some((x) => x !== 0), "not silence").toBe(true);
      expect(audio.violations).toEqual([]);
    }
  });

  it("B14 noiseBuffer fills every context with the same samples (a fixed seed)", () => {
    const a = new FakeAudio();
    const b = new FakeAudio();
    const first = noiseBuffer(a.context);
    const second = noiseBuffer(b.context);

    expect(second).not.toBe(first);
    const da = a.bufferOf(first).channel(0);
    const db = b.bufferOf(second).channel(0);
    expect(da.length).toBe(db.length);
    expect(da.findIndex((x, i) => x !== db[i])).toBe(-1);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * R506: patch v0.2.0's families and card moments keep the recipe contract, and vary as they say
 * --------------------------------------------------------------------------------------------- */

describe("R506 the new families and moments keep the recipe contract", () => {
  const variants: Run[] = [
    ...[0, 1, 2, 3, 25].map((amount) => runRecipe(`chaosRoll {amount: ${String(amount)}}`, SFX.chaosRoll.recipe, SFX.chaosRoll.durationMs, { amount })),
    runRecipe("brand {release: true}", SFX.brand.recipe, SFX.brand.durationMs, { release: true }),
    ...Array.from({ length: IMPACT_AMOUNT_CAP }, (_, i) =>
      runRecipe(`clockTick {amount: ${String(i + 1)}}`, SFX.clockTick.recipe, SFX.clockTick.durationMs, { amount: i + 1 }),
    ),
    ...(["book", "pancake", "ai"] as const).flatMap((timbre) => [
      runRecipe(`summon {timbre: ${timbre}, amount: 1}`, SFX.summon.recipe, SFX.summon.durationMs, { timbre, amount: 1 }),
      runRecipe(`spell {timbre: ${timbre}}`, SFX.spell.recipe, SFX.spell.durationMs, { timbre }),
    ]),
  ];

  it("no variant breaks a clause of the contract", () => {
    expect([
      ...variants.flatMap(subsetProblems),
      ...variants.flatMap(lengthProblems),
      ...variants.flatMap(scheduleProblems),
      ...variants.flatMap(stopProblems),
      ...variants.flatMap(wiringProblems),
      ...variants.flatMap(rampProblems),
    ]).toEqual([]);
  });

  it("SFX_TIMBRES carries the Book, Pancake and AI families", () => {
    expect(SFX_TIMBRES).toEqual(expect.arrayContaining(["book", "pancake", "ai"]));
  });

  it("a Book, Pancake or AI spell lays its family's texture under the chimes: more than the plain spell", () => {
    const plain = runRecipe("spell {}", SFX.spell.recipe, SFX.spell.durationMs, {});
    for (const timbre of ["book", "pancake", "ai"] as const) {
      const coloured = runRecipe(`spell {timbre: ${timbre}}`, SFX.spell.recipe, SFX.spell.durationMs, { timbre });
      expect(coloured.made.length, timbre).toBeGreaterThan(plain.made.length);
    }
  });

  it("the AI family's blips play a bit-crushed wavetable: a looped buffer of a few stepped levels", () => {
    const run = runRecipe("summon {timbre: ai}", SFX.summon.recipe, SFX.summon.durationMs, { timbre: "ai" });
    const crushed = run.made.filter((n) => n.kind === "bufferSource" && n.buffer !== null && n.buffer.length < 1000);
    expect(crushed.length, "two blips").toBe(2);
    for (const blip of crushed) {
      expect(blip.loop).toBe(true);
      const levels = new Set(blip.buffer?.channel(0) ?? []);
      expect(levels.size, "stepped to a handful of levels").toBeLessThanOrEqual(7);
      expect(levels.size).toBeGreaterThan(2);
    }
  });

  it("Call to Chaos's roll dings once for each effect named, up to CHAOS_REVEAL_MAX, and lasts longer for each", () => {
    const made = (amount: number): Run => runRecipe(`chaosRoll ${String(amount)}`, SFX.chaosRoll.recipe, SFX.chaosRoll.durationMs, { amount });
    const lengths = [0, 1, 2, 3].map((amount) => returnedSeconds(made(amount)));
    expect([...lengths].sort((a, b) => a - b)).toEqual(lengths);
    expect(new Set(lengths).size).toBe(4);
    // A ding is an FM bell: two oscillators. Each effect named adds one.
    const oscillators = (amount: number): number => made(amount).made.filter((n) => n.kind === "oscillator").length;
    expect(oscillators(1) - oscillators(0)).toBe(2);
    expect(oscillators(3) - oscillators(0)).toBe(2 * CHAOS_REVEAL_MAX);
    expect(oscillators(25), "clamped").toBe(oscillators(CHAOS_REVEAL_MAX));
    expect(returnedSeconds(made(25))).toBe(returnedSeconds(made(CHAOS_REVEAL_MAX)));
  });

  it("a mark lifting is a soft release, not the brand landing", () => {
    const landing = runRecipe("brand {}", SFX.brand.recipe, SFX.brand.durationMs, {});
    const lifting = runRecipe("brand {release: true}", SFX.brand.recipe, SFX.brand.durationMs, { release: true });
    expect(returnedSeconds(lifting)).toBeLessThan(returnedSeconds(landing));
    expect(lifting.made.some((n) => n.kind === "bufferSource"), "no searing hiss").toBe(false);
    expect(landing.made.some((n) => n.kind === "bufferSource")).toBe(true);
  });

  it("the clock's tick grows sharper each second: higher and shorter as amount climbs from 1 to 10", () => {
    const tick = (amount: number): Run => runRecipe(`clockTick ${String(amount)}`, SFX.clockTick.recipe, SFX.clockTick.durationMs, { amount });
    const pitch = (run: Run): number => {
      const set = run.made.find((n) => n.kind === "oscillator")?.param("frequency").events.find((e) => e.method === "setValueAtTime");
      return set?.method === "setValueAtTime" ? set.value : 0;
    };
    const pitches = Array.from({ length: IMPACT_AMOUNT_CAP }, (_, i) => pitch(tick(i + 1)));
    const lengths = Array.from({ length: IMPACT_AMOUNT_CAP }, (_, i) => returnedSeconds(tick(i + 1)));
    for (let i = 1; i < IMPACT_AMOUNT_CAP; i += 1) {
      expect(pitches[i] ?? 0, `amount ${String(i + 1)} is higher`).toBeGreaterThan(pitches[i - 1] ?? Infinity);
      expect(lengths[i] ?? Infinity, `amount ${String(i + 1)} is shorter`).toBeLessThan(lengths[i - 1] ?? 0);
    }
  });
});

describe("R655 a recipe pitched for a card's effect", () => {
  /** `id`'s recipe through `renderSfx` at `pitch`, as the engine plays a card's effect. */
  const pitchedRun = (id: SfxId, pitch: number, params: SfxParams = {}): Run =>
    runRecipe(`${id} ${JSON.stringify(params)} at pitch ${String(pitch)}`, (c, out, at, p) => renderSfx(id, c, out, at, p, pitch), SFX[id].durationMs, params);
  const allProblems = (runs: readonly Run[]): string[] => [
    ...runs.flatMap(subsetProblems),
    ...runs.flatMap(lengthProblems),
    ...runs.flatMap(scheduleProblems),
    ...runs.flatMap(stopProblems),
    ...runs.flatMap(wiringProblems),
    ...runs.flatMap(rampProblems),
  ];
  const detunes = (run: Run): number[] =>
    run.made.filter((n) => n.kind === "oscillator" || n.kind === "biquad").map((n) => n.param("detune").settled());

  it("R655 every recipe keeps the recipe contract at the bank's lowest and highest pitch", () => {
    expect(allProblems(SFX_IDS.flatMap((id) => [pitchedRun(id, 0.25), pitchedRun(id, 4)]))).toEqual([]);
  });

  it("R655 every effect in the shipped bank keeps the contract at its own pitch and params, varied either way", () => {
    const effects = Object.entries(CARD_AUDIO.effects);
    expect(effects.length, "the bank has effects").toBeGreaterThan(0);
    const runs = effects.flatMap(([, effect]) =>
      [1 - EFFECT_PITCH_JITTER, 1 + EFFECT_PITCH_JITTER].map((jitter) => pitchedRun(effect.sfx, effect.pitch * jitter, effect.params ?? {})),
    );
    expect(allProblems(runs)).toEqual([]);
  });

  it("R655 renderSfx detunes every oscillator and filter by the pitch in cents and keeps the recipe's length", () => {
    const plain = pitchedRun("impact", 1, { amount: 8 });
    const up = pitchedRun("impact", 2, { amount: 8 });
    expect(detunes(plain).length).toBeGreaterThan(0);
    expect(new Set(detunes(plain))).toEqual(new Set([0]));
    expect(detunes(up)).toEqual(detunes(plain).map(() => 1200));
    expect(returnedSeconds(up)).toBe(returnedSeconds(plain));
    // The same nodes, at the same frequencies: only the detune differs.
    expect(up.made.map((n) => n.kind)).toEqual(plain.made.map((n) => n.kind));
  });

  it("R655 the crushed wavetable, which has no detune, plays as much faster as the pitch is higher", () => {
    const rate = (run: Run): number[] =>
      run.made.filter((n) => n.kind === "bufferSource" && n.buffer !== null && n.buffer.length < 1000).map((n) => n.param("playbackRate").settled());
    const plain = rate(pitchedRun("summon", 1, { timbre: "ai" }));
    const down = rate(pitchedRun("summon", 0.5, { timbre: "ai" }));
    expect(plain).toHaveLength(2);
    down.forEach((value, i) => expect(value).toBeCloseTo((plain[i] ?? 0) * 0.5, 9));
  });

  it("R655 the pitch is the one run's: a recipe run after a pitched one, or straight from SFX, is not detuned", () => {
    pitchedRun("death", 0.5);
    const after = runRecipe("death after a pitched run", SFX.death.recipe, SFX.death.durationMs, {});
    expect(new Set(detunes(after))).toEqual(new Set([0]));
  });
});

describe("R669 the play sting", () => {
  const stingRun = (params: SfxParams): Run => runRecipe(`sting ${JSON.stringify(params)}`, SFX.sting.recipe, SFX.sting.durationMs, params);

  it("R669 each tier keeps the recipe contract, and a rarer card's sting is longer and fuller", () => {
    const runs = [stingRun({}), stingRun({ tier: "rare" }), stingRun({ tier: "epic" })];
    expect([
      ...runs.flatMap(subsetProblems),
      ...runs.flatMap(lengthProblems),
      ...runs.flatMap(scheduleProblems),
      ...runs.flatMap(stopProblems),
      ...runs.flatMap(wiringProblems),
      ...runs.flatMap(rampProblems),
    ]).toEqual([]);
    const lengths = runs.map(returnedSeconds);
    expect(lengths[0]).toBeLessThan(lengths[1] ?? 0);
    expect(lengths[1]).toBeLessThan(lengths[2] ?? 0);
    const voices = runs.map((run) => run.made.filter((n) => n.kind === "oscillator").length);
    expect(voices[0]).toBeLessThan(voices[1] ?? 0);
    expect(voices[1]).toBeLessThan(voices[2] ?? 0);
  });

  it("R669 the pan param changes nothing in a recipe: the engine pans", () => {
    const plain = stingRun({ tier: "rare" });
    const panned = stingRun({ tier: "rare", pan: 0.6 });
    expect(panned.made.map((n) => n.kind)).toEqual(plain.made.map((n) => n.kind));
    expect(returnedSeconds(panned)).toBe(returnedSeconds(plain));
  });
});
