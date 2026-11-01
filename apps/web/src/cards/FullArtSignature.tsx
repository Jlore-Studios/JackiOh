// R2655000, R2655003: a Mythic full art signs its card's name across its art, as a pen would. It is
// always the English name, a Chinese face's too: the face is the card's art, and the script faces draw
// no Chinese.
//
// R2715002: it signs fast and lands hard, in a hand that suits its card. The name is drawn in its set's
// face or its card's own (signatureFaces.ts, `data-signature-face`), set to an exact length
// (`textLength`, its glyphs scaled to it), so where it starts and ends is known. In cosmetics.css's
// motion gate a mask uncovers it left to right (`.cf-signature-wipe`, an edge slanted as a pen stroke
// and feathered), a glowing pen tip leads the ink (`.cf-signature-pen`), and once the last letter lands
// a flash bursts after it (`.cf-signature-burst`), a flourish slashes under the name
// (`.cf-signature-flourish`) and a shine runs through the ink, and again every
// `SIGNATURE_SHINE_PERIOD_MS` (`.cf-signature-shine`). It starts `SIGNATURE_START_MS` after the face
// mounts and is signed `SIGNATURE_SIGNED_MS` after, under a second. Everything moves by transform and
// opacity; under reduced motion, or with Animated foil off, nothing moves and the name stands whole,
// its flourish under it, from the first frame.
//
// The signature is SVG in a fixed view box, so it scales with the card as one, and a longer name is set
// smaller to fit the box's width. It is decoration (`aria-hidden`): the name banner below says it.
// Presentation only (CLAUDE.md rule 7).

import { useId, type CSSProperties, type ReactElement } from "react";

import { SIGNATURE_FACES, signatureFaceOf } from "./signatureFaces.ts";

/** The signature's view box: wide and low, as a signature is. */
const SIGNATURE_VIEW_WIDTH = 200;
const SIGNATURE_VIEW_HEIGHT = 48;
/** Where the name's baseline sits in the view box, room left under it for the descenders and the flourish. */
const SIGNATURE_BASELINE = 34;
/** The largest the name is set, in the view box's units: a short name's size. */
export const SIGNATURE_FONT_MAX = 30;
/** The width the name may fill, in the view box's units, a margin left at each end. */
export const SIGNATURE_WIDTH = 188;

/** R2715002: how long after the face mounts the pen touches down: at once, no slow start. */
export const SIGNATURE_START_MS = 60;
/** R2715002: how long the pen takes to write the name, end to end. */
export const SIGNATURE_WRITE_MS = 520;
/** R2715002: the landing after the last letter: the burst and the flourish. */
export const SIGNATURE_LAND_MS = 260;
/** R2715002: when the signing is done, the name written and landed: held under a second. */
export const SIGNATURE_SIGNED_MS = SIGNATURE_START_MS + SIGNATURE_WRITE_MS + SIGNATURE_LAND_MS;
/** R2715002: how often a shine runs through the landed ink, its first pass as the last letter lands. */
export const SIGNATURE_SHINE_PERIOD_MS = 4200;

/** R2715002: the pen stroke's lean: the wipe's edge, slanted as a pen's, in degrees from upright. */
export const SIGNATURE_EDGE_SLANT_DEG = 12;
/** R2715002: how soft the wipe's edge is: the width of its fade, in the view box's units. */
export const SIGNATURE_EDGE_FEATHER = 10;
/** How far past the name's ends the ink may reach (a script's swashes overhang), in the view box's units. */
const SIGNATURE_INK_MARGIN = 4;
/** How far above and below the view box the wipe uncovers, for tall ascenders and long descenders. */
const SIGNATURE_INK_REACH = 6;
/** The pen tip: its glow's radius and its point's, in the view box's units. */
const SIGNATURE_PEN_GLOW_R = 6.5;
const SIGNATURE_PEN_TIP_R = 2;
/** Where on a letter the pen rides: this share of the name's size above the baseline. */
const SIGNATURE_PEN_HEIGHT = 0.35;
/** The landing's burst, where the pen lifts after the last letter: its radius. */
const SIGNATURE_BURST_R = 5;
/** The flourish under the name: where it starts along the name, how far below the baseline it dips and ends, and its flick past the end. */
const SIGNATURE_FLOURISH_START = 0.08;
const SIGNATURE_FLOURISH_DIP = 11;
const SIGNATURE_FLOURISH_END = 4;
const SIGNATURE_FLOURISH_FLICK = 6;
const SIGNATURE_FLOURISH_RISE = 7;
/** How close to the view box's right edge the flourish's flick may reach. */
const SIGNATURE_FLOURISH_INSET = 2;
/** The shine: the width of its band, in the view box's units, and its lean. */
const SIGNATURE_SHINE_WIDTH = 10;
const SIGNATURE_SHINE_SLANT_DEG = 20;

