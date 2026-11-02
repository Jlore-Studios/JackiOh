// R2784000: the damage and removal motifs (fx/motifs/) pinned on their gallery fixtures, held inside
// their entry (R200) and scaled through it (R201), and their dispatch by the readable play resolving now
// (cardFx.ts `planCardMotif`), planned the way FxLayer plans them: remember, then plan.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { planEntries, type AnimationEntry } from "../../game/animations.ts";
import { testid } from "../../game/contract.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { planCardMotif } from "../cardFx.ts";
import { FX_CARD_TABLE, type FxCardEntry } from "../cards/tables.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_MOTIF_SCALE_MAX, FX_MOTIF_TRAUMA } from "../constants.ts";
import { planFx } from "../cues.ts";
import { FLAVOR_LOOKS } from "../flavors.ts";
import { flavorLookOf } from "../flavorSource.ts";
import { createFxMemory } from "../memory.ts";
import type { FxCue, FxPlanEnv } from "../types.ts";
import { MOTIF_TUNING, motifScale, timesOf, type MotifSpec } from "./build.ts";
import { galleryEntry, gallerySource, motifFixture } from "./gallery.ts";
import { planMotif } from "./index.ts";
import { FX_DRAWN_MOTIFS, type DrawnMotif } from "./names.ts";

const SCALE = 1.5;
const FIRE = FLAVOR_LOOKS.Fire;

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

function specOf(motif: DrawnMotif, over: Partial<MotifSpec> = {}): MotifSpec {
  return { motif, look: FIRE, scale: SCALE, row: "replace", ...over };
}

/** The gallery's fixture entry for `motif` on the full board, its shots from the viewer's lane-1 unit, lasting D. */
function fixtureEntry(motif: DrawnMotif, D: number, view: PlayerView = fullBoardView()): AnimationEntry {
  return { ...galleryEntry(motifFixture(motif, view, gallerySource(view), "p1"), view), durationMs: D };
}

function planned(motif: DrawnMotif, D = 400, over: Partial<MotifSpec> = {}): FxCue[] {
  const entry = fixtureEntry(motif, D);
  return planMotif(specOf(motif, over), entry, entry.view, envOf());
}

const beats = (cues: readonly FxCue[]): string[] => cues.map((cue) => `${cue.kind}@${String(cue.delayMs)}`);

/** Where a cue ends, in ms after its entry starts (R200's bound is D + FX_MAX_TAIL_MS). */
function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
    case "shot":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const repeat = (times: number, ...beat: string[]): string[] => Array.from({ length: times }, () => beat).flat();

// At D 400: gather 0, travel 80, land 240, linger 300.
const PINNED: { readonly [M in DrawnMotif]: readonly string[] } = {
  bolt: ["burst@0", "shot@80", "burst@240", "splat@240", "burst@300", "shake@240"],
  volley: ["burst@0", ...repeat(3, "shot@80", "burst@240", "splat@240"), "shake@240"],
  // Six hits, FX_MOTIF_BARRAGE_STEP (8 ms at D 400) apart, each bolt flying the same 160 ms.
  barrage: ["burst@0", ...[0, 8, 16, 24, 32, 40].flatMap((k) => [`shot@${String(80 + k)}`, `burst@${String(240 + k)}`, `splat@${String(240 + k)}`]), "shake@240"],
  // Four hops of floor((300 − 80) / 4) = 55 ms.
  chain: ["burst@0", ...[0, 1, 2, 3].flatMap((k) => [`shot@${String(80 + k * 55)}`, `burst@${String(135 + k * 55)}`, `splat@${String(135 + k * 55)}`]), "shake@300"],
  lance: ["burst@0", "shot@80", ...repeat(4, "burst@240"), "ring@240", "ring@240", "burst@240", "splat@240", "burst@300", "shake@240"],
  nova: ["burst@0", "ring@80", "burst@240", "ring@240", "ring@300", ...repeat(5, "burst@240", "splat@240"), "shake@240"],
  scorch: ["burst@0", "burst@240", "splat@240", "burst@240", "burst@300", "shake@240"],
  // The wave reaches lane n at 80 + (n − 1) × 40 on both sides, then every one of the ten units flares.
  wipe: [...[80, 120, 160, 200, 240].flatMap((at) => repeat(2, `burst@${String(at)}`)), "ring@240", "shake@240", ...repeat(10, "burst@240")],
  vanish: ["burst@0", "sheen@80", "burst@240", "ring@240"],
  negate: ["ring@0", "burst@240", "crack@240", "burst@300", "shake@240"],
};

