// #3609: the slice of a `Highlight` one card's tree reads. A selection or a hover builds a new `Highlight`
// whose sets mostly hold what they held, and every card is handed the same one, so comparing it by identity
// drew every card on the board for a change that touched one or two (Card.tsx, `sameCardProps`).
//
// What a card's tree reads (Card.tsx, ActivateControl.tsx, FieldBadges.tsx, which only forwards it to the
// Activate controls): `legal`, `selected`, `glow`, `condition` and `counted` for its own testid, its unit's
// `card-` one and its switch's; the three of `legal`, `selected` and `glow` for its Activate controls
// (`activate-<id>` and `activate-<id>-<ability>`); `outcomes` at its own testid; and the hovered attack's
// markers, on the target and on the attacker only. If a card's tree reads anything else off a `Highlight`,
// count it here, or a card would keep drawing what it drew.

import { NO_HIGHLIGHT, testid, type Highlight } from "./contract.ts";

/** What names a card to a `Highlight`: the testid its `Card` carries, and the id of the card in play it draws. */
export type CardReads = { testId: string | undefined; instanceId: string | undefined };

const SETS = ["legal", "selected", "glow", "condition", "counted"] as const;

/** The members of `set` that name the card's Activate controls, sorted into one string. */
function activateSlice(set: ReadonlySet<string> | undefined, prefix: string): string {
  if (set === undefined) return "";
  return [...set].filter((id) => id.startsWith(prefix)).sort().join("\n");
}

/** R3615000: the hovered attack as the card at `testId` sees it: nothing, unless it is the target or the attacker. */
function hoveredAt(highlight: Highlight, testId: string | undefined): string {
  const hovered = highlight.hoveredAttack;
  if (hovered === undefined || testId === undefined) return "";
  if (hovered.target === testId) return `target\0${hovered.attacker}\0${String(hovered.strikesBack)}`;
  if (hovered.attacker === testId) return `attacker\0${hovered.target}\0${String(hovered.strikesBack)}`;
  return "";
}

/** The forecast the attacker wears as the blow it takes back, which Card.tsx reads at the hovered target. */
function takenAt(highlight: Highlight, testId: string | undefined): unknown {
  const hovered = highlight.hoveredAttack;
  return hovered !== undefined && hovered.attacker === testId && hovered.strikesBack ? highlight.outcomes?.get(hovered.target) : undefined;
}

/** Whether two highlights say the same of the card `reads` names (`undefined` is `NO_HIGHLIGHT`). */
export function sameHighlightAt(prev: Highlight | undefined, next: Highlight | undefined, reads: CardReads): boolean {
  const a = prev ?? NO_HIGHLIGHT;
  const b = next ?? NO_HIGHLIGHT;
  if (a === b) return true;
  const { testId, instanceId } = reads;
  const ids: string[] = [];
  if (testId !== undefined) ids.push(testId);
  if (instanceId !== undefined) ids.push(testid.card(instanceId), testid.switchPosition(instanceId));
  for (const id of ids) {
    for (const set of SETS) if ((a[set]?.has(id) === true) !== (b[set]?.has(id) === true)) return false;
    if (a.outcomes?.get(id) !== b.outcomes?.get(id)) return false;
  }
  if (instanceId !== undefined) {
    const prefix = testid.activate(instanceId);
    for (const set of ["legal", "selected", "glow"] as const) {
      if (activateSlice(a[set], prefix) !== activateSlice(b[set], prefix)) return false;
    }
  }
  return hoveredAt(a, testId) === hoveredAt(b, testId) && takenAt(a, testId) === takenAt(b, testId);
}
