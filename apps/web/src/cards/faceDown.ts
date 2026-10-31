// The words a face-down backrow card is drawn with (SPEC §10.10, R370, R371), in one module so the
// board, the inspect overlays, the showcase and the log say the same thing; read off the view, never
// a rule (CLAUDE.md rule 7). A back is always a Trap or a Field Trap (§10.8 makes a Field Spell
// public), so it is called a trap.

/** R371: the tag on your own face-down trap. */
export const FACE_DOWN_TAG = "Face down";

/** R371: what your own face-down trap's tag, tooltip and inspect note say. */
export const UNREVEALED_NOTE = "Face down — your opponent can't see this card";

export const FACE_DOWN_TITLE = "Face-down trap";

/**
 * R1046: when your own face-down play reveals, in the D3 words the picker offered it in. Keyed by
 * the wire timing so the badge and the picker cannot disagree.
 */
export const REVEAL_AT_LABEL: Record<string, string> = {
  endOfThisTurn: "End of your turn",
  startOfNextTurn: "Start of your next turn",
  endOfNextTurn: "End of your next turn",
  // R783004, R783006: a Cloaked set has no timing; it reveals only by its own Activate: Reveal.
  cloaked: "When you reveal it",
};

/** R1046: what your own face-down play's timing badge tooltips. */
export const REVEAL_AT_NOTE = "Reveals — your opponent can't see when";

/** R370: the line under it in its inspect overlay. */
export const FACE_DOWN_HINT = "Only the player who set it can see what it is. It springs by itself when its condition is met.";

/** R471: the Plague Counters a face-down card's inspect overlay states; tokens are public (B5 E19). */
export const PLAGUE_COUNT_WORDS = "Plague Counters: {count}";

/** "(2) Cost": a specific cost, written as card text writes one, the cost as a noun (R432). */
export function costPhrase(cost: number): string {
  return `(${String(cost)}) Cost`;
}

/** R370, R432: a back's label and tooltip, without a cost the view does not give. */
export function faceDownLabel(cost: number | undefined): string {
  return cost === undefined ? FACE_DOWN_TITLE : `${FACE_DOWN_TITLE}, ${costPhrase(cost)}`;
}
