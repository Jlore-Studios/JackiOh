// R3407000: what the signature passes' tests share (fx/cards/stinkyscape/*.test.ts): a run of events planned
// the way the board's effects layer plans it (entered, held as the queue holds it at speed 1 with Big
// moments full, remembered, then planned), each entry's cues at its offset from the run's start, a cue's
// end as R200 counts it, each card's signature ceiling, and R200 held over every entry.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { expect } from "vitest";

import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type AnimationEntry, type EntranceResolver } from "../game/animations.ts";
import {
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  SIGNATURE_LEGENDARY_MAX_MS,
  SIGNATURE_MYTHIC_MAX_MS,
  SIGNATURE_SPELL_SHARE,
} from "../fx/constants.ts";
import { planCardFx } from "../fx/cardFx.ts";
import type { CardFxResult } from "../fx/cards/recipe.ts";
import { FX_CARD_TABLE } from "../fx/cards/tables.ts";
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import { galleryBoardOf, galleryCardEvents, galleryEntry, galleryFieldView, galleryPlayOf } from "../fx/motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../fx/setPieces.ts";
import type { FxCue, FxPlanEnv } from "../fx/types.ts";
import { CATALOG } from "../wire/cards.ts";
import { fullBoardView, withEvents } from "./fixtures.ts";

export function envOf(intensity: number = FX_INTENSITY_SCALE.normal): FxPlanEnv {
  return { intensity, card: () => undefined, memory: createFxMemory() };
}

/** As the board resolves an entrance: the card's type and rarity, a token's the one it prints (B2.5). */
export const resolveEntrance: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  if (def === undefined) return null;
  return { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

/** A cue's end, as R200 counts it: bursts burn, shots fly then burst, shakes ride, the rest run out. */
export function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shot":
    case "projectile":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

/** One entry as planned, with its offset from the run's start. */
export type PlannedEntry = { entry: AnimationEntry; offset: number; cues: FxCue[] };

/**
 * R3407000: `events` over `view` as the board plans them: each entry entered, held (a Mythic's takeover, a
 * wipe's set piece), remembered and planned in turn, with its offset.
 */
export function planWithOffsets(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf()): PlannedEntry[] {
  const shown = withEvents(view, [...events]);
  const entered = entranceEntries(planEntries(events, shown, false, undefined, hasSetPiece), resolveEntrance, { seen: new Set() });
  const held = setPieceEntries(entered, (entry) => wipeClaim(entry) ?? takeoverClaim(entry), { reduced: false, bigMoments: "full", speed: 1 });
  let offset = 0;
  return held.map((entry) => {
    env.memory.remember(entry.events);
    const planned = { entry, offset, cues: planFx(entry, entry.view, env) };
    offset += entry.durationMs;
    return planned;
  });
}

/** Every cue of a planned run. */
export function cuesOfRun(run: readonly PlannedEntry[]): FxCue[] {
  return run.flatMap((planned) => planned.cues);
}

/** The latest end of any cue of `run` from its start; a pull and a conceal are the stage's (R200). */
export function latestEnd(run: readonly PlannedEntry[]): number {
  return Math.max(0, ...run.flatMap(({ offset, cues }) => cues.filter((cue) => cue.kind !== "pull" && cue.kind !== "conceal").map((cue) => offset + endOf(cue))));
}

/** R3332000: `defId`'s signature ceiling: its rarity's (a token's printed one), a Spell's SIGNATURE_SPELL_SHARE of it. */
export function ceilingOf(defId: string): number {
  const def = CATALOG[defId];
  const rarity = def?.printedRarity ?? def?.rarity;
  return (rarity === "Mythic" ? SIGNATURE_MYTHIC_MAX_MS : SIGNATURE_LEGENDARY_MAX_MS) * (def?.type === "Spell" ? SIGNATURE_SPELL_SHARE : 1);
}

/** R200: every cue starts inside its entry and is gone within FX_MAX_TAIL_MS of its end. */
export function expectR200(run: readonly PlannedEntry[], where: string): void {
  for (const { entry, cues } of run) {
    for (const cue of cues) {
      expect(cue.delayMs, `${where} ${cue.kind}`).toBeGreaterThanOrEqual(0);
      expect(cue.delayMs, `${where} ${cue.kind}`).toBeLessThanOrEqual(entry.durationMs);
      if (cue.kind === "pull" || cue.kind === "conceal") continue;
      expect(endOf(cue), `${where} ${cue.kind}`).toBeLessThanOrEqual(entry.durationMs + FX_MAX_TAIL_MS);
    }
  }
}

/**
 * R3407000: `defId`'s gallery play over its own board, as tiers.test.ts plans it (its play, its landing, its
 * recipe's fixture and its resolve; held as the queue holds it at speed 1 with Big moments full): the run
 * and the events it planned.
 */
export function galleryRun(defId: string, env: FxPlanEnv = envOf(), view: PlayerView = fullBoardView()): { run: PlannedEntry[]; events: GameEvent[] } {
  const entry = FX_CARD_TABLE.get(defId);
  const play = entry === undefined ? null : galleryPlayOf(defId, entry);
  if (play === null) throw new Error(`${defId} has no gallery play`);
  const board = galleryBoardOf(defId, view);
  const events = galleryCardEvents(defId, play, board, 1);
  return { run: planWithOffsets(events, galleryFieldView(defId, board), env), events };
}

/** R3332000, R200: `defId`'s gallery run draws, ends within its signature ceiling and keeps R200 in every entry. */
export function expectGalleryWithinCeiling(defId: string): void {
  const { run } = galleryRun(defId);
  expect(cuesOfRun(run).length, `${defId} draws`).toBeGreaterThan(0);
  expect(latestEnd(run), `${defId} within its ceiling`).toBeLessThanOrEqual(ceilingOf(defId));
  expectR200(run, defId);
}

/** A run's entries as lines, for a failing test's message: each entry's offset, D, events and cues. */
export function describeRun(run: readonly PlannedEntry[]): string {
  return run
    .map(({ entry, offset, cues }) => `@${String(offset)} D${String(entry.durationMs)} [${entry.events.map((e) => e.type).join(",")}] ${cues.map((c) => `${c.kind}@${String(c.delayMs)}~${String(endOf(c))}`).join(" ")}`)
    .join("\n");
}

/** R202: a card id of any set; no cue may carry one. */
export const CARD_ID = /(core|classic|meditative|stinkyscape|traininggrounds)[a-z]*-\d/;

/** R202: no cue of `cues` names a card or reaches into a hand but through its pile. */
export function expectNoCard(cues: readonly FxCue[], where: string): void {
  for (const cue of cues) {
    const json = JSON.stringify(cue);
    expect(json, `${where} ${cue.kind}`).not.toMatch(CARD_ID);
    expect(json, `${where} ${cue.kind}`).not.toContain('"handCard"');
  }
}

/**
 * R3407000: each of `events` as its own entry lasting D, `before` remembered first (the play's
 * `cardPlayed` or its hook's marker): what the card's recipe answers for it, null where it draws nothing.
 */
export function beatsOf(events: readonly GameEvent[], view: PlayerView, D: number, before: readonly GameEvent[] = [], env: FxPlanEnv = envOf()): (CardFxResult | null)[] {
  env.memory.remember(before);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    return planCardFx(event, { entry, view, env, D });
  });
}

