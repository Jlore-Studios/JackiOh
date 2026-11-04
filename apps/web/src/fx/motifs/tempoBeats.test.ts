// R2791000: the tempo keyword beats (fx/motifs/beats.ts) pinned on fixture streams, each held
// inside its entry (R200), scaled through it (R201), silent under reduced motion but for a still
// callout, and blind to what the view hides (R202). Planned the way FxLayer plans them: remember
// an entry's events, then plan it.

import type { CardView, GameEvent, Keyword, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { TEMPORARY_GLYPH } from "../../cards/cardState.ts";
import { KEYWORD_CALLOUT_ICONS } from "../../cards/keywordVisuals.ts";
import { animTestid, planEntries, type AnimationEntry } from "../../game/animations.ts";
import { testid } from "../../game/contract.ts";
import { card, fullBoardView, withEvents } from "../../test/fixtures.ts";
import { must } from "../../test/stubServer.ts";
import {
  FX_CALLOUT_MS,
  FX_CALLOUTS_PER_HIT,
  FX_CHARGE_LINE_SPREAD,
  FX_DEFT_FEINT_STEP,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_STACK_LAND,
  FX_ZONE_COUNT_AT,
} from "../constants.ts";
import { createFxMemory } from "../memory.ts";
import { extraDraws, rollBatchOf, rollLandAt } from "../rolls.ts";
import type { FxCardFacts, FxCue, FxPlanEnv } from "../types.ts";
import { FX_BEAT_KEYWORDS, planBeats, planStillCallouts } from "./beats.ts";

const VIEW = fullBoardView();
const id = (side: "you" | "opponent", index: number): string => must(VIEW[side].units[index], `${side} unit ${index}`).instanceId;

/** The viewer's lane-1 unit: Taunt only. */
const MINE = id("you", 0);
/** The viewer's lane-2 unit: every keyword. */
const POISONER = id("you", 1);
/** The opponent's lane-1 unit: Divine Shield only. */
const FOE = id("opponent", 0);
const FOE_2 = id("opponent", 1);

const envOf = (over: Partial<FxPlanEnv> = {}): FxPlanEnv => ({ intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over });

/** The board with `MINE`'s keywords replaced. */
function viewWith(keywords: readonly Keyword[]): PlayerView {
  const view = fullBoardView();
  const first = must(view.you.units[0], "the viewer's first unit");
  view.you.units[0] = { ...first, keywords: [...keywords] };
  return view;
}

/** The board with the viewer's hand replaced. */
function handView(hand: CardView[]): PlayerView {
  const view = fullBoardView();
  view.you.hand = hand;
  return view;
}

/** The board with one unit lane emptied. */
function openLane(view: PlayerView, lane: number): PlayerView {
  view.you.units[lane - 1] = null;
  return view;
}

const attack = (attackerId: string, targetId: string): GameEvent => ({ type: "attackDeclared", attackerId, targetId, forced: false });
const played = (player: "p1" | "p2", instanceId: string, defId: string, from?: "graveyard"): GameEvent => ({
  type: "cardPlayed",
  player,
  instanceId,
  defId,
  costPaid: 1,
  ...(from === undefined ? {} : { from }),
});
const summon = (player: "p1" | "p2", instanceId: string, defId: string, lane: number): GameEvent => ({
  type: "summoned",
  player,
  instanceId,
  defId,
  row: "units",
  lane,
});
const hitOf = (sourceId: string | null, targetId: string, amount: number): GameEvent => ({
  type: "damage",
  sourceId,
  targetId,
  amount,
  combat: false,
});

type Played = { entry: AnimationEntry; beats: ReturnType<typeof planBeats>; env: FxPlanEnv };

/** Each entry of `events` as FxLayer plans it on `start`: remember its events, then plan; `D` overrides every entry's duration. */
function stream(events: readonly GameEvent[], view: PlayerView = fullBoardView(), D?: number, env: FxPlanEnv = envOf()): Played[] {
  return planEntries(events, withEvents(view, [...events]), false).map((planned) => {
    const entry = D === undefined ? planned : { ...planned, durationMs: D };
    env.memory.remember(entry.events);
    return { entry, beats: planBeats(entry, entry.view, env), env };
  });
}

const kinds = (cues: readonly FxCue[]): string[] => cues.map((cue) => `${cue.kind}@${String(cue.delayMs)}`);

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

/** R200: every cue starts inside the entry, a flight lands inside it and no tail outlives FX_MAX_TAIL_MS. */
function expectInside(cues: readonly FxCue[], D: number, label: string): void {
  for (const cue of cues) {
    const name = `${label} ${cue.kind} at D=${String(D)}`;
    expect(cue.delayMs, name).toBeGreaterThanOrEqual(0);
    expect(cue.delayMs, name).toBeLessThanOrEqual(D);
    if (cue.kind === "shot" || cue.kind === "projectile") expect(cue.delayMs + cue.flightMs, name).toBeLessThanOrEqual(D);
    expect(endOf(cue), name).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
    if (cue.kind === "shake") expect(cue.trauma).toBeLessThanOrEqual(1);
  }
}

/** The beats of the entry `at` of `events`, pinned at its table duration and held inside it at three others. */
function pinned(events: readonly GameEvent[], at: number, view: PlayerView = fullBoardView(), card: FxPlanEnv["card"] = () => undefined): FxCue[] {
  const fresh = (): FxPlanEnv => envOf({ card });
  const own = must(stream(events, view, undefined, fresh())[at], "the entry");
  for (const D of [120, 300, 1200]) {
    const cues = must(stream(events, view, D, fresh())[at], "the entry").beats.cues;
    expectInside(cues, D, `entry ${String(at)}`);
  }
  return own.beats.cues;
}

const cardOf = (instanceId: string): { kind: "testid"; testid: string } => ({ kind: "testid", testid: testid.card(instanceId) });
const spellCard = (defId: string): FxCardFacts | undefined => (defId === "core-005" ? { type: "Spell" } : undefined);

describe("R2791000 the tempo keyword beats", () => {
  it("R2791000 Charge lands with speed lines and a lean at the enemy hero, then dashes on its first attack", () => {
    const view = openLane(handView([card({ instanceId: "hc", defId: "core-045" })]), 5);
    const events = [played("p1", "hc", "core-045"), summon("p1", "hc", "core-045", 5)];
    // cardPlayed (400) and its summoned collapse into one entry: three speed-line shots, a
    // landing, the lean at the enemy hero and its shake.
    expect(kinds(pinned(events, 0, view))).toEqual(["callout@0", "shot@0", "shot@0", "shot@0", "burst@240", "shot@240", "shake@300"]);
    const cues = must(stream(events, view)[0], "the landing").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Charge", at: { kind: "testid", testid: testid.zone("you", "units", 5) } });
    expect(cues[1]).toMatchObject({ kind: "shot", from: { kind: "testid", testid: "hand-you" } });
    expect(cues[5]).toMatchObject({
      kind: "shot",
      from: { kind: "testid", testid: testid.zone("you", "units", 5) },
      to: { kind: "testid", testid: testid.hero("opponent") },
    });
    // The speed lines leave spread across the hand region.
    const froms = cues.slice(1, 4).map((cue) => (cue.kind === "shot" && cue.from.kind === "testid" ? cue.from.at?.x : null));
    expect(froms[0]).toBeUndefined();
    expect(froms[1]).toBeCloseTo(0.5 - FX_CHARGE_LINE_SPREAD, 10);
    expect(froms[2]).toBeCloseTo(0.5 + FX_CHARGE_LINE_SPREAD, 10);
    // The first attack dashes; the second does not.
    const charger = viewWith([{ kind: "Charge" }]);
    const attacks = [attack(MINE, FOE), attack(MINE, FOE_2)];
    expect(kinds(pinned(attacks, 0, charger))).toEqual(["callout@0", "shot@0", "burst@210", "shake@210"]);
    expect(must(stream(attacks, charger)[0], "the first attack").beats.cues[1]).toMatchObject({
      kind: "shot",
      from: cardOf(MINE),
      to: cardOf(FOE),
    });
    expect(must(stream(attacks, charger)[1], "the second attack").beats.cues).toEqual([]);
    // The other seat's Charge landing plans nothing: its hand is a count.
    const foeView = fullBoardView();
    foeView.opponent.units[4] = null;
    const foe = [played("p2", "hc", "core-045"), summon("p2", "hc", "core-045", 5)];
    expect(stream(foe, foeView).flatMap(({ beats }) => beats.cues)).toEqual([]);
  });

  it("R2791000 Rush lands leaning at an enemy unit, never the hero, and dashes only on its first attack", () => {
    const view = openLane(handView([card({ instanceId: "hr", defId: "core-011" })]), 5);
    const events = [played("p1", "hr", "core-011"), summon("p1", "hr", "core-011", 5)];
    expect(kinds(pinned(events, 0, view))).toEqual(["callout@0", "shot@0", "shot@0", "shot@0", "burst@240", "shot@240", "shake@300"]);
    const cues = must(stream(events, view)[0], "the landing").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Rush" });
    expect(cues[5]).toMatchObject({ kind: "shot", to: cardOf(FOE) });
    expect(JSON.stringify(cues)).not.toContain(testid.hero("opponent"));
    // With no enemy unit shown, the landing skips its lean but keeps its callout.
    const bare = openLane(handView([card({ instanceId: "hr", defId: "core-011" })]), 5);
    bare.opponent.units = [null, null, null, null, null];
    expect(kinds(pinned(events, 0, bare))).toEqual(["callout@0", "shot@0", "shot@0", "shot@0", "burst@240"]);
    // The first attack dashes; the second does not.
    const rusher = viewWith([{ kind: "Rush" }]);
    const attacks = [attack(MINE, FOE), attack(MINE, FOE_2)];
    expect(kinds(pinned(attacks, 0, rusher))).toEqual(["callout@0", "shot@0", "burst@210", "shake@210"]);
    expect(must(stream(attacks, rusher)[1], "the second attack").beats.cues).toEqual([]);
  });

  it("R2791000 Equip flies onto its host and snaps on; a fresh fusion plans nothing, the other seat off regions", () => {
    const view = handView([card({ instanceId: "eq", defId: "meditative-065" })]);
    const fused: GameEvent = { type: "fused", instanceIds: [MINE, "eq"], resultInstanceId: MINE, defId: "meditative-065" };
    // D=500: the ghost lands with the ring and the snap burst.
    expect(kinds(pinned([fused], 0, view))).toEqual(["callout@0", "ghost@0", "ring@300", "burst@300"]);
    const cues = must(stream([fused], view)[0], "the fusion").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Equip", at: cardOf(MINE) });
    expect(cues[1]).toMatchObject({
      kind: "ghost",
      from: { kind: "testid", testid: "hand-you" },
      to: cardOf(MINE),
    });
    expect(cues[2]).toMatchObject({ kind: "ring", preset: "gold", at: cardOf(MINE) });
    // A fusion with a fresh result is no equipment landing.
    const fresh: GameEvent = { type: "fused", instanceIds: [MINE, POISONER], resultInstanceId: "c91", defId: "core-088" };
    expect(stream([fresh], view).flatMap(({ beats }) => beats.cues)).toEqual([]);
    // A fusion from a named hand card with no Equip plans nothing either.
    const plain = handView([card({ instanceId: "eq", defId: "core-002" })]);
    expect(stream([fused], plain).flatMap(({ beats }) => beats.cues)).toEqual([]);
    // On the other seat the hidden top still lands, off the hand region, naming no card.
    const foe: GameEvent = { type: "fused", instanceIds: [FOE, "eq9"], resultInstanceId: FOE, defId: "meditative-065" };
    const foeCues = must(stream([foe])[0], "the fusion").beats.cues;
    expect(kinds(foeCues)).toEqual(["callout@0", "ghost@0", "ring@300", "burst@300"]);
    expect(foeCues[1]).toMatchObject({
      kind: "ghost",
      from: { kind: "testid", testid: "hand-opponent" },
      to: cardOf(FOE),
    });
    expect(JSON.stringify(foeCues)).not.toContain("hand-card-");
  });

  it("R2791000 Tradable swirls the card into the Deck and the drawn card out; alone each plans nothing, the other seat names no card", () => {
    const traded: GameEvent = { type: "traded", player: "p1" };
    expect(kinds(pinned([traded], 0))).toEqual(["callout@0", "ghost@0"]);
    const out = must(stream([traded])[0], "the trade").beats.cues;
    expect(out[0]).toMatchObject({ kind: "callout", keyword: "Tradable", at: { kind: "testid", testid: "library-you" } });
    expect(out[1]).toMatchObject({
      kind: "ghost",
      from: { kind: "testid", testid: "hand-you" },
      to: { kind: "testid", testid: "library-you" },
    });
    // The trade's shuffle and draw answer hand-side; either alone plans nothing.
    const shuffled: GameEvent = { type: "shuffledIn", player: "p1", instanceId: "cW", defId: "core-070", position: 3 };
    const drawn: GameEvent = { type: "drawn", player: "p1", instanceId: "cY", defId: "core-055" };
    expect(stream([shuffled]).flatMap(({ beats }) => beats.cues)).toEqual([]);
    expect(stream([drawn]).flatMap(({ beats }) => beats.cues)).toEqual([]);
    const flow = [traded, shuffled, drawn];
    expect(kinds(pinned(flow, 2))).toEqual(["callout@0", "burst@150"]);
    const back = must(stream(flow)[2], "the draw").beats.cues;
    expect(back[1]).toMatchObject({ kind: "burst", preset: "sparkle", at: { kind: "testid", testid: "hand-you" } });
    // Only the trade's own first draw answers: a later unrelated draw for the same player,
    // with the trade's own draw between, plans nothing — even across a turn boundary.
    const later: GameEvent = { type: "drawn", player: "p1", instanceId: "cZ", defId: "core-055" };
    expect(must(stream([...flow, later])[3], "the later draw").beats.cues).toEqual([]);
    const nextTurn: GameEvent[] = [...flow, { type: "turnStarted", player: "p2", turn: 4 }, later];
    expect(must(stream(nextTurn)[4], "the next-turn draw").beats.cues).toEqual([]);
    // A draw after a trade with no shuffle between answers nothing either.
    expect(must(stream([traded, drawn])[1], "the unshuffled draw").beats.cues).toEqual([]);
    // The other seat's trade swirls region to region and names no card.
    const foeOut = must(stream([{ type: "traded", player: "p2" }])[0], "the trade").beats.cues;
    expect(foeOut[1]).toMatchObject({
      kind: "ghost",
      from: { kind: "testid", testid: "hand-opponent" },
      to: { kind: "testid", testid: "library-opponent" },
    });
    expect(JSON.stringify(foeOut)).not.toMatch(/card-|hand-card-/);
  });

  it("R2791000 Stack lands on the pile with a stacked offset and a ×3 pulse; an empty lane plans nothing", () => {
    const view = fullBoardView();
    const occupant = must(view.you.units[0], "the pile top");
    view.you.units[0] = { ...occupant, buried: 1 };
    const arrival: GameEvent = summon("p1", "new1", "core-092", 1);
    expect(kinds(pinned([arrival], 0, view))).toEqual(["callout@0", "burst@150", "zone@88"]);
    const cues = must(stream([arrival], view)[0], "the landing").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Stack", at: cardOf(MINE) });
    expect(cues[1]).toMatchObject({ kind: "burst", preset: "gold", at: { kind: "testid", testid: testid.card(MINE), at: { ...FX_STACK_LAND } } });
    expect(cues[2]).toMatchObject({ kind: "zone", text: "×3", direction: "up", at: cardOf(MINE) });
    expect(cues[2]).toMatchObject({ delayMs: Math.round(FX_ZONE_COUNT_AT * 250) });
    const open = openLane(fullBoardView(), 5);
    expect(stream([summon("p1", "new1", "core-092", 5)], open).flatMap(({ beats }) => beats.cues)).toEqual([]);
  });

  it("R2791000 Spell Damage burns hotter with a +N sigil matching the view total, once per resolution", () => {
    const cast = played("p1", "s1", "core-005");
    const events = [cast, hitOf("s1", FOE, 3)];
    // POISONER's view total is Spell Damage 1: the burst lands on the target, the sigil on the
    // resolving hero, since the Spell itself stands in no zone.
    expect(kinds(pinned(events, 1, fullBoardView(), spellCard))).toEqual(["callout@0", "burst@180", "zone@180"]);
    const env = envOf({ card: spellCard });
    const cues = must(stream(events, fullBoardView(), undefined, env)[1], "the hit").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Spell Damage", at: { kind: "testid", testid: testid.hero("you") } });
    expect(cues[1]).toMatchObject({ kind: "burst", at: cardOf(FOE) });
    expect(cues[2]).toMatchObject({ kind: "zone", text: "+1", direction: "up", at: { kind: "testid", testid: testid.hero("you") } });
    // A second source makes it +2.
    const doubled = fullBoardView();
    const first = must(doubled.you.units[0], "the second source");
    doubled.you.units[0] = { ...first, keywords: [{ kind: "Spell Damage", n: 1 }] };
    const two = must(stream(events, doubled, undefined, envOf({ card: spellCard }))[1], "the hit").beats.cues;
    expect(two[2]).toMatchObject({ kind: "zone", text: "+2" });
    // Nothing for a non-Spell resolution, and no sigil on the resolution's second hit.
    const unitEnv = envOf({ card: () => ({ type: "Unit" }) });
    expect(stream(events, fullBoardView(), undefined, unitEnv)[1]?.beats.cues).toEqual([]);
    const twice = [cast, hitOf("s1", FOE, 3), hitOf("s1", FOE_2, 2)];
    expect(must(stream(twice, fullBoardView(), undefined, envOf({ card: spellCard }))[2], "the second hit").beats.cues).toEqual([]);
    // On the other seat the sigil sits at its hero.
    const foeView = fullBoardView();
    const foeUnit = must(foeView.opponent.units[0], "the foe source");
    foeView.opponent.units[0] = { ...foeUnit, keywords: [{ kind: "Spell Damage", n: 1 }] };
    const foeCast = played("p2", "s9", "core-005");
    const foeHit = hitOf("s9", MINE, 3);
    const foeCues = must(stream([foeCast, foeHit], foeView, undefined, envOf({ card: spellCard }))[1], "the hit").beats.cues;
    expect(kinds(foeCues)).toEqual(["callout@0", "burst@180", "zone@180"]);
    expect(foeCues[2]).toMatchObject({ kind: "zone", text: "+1", at: { kind: "testid", testid: testid.hero("opponent") } });
  });

  it("R2791000 Lucky glints over a batch's extra rolls and adds nothing that pauses the panel longer", () => {
    const rolled: Extract<GameEvent, { type: "rolled" }> = {
      type: "rolled",
      player: "p1",
      instanceId: POISONER,
      defId: "core-011",
      kind: "die",
      sides: 6,
      rolls: [
        { faces: [2, 5], kept: 1 },
        { faces: [4], kept: 0 },
      ],
    };
    const batch = rollBatchOf(rolled);
    expect(batch === null ? 0 : extraDraws(batch)).toBe(1);
    // D=1200: the glint lands with the extra roll at rollLandAt(0, 2).
    expect(kinds(pinned([rolled], 0))).toEqual(["callout@0", "burst@600", "ring@600"]);
    const cues = must(stream([rolled])[0], "the batch").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Lucky" });
    expect(cues[1]).toMatchObject({ kind: "burst", preset: "leaf", delayMs: Math.round(rollLandAt(0, 2) * 1200) });
    expect(cues[2]).toMatchObject({ kind: "ring", preset: "gold", delayMs: Math.round(rollLandAt(0, 2) * 1200) });
    // Every glint lands by the last item's landing and ends with the panel, so the overlay's own
    // suspense is the whole pause: the kept face stays the panel's highlight.
    const last = Math.round(rollLandAt(1, 2) * 1200);
    for (const cue of cues) {
      expect(cue.delayMs).toBeLessThanOrEqual(last);
      expect(endOf(cue)).toBeLessThanOrEqual(1200 + 600);
    }
    // A batch with no extras plans nothing.
    const plain: Extract<GameEvent, { type: "rolled" }> = { ...rolled, rolls: [{ faces: [4], kept: 0 }] };
    expect(stream([plain]).flatMap(({ beats }) => beats.cues)).toEqual([]);
  });

  it("R2791000 Deft feints after a same-turn switch; without one, or across a turn, nothing", () => {
    const deft = viewWith([{ kind: "Deft" }]);
    const switched: GameEvent = { type: "positionSwitched", instanceId: MINE, position: "DEF" };
    const events = [switched, attack(MINE, FOE)];
    // D=350: out to the sidestep by Travel, back by Land, under a sheen.
    expect(kinds(pinned(events, 1, deft))).toEqual(["callout@0", "shot@0", "shot@70", "sheen@0"]);
    const cues = must(stream(events, deft)[1], "the attack").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Deft", at: cardOf(MINE) });
    expect(cues[1]).toMatchObject({ kind: "shot", from: cardOf(MINE) });
    const feint = cues[1]?.kind === "shot" && cues[1].to.kind === "testid" ? cues[1].to.at : undefined;
    expect(feint?.x).toBeCloseTo(0.5 + FX_DEFT_FEINT_STEP, 10);
    expect(cues[2]).toMatchObject({ kind: "shot", to: cardOf(MINE) });
    expect(stream([attack(MINE, FOE)], deft).flatMap(({ beats }) => beats.cues)).toEqual([]);
    const nextTurn: GameEvent[] = [switched, { type: "turnStarted", player: "p2", turn: 4 }, attack(MINE, FOE)];
    expect(stream(nextTurn, deft).flatMap(({ beats }) => beats.cues)).toEqual([]);
  });

  it("R2791000 Flashback rises from the graveyard pile, and dissolves in exile with its history", () => {
    const rise = played("p1", "fb1", "stinkyscape-021", "graveyard");
    // D=400: the ghost frame climbs as the sheen and the Dark burst land.
    expect(kinds(pinned([rise], 0))).toEqual(["callout@0", "ghost@0", "sheen@0", "burst@240"]);
    const cues = must(stream([rise])[0], "the play").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Flashback", at: { kind: "testid", testid: "graveyard-you" } });
    expect(cues[1]).toMatchObject({
      kind: "ghost",
      from: { kind: "testid", testid: "graveyard-you" },
      to: { kind: "testid", testid: "graveyard-you", at: { x: 0.5, y: 0 } },
    });
    expect(cues[3]).toMatchObject({ kind: "burst", preset: "void", at: { kind: "testid", testid: "graveyard-you" } });
    // A play from hand rises nowhere.
    expect(stream([played("p1", "fb1", "stinkyscape-021")]).flatMap(({ beats }) => beats.cues)).toEqual([]);
    // The exile dissolves with its history, and plans nothing without it.
    const exiled: GameEvent = { type: "exiled", instanceId: "fb1", defId: "stinkyscape-021", owner: "p1" };
    expect(kinds(pinned([rise, exiled], 1))).toEqual(["callout@0", "burst@210", "ring@210"]);
    const gone = must(stream([rise, exiled])[1], "the exile").beats.cues;
    expect(gone[0]).toMatchObject({ kind: "callout", keyword: "Flashback", at: { kind: "testid", testid: animTestid.exile("you") } });
    expect(gone[1]).toMatchObject({ kind: "burst", preset: "smoke" });
    expect(gone[2]).toMatchObject({ kind: "ring", preset: "void" });
    expect(stream([exiled]).flatMap(({ beats }) => beats.cues)).toEqual([]);
  });

  it("R2791000 Temporary dissolves with its hourglass count, delta or printed, never on the other seat", () => {
    const delta = handView([card({ instanceId: "t1", defId: "core-002", keywords: [{ kind: "Temporary", n: 2 }] })]);
    const discarded: GameEvent = { type: "discarded", instanceId: "t1", defId: "core-002", owner: "p1" };
    // D=300: the count over the hand card, in smoke and embers.
    expect(kinds(pinned([discarded], 0, delta))).toEqual(["callout@0", "zone@0", "burst@0", "burst@60"]);
    const cues = must(stream([discarded], delta)[0], "the discard").beats.cues;
    expect(cues[0]).toMatchObject({ kind: "callout", keyword: "Temporary", at: { kind: "testid", testid: "hand-card-t1" } });
    expect(cues[1]).toMatchObject({ kind: "zone", text: `${TEMPORARY_GLYPH}2` });
    // A printed Temporary reads its face: a plain one counts 1.
    const printed = handView([card({ instanceId: "t2", defId: "meditative-022-1" })]);
    const gone: GameEvent = { type: "discarded", instanceId: "t2", defId: "meditative-022-1", owner: "p1" };
    const face = must(stream([gone], printed)[0], "the discard").beats.cues;
    expect(face[1]).toMatchObject({ kind: "zone", text: `${TEMPORARY_GLYPH}1` });
    // A hidden hand card on the other seat plans nothing.
    const hidden: GameEvent = { type: "discarded", instanceId: "hidden", defId: "hidden", owner: "p2" };
    expect(stream([hidden]).flatMap(({ beats }) => beats.cues)).toEqual([]);
  });

  it("R201 R2791000 speed scales every tempo beat: the same cues at 0.5, 1 and 2, each within 4 ms of its scaled time", () => {
    const stackView = fullBoardView();
    const top = must(stackView.you.units[0], "the pile top");
    stackView.you.units[0] = { ...top, buried: 1 };
    const equipView = handView([card({ instanceId: "eq", defId: "meditative-065" })]);
    const cases: { events: GameEvent[]; at: number; D: number; view: PlayerView; card?: FxPlanEnv["card"] }[] = [
      { events: [summon("p1", "new1", "core-092", 1)], at: 0, D: 250, view: stackView },
      {
        events: [{ type: "rolled", player: "p1", instanceId: POISONER, defId: "core-011", kind: "die", sides: 6, rolls: [{ faces: [2, 5], kept: 1 }] }],
        at: 0,
        D: 1200,
        view: fullBoardView(),
      },
      {
        events: [{ type: "fused", instanceIds: [MINE, "eq"], resultInstanceId: MINE, defId: "meditative-065" }],
        at: 0,
        D: 500,
        view: equipView,
      },
    ];
    for (const { events, at, D, view, card: facts } of cases) {
      const fresh = (): FxPlanEnv => envOf({ card: facts ?? (() => undefined) });
      const base = must(stream(events, view, D, fresh())[at], "the entry").beats.cues;
      expect(base.length).toBeGreaterThan(0);
      for (const speed of [0.5, 2]) {
        const cues = must(stream(events, view, D / speed, fresh())[at], "the entry").beats.cues;
        expect(cues.map((cue) => cue.kind)).toEqual(base.map((cue) => cue.kind));
        cues.forEach((cue, k) => {
          expect(Math.abs(cue.delayMs - (base[k]?.delayMs ?? 0) / speed), `${cue.kind} at speed ${String(speed)}`).toBeLessThanOrEqual(4);
        });
      }
    }
  });

  it("R2791000 reduced motion keeps only still callouts: no motion cue, and no beat on an entry of no duration", () => {
    const view = openLane(handView([card({ instanceId: "hc", defId: "core-045" })]), 5);
    const events = [played("p1", "hc", "core-045"), summon("p1", "hc", "core-045", 5)];
    const playedEntry = must(stream(events, view)[0], "the landing");
    const still = { ...playedEntry.entry, durationMs: 0 };
    expect(planBeats(still, still.view, playedEntry.env)).toEqual({ cues: [], replaces: new Set() });
    const callouts = planStillCallouts(still, still.view, playedEntry.env);
    expect(callouts).toHaveLength(1);
    expect(callouts.every((cue) => cue.kind === "callout" && cue.still === true && cue.delayMs === 0 && cue.durationMs === FX_CALLOUT_MS)).toBe(true);
    expect(callouts.map((cue) => (cue.kind === "callout" ? cue.keyword : ""))).toEqual(["Charge"]);
    for (const cue of callouts) expectInside([cue], 0, "a still callout");
    // A summon with no keyword draws none.
    const plain = must(stream([summon("p1", "new9", "core-002", 5)], openLane(fullBoardView(), 5))[0], "the landing");
    expect(planStillCallouts(plain.entry, plain.entry.view, plain.env)).toEqual([]);
  });

  it("R2791000 intensity off draws nothing, still callouts included", () => {
    const view = openLane(handView([card({ instanceId: "hc", defId: "core-045" })]), 5);
    const events = [played("p1", "hc", "core-045"), summon("p1", "hc", "core-045", 5)];
    const env = envOf({ intensity: FX_INTENSITY_SCALE.off });
    for (const { entry } of stream(events, view, undefined, env)) {
      expect(planBeats(entry, entry.view, env)).toEqual({ cues: [], replaces: new Set() });
      expect(planStillCallouts(entry, entry.view, env)).toEqual([]);
    }
  });

  it("R2791000 every tempo keyword has a callout icon", () => {
    const tempo = ["Charge", "Rush", "Equip", "Tradable", "Stack", "Spell Damage", "Lucky", "Deft", "Flashback", "Temporary"] as const;
    for (const keyword of tempo) {
      expect(FX_BEAT_KEYWORDS, keyword).toContain(keyword);
      expect(KEYWORD_CALLOUT_ICONS[keyword]?.d, keyword).toMatch(/^M/);
    }
    expect(FX_BEAT_KEYWORDS).toHaveLength(19);
    expect(FX_CALLOUTS_PER_HIT).toBe(2);
  });
});