const DEGREES = Math.PI / 180;

/** The size `name` is signed at in a face of `charEm`, in the view box's units, so it fits `SIGNATURE_WIDTH`. */
export function signatureSize(name: string, charEm: number): number {
  return Math.min(SIGNATURE_FONT_MAX, SIGNATURE_WIDTH / (Math.max(name.length, 1) * charEm));
}

/** R2715002: the length `name` is drawn to at `size` in a face of `charEm`: its natural width, never past `SIGNATURE_WIDTH`. */
export function signatureLength(name: string, size: number, charEm: number): number {
  return Math.min(SIGNATURE_WIDTH, Math.max(name.length, 1) * size * charEm);
}

/** An id React gives, as an SVG fragment can name it. */
function svgId(id: string, part: string): string {
  return `sig-${id.replace(/[^A-Za-z0-9_-]/gu, "")}-${part}`;
}

/** R2655000, R2715002: the card's name, signed across a Mythic full art (CardFace), in `defId`'s hand. */
export function FullArtSignature({ name, defId }: { name: string; defId: string }): ReactElement {
  const id = useId();
  const face = signatureFaceOf(defId);
  const { charEm } = SIGNATURE_FACES[face];
  const size = signatureSize(name, charEm);
  const length = signatureLength(name, size, charEm);
  const left = SIGNATURE_VIEW_WIDTH / 2 - length / 2;
  const right = SIGNATURE_VIEW_WIDTH / 2 + length / 2;
  const slant = Math.tan(SIGNATURE_EDGE_SLANT_DEG * DEGREES);
  // The wipe, in its slanted frame: written ink from before the name's start to past its end at every
  // height it reaches, then the feathered edge. At rest it uncovers the whole name.
  const top = -SIGNATURE_INK_REACH;
  const bottom = SIGNATURE_VIEW_HEIGHT + SIGNATURE_INK_REACH;
  const inkLeft = left - SIGNATURE_INK_MARGIN - SIGNATURE_INK_REACH * slant;
  const edge = right + SIGNATURE_INK_MARGIN + bottom * slant;
  const reach = { y: top, height: bottom - top };
  const skew = `skewX(-${SIGNATURE_EDGE_SLANT_DEG})`;
  const penY = SIGNATURE_BASELINE - size * SIGNATURE_PEN_HEIGHT;
  const penX = edge + SIGNATURE_EDGE_FEATHER / 2 - penY * slant;
  const flourishEnd = Math.min(right + SIGNATURE_FLOURISH_FLICK, SIGNATURE_VIEW_WIDTH - SIGNATURE_FLOURISH_INSET);
  const flourish =
    `M${left + length * SIGNATURE_FLOURISH_START} ${SIGNATURE_BASELINE + SIGNATURE_FLOURISH_END}` +
    `Q${SIGNATURE_VIEW_WIDTH / 2} ${SIGNATURE_BASELINE + SIGNATURE_FLOURISH_DIP} ${right} ${SIGNATURE_BASELINE + SIGNATURE_FLOURISH_END}` +
    `L${flourishEnd} ${SIGNATURE_BASELINE + SIGNATURE_FLOURISH_END - SIGNATURE_FLOURISH_RISE}`;
  const shineLeft = left - SIGNATURE_INK_MARGIN - SIGNATURE_SHINE_WIDTH;
  const shineSpan = length + 2 * (SIGNATURE_INK_MARGIN + SIGNATURE_SHINE_WIDTH);
  const maskId = svgId(id, "mask");
  const featherId = svgId(id, "feather");
  const clipId = svgId(id, "clip");
  const timing = {
    "--cf-sign-start": `${SIGNATURE_START_MS}ms`,
    "--cf-sign-write": `${SIGNATURE_WRITE_MS}ms`,
    "--cf-sign-land": `${SIGNATURE_LAND_MS}ms`,
    "--cf-sign-landed": `${SIGNATURE_START_MS + SIGNATURE_WRITE_MS}ms`,
    "--cf-sign-shine": `${SIGNATURE_SHINE_PERIOD_MS}ms`,
  } as CSSProperties;
  const text = (
    <text
      className="cf-signature-text"
      x={SIGNATURE_VIEW_WIDTH / 2}
      y={SIGNATURE_BASELINE}
      textAnchor="middle"
      fontSize={size}
      textLength={length}
      lengthAdjust="spacingAndGlyphs"
    >
      {name}
    </text>
  );
  return (
    <span className="cf-signature" aria-hidden="true" data-signature-face={face} style={timing}>
      <svg viewBox={`0 0 ${SIGNATURE_VIEW_WIDTH} ${SIGNATURE_VIEW_HEIGHT}`} preserveAspectRatio="xMidYMid meet" focusable="false">
        <defs>
          <linearGradient id={featherId} gradientUnits="userSpaceOnUse" x1={edge} x2={edge + SIGNATURE_EDGE_FEATHER} y1="0" y2="0">
            <stop offset="0" stopColor="#fff" />
            <stop offset="1" stopColor="#000" />
          </linearGradient>
          <mask id={maskId} maskUnits="userSpaceOnUse" x={-SIGNATURE_VIEW_WIDTH} y={top} width={3 * SIGNATURE_VIEW_WIDTH} height={reach.height}>
            <g className="cf-signature-wipe">
              <g transform={skew}>
                <rect x={inkLeft} y={reach.y} width={edge - inkLeft} height={reach.height} fill="#fff" />
                <rect x={edge} y={reach.y} width={SIGNATURE_EDGE_FEATHER} height={reach.height} fill={`url(#${featherId})`} />
              </g>
            </g>
          </mask>
        </defs>
        <g className="cf-signature-ink" mask={`url(#${maskId})`}>
          {text}
          {/* The name again, after the drawn one, so the shine shows through its ink alone. */}
          <clipPath id={clipId}>{text}</clipPath>
          <g clipPath={`url(#${clipId})`}>
            <g className="cf-signature-shine">
              <rect x={shineLeft} y="0" width={shineSpan} height={SIGNATURE_VIEW_HEIGHT} fill="none" />
              <rect
                className="cf-signature-shine-band"
                x={shineLeft}
                y="0"
                width={SIGNATURE_SHINE_WIDTH}
                height={SIGNATURE_VIEW_HEIGHT}
                transform={`skewX(-${SIGNATURE_SHINE_SLANT_DEG})`}
              />
            </g>
          </g>
        </g>
        <path className="cf-signature-flourish" d={flourish} />
        <g className="cf-signature-pen">
          <g transform={skew}>
            <rect x={inkLeft} y={reach.y} width={edge + SIGNATURE_EDGE_FEATHER - inkLeft} height={reach.height} fill="none" />
          </g>
          <circle className="cf-signature-pen-glow" cx={penX} cy={penY} r={SIGNATURE_PEN_GLOW_R} />
          <circle className="cf-signature-pen-tip" cx={penX} cy={penY} r={SIGNATURE_PEN_TIP_R} />
        </g>
        <circle className="cf-signature-burst" cx={Math.min(penX, SIGNATURE_VIEW_WIDTH - SIGNATURE_BURST_R)} cy={penY} r={SIGNATURE_BURST_R} />
      </svg>
    </span>
  );
}
