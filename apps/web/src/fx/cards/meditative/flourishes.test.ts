// R2784000 #2796: Meditative's and Meditative+'s three Flourishes (fx/cards/meditative/,
// fx/cards/meditativeplus/), planned the way FxLayer plans them: remember, then plan. Each reads the
// resolving play, public ids and `hiddenFrom` alone (R202), starts inside its entry and ends within
// FX_MAX_TAIL_MS of it (R200), and plays nothing with the effects off.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid, planEntries, type AnimationEntry } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { planCardFx } from "../../cardFx.ts";
import {
  FX_CN_SWEEP_TRAUMA,
  FX_HARMONY_TRAUMA,
  FX_INTENSITY_SCALE,
  FX_JADE_CRACK_TRAUMA,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_MIND_JUDGE_TRAUMA,
} from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { createFxMemory } from "../../memory.ts";
import { MOTIF_TUNING, motifScale, timesOf } from "../../motifs/build.ts";
import { RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../../types.ts";
import { CARD_RECIPES } from "../bespoke.ts";
import { FX_CARD_TABLE } from "../tables.ts";

const ROCK = "meditative-039-1";
const ZHONGGUO = "meditative-032";
const SPIRIT = "meditativeplus-021";

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

/** Plans each entry of `events` in order, as FxLayer does on every `start`: remember, then plan. */
function planAll(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv, D?: number): { entry: AnimationEntry; cues: FxCue[] }[] {
  const planned = planEntries(events, withEvents(view, [...events]), false);
  return planned.map((original) => {
    const entry = D === undefined ? original : { ...original, durationMs: D };
    env.memory.remember(entry.events);
    return { entry, cues: planFx(entry, entry.view, env) };
  });
}

/** The Flourish's own cues of every entry (`planCardFx` on each event), as `planAll` plans them. */
function flourishCues(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf(), D?: number): FxCue[] {
  const planned = planEntries(events, withEvents(view, [...events]), false);
  return planned.flatMap((original) => {
    const entry = D === undefined ? original : { ...original, durationMs: D };
    env.memory.remember(entry.events);
    return entry.events.flatMap((event) => planCardFx(event, { entry, view: entry.view, env, D: entry.durationMs })?.cues ?? []);
  });
}

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

const tid = (value: string): FxAnchor => ({ kind: "testid", testid: value });
const at = (cue: FxCue): string | null => ("at" in cue && cue.at.kind === "testid" ? cue.at.testid : null);

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 0 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 0 });

/** Auspicious Rock played by `player`: its life paid, then the card it rolled reaching the hand (a hidden id on the other seat). */
function rockStream(player: PlayerId, rolled: string, view: PlayerView): GameEvent[] {
  const seen = player === view.viewer;
  return [
    played(player, "rock", ROCK),
    { type: "healthLost", player, amount: 2 },
    { type: "addedToHand", player, instanceId: seen ? "rolled" : "hidden", defId: seen ? rolled : "hidden" },
    resolved(player, "rock", ROCK),
  ];
}

/** Spiritually 中国 played by the viewer over its fixture: every hand card turned, and one CN Unit summoned. */
function sweepStream(view: PlayerView): GameEvent[] {
  return [played("p1", "cn", ZHONGGUO), ...RECIPE_FIXTURES.cnSweep(view, "cn", "p1"), resolved("p1", "cn", ZHONGGUO)];
}

/** Harmony Spirit's Cry from the viewer's first unit: every other unit set to 3/3, Attack then Health. */
function harmonyStream(view: PlayerView): { stream: GameEvent[]; spirit: string; others: string[] } {
  const spirit = view.you.units[0]?.instanceId ?? "";
  const set = RECIPE_FIXTURES.harmonyWave(view, spirit, "p1").filter((event) => event.type === "numberChanged" && event.instanceId !== spirit);
  const others = [...new Set(set.flatMap((event) => (event.type === "numberChanged" ? [event.instanceId] : [])))];
  return { stream: [played("p1", spirit, SPIRIT), ...set, resolved("p1", spirit, SPIRIT)], spirit, others };
}

