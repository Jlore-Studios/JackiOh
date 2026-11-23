// The 95% range and the tier fit /stats prints under a figure (SPEC §9.11, R2895000, R3184001), shared by the Cards
// tab and the Created tables (R4083000).

import type { ReactElement } from "react";
import { statsTestid } from "./testids.ts";

/**
 * R2895000: a rate's 95% range under its figure, its ends to `digits` decimals of a percent (Adjusted's tenths,
 * Win rate's whole points); nothing when the server sent none.
 */
export function RangeText({ low, high, digits, testid }: { low?: number | null; high?: number | null; digits: number; testid: string }): ReactElement | null {
  if (typeof low !== "number" || typeof high !== "number") return null;
  return (
    <span className="stats-sample-hint stats-range" data-testid={testid}>
      {`${(low * 100).toFixed(digits)}–${(high * 100).toFixed(digits)}%`}
    </span>
  );
}

/** R3184001: under a tier, the share of the card's Adjusted range inside the tier's band, to a whole percent; nothing without one. */
export function TierFitText({ fit }: { fit: number | null }): ReactElement | null {
  if (fit === null) return null;
  return (
    <span className="stats-sample-hint stats-range" data-testid={statsTestid.tierFit}>
      {`${Math.round(fit * 100)}%`}
    </span>
  );
}
