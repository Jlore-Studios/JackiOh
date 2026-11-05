// A card's balance tier chip and adjusted win rate as /stats prints them (SPEC §9.11, R2777000, R3022000), shared
// by /stats' Cards tab and the card detail's Statistics panel (R3185000), so both read the same.

import type { ReactElement } from "react";
import type { CardTier } from "../wire/generated/CardTier.ts";

import "./tier.css";

/** R2777000: an adjusted win rate to a tenth of a point, as it moves in tenths near 50%; "—" when there is none. */
export function adjustedText(rate: number | null | undefined): string {
  return typeof rate === "number" ? `${(rate * 100).toFixed(1)}%` : "—";
}

/** R3022000: a card's tier chip, or "—" below the floor or from a server deployed before tiers. */
export function TierChip({ tier, testid }: { tier: CardTier | null | undefined; testid: string }): ReactElement {
  if (!tier) return <span data-testid={testid}>—</span>;
  return (
    <span className="stats-tier" data-tier={tier} data-testid={testid}>
      {tier}
    </span>
  );
}
