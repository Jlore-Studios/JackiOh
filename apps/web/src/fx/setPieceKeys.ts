// R2800000: the names a card's effects table may give a set piece (`{ setPiece: "<key>" }`), apart from
// their drawing (setPieces.ts), so the tables parse without importing what draws them.

/** Twisting Nether's vortex, Ceaseless Void's fold, and the shared vortex of every other board clear. */
export const SET_PIECE_KEYS = ["nether", "voidFold", "vortex"] as const;

export type SetPieceKey = (typeof SET_PIECE_KEYS)[number];

export function isSetPieceKey(key: string): key is SetPieceKey {
  return (SET_PIECE_KEYS as readonly string[]).includes(key);
}
