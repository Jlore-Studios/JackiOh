// R2784000: the motifs (fx/motifs/) pinned on their gallery fixtures, held inside their entry (R200)
// and scaled through it (R201), and their dispatch by the readable play resolving now (cardFx.ts
// `planCardMotif`), planned the way FxLayer plans them: remember, then plan.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid, planEntries, type AnimationEntry } from "../../game/animations.ts";
import { testid } from "../../game/contract.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { planCardMotif } from "../cardFx.ts";
import { FX_CARD_TABLE, type FxCardEntry } from "../cards/tables.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_MOTIF_SCALE_MAX, FX_MOTIF_TRAUMA } from "../constants.ts";
import { planFx } from "../cues.ts";
import { FLAVOR_LOOKS, FLAVOR_SUB_LOOKS, paletteLook, type FxPalette } from "../flavors.ts";
import { flavorLookOf } from "../flavorSource.ts";
import { createFxMemory } from "../memory.ts";
import type { FxBurstCue, FxCue, FxPlanEnv } from "../types.ts";
import { MOTIF_TUNING, motifScale, timesOf, type MotifSpec } from "./build.ts";
import { galleryEntry, gallerySource, motifFixture } from "./gallery.ts";
import { motifReplaces, planMotif } from "./index.ts";
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
  lance: ["burst@0", "shot@80", ...repeat(4, "burst@240"), "ring@240", "ring@240", "burst@240", "splat@240", "burst@300", "shake@240"],
  nova: ["burst@0", "ring@80", "burst@240", "ring@240", "ring@300", ...repeat(5, "burst@240", "splat@240"), "shake@240"],
  scorch: ["burst@0", "burst@240", "splat@240", "burst@240", "burst@300", "shake@240"],
  // The wave reaches lane n at 80 + (n − 1) × 40 on both sides, then every one of the ten units flares.
  wipe: [...[80, 120, 160, 200, 240].flatMap((at) => repeat(2, `burst@${String(at)}`)), "ring@240", "shake@240", ...repeat(10, "burst@240")],
  vanish: ["burst@0", "sheen@80", "burst@240", "ring@240"],
  negate: ["ring@0", "burst@240", "crack@240", "burst@300", "shake@240"],
  // The stream leaves the enemy hero at gather, flies travel to land and pools where it lands.
  drain: ["burst@0", "projectile@80", "burst@240", "splat@240", "burst@300", "shake@240"],
  bless: ["arrows@0", "sheen@80", "burst@240", "shake@240"],
  curse: ["arrows@0", "burst@240", "shake@240"],
  conjure: ["burst@0", "burst@240", "ring@240", "burst@300", "shake@240"],
  rise: ["ghost@80", "burst@240", "ring@240", "shake@240"],
  // The glyph rings the lane while the unit sinks where it stood.
  offering: ["ring@0", "burst@0", "burst@240", "ring@300", "shake@240"],
  // Every one of the ten units, then the ten backrow cards, walks its own path across the centre line.
  shift: [...repeat(20, "ghost@80", "burst@240"), "shake@240"],
  bind: ["ring@0", "burst@240", "shake@240"],
  tether: ["shot@80", "burst@240", "shake@240"],
  transmute: ["burst@0", "sheen@80", "burst@240", "burst@300", "shake@240"],
  flight: ["ghost@80", "burst@240"],
  // A gather where the mana sits, one sparkle per crystal gained across Travel to Land, and the fill landing.
  mana: ["burst@0", "burst@80", "burst@160", "burst@240"],
  tithe: ["burst@240", "shake@240"],
  tally: ["burst@240", "shake@240"],
  // The ring closes at gather, the reveal lands on jk-trap-flip's stop, the shake on Land.
  snap: ["ring@0", "burst@120", "shake@240"],
  // #2793: a gather at the caster's hero, a card back flying to the hand, its sparkle there; no shake.
  send: ["burst@0", "ghost@80", "burst@240"],
  // A sheen over the hand card, its landing, the Radiant rays once a play, the shake.
  radiance: ["sheen@80", "burst@240", "rays@240", "shake@240"],
  // A sheen over the healed unit, a ring burst and ring closing as it lands, the afterglow; no shake.
  mend: ["sheen@80", "burst@240", "ring@240", "burst@300"],
  // #2796: a gather at the receiver's hero (no play resolves), then a back into the hand and one into the deck.
  deal: ["burst@0", "ghost@80", "burst@240", "ghost@80", "burst@240"],
  // #2796: with no play there is no caster to bolt from: the ring closes on the modifiers, then the afterglow.
  seal: ["ring@240", "burst@240", "burst@300", "shake@240"],
  // R2794000: a gather on the graveyard pile, the back's flight to the hand and its sparkle there; no shake.
  recall: ["burst@0", "ghost@80", "burst@240"],
  // R2797000: a ring closes on the frozen card, a sheen crosses it and the mark takes on Land.
  hex: ["ring@0", "sheen@80", "burst@240", "shake@240"],
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
        // Every motif lands on FX_MOTIF_PHASES.land of D.
        expect(cues.some((cue) => cue.delayMs === timesOf(D).L)).toBe(true);
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

  it("R2784000 with `row: keep` a motif draws no numbers (no splat, no Armor flash), leaving them to the event's own recipe", () => {
    for (const motif of FX_DRAWN_MOTIFS) expect(planned(motif, 400, { row: "keep" }).some((cue) => cue.kind === "splat" || cue.kind === "shield"), motif).toBe(false);
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
const destroyed = (unit: PlayerView["you"]["units"][number] | undefined, killerId: string | null = "ex"): GameEvent => ({
  type: "destroyed",
  instanceId: unit?.instanceId ?? "",
  defId: unit?.defId ?? "",
  owner: unit?.owner ?? "p1",
  controller: unit?.controller ?? "p1",
  attack: unit?.attack ?? 0,
  maxHealth: unit?.maxHealth ?? 0,
  killerId,
});

/** The twelve example cards and the motif each names. */
const EXAMPLES: readonly (readonly [string, DrawnMotif])[] = [
  ["core-035", "bolt"],
  ["classicplus-032-2", "wipe"],
  ["classicplus-021", "nova"],
  ["classicplus-025", "bolt"],
  ["meditative-078", "vanish"],
  ["meditativeplus-005", "nova"],
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
      const look = paletteLook(hook?.palette ?? "") ?? flavorLookOf(defId, "ex", fullBoardView());
      const colors = look?.colors ?? [];
      const drawn = effectCues(defId, motif);
      const plain = effectCues("core-008", motif);
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

  it("R2784000 a not-yet-drawn motif, or a death, trigger or activate hook during the card's own play, plays the event rows only", () => {
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

  it("#2792 a palette naming a sub-look draws in its colors and particle, a flavor's name in its base look", () => {
    const view = fullBoardView();
    const env = envOf();
    const events = [played("p1", "ex", "core-035"), ...motifFixture("bolt", view, "ex", "p1")];
    env.memory.remember(events);
    const entry = { ...galleryEntry(events.slice(1), view), durationMs: 400 };
    const tableOf = (card: FxCardEntry): ReadonlyMap<string, FxCardEntry> => new Map([["core-035", card]]);
    const burstsOf = (palette: FxPalette) => {
      const result = planCardMotif(entry, view, env, tableOf({ kind: "hooks", hooks: { cast: { motif: "bolt", palette } } }));
      expect(result, palette).not.toBeNull();
      return (result?.cues ?? []).filter((cue): cue is FxBurstCue => cue.kind === "burst" && cue.preset !== "smoke");
    };
    const laser = FLAVOR_SUB_LOOKS.Tech?.laser;
    expect(laser).toBeDefined();
    const laserBursts = burstsOf("Tech/laser");
    expect(laserBursts.length).toBeGreaterThan(0);
    for (const burst of laserBursts) expect(burst.colors, "Tech/laser").toEqual(laser?.colors);
    expect(laserBursts.some((burst) => burst.preset === laser?.particle), "Tech/laser").toBe(true);
    const techBursts = burstsOf("Tech");
    expect(techBursts.length).toBeGreaterThan(0);
    for (const burst of techBursts) expect(burst.colors, "Tech").toEqual(FLAVOR_LOOKS.Tech.colors);
    expect(techBursts.some((burst) => burst.preset === "circuit"), "Tech").toBe(true);
  });

  it("R2803000 Spirit Lash's nova blasts every enemy unit at once: one gather, then a burst and splat per foe", () => {
    const view = fullBoardView();
    const foes = view.opponent.units.filter((unit) => unit !== null);
    const stream = [played("p1", "ex", "meditativeplus-005"), ...motifFixture("nova", view, "ex", "p1"), resolved("p1", "ex", "meditativeplus-005")];
    const cues = planAll(stream, view)
      .filter(({ entry }) => entry.events.every((event) => event.type === "damage"))
      .flatMap(({ cues }) => cues);
    const gathers = cues.filter((cue) => cue.kind === "burst" && cue.power === MOTIF_TUNING.gather.power);
    expect(gathers).toHaveLength(1);
    expect(cues.filter((cue) => cue.kind === "splat")).toHaveLength(foes.length);
    expect(cues.some((cue) => cue.kind === "shake")).toBe(true);
  });

  it("R2784000 Lane Eater's cry lances its lane once: the later card it destroys in that lane only flares it", () => {
    const view = fullBoardView();
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

  it("R2784000 a card the motif already struck dies under its own row: a lethal bolt flies once", () => {
    const view = fullBoardView();
    const killed = destroyed(view.opponent.units[0]);
    const stream = [played("p1", "ex", "core-035"), ...motifFixture("bolt", view, "ex", "p1"), killed, resolved("p1", "ex", "core-035")];
    const entries = planAll(stream, view);
    expect(entries.flatMap(({ cues }) => cues).filter((cue) => cue.kind === "shot")).toHaveLength(1);
    const death = entries.filter(({ entry }) => entry.events.every((event) => event.type === "destroyed")).flatMap(({ cues }) => cues);
    const bare = planAll([...motifFixture("bolt", view, "ex", "p1"), killed], view)
      .filter(({ entry }) => entry.events.every((event) => event.type === "destroyed"))
      .flatMap(({ cues }) => cues);
    expect(death.length).toBeGreaterThan(0);
    expect(death).toEqual(bare);
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

describe("R2784000 the board and life motifs follow the rules they draw", () => {
  /** Every ghost `shift` plans for a rotation of `direction` while `player`'s card resolves (none: no play), as `from>to` zone testids. */
  function rotationPaths(direction: "left" | "right", player: PlayerId | null): string[] {
    const view = fullBoardView();
    const env = envOf();
    if (player !== null) env.memory.remember([played(player, "ex", "core-052")]);
    const entry = { ...galleryEntry([{ type: "rotated", direction }], view), durationMs: 400 };
    return planMotif(specOf("shift"), entry, view, env).flatMap((cue) =>
      cue.kind === "ghost" && cue.from.kind === "testid" && cue.to.kind === "testid" ? [`${cue.from.testid}>${cue.to.testid}`] : [],
    );
  }

  /** R14's ring from the viewer's seat, one step right: their lanes 1 to 5, then the opponent's 5 down to 1. */
  function viewerRingRight(row: "units" | "backrow"): string[] {
    const ring = [...[1, 2, 3, 4, 5].map((lane) => testid.zone("you", row, lane)), ...[5, 4, 3, 2, 1].map((lane) => testid.zone("opponent", row, lane))];
    return ring.map((zone, k) => `${zone}>${ring[(k + 1) % ring.length] ?? ""}`);
  }

  it("R14 R2784000 shift walks a rotation along each row's ring from the rotating player's seat: an end card crosses, the far side steps the other way", () => {
    const right = rotationPaths("right", "p1");
    expect(right).toEqual([...viewerRingRight("units"), ...viewerRingRight("backrow")]);
    expect(right).toContain(`${testid.zone("you", "units", 5)}>${testid.zone("opponent", "units", 5)}`);
    expect(right).toContain(`${testid.zone("opponent", "units", 1)}>${testid.zone("you", "units", 1)}`);
    expect(right).toContain(`${testid.zone("opponent", "units", 3)}>${testid.zone("opponent", "units", 2)}`);
    // Left is the same ring run backwards.
    const back = (path: string): string => path.split(">").reverse().join(">");
    expect([...rotationPaths("left", "p1")].sort()).toEqual(right.map(back).sort());
    // The ring turns from its player's seat: the opponent's left is the viewer's right, and its right is not.
    expect([...rotationPaths("left", "p2")].sort()).toEqual([...right].sort());
    expect([...rotationPaths("right", "p2")].sort()).toEqual(right.map(back).sort());
  });

  it("R14 R2784000 with no play resolving, which way a rotation ran is unknown: shift draws no path for it", () => {
    expect(rotationPaths("right", null)).toEqual([]);
  });

  it("R2784000 #2793 radiance's rays play once a play: at the first card of its first Radiant entry, not on a later one", () => {
    const view = fullBoardView();
    const env = envOf();
    env.memory.remember([played("p1", "ex", "core-029")]);
    const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
    const radiant = (k: number): GameEvent => ({ type: "radiantSet", instanceId: hand[k]?.instanceId ?? "", defId: hand[k]?.defId ?? "", zone: { z: "hand", player: "p1" } });
    const raysOf = (events: GameEvent[]): number => {
      env.memory.remember(events);
      const entry = { ...galleryEntry(events, view), durationMs: 400 };
      return planMotif(specOf("radiance"), entry, view, env).filter((cue) => cue.kind === "rays").length;
    };
    expect(raysOf([radiant(0), radiant(1)])).toBe(1);
    expect(raysOf([radiant(2)])).toBe(0);
  });

  it("R2784000 drain shows a life loss's number once: in place of its row under replace, left to the row under keep", () => {
    const view = fullBoardView();
    const env = envOf();
    const events = [played("p1", "ex", "core-035"), ...motifFixture("drain", view, "ex", "p1")];
    env.memory.remember(events);
    const entry = { ...galleryEntry(events.slice(1), view), durationMs: 400 };
    const losses = (cues: readonly FxCue[]): number => cues.filter((cue) => cue.kind === "splat" && cue.tone === "loss").length;
    // With no play resolving, the rows alone draw the entry: the life row's number.
    expect(losses(planFx(entry, view, envOf()))).toBe(1);
    for (const row of ["replace", "keep"] as const) {
      const table: ReadonlyMap<string, FxCardEntry> = new Map([["core-035", { kind: "hooks", hooks: { cast: { motif: "drain", row } } }]]);
      const motif = planCardMotif(entry, view, env, table);
      expect(motif, row).not.toBeNull();
      const rows = planFx({ ...entry, events: entry.events.filter((event) => motif?.replaces.has(event) !== true) }, view, envOf());
      expect(losses(motif?.cues ?? []) + losses(rows), row).toBe(1);
    }
  });

  it("R2803000 the folded ward's Armor flash draws once in the rows: the armor row's full shield in Holy", () => {
    const view = fullBoardView();
    const foe = view.opponent.units[0];
    if (foe === null || foe === undefined) throw new Error("the fixture board has no enemy unit");
    const events: GameEvent[] = [played("p1", "ex", "core-035"), { type: "damageAbsorbed", sourceId: null, targetId: foe.instanceId, absorbed: 5, combat: false }];
    const entry = { ...galleryEntry(events.slice(1), view), durationMs: 400 };
    const cues = planFx(entry, view, envOf());
    // The armor row's flash, recolored into Holy where `ward` drew it, and the one shake.
    expect(cues.filter((cue) => cue.kind === "shield")).toHaveLength(1);
    expect(cues.filter((cue) => cue.kind === "burst" && cue.colors !== undefined && JSON.stringify(cue.colors) === JSON.stringify(FLAVOR_LOOKS.Holy.colors)).length).toBeGreaterThan(0);
    expect(cues.filter((cue) => cue.kind === "shake")).toHaveLength(1);
  });

  it("R2794000 recall flies a card back from its owner's graveyard pile into the hand on either seat", () => {
    const view = fullBoardView();
    for (const [player, side] of [
      ["p1", "you"],
      ["p2", "opponent"],
    ] as const) {
      const entry = { ...galleryEntry([{ type: "addedToHand", player, instanceId: "fx-recalled", defId: "core-003" }], view), durationMs: 400 };
      const cues = planMotif(specOf("recall"), entry, view, envOf());
      const grave = { kind: "testid", testid: animTestid.graveyard(side) };
      const hand = { kind: "testid", testid: animTestid.hand(side) };
      expect(beats(cues), side).toEqual(PINNED.recall);
      expect(cues[0], side).toEqual(expect.objectContaining({ kind: "burst", at: grave }));
      expect(cues[1], side).toEqual({ kind: "ghost", from: grave, to: hand, delayMs: timesOf(400).T, durationMs: timesOf(400).L - timesOf(400).T });
      expect(cues[2], side).toEqual(expect.objectContaining({ kind: "burst", at: hand, preset: "sparkle" }));
      // The hand's own glint still plays: recall replaces no row, and nothing shakes.
      expect(motifReplaces(specOf("recall"), entry, view, envOf()).size, side).toBe(0);
      expect(cues.some((cue) => cue.kind === "shake"), side).toBe(false);
    }
  });
});

describe("R2797000 hex and transmute's fusion", () => {
  const at = (cue: FxCue | undefined): string => (cue !== undefined && "at" in cue && cue.at.kind === "testid" ? cue.at.testid : "");

  it("R2797000 hex rings a card marked and flares one unmarked", () => {
    const view = fullBoardView();
    const [first, second] = view.opponent.units.map((unit) => unit?.instanceId ?? "");
    const marks: GameEvent[] = [
      { type: "marked", instanceId: first ?? "", mark: "frozen", color: "cyan", added: true },
      { type: "marked", instanceId: second ?? "", mark: "allure", color: "pink", added: false },
      // R202: a mark on a card the viewer may not read lands nowhere.
      { type: "marked", instanceId: "hidden", mark: "frozen", color: "cyan", added: true },
    ];
    const cues = planMotif(specOf("hex"), { ...galleryEntry(marks, view), durationMs: 400 }, view, envOf());
    expect(cues.map((cue) => `${cue.kind}@${String(cue.delayMs)}:${at(cue)}`)).toEqual([
      `ring@0:${testid.card(first ?? "")}`,
      `sheen@80:${testid.card(first ?? "")}`,
      `burst@240:${testid.card(first ?? "")}`,
      `burst@240:${testid.card(second ?? "")}`,
      "shake@240:",
    ]);
    const lifted = cues[3];
    expect(lifted?.kind === "burst" && lifted.preset === "sparkle" && lifted.spread === "ring").toBe(true);
    // The brand row keeps the mark's own colors.
    expect(motifReplaces(specOf("hex"), { ...galleryEntry(marks, view), durationMs: 400 }, view, envOf()).size).toBe(0);
  });

  it("R2797000 transmute draws an Equip's fused at the host", () => {
    const view = fullBoardView();
    const host = view.you.units[1]?.instanceId ?? "";
    const fused: GameEvent = { type: "fused", instanceIds: [host, "fx-pick"], resultInstanceId: host, defId: "core-011" };
    const cues = planMotif(specOf("transmute"), { ...galleryEntry([fused], view), durationMs: 400 }, view, envOf());
    expect(beats(cues)).toEqual(PINNED.transmute);
    for (const cue of cues) if (cue.kind !== "shake") expect(at(cue)).toBe(testid.card(host));
  });
});
