// The words a face-down backrow card is drawn with (SPEC §10.10, R370, R371). One module, so the
// board, the inspect overlays, the showcase and the log say the same thing.
//
// Two cases, both read off the view and neither a rule (CLAUDE.md rule 7):
//   - a card face-down to the viewer (`BackrowView { faceDown: true }`): a back, with the cost the
//     view carries for it since v0.1.1 (R370). It is always a Trap or a Field Trap, since §10.8 makes
//     a Field Spell public, so it is called a trap and nothing more;
//   - the viewer's own trap that the other player has not seen (`unrevealed: true`, R371): its face,
//     marked "Face down" with the struck-through eye, and a line saying the opponent can't see it.

/** R371: the tag on your own face-down trap. */
export const FACE_DOWN_TAG = "Face down";

/** R371: what your own face-down trap's tag, tooltip and inspect note say. */
export const UNREVEALED_NOTE = "Face down — your opponent can't see this card";

/** R370: what a back in the backrow is. */
export const FACE_DOWN_TITLE = "Face-down trap";

/** R370: the line under it in its inspect overlay. */
export const FACE_DOWN_HINT = "Only the player who set it can see what it is. It springs by itself when its condition is met.";

/** "(2) Cost": a specific cost, written as card text writes one, the cost as a noun (R432). */
export function costPhrase(cost: number): string {
  return `(${String(cost)}) Cost`;
}

/** R370, R432: a back's label and tooltip, "Face-down trap, (2) Cost", or without a cost the view does not give. */
export function faceDownLabel(cost: number | undefined): string {
  return cost === undefined ? FACE_DOWN_TITLE : `${FACE_DOWN_TITLE}, ${costPhrase(cost)}`;
}
