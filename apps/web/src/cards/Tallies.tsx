// R2321000: the counts cards on the field keep on a Unit, drawn on the Unit (Training Grounds #31
// Level Up!'s attacks toward its Radiant: "▲ 2/3"), in the yard's straw and wood (R2202001), a
// chevron making it a shape as well as a color; inspect/StateNotes.tsx spells its words out on touch.
// It takes no pointer event, so a click or a drag still reaches the card, and reads only the counts
// it is given (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import { useLocale } from "../i18n/i18n.ts";
import { TALLY_GLYPH, tallyShort, tallyWords, type NamedTally } from "./tallies.ts";
import "./tallies.css";

/** The pills' testid: `tallies-<instanceId>`. */
export function talliesTestid(instanceId: string): string {
  return `tallies-${instanceId}`;
}

export default function Tallies({ tallies, instanceId }: { tallies: readonly NamedTally[]; instanceId: string }): ReactElement | null {
  useLocale();
  if (tallies.length === 0) return null;
  return (
    <span className="tallies" data-testid={talliesTestid(instanceId)}>
      {tallies.map((tally, index) => (
        <span
          key={`${String(index)}:${tally.name}`}
          className="tally-badge"
          data-value={tally.value}
          data-goal={tally.goal}
          data-reached={tally.value >= tally.goal ? "true" : undefined}
          title={tallyWords(tally)}
        >
          <span className="tally-badge__glyph" aria-hidden="true">
            {TALLY_GLYPH}
          </span>
          <span aria-hidden="true">{tallyShort(tally)}</span>
          <span className="tally-badge__sr">{tallyWords(tally)}</span>
        </span>
      ))}
    </span>
  );
}
