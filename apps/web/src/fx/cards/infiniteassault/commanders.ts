// R4596000: the entrances of the Infinite Assault's twelve Commanders (#217 to #239, odd numbers), each its
// card's signature (R3332000): the boss a generator wrote boots onto the board. Each draws a prop of its own
// over its zone in its doctrine's Tech look (fx/figures.ts), lands at FX_SLAM_AT after its entry's lead (a
// Mythic's takeover hold, R3405000) in a burst and a ring, with one beat of its own, and shakes once:
// - #217 Echo-1, First Hallucination ("echoBoot"): a wraith in hologram light, two afterimages flickering
//   either side of it before it lands.
// - #219 Redline, the Overclocked ("redlineBoot"): its telemetry needle in laser red, fire at its foot.
// - #221 Hivemaster Batch-01 ("hiveBoot"): a convoy in rocket exhaust, three exhaust puffs across its zone.
// - #223 Loopback, Who Returns ("loopBoot"): lock rings in glitch light, a ring as it starts and one as it lands.
// - #225 Root Kit, the Exploiter ("rootkitBoot"): a key turning in glitch light, circuits sparking from the lock.
// - #227 Bastion.exe ("bastionBoot"): a hex shield in hologram light, a small shield closing over it.
// - #229 The Harvester ("harvestBoot"): a maw in glitch light, a Mythic's rays.
// - #231 The Black Box ("blackBoxBoot"): a vault door under starfield, a burst of void and a Mythic's rays.
// - #233 Model Collapse ("collapseBoot"): a falling curve in glitch light, its zone cracking in shards.
// - #235 The Prime Generator ("primeBoot"): a blueprint in glitch light, a ring of circuits and a Mythic's rays.
// - #237 Thermal Runaway ("runawayBoot"): a forge shell in laser red, fire, embers and a ring of flame.
// - #239 Zero-Day ("zeroDayBoot"): a virus face in glitch light, its zone fracturing.
// Each keys on its readable `summoned` alone (entrances.ts, R202) and names no card; every cue starts inside
// its entry and ends within FX_MAX_TAIL_MS of it (R200).

import { raysCue, ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxFigure } from "../../figures.ts";
import type { FxPalette } from "../../flavors.ts";
import type { FxCue, FxPoint, FxPreset } from "../../types.ts";
import { FOOT, type Entrance, type EntranceKey, type EntrancePlan } from "../recipe.ts";
import { entranceFigure, leadOf, lookOf, slamAt, slamShake } from "../stinkyscape/signatureKit.ts";

/** Every particle count the twelve throw, at intensity "normal" (rule 9). */
export const COMMANDER_TUNING = {
  land: { count: 24, power: 1.1 },
  accent: { count: 12, power: 0.9 },
} as const;

/** Echo-1's afterimages, either side of its zone, and when they flicker, as a share of the motion before the slam (rule 9). */
const AFTERIMAGES: readonly FxPoint[] = [
  { x: 0.1, y: 0.5 },
  { x: 0.9, y: 0.5 },
];
const AFTERIMAGE_AT = 0.4;

/** Hivemaster's three exhaust puffs across its zone (rule 9). */
const PUFFS: readonly FxPoint[] = [
  { x: 0.2, y: 0.8 },
  { x: 0.5, y: 0.7 },
  { x: 0.8, y: 0.8 },
];

/** The lock Root Kit turns, on its zone. */
const LOCK: FxPoint = { x: 0.5, y: 0.45 };

/** What a Commander's boot draws: its prop, its look, its landing's preset, and its own beat. */
type Boot = { figure: FxFigure; palette: FxPalette; preset: FxPreset; beat: (plan: EntrancePlan, slam: number) => FxCue[] };

/** A Mythic's rays from its slam. */
const rays = (plan: EntrancePlan, slam: number): FxCue[] => [raysCue(plan.D, "mythic", tid(plan.tgt), slam)];

/** A burst of `preset` in `palette`'s colors at `at` of the zone. */
function accent(plan: EntrancePlan, preset: FxPreset, palette: FxPalette, at: FxPoint | undefined, delayMs: number, spread: "point" | "area" | "ring" = "point"): FxCue {
  return { ...tunedBurst(plan.intensity, preset, tid(plan.tgt, at), spread, delayMs, COMMANDER_TUNING.accent), colors: lookOf(palette).colors };
}

