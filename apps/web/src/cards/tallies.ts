// R2321000: the counts cards on the field keep on a Unit (`UnitView.tallies`, on both seats when the
// keeping card is public; Training Grounds #31 Level Up!'s attacks toward its Radiant), as the client
// words them for the board (cards/Tallies.tsx) and the inspect notes (inspect/StateNotes.tsx), by the
// keeping card's name the catalog gives. Nothing here counts or decides anything (CLAUDE.md rule 7).

import { t } from "../i18n/i18n.ts";

/** One count on a Unit, by the name of the card keeping it. */
export type NamedTally = { name: string; value: number; goal: number };

/** The chevrons a count wears, Level Up!'s rank marks (its art's poster, R2202000), so it is a shape as well as a number. */
export const TALLY_GLYPH = "▲";

/** "2/3": the badge's short form. */
export function tallyShort(tally: NamedTally): string {
  return `${String(tally.value)}/${String(tally.goal)}`;
}

/** "Level Up!: 2 of 3": the badge's tooltip and hidden text, and the inspect note. */
export function tallyWords(tally: NamedTally): string {
  return t("{name}: {value} of {goal}", { name: tally.name, value: String(tally.value), goal: String(tally.goal) });
}