describe("R2784000 #2796 Meditative's Flourishes", () => {
  it("R2784000 #2796 the three Flourishes are their cards' bespoke entries, each a card recipe", () => {
    expect(FX_CARD_TABLE.get(ROCK)).toEqual({ kind: "bespoke", key: "jadeCrack" });
    expect(FX_CARD_TABLE.get(ZHONGGUO)).toEqual({ kind: "bespoke", key: "cnSweep" });
    expect(FX_CARD_TABLE.get(SPIRIT)).toEqual({ kind: "bespoke", key: "harmonyWave" });
    for (const key of ["jadeCrack", "cnSweep", "harmonyWave"] as const) expect(typeof CARD_RECIPES[key]).toBe("function");
  });

  it("R2784000 R202 #2796 jadeCrack cracks the caster's hero and flies jade into their hand, the same whatever it rolled", () => {
    const view = fullBoardView();
    const D = 400;
    const t = timesOf(D);
    const jade = flourishCues(rockStream("p1", "meditative-039-2", view), view, envOf(), D);
    const red = flourishCues(rockStream("p1", "meditative-039-4", view), view, envOf(), D);
    expect(red).toEqual(jade);
    const hero = testid.hero("you");
    const hand = animTestid.hand("you");
    expect(jade).toContainEqual({ kind: "crack", at: tid(hero), delayMs: t.T, durationMs: expect.any(Number) });
    expect(jade.some((cue) => cue.kind === "burst" && cue.preset === "crumb" && at(cue) === hero && cue.delayMs === t.G)).toBe(true);
    expect(jade.some((cue) => cue.kind === "burst" && cue.spread === "area" && at(cue) === hero && cue.delayMs === t.L)).toBe(true);
    const shot = jade.find((cue) => cue.kind === "shot");
    expect(shot).toEqual(expect.objectContaining({ from: tid(hero), to: tid(hand), delayMs: t.T, flightMs: t.L - t.T }));
    expect(jade.some((cue) => cue.kind === "burst" && cue.preset === "sparkle" && at(cue) === hand && cue.delayMs === t.L)).toBe(true);
    const shake = jade.find((cue) => cue.kind === "shake");
    expect(shake?.kind === "shake" && shake.trauma).toBeCloseTo(FX_JADE_CRACK_TRAUMA * motifScale(ROCK, view, FX_INTENSITY_SCALE.normal));
    expect(JSON.stringify(jade)).not.toMatch(/meditative/);
    // The other seat's Rock: its hero and its hand, though the viewer cannot read what it rolled.
    const theirs = flourishCues(rockStream("p2", "meditative-039-2", view), view, envOf(), D);
    expect(theirs).toContainEqual({ kind: "crack", at: tid(testid.hero("opponent")), delayMs: t.T, durationMs: expect.any(Number) });
    expect(theirs.find((cue) => cue.kind === "shot")).toEqual(expect.objectContaining({ from: tid(testid.hero("opponent")), to: tid(animTestid.hand("opponent")) }));
    // Health another seat loses while the Rock resolves is not the Rock's.
    const other = flourishCues([played("p1", "rock", ROCK), { type: "healthLost", player: "p2", amount: 2 }, resolved("p1", "rock", ROCK)], view);
    expect(other).toEqual([]);
  });

  it("R2784000 R202 #2796 cnSweep sweeps the hand once, glints each readable card and rings each summon; a hidden hand glints no card", () => {
    const view = fullBoardView();
    const D = 400;
    const t = timesOf(D);
    const cues = flourishCues(sweepStream(view), view, envOf(), D);
    const hand = animTestid.hand("you");
    expect(cues.filter((cue) => cue.kind === "sheen" && at(cue) === hand)).toHaveLength(1);
    expect(cues.filter((cue) => cue.kind === "burst" && cue.power === MOTIF_TUNING.gather.power && at(cue) === hand)).toHaveLength(1);
    const cards = Array.isArray(view.you.hand) ? view.you.hand : [];
    expect(cards.length).toBeGreaterThan(1);
    for (const card of cards) {
      const glints = cues.filter((cue) => cue.kind === "burst" && cue.preset === "sparkle" && at(cue) === testid.handCard(card.instanceId));
      expect(glints, card.instanceId).toHaveLength(1);
      expect(glints[0]?.delayMs).toBe(t.L);
    }
    const zone = testid.zone("you", "units", 1);
    expect(cues.filter((cue) => cue.kind === "ring" && at(cue) === zone)).toHaveLength(1);
    const shake = cues.filter((cue) => cue.kind === "shake");
    expect(shake).toHaveLength(1);
    expect(shake[0]?.kind === "shake" && shake[0].trauma).toBeCloseTo(FX_CN_SWEEP_TRAUMA * motifScale(ZHONGGUO, view, FX_INTENSITY_SCALE.normal));
    expect(JSON.stringify(cues)).not.toMatch(/meditative/);
    // The other seat's hand: one sweep at its pile, and no glint, since every card is hidden from the viewer.
    const hidden: GameEvent[] = [
      played("p2", "cn", ZHONGGUO),
      ...[0, 1, 2].map((): GameEvent => ({ type: "transformed", instanceId: "hidden", fromDefId: "hidden", toDefId: "hidden", newInstanceId: "hidden", hiddenFrom: ["p1"] })),
      resolved("p2", "cn", ZHONGGUO),
    ];
    const theirs = flourishCues(hidden, view, envOf(), D);
    expect(theirs.filter((cue) => cue.kind === "sheen" && at(cue) === animTestid.hand("opponent"))).toHaveLength(1);
    expect(theirs.some((cue) => at(cue)?.startsWith("hand-card-") === true)).toBe(false);
  });

  it("R2784000 #2796 harmonyWave settles every unit once and rings from the Spirit on the board", () => {
    const view = fullBoardView();
    const D = 400;
    const t = timesOf(D);
    const { stream, spirit, others } = harmonyStream(view);
    expect(others.length).toBe(9);
    const cues = flourishCues(stream, view, envOf(), D);
    for (const id of others) {
      expect(cues.filter((cue) => cue.kind === "sheen" && at(cue) === testid.card(id)), id).toHaveLength(1);
      expect(cues.filter((cue) => cue.kind === "burst" && cue.preset === "sparkle" && at(cue) === testid.card(id)), id).toHaveLength(1);
    }
    const from = testid.card(spirit);
    expect(cues.filter((cue) => cue.kind === "ring" && at(cue) === from)).toEqual([expect.objectContaining({ delayMs: t.G })]);
    expect(cues.filter((cue) => cue.kind === "burst" && cue.spread === "ring" && at(cue) === from)).toHaveLength(1);
    const shake = cues.filter((cue) => cue.kind === "shake");
    expect(shake).toHaveLength(1);
    expect(shake[0]?.kind === "shake" && shake[0].trauma).toBeCloseTo(FX_HARMONY_TRAUMA * motifScale(SPIRIT, view, FX_INTENSITY_SCALE.normal));
    expect(JSON.stringify(cues)).not.toMatch(/meditative/);
    // A Spirit the board no longer shows rings nothing, and a number hidden from the viewer settles nothing.
    const gone = flourishCues([played("p1", "gone", SPIRIT), ...stream.slice(1, -1), resolved("p1", "gone", SPIRIT)], view, envOf(), D);
    expect(gone.some((cue) => cue.kind === "ring" || cue.kind === "shake")).toBe(false);
    expect(gone.filter((cue) => cue.kind === "sheen")).toHaveLength(9);
    const veiled = flourishCues(
      [played("p1", spirit, SPIRIT), { type: "numberChanged", instanceId: others[0] ?? "", defId: "hidden", key: "attack", value: 3, hiddenFrom: ["p1"] }, resolved("p1", spirit, SPIRIT)],
      view,
    );
    expect(veiled).toEqual([]);
  });

  it("R200 #2796 every Flourish cue starts inside its entry and ends within FX_MAX_TAIL_MS", () => {
    for (const D of [120, 200, 400, 732, 1400]) {
      const view = fullBoardView();
      const cues = [
        ...flourishCues(rockStream("p1", "meditative-039-2", view), view, envOf(), D),
        ...flourishCues(rockStream("p2", "meditative-039-2", view), view, envOf(), D),
        ...flourishCues(sweepStream(view), view, envOf(), D),
        ...flourishCues(harmonyStream(view).stream, view, envOf(), D),
      ];
      expect(cues.length).toBeGreaterThan(10);
      for (const cue of cues) {
        const label = `${cue.kind} at D=${String(D)}`;
        expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
        expect(cue.delayMs, label).toBeLessThanOrEqual(D);
        if (cue.kind === "shot" || cue.kind === "projectile") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
        expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
      }
    }
  });

  it("R200 #2796 intensity 0 draws no Flourish", () => {
    const view = fullBoardView();
    const streams = [rockStream("p1", "meditative-039-2", view), sweepStream(view), harmonyStream(view).stream];
    for (const stream of streams) {
      expect(planAll(stream, view, envOf({ intensity: 0 })).flatMap((result) => result.cues)).toEqual([]);
      expect(flourishCues(stream, view, envOf({ intensity: 0 }))).toEqual([]);
      // Reduced motion gives every entry a duration of 0.
      expect(flourishCues(stream, view, envOf(), 0)).toEqual([]);
    }
  });
});