const BOOTS = {
  echoBoot: {
    figure: "wraith",
    palette: "Tech/hologram",
    preset: "sparkle",
    beat: (plan, slam) => AFTERIMAGES.map((at) => accent(plan, "sparkle", "Tech/hologram", at, leadOf(plan) + Math.round(AFTERIMAGE_AT * (slam - leadOf(plan))))),
  },
  redlineBoot: { figure: "telemetry", palette: "Tech/laser", preset: "spark", beat: (plan, slam) => [accent(plan, "fire", "Tech/laser", FOOT, slam, "area")] },
  hiveBoot: { figure: "convoy", palette: "Tech/rocket", preset: "exhaust", beat: (plan, slam) => PUFFS.map((at) => accent(plan, "exhaust", "Tech/rocket", at, slam)) },
  loopBoot: { figure: "lockRings", palette: "Tech/glitch", preset: "circuit", beat: (plan) => [ringCue(plan.D, "circuit", tid(plan.tgt), leadOf(plan))] },
  rootkitBoot: { figure: "keyTurn", palette: "Tech/glitch", preset: "circuit", beat: (plan, slam) => [accent(plan, "circuit", "Tech/glitch", LOCK, slam)] },
  bastionBoot: {
    figure: "hexShield",
    palette: "Tech/hologram",
    preset: "sparkle",
    beat: (plan, slam) => [{ kind: "shield", size: "small", at: tid(plan.tgt), delayMs: slam, durationMs: plan.D - slam }],
  },
  harvestBoot: { figure: "maw", palette: "Tech/glitch", preset: "circuit", beat: rays },
  blackBoxBoot: { figure: "vaultDoor", palette: "Tech/starfield", preset: "star", beat: (plan, slam) => [accent(plan, "void", "Tech/starfield", undefined, slam, "area"), ...rays(plan, slam)] },
  collapseBoot: {
    figure: "paretoCurve",
    palette: "Tech/glitch",
    preset: "circuit",
    beat: (plan, slam) => [{ kind: "crack", at: tid(plan.tgt), delayMs: slam, durationMs: plan.D - slam }, accent(plan, "shard", "Tech/glitch", FOOT, slam, "area")],
  },
  primeBoot: { figure: "blueprint", palette: "Tech/glitch", preset: "circuit", beat: (plan, slam) => [accent(plan, "circuit", "Tech/glitch", undefined, slam, "ring"), ...rays(plan, slam)] },
  runawayBoot: { figure: "forgeShell", palette: "Tech/laser", preset: "fire", beat: (plan, slam) => [accent(plan, "ember", "Tech/laser", FOOT, slam, "area"), accent(plan, "fire", "Tech/laser", undefined, slam, "ring")] },
  zeroDayBoot: {
    figure: "virusFace",
    palette: "Tech/glitch",
    preset: "circuit",
    beat: (plan, slam) => [{ kind: "fracture", at: tid(plan.tgt), delayMs: slam, durationMs: plan.D - slam }],
  },
} as const satisfies Readonly<Record<string, Boot>>;

/** The twelve entrance keys, in catalog order. */
type CommanderBootKey = keyof typeof BOOTS;

function boot(key: CommanderBootKey & EntranceKey): Entrance {
  const { figure, palette, preset, beat }: Boot = BOOTS[key];
  return (plan) => {
    const slam = slamAt(plan);
    const at = tid(plan.tgt);
    return [
      entranceFigure(figure, palette, plan),
      { ...tunedBurst(plan.intensity, preset, tid(plan.tgt, FOOT), "area", slam, COMMANDER_TUNING.land), colors: lookOf(palette).colors },
      ringCue(plan.D, lookOf(palette).particle, at, slam),
      ...beat(plan, slam),
      ...slamShake(plan, key),
    ];
  };
}

export const echoBoot = boot("echoBoot");
export const redlineBoot = boot("redlineBoot");
export const hiveBoot = boot("hiveBoot");
export const loopBoot = boot("loopBoot");
export const rootkitBoot = boot("rootkitBoot");
export const bastionBoot = boot("bastionBoot");
export const harvestBoot = boot("harvestBoot");
export const blackBoxBoot = boot("blackBoxBoot");
export const collapseBoot = boot("collapseBoot");
export const primeBoot = boot("primeBoot");
export const runawayBoot = boot("runawayBoot");
export const zeroDayBoot = boot("zeroDayBoot");