describe("R2784000 the damage and removal motifs", () => {
  for (const motif of FX_DRAWN_MOTIFS) {
    it(`R2784000 ${motif} pins its cues for its fixture, in the palette's colors at the scale's counts`, () => {
      const cues = planned(motif);
      expect(beats(cues)).toEqual(PINNED[motif]);
      for (const cue of cues) {
        if (cue.kind === "burst" && cue.preset !== "smoke") expect(cue.colors, `${motif} ${cue.preset}`).toEqual(FIRE.colors);
        if (cue.kind === "shot") expect(cue.colors).toEqual(FIRE.colors);
        if (cue.kind === "shake") expect(cue.trauma).toBeCloseTo(FX_MOTIF_TRAUMA[motif] * SCALE);
      }
      const land = cues.filter((cue) => cue.kind === "burst" && cue.power === MOTIF_TUNING.land.power && cue.spread === "area");
      for (const cue of land) expect(cue.kind === "burst" && cue.count).toBe(Math.round(MOTIF_TUNING.land.count * SCALE));
      // R202: no cue names a definition.
      expect(JSON.stringify(cues)).not.toMatch(/core-\d{3}/);
    });

    it(`R200 ${motif}'s every cue starts inside its entry and ends within FX_MAX_TAIL_MS of its end, for short and long entries`, () => {
      for (const D of [120, 400, 1400]) {
        const cues = planned(motif, D);
        expect(cues.length).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${motif} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "shot") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma).toBeLessThanOrEqual(1);
        }
      }
    });

    it(`R201 ${motif} plays the same beats at every effects speed, each the same fraction of D`, () => {
      const base = planned(motif, 400);
      for (const speed of [0.5, 1, 2]) {
        const D = 400 / speed;
        const cues = planned(motif, D);
        expect(cues.map((cue) => cue.kind)).toEqual(base.map((cue) => cue.kind));
        cues.forEach((cue, k) => {
          expect(Math.abs(cue.delayMs - (base[k]?.delayMs ?? 0) / speed), `${motif} ${cue.kind} at speed ${String(speed)}`).toBeLessThanOrEqual(4);
        });
        // Every motif but chain (whose last hop lands at linger) lands on FX_MOTIF_PHASES.land of D.
        if (motif !== "chain") expect(cues.some((cue) => cue.delayMs === timesOf(D).L)).toBe(true);
      }
    });
  }

  it("R2784000 lance runs along one lane through both rows: a beam from the far backrow into the target's, every zone of the lane flaring", () => {
    const view = fullBoardView();
    const cues = planned("lance");
    const beam = cues.find((cue) => cue.kind === "shot");
    expect(beam).toEqual(
      expect.objectContaining({
        shot: { motion: "beam" },
        from: { kind: "testid", testid: testid.zone("you", "backrow", 3) },
        to: { kind: "testid", testid: testid.zone("opponent", "backrow", 3) },
      }),
    );
    const L = timesOf(400).L;
    const flared = cues.flatMap((cue) => (cue.kind === "burst" && cue.delayMs === L && cue.at.kind === "testid" ? [cue.at.testid] : []));
    for (const zone of [testid.zone("you", "backrow", 3), testid.zone("you", "units", 3), testid.zone("opponent", "units", 3), testid.zone("opponent", "backrow", 3)]) {
      expect(flared, zone).toContain(zone);
    }
    // The target is the opponent's lane-3 unit.
    expect(flared).toContain(testid.card(view.opponent.units[2]?.instanceId ?? ""));
  });

  it("R2784000 a target in no lane (a hero) takes bolt's cues from lance", () => {
    const view = fullBoardView();
    const entry = { ...galleryEntry([{ type: "damage", sourceId: gallerySource(view), targetId: "hero-p2", amount: 2, combat: false }], view), durationMs: 400 };
    expect(beats(planMotif(specOf("lance"), entry, view, envOf()))).toEqual(PINNED.bolt);
  });

  it("R2784000 with `row: keep` a motif draws no numbers, leaving them to the event's own recipe", () => {
    expect(planned("bolt", 400, { row: "keep" }).some((cue) => cue.kind === "splat")).toBe(false);
  });

  it("R2784000 reduced motion and the effects off: no shake, no flash, nothing at all", () => {
    for (const motif of FX_DRAWN_MOTIFS) {
      const entry = fixtureEntry(motif, 400);
      expect(planMotif(specOf(motif), entry, entry.view, envOf({ intensity: 0 })), motif).toEqual([]);
      // Reduced motion gives every entry a duration of 0.
      expect(planMotif(specOf(motif), { ...entry, durationMs: 0 }, entry.view, envOf()), motif).toEqual([]);
    }
  });

  it("R2784000 motifScale grows with cost and rarity, counts X and Embiggen costs, and is capped", () => {
    const view = fullBoardView();
    const i = FX_INTENSITY_SCALE.normal;
    expect(motifScale("core-016", view, i)).toBeCloseTo(1 * (1 + 3 * 0.08)); // Common, 3
    expect(motifScale("core-035", view, i)).toBeCloseTo(1.1 * (1 + 1 * 0.08)); // Rare, 1
    expect(motifScale("classic-080", view, i)).toBeCloseTo(1.4 * (1 + 4 * 0.08)); // Legendary, 4
    expect(motifScale("core-024", view, i)).toBeCloseTo(1.25 * (1 + 3 * 0.08)); // Epic, X counts 3
    expect(motifScale("meditativeplus-005", view, i)).toBeCloseTo(1.1); // Rare, Embiggen base 0
    expect(motifScale("core-100", view, i)).toBe(FX_MOTIF_SCALE_MAX); // Mythic, 100: capped
    expect(motifScale("core-100", view, FX_INTENSITY_SCALE.low)).toBeCloseTo(FX_MOTIF_SCALE_MAX * FX_INTENSITY_SCALE.low);
    expect(motifScale("hidden", view, i)).toBe(i);
    expect(motifScale("no-such-card", view, i)).toBe(i);
  });
});