const MIND = "meditative-022-1";

/** Fortify Mind played by `player`: its guess judged against its linked secret. */
function mindStream(player: PlayerId, view: PlayerView): GameEvent[] {
  return [played(player, "mind", MIND), ...RECIPE_FIXTURES.mindJudge(view, "mind", player), resolved(player, "mind", MIND)];
}

describe("R2803000 Fortify Mind's mindJudge", () => {
  it("R2803000 mindJudge is Fortify Mind's bespoke entry, a sheen and flare on the judging hero's badge", () => {
    expect(FX_CARD_TABLE.get(MIND)).toEqual({ kind: "bespoke", key: "mindJudge" });
    expect(typeof CARD_RECIPES.mindJudge).toBe("function");
    const view = fullBoardView();
    const D = 400;
    const t = timesOf(D);
    const cues = flourishCues(mindStream("p1", view), view, envOf(), D);
    const hero = testid.hero("you");
    expect(cues.filter((cue) => cue.kind === "sheen" && at(cue) === hero).length).toBeGreaterThan(0);
    expect(cues.some((cue) => cue.kind === "burst" && cue.preset === "arcane" && at(cue) === hero && cue.delayMs === t.L)).toBe(true);
    const shake = cues.filter((cue) => cue.kind === "shake");
    expect(shake.length).toBeGreaterThan(0);
    expect(shake[0]?.kind === "shake" && shake[0].trauma).toBeCloseTo(FX_MIND_JUDGE_TRAUMA * motifScale(MIND, view, FX_INTENSITY_SCALE.normal));
    expect(JSON.stringify(cues)).not.toMatch(/meditative/);
    // The other seat's judgment lands on its own hero's badge.
    const theirs = flourishCues(mindStream("p2", view), view, envOf(), D);
    expect(theirs.some((cue) => cue.kind === "sheen" && at(cue) === testid.hero("opponent"))).toBe(true);
  });

  it("R200 R2803000 every mindJudge cue starts inside its entry and ends within FX_MAX_TAIL_MS", () => {
    for (const D of [120, 200, 400, 732, 1400]) {
      const view = fullBoardView();
      const cues = flourishCues(mindStream("p1", view), view, envOf(), D);
      expect(cues.length).toBeGreaterThan(0);
      for (const cue of cues) {
        const label = `mindJudge ${cue.kind} at D=${String(D)}`;
        expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
        expect(cue.delayMs, label).toBeLessThanOrEqual(D);
        expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
      }
    }
  });

  it("R200 R2803000 intensity 0 draws no mindJudge", () => {
    const view = fullBoardView();
    expect(flourishCues(mindStream("p1", view), view, envOf({ intensity: 0 }))).toEqual([]);
    expect(flourishCues(mindStream("p1", view), view, envOf(), 0)).toEqual([]);
  });
});
