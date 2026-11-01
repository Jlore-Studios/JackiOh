// R2715002: the faces a Mythic full art signs its card's name in (FullArtSignature.tsx). Each set signs
// in a hand of its own, and a small map gives a card its own: every face is open-licensed (SIL OFL, or
// Apache 2.0 for Permanent Marker), self-hosted in public/fonts with its licence beside it, Latin only
// and served from no CDN (fonts.css, `--font-signature-<face>`). A browser fetches a face only once
// some text is drawn in it, so a face costs nothing until a Mythic of its set is shown. A set with no
// face of its own signs in Dancing Script, R2655003's face.
//
// Presentation only (CLAUDE.md rule 7).

import { fullArtSource } from "./fullArt.ts";

/** R2715002: a signature's hand, each one face of fonts.css. */
export type SignatureFace = "script" | "brush" | "copperplate" | "flourish" | "ink" | "tag" | "marker";

/**
 * R2715002: each hand's family (as fonts.css names it), its file and licence in public/fonts, and
 * about how wide one of its letters is, in ems: the mean of the signed names' letters, measured in a
 * browser, which sets a name's size and the length it is drawn to (FullArtSignature's `textLength`).
 */
export const SIGNATURE_FACES: Readonly<Record<SignatureFace, { family: string; file: string; licence: string; charEm: number }>> = {
  // R2655003's: Dancing Script, a plain cursive.
  script: { family: "Dancing Script", file: "dancing-script-5.3.0-latin-wght-normal.woff2", licence: "OFL-DancingScript.txt", charEm: 0.41 },
  // Core's: Kaushan Script, a fast brush script.
  brush: { family: "Kaushan Script", file: "kaushan-script-5.3.0-latin-400-normal.woff2", licence: "OFL-KaushanScript.txt", charEm: 0.44 },
  // Classic's: Pinyon Script, a copperplate hand.
  copperplate: { family: "Pinyon Script", file: "pinyon-script-5.3.0-latin-400-normal.woff2", licence: "OFL-PinyonScript.txt", charEm: 0.41 },
  // Classic+'s: Mr Dafoe, a bold showman's brush signature.
  flourish: { family: "Mr Dafoe", file: "mr-dafoe-5.3.0-latin-400-normal.woff2", licence: "OFL-MrDafoe.txt", charEm: 0.39 },
  // Meditative's: Nanum Brush Script, an ink brush.
  ink: { family: "Nanum Brush Script", file: "nanum-brush-script-5.3.0-latin-400-normal.woff2", licence: "OFL-NanumBrushScript.txt", charEm: 0.35 },
  // Stinkyscape's: Sedgwick Ave Display, a graffiti tag.
  tag: { family: "Sedgwick Ave Display", file: "sedgwick-ave-display-5.3.0-latin-400-normal.woff2", licence: "OFL-SedgwickAveDisplay.txt", charEm: 0.44 },
  // Training Grounds': Permanent Marker, a coach's marker on the whiteboard.
  marker: { family: "Permanent Marker", file: "permanent-marker-5.3.0-latin-400-normal.woff2", licence: "Apache-PermanentMarker.txt", charEm: 0.54 },
};

/** The hand a card of a set with no face of its own signs in: R2655003's. */
export const DEFAULT_SIGNATURE_FACE: SignatureFace = "script";

/** R2715002: each set's hand, by the set's id prefix (a catalog id's part before its first "-"). */
export const SIGNATURE_FACE_OF_SET: Readonly<Record<string, SignatureFace>> = {
  core: "brush",
  classic: "copperplate",
  classicplus: "flourish",
  meditative: "ink",
  stinkyscape: "tag",
  traininggrounds: "marker",
};

/** R2715002: the cards that sign in a hand of their own, by card id, over their set's. */
export const SIGNATURE_FACE_OF_CARD: Readonly<Record<string, SignatureFace>> = {
  // Mr. Vanilla signs in the plain face: the vanilla one.
  "core-008": "script",
  // "Miss" Mrow signs as a lady of letters would, in copperplate.
  "core-086": "copperplate",
};

/**
 * R2715002: the hand the full art `defId` prints signs in: its card's own (a listed token's parent's,
 * R2655002), else its set's, else Dancing Script.
 */
export function signatureFaceOf(defId: string): SignatureFace {
  const card = fullArtSource(defId);
  const own = SIGNATURE_FACE_OF_CARD[card];
  if (own !== undefined) return own;
  const dash = card.indexOf("-");
  const set = dash < 0 ? card : card.slice(0, dash);
  return Object.hasOwn(SIGNATURE_FACE_OF_SET, set) ? (SIGNATURE_FACE_OF_SET[set] ?? DEFAULT_SIGNATURE_FACE) : DEFAULT_SIGNATURE_FACE;
}
