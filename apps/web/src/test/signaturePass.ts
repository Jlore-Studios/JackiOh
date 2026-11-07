// R3407000: what the signature passes' tests share (fx/cards/stinkyscape/*.test.ts): a run of events planned
// the way the board's effects layer plans it (entered, held as the queue holds it at speed 1 with Big
// moments full, remembered, then planned), each entry's cues at its offset from the run's start, a cue's
// end as R200 counts it, each card's signature ceiling, and R200 held over every entry.

import type { GameEvent, PlayerView } from "@jackioh/shared";
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
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import { hasSetPiece, wipeClaim } from "../fx/setPieces.ts";
import type { FxCue, FxPlanEnv } from "../fx/types.ts";
import { CATALOG } from "../wire/cards.ts";
import { withEvents } from "./fixtures.ts";

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
