// Degrade and Upgrade's lasting changes to a card (docs/classic-sets.md B3.4, R386): the readers every
// other module uses. A card's `tuning` rides it through every zone (R78 leaves it alone); what writes
// it is `effects/tune.ts`.
//
// The numbered keywords a script reads as a flag rather than as a `Keyword` — Echo (`staticFlags.echo`),
// Activate (`ActivationDecl.uses`), Tribute (`staticFlags.tribute`) — and an X-cost card's X are read
// through `tunedCount` wherever the engine reads them, so a Degrade or Upgrade of that number is felt
// where the number is used. The numbered keywords that are `Keyword`s (Armor, Lucky, Brittle, Spell
// Damage) are tuned in the layers (§10.4) by the same step.

import type { Tuning } from "@jackioh/shared";
import type { CardInstance } from "./state";

/** B3.4: the least a tuned number may come to — "an amount never drops below 1". */
export const TUNED_FLOOR = 1;

/**
 * B3.4, R386: the value a card's numbered keyword or X has now — its printed value moved by the
 * card's tuning step for `key`, or the value KY's Constant set outright (`tuning.set`). A number the
 * card does not print (0) is never tuned into existence. Never below `min`.
 */
export function tunedCount(
  instance: Pick<CardInstance, "tuning">,
  key: string,
  printed: number,
  min: number = TUNED_FLOOR,
): number {
  if (printed <= 0) return printed;
  const set = instance.tuning?.set?.[key];
  if (set !== undefined) return Math.max(min, set);
  const step = instance.tuning?.x?.[key] ?? 0;
  return step === 0 ? printed : Math.max(min, printed + step);
}

/** B3.4: whether the card carries any tuning at all. */
export function isTuned(instance: Pick<CardInstance, "tuning">): boolean {
  const tuning: Tuning | undefined = instance.tuning;
  if (tuning === undefined) return false;
  return (
    (tuning.attack ?? 0) !== 0 ||
    (tuning.health ?? 0) !== 0 ||
    (tuning.addKeywords?.length ?? 0) > 0 ||
    (tuning.removeKeywords?.length ?? 0) > 0 ||
    Object.values(tuning.x ?? {}).some((step) => step !== 0) ||
    Object.values(tuning.numbers ?? {}).some((step) => step !== 0) ||
    Object.keys(tuning.set ?? {}).length > 0
  );
}