/** R200: every cue of an entry lasting D starts inside it and ends within FX_MAX_TAIL_MS of its end; each shake within `maxTrauma`. */
export function expectCuesR200(cues: readonly FxCue[], D: number, where: string, maxTrauma = Infinity): void {
  for (const cue of cues) {
    const label = `${where} ${cue.kind} at D=${String(D)}`;
    expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
    expect(cue.delayMs, label).toBeLessThanOrEqual(D);
    if (cue.kind === "pull" || cue.kind === "conceal") continue;
    expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
    if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(maxTrauma);
  }
}

const SEATS: Readonly<Record<PlayerId, PlayerId>> = { p1: "p2", p2: "p1" };
const SEAT_KEYS = ["player", "controller", "owner", "creator"] as const;

/**
 * R202: `event` as the other seat's: its seat fields swapped, and as the server sends it to the viewer
 * (`view_for.rs` `redact_event`): an added, drawn or shuffled card's ids hidden; a prompt is never sent
 * to the other seat (`undefined`).
 */
export function asTheirs(event: GameEvent): GameEvent | undefined {
  if (event.type === "promptOpened") return undefined;
  const swapped: Record<string, unknown> = { ...event };
  for (const key of SEAT_KEYS) {
    const seat = swapped[key];
    if (seat === "p1" || seat === "p2") swapped[key] = SEATS[seat];
  }
  if (event.type === "addedToHand" || event.type === "drawn" || event.type === "shuffledIn") {
    swapped.instanceId = "hidden";
    swapped.defId = "hidden";
  }
  return swapped as GameEvent;
}

/** R202: `events` as the other seat receives them, each `asTheirs`. */
export function asTheirsAll(events: readonly GameEvent[]): GameEvent[] {
  return events.flatMap((event) => {
    const sent = asTheirs(event);
    return sent === undefined ? [] : [sent];
  });
}
