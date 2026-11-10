// R3477000: what an aimed attack will do, as the engine's forecast says (`combat_forecast`, R3100000).
//
// The forecast rides with the seat's own view (a server frame's or a practice snapshot's `forecast`). The
// page reads nothing from the rules: the hovered legal target of the selected or dragged attacker is
// matched to the forecast's entry for the same `attack` ids, and the markers (AttackOutcome.tsx) draw
// that entry's numbers as they are (CLAUDE.md rule 7). An attack the forecast has no entry for, or no
// hovered target, shows nothing.

import type { AttackForecast, CombatForecast, PlayerView } from "@jackioh/shared";

import { isRecord } from "../wire/json.ts";
import { attackTargetTestid, heroPlayerOfTargetId, type Interaction } from "./actions.ts";
import { testid, type Highlight } from "./contract.ts";

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

/**
 * R3615000: the one hovered attack whose markers show, by the cards' testids. The target wears the
 * blow it takes (`dealt`); the attacker wears what it takes back (`taken`). `strikesBack` is false on
 * a hero target, which never strikes back (R3100000). Never read by a click.
 */
export type HoveredAttack = { attacker: string; target: string; strikesBack: boolean };

/**
 * R3615000: the hovered attack to preview, or undefined. Only while an attacker is picked, with a
 * forecast, and the hovered target has an entry for this attack; leaving the target clears it.
 */
export function hoveredAttack(
  view: PlayerView,
  interaction: Interaction,
  outcomes: ReadonlyMap<string, AttackForecast> | undefined,
  hovered: string | null,
): HoveredAttack | undefined {
  if (interaction.stage !== "attacking" || outcomes === undefined || hovered === null) return undefined;
  if (!outcomes.has(hovered)) return undefined;
  const candidate = interaction.candidates.find(
    (body) =>
      body.type === "attack" &&
      body.attackerId === interaction.attackerId &&
      attackTargetTestid(view, body.targetId) === hovered,
  );
  if (candidate === undefined || candidate.type !== "attack") return undefined;
  return {
    attacker: testid.card(interaction.attackerId),
    target: hovered,
    strikesBack: heroPlayerOfTargetId(view, candidate.targetId) === null,
  };
}

/** `highlight` with `outcomes` (and the hovered attack) beside it; the same object when there are none. */
export function withOutcomes(
  highlight: Highlight,
  outcomes: ReadonlyMap<string, AttackForecast> | undefined,
  hovered?: HoveredAttack,
): Highlight {
  return outcomes === undefined ? highlight : { ...highlight, outcomes, hoveredAttack: hovered };
}

/** The forecast the target `testId` shows, or undefined. */
export function outcomeFor(highlight: Highlight | undefined, testId: string | undefined): AttackForecast | undefined {
  return testId === undefined ? undefined : highlight?.outcomes?.get(testId);
}
