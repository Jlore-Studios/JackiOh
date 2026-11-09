// R3477000: what an aimed attack will do, as the engine's forecast says (`combat_forecast`, R3100000).
//
// The forecast rides with the seat's own view (a server frame's or a practice snapshot's `forecast`). The
// page reads nothing from the rules: each legal target of the selected or dragged attacker is matched to
// the forecast's entry for the same `attack` ids, and the chip (AttackOutcome.tsx) draws that entry's
// numbers as they are (CLAUDE.md rule 7). An attack the forecast has no entry for shows nothing.

import type { AttackForecast, CombatForecast, PlayerView } from "@jackioh/shared";

import { isRecord } from "../wire/json.ts";
import { attackTargetTestid, type Interaction } from "./actions.ts";
import type { Highlight } from "./contract.ts";

/** A frame's `forecast` as the field a parsed frame holds: none when it carries none, as `economyField` does. */
export function forecastField(value: unknown): { forecast?: CombatForecast } {
  return isRecord(value) && Array.isArray(value.attacks) ? { forecast: value as unknown as CombatForecast } : {};
}

/**
 * The forecast of each attack `interaction` aims, keyed by its target's testid; undefined while no attacker
 * is picked, with no forecast, or when none of its attacks has an entry.
 */
export function attackOutcomes(
  view: PlayerView,
  interaction: Interaction,
  forecast: CombatForecast | undefined,
): ReadonlyMap<string, AttackForecast> | undefined {
  if (forecast === undefined || interaction.stage !== "attacking") return undefined;
  const outcomes = new Map<string, AttackForecast>();
  for (const candidate of interaction.candidates) {
    if (candidate.type !== "attack" || candidate.attackerId !== interaction.attackerId) continue;
    const entry = forecast.attacks.find(
      (attack) => attack.attackerId === candidate.attackerId && attack.targetId === candidate.targetId,
    );
    if (entry !== undefined) outcomes.set(attackTargetTestid(view, candidate.targetId), entry);
  }
  return outcomes.size === 0 ? undefined : outcomes;
}

/** `highlight` with `outcomes` beside it; the same object when there are none. */
export function withOutcomes(highlight: Highlight, outcomes: ReadonlyMap<string, AttackForecast> | undefined): Highlight {
  return outcomes === undefined ? highlight : { ...highlight, outcomes };
}

/** The forecast the target `testId` shows, or undefined. */
export function outcomeFor(highlight: Highlight | undefined, testId: string | undefined): AttackForecast | undefined {
  return testId === undefined ? undefined : highlight?.outcomes?.get(testId);
}
