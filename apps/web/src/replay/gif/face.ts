// The key a card face is stored under in the raster's assets (#513). Kept apart from `plan.ts` so
// the worker, which draws faces, does not load the board's animation module the plan reads.

/** `defId`, or `${defId}+radiant` for a Radiant card: the two faces of a card differ in text and art. */
export function faceKey(defId: string, radiant: boolean): string {
  return radiant ? `${defId}+radiant` : defId;
}
