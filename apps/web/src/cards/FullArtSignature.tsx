// R2655000, R2655003: a Mythic full art signs its card's name across its art, in cursive, as a pen
// would: the name is drawn in the signature face (`--font-signature`, the self-hosted Dancing Script,
// fonts.css), and in the motion gate (cosmetics.css) a wipe uncovers it left to right, stroke by
// stroke, and it then holds on the face. Under reduced motion, or with Animated foil off, the name
// stands whole from the start. It is always the English name, a Chinese face's too: the face is the
// card's art, and the script face draws no Chinese.
//
// The name is SVG text in a fixed view box, so it scales with the card as one, and a longer name is
// set smaller to fit the box's width. It is decoration (`aria-hidden`): the name banner below says it.
// Presentation only (CLAUDE.md rule 7).

import type { ReactElement } from "react";

/** The signature's view box: wide and low, as a signature is. */
const SIGNATURE_VIEW_WIDTH = 200;
const SIGNATURE_VIEW_HEIGHT = 48;
/** Where the name's baseline sits in the view box, room left under it for the script's descenders. */
const SIGNATURE_BASELINE = 34;
/** The largest the name is set, in the view box's units: a short name's size. */
export const SIGNATURE_FONT_MAX = 30;
/** The width the name may fill, in the view box's units, a margin left at each end. */
export const SIGNATURE_WIDTH = 188;
/**
 * About how wide one letter of the name is, in ems: what sets a long name's size. Wide enough for a
 * fallback face's bold letters too (the script face's are narrower), so no name runs out of its box.
 */
export const SIGNATURE_CHAR_EM = 0.5;

/** The size `name` is signed at, in the view box's units, so it fits `SIGNATURE_WIDTH`. */
export function signatureSize(name: string): number {
  return Math.min(SIGNATURE_FONT_MAX, SIGNATURE_WIDTH / (Math.max(name.length, 1) * SIGNATURE_CHAR_EM));
}

/** R2655000: the card's name, signed in cursive across a Mythic full art (CardFace). */
export function FullArtSignature({ name }: { name: string }): ReactElement {
  return (
    <span className="cf-signature" aria-hidden="true">
      <span className="cf-signature-wipe">
        <span className="cf-signature-ink">
          <svg viewBox={`0 0 ${SIGNATURE_VIEW_WIDTH} ${SIGNATURE_VIEW_HEIGHT}`} preserveAspectRatio="xMidYMid meet" focusable="false">
            <text className="cf-signature-text" x={SIGNATURE_VIEW_WIDTH / 2} y={SIGNATURE_BASELINE} textAnchor="middle" fontSize={signatureSize(name)}>
              {name}
            </text>
          </svg>
        </span>
      </span>
    </span>
  );
}