/* Dispatch: the card's table entry, by the play resolving now */

/** Plans each entry of `events` in order, as FxLayer does on every `start`: remember, then plan. */
function planAll(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf()): { entry: AnimationEntry; cues: FxCue[] }[] {
  const planned = planEntries(events, withEvents(view, [...events]), false);
  return planned.map((entry) => {
    env.memory.remember(entry.events);
    return { entry, cues: planFx(entry, entry.view, env) };
  });
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({
  type: "cardResolved",
  player,
  instanceId,
  defId,
  permanent: false,
  costPaid: 1,
});

/** The twelve example cards and the motif each names. */
const EXAMPLES: readonly (readonly [string, DrawnMotif])[] = [
  ["core-035", "bolt"],
  ["core-088", "wipe"],
  ["classicplus-021", "nova"],
  ["classicplus-025", "bolt"],
  ["meditative-078", "vanish"],
  ["meditativeplus-005", "chain"],
  ["stinkyscape-003", "nova"],
  ["stinkyscape-097", "barrage"],
  ["traininggrounds-010", "volley"],
  ["traininggrounds-026", "vanish"],
  ["classic-016", "scorch"],
  ["classic-071", "lance"],
];

/** The cues of every entry but the play's own `cardPlayed` and `cardResolved`. */
function effectCues(defId: string, motif: DrawnMotif, instanceId = "ex"): FxCue[] {
  const view = fullBoardView();
  const stream = [played("p1", instanceId, defId), ...motifFixture(motif, view, instanceId, "p1"), resolved("p1", instanceId, defId)];
  return planAll(stream, view)
    .filter(({ entry }) => !entry.events.some((event) => event.type === "cardPlayed" || event.type === "cardResolved"))
    .flatMap(({ cues }) => cues);
}

function colored(cues: readonly FxCue[], colors: readonly string[]): number {
  return cues.filter((cue) => cue.kind === "burst" && JSON.stringify(cue.colors) === JSON.stringify(colors)).length;
}

describe("R2784000 a card's table entry plays by the play resolving now", () => {
  it("R2784000 cast and cry dispatch by the resolving play: each example draws its motif in its look, a card with none does not", () => {
    for (const [defId, motif] of EXAMPLES) {
      const entry = FX_CARD_TABLE.get(defId);
      const hook = entry?.kind === "hooks" ? (entry.hooks.cast ?? entry.hooks.cry) : undefined;
      expect(hook?.motif, defId).toBe(motif);
      const look = hook?.palette !== undefined ? FLAVOR_LOOKS[hook.palette] : flavorLookOf(defId, "ex", fullBoardView());
      const colors = look?.colors ?? [];
      const drawn = effectCues(defId, motif);
      const plain = effectCues("core-016", motif);
      expect(colored(drawn, colors), defId).toBeGreaterThan(colored(plain, colors));
      expect(drawn, defId).not.toEqual(plain);
    }
  });

  it("R2784000 classic-071 Lane Eater, a Unit, plays its motif on cry", () => {
    const entry = FX_CARD_TABLE.get("classic-071");
    expect(entry).toEqual({ kind: "hooks", hooks: { cry: { motif: "lance" } } });
    const drawn = effectCues("classic-071", "lance");
    expect(drawn.some((cue) => cue.kind === "shot" && cue.shot.motion === "beam")).toBe(true);
  });

  it("R2784000 a not-yet-drawn motif, or a death, trigger or activate hook, plays the event rows only", () => {
    const view = fullBoardView();
    const env = envOf();
    const events = [played("p1", "ex", "core-035"), ...motifFixture("bolt", view, "ex", "p1")];
    env.memory.remember(events);
    const entry = { ...galleryEntry(events.slice(1), view), durationMs: 400 };
    const tableOf = (card: FxCardEntry): ReadonlyMap<string, FxCardEntry> => new Map([["core-035", card]]);
    expect(planCardMotif(entry, view, env, tableOf({ kind: "hooks", hooks: { cast: { motif: "bolt" } } }))).not.toBeNull();
    expect(planCardMotif(entry, view, env, tableOf({ kind: "hooks", hooks: { cast: { motif: "aura" } } }))).toBeNull();
    for (const hook of ["death", "trigger", "activate"] as const) {
      expect(planCardMotif(entry, view, env, tableOf({ kind: "hooks", hooks: { [hook]: { motif: "bolt" } } })), hook).toBeNull();
    }
    expect(planCardMotif(entry, view, env, tableOf({ kind: "none", reason: "pending" }))).toBeNull();
    // A Spell's `cry` is never read: its play is `cast`.
    expect(planCardMotif(entry, view, env, tableOf({ kind: "hooks", hooks: { cry: { motif: "bolt" } } }))).toBeNull();
  });

  it("R2784000 a run the runner splits into an entry per hit still chains: each hop leaves from the play's last hit, and it gathers once", () => {
    const view = fullBoardView();
    const foes = view.opponent.units.map((unit) => unit?.instanceId ?? "");
    const stream = [played("p1", "ex", "meditativeplus-005"), ...motifFixture("chain", view, "ex", "p1"), resolved("p1", "ex", "meditativeplus-005")];
    const hits = planAll(stream, view).filter(({ entry }) => entry.events.every((event) => event.type === "damage"));
    expect(hits).toHaveLength(4);
    const from = hits.map(({ cues }) => cues.find((cue) => cue.kind === "shot")).map((shot) => (shot?.kind === "shot" ? shot.from : null));
    expect(from).toEqual([
      { kind: "testid", testid: testid.hero("you") },
      { kind: "testid", testid: testid.card(foes[0] ?? "") },
      { kind: "testid", testid: testid.card(foes[1] ?? "") },
      { kind: "testid", testid: testid.card(foes[2] ?? "") },
    ]);
    const gathers = hits.map(({ cues }) => cues.filter((cue) => cue.kind === "burst" && cue.power === MOTIF_TUNING.gather.power).length);
    expect(gathers).toEqual([1, 0, 0, 0]);
  });

  it("R2784000 Lane Eater's cry lances its lane once: the later card it destroys in that lane only flares it", () => {
    const view = fullBoardView();
    const destroyed = (unit: (typeof view.you.units)[number] | undefined): GameEvent => ({
      type: "destroyed",
      instanceId: unit?.instanceId ?? "",
      defId: unit?.defId ?? "",
      owner: unit?.owner ?? "p1",
      controller: unit?.controller ?? "p1",
      attack: unit?.attack ?? 0,
      maxHealth: unit?.maxHealth ?? 0,
      killerId: "ex",
    });
    const stream = [played("p1", "ex", "classic-071"), destroyed(view.opponent.units[2]), destroyed(view.you.units[2]), resolved("p1", "ex", "classic-071")];
    const removals = planAll(stream, view).filter(({ entry }) => entry.events.every((event) => event.type === "destroyed"));
    expect(removals).toHaveLength(2);
    expect(removals.map(({ cues }) => cues.filter((cue) => cue.kind === "shot" && cue.shot.motion === "beam").length)).toEqual([1, 0]);
    // The second still flares the lane's zones and cracks its card.
    const second = removals[1]?.cues ?? [];
    const flared = second.flatMap((cue) => (cue.kind === "burst" && cue.at.kind === "testid" ? [cue.at.testid] : []));
    for (const zone of [testid.zone("you", "backrow", 3), testid.zone("you", "units", 3), testid.zone("opponent", "units", 3), testid.zone("opponent", "backrow", 3)]) {
      expect(flared, zone).toContain(zone);
    }
    expect(second.some((cue) => cue.kind === "crack")).toBe(true);
  });

  it("R2784000 a hit another card deals while the play resolves keeps its own row", () => {
    const view = fullBoardView();
    const own = effectCues("core-035", "bolt");
    const stream = [played("p1", "ex", "core-035"), ...motifFixture("bolt", view, "trap-1", "p1"), resolved("p1", "ex", "core-035")];
    const theirs = planAll(stream, view)
      .filter(({ entry }) => entry.events.every((event) => event.type === "damage"))
      .flatMap(({ cues }) => cues);
    const none = planAll(motifFixture("bolt", view, "trap-1", "p1"), view).flatMap(({ cues }) => cues);
    expect(theirs).toEqual(none);
    expect(own).not.toEqual(none);
  });

  it("R2784000 R202 a hidden play keys no entry: its effects are those of no play at all", () => {
    const view = fullBoardView();
    const fixture = (source: string): GameEvent[] => motifFixture("bolt", view, source, "p2");
    const hidden = planAll([played("p2", "hidden", "hidden"), ...fixture("hidden"), resolved("p2", "hidden", "hidden")], view)
      .filter(({ entry }) => entry.events.every((event) => event.type === "damage"))
      .flatMap(({ cues }) => cues);
    const bare = planAll(fixture("hidden"), view).flatMap(({ cues }) => cues);
    expect(hidden).toEqual(bare);
    const readable = planAll([played("p2", "ex", "core-035"), ...fixture("ex"), resolved("p2", "ex", "core-035")], view)
      .filter(({ entry }) => entry.events.every((event) => event.type === "damage"))
      .flatMap(({ cues }) => cues);
    expect(readable).not.toEqual(bare);
    expect(JSON.stringify(readable)).not.toContain("core-035");
  });

  it("R2784000 the effects off and reduced motion plan none of a card's motif", () => {
    for (const [defId, motif] of EXAMPLES) {
      const view = fullBoardView();
      const stream = [played("p1", "ex", defId), ...motifFixture(motif, view, "ex", "p1"), resolved("p1", "ex", defId)];
      expect(planAll(stream, view, envOf({ intensity: 0 })).flatMap(({ cues }) => cues), defId).toEqual([]);
      const env = envOf();
      const still = planEntries(stream, withEvents(view, stream), false).map((entry) => ({ ...entry, durationMs: 0 }));
      for (const entry of still) {
        env.memory.remember(entry.events);
        expect(planCardMotif(entry, entry.view, env), defId).toBeNull();
      }
    }
  });
});
