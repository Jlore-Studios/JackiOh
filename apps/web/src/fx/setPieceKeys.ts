// R2800000: the names a card's effects table may give a set piece (`{ setPiece: "<key>" }`), apart from
// their drawing (setPieces.ts), so the tables parse without importing what draws them. R3338000: the
// Terminus cards' signatures join them, each its own card's (fx/terminus/).

/**
 * Twisting Nether's vortex, Ceaseless Void's fold, and the shared vortex of every other board clear; then
 * Olivian Expanse's rift, the six Terminus tokens' and Dice of Doom's d100 (R3338000).
 */
export const SET_PIECE_KEYS = [
  "nether",
  "voidFold",
  "vortex",
  "terminusRift",
  "farewell",
  "lastExam",
  "eventHorizon",
  "lastWord",
  "finalDestination",
  "bigCrunch",
  "diceOfDoom",
] as const;

export type SetPieceKey = (typeof SET_PIECE_KEYS)[number];

export function isSetPieceKey(key: string): key is SetPieceKey {
  return (SET_PIECE_KEYS as readonly string[]).includes(key);
}
