// R3338002: Ceaseless Void's cost gem drains as its cost falls. Presentation of the card's own visible cost
// only (CLAUDE.md rule 7): the share comes from the cost the view gives the card and the catalog's printed
// one, and Card.tsx asks for it only for a card in its owner's own hand, so no other seat's view draws it
// (R202). The gem's level is cards.css's (`.cost-gem[data-gem-drain]`).

/** The cards whose gem drains: the Void, whose 100 falls with every card drawn, played, destroyed or exiled. */
export const GEM_DRAIN_CARDS: ReadonlySet<string> = new Set(["core-100"]);

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

/**
 * The share of the gem drained, 0 (full, at its printed cost) to 1 (empty, at 0), for a card whose gem drains:
 * null for any other card, a printed cost of 0 or less, or a cost not known.
 */
export function gemDrainOf(defId: string, cost: number | undefined, printed: number | undefined): number | null {
  if (!GEM_DRAIN_CARDS.has(defId)) return null;
  if (cost === undefined || printed === undefined || !Number.isFinite(cost) || !Number.isFinite(printed) || printed <= 0) return null;
  return clamp01(1 - cost / printed);
}
