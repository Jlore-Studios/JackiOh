// The face's glyphs: the attack sword, the health drop, the rarity gem's facets and the Legendary
// crest (docs/polish/6-cards.md, Surface B), and the struck-through eye a face-down card you
// control wears (R371).
//
// Each glyph is an original SVG with no <text> and no <title>, so it names nothing and reads as
// nothing to a screen reader. It is drawn as an `<img>` of a `data:` URI rather than inline <svg>,
// because a full face renders only span, strong and img elements (B12): that is what lets a face
// sit inside the deck builder's <button>. Gradient ids are scoped to their own image, so two
// glyphs on one page can never collide.

import type { ReactElement } from "react";

export type IconName =
  | "sword"
  | "drop"
  | "gem"
  | "crest"
  | "eyeOff"
  | "brittle"
  | "returnHand"
  | "castOnDraw"
  | "target"
  | "cog";

const SVG_NS = "http://www.w3.org/2000/svg";

// A gold medallion, as large as the drop beside it, over a long blade laid corner to corner: the
// tip shows top right and the guard and pommel bottom left, so the number sits on the medallion
// and both stats carry the same weight.
const SWORD = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<defs><radialGradient id='m' cx='.38' cy='.3' r='.8'><stop offset='0' stop-color='#fff6c4'/><stop offset='.42' stop-color='#f5bd2e'/><stop offset='.85' stop-color='#a86400'/><stop offset='1' stop-color='#6b3c00'/></radialGradient>
<linearGradient id='b' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='#ffffff'/><stop offset='.5' stop-color='#cfd6e4'/><stop offset='1' stop-color='#7d879c'/></linearGradient></defs>
<path d='M53 4 L61 3 L60 11 L42 29 L35 22 Z' fill='url(#b)' stroke='#1f2430' stroke-width='2.2' stroke-linejoin='round'/>
<path d='M7 43 L21 57 L18 60 L4 46 Z' fill='#c9892b' stroke='#2a1906' stroke-width='2.2' stroke-linejoin='round'/>
<path d='M10 51 L4 57 L7 60 L13 54 Z' fill='#6b4215' stroke='#2a1906' stroke-width='2'/>
<circle cx='4.5' cy='59.5' r='3' fill='#e0a53a' stroke='#2a1906' stroke-width='1.6'/>
<circle cx='31' cy='33' r='24.5' fill='url(#m)' stroke='#3b2400' stroke-width='3'/>
<circle cx='31' cy='33' r='20' fill='none' stroke='#fff2b8' stroke-opacity='.45' stroke-width='1.5'/>
<ellipse cx='23' cy='22' rx='9' ry='5' fill='#fff' opacity='.3' transform='rotate(-25 23 22)'/>
</svg>`;

const DROP = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<defs><radialGradient id='d' cx='.4' cy='.58' r='.72'><stop offset='0' stop-color='#ff9f92'/><stop offset='.5' stop-color='#cf2626'/><stop offset='1' stop-color='#650909'/></radialGradient></defs>
<path d='M32 3 C38 16 54 29 54 41 A22 22 0 0 1 10 41 C10 29 26 16 32 3 Z' fill='url(#d)' stroke='#3a0505' stroke-width='3' stroke-linejoin='round'/>
<ellipse cx='23' cy='39' rx='5' ry='8' fill='#fff' opacity='.3'/>
</svg>`;

const GEM = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<path d='M4 4 L32 32 L60 4 M4 60 L32 32 L60 60' fill='none' stroke='#fff' stroke-opacity='.5' stroke-width='3'/>
<path d='M16 16 H48 V48 H16 Z' fill='#fff' fill-opacity='.2' stroke='#fff' stroke-opacity='.55' stroke-width='2'/>
<path d='M10 10 L22 10 L10 22 Z' fill='#fff' fill-opacity='.7'/>
</svg>`;

const CREST = `<svg xmlns='${SVG_NS}' viewBox='0 0 120 40'>
<defs><linearGradient id='g' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='#fff4c0'/><stop offset='.5' stop-color='#e2a526'/><stop offset='1' stop-color='#7a4a00'/></linearGradient>
<radialGradient id='j' cx='.4' cy='.35' r='.7'><stop offset='0' stop-color='#fff'/><stop offset='.4' stop-color='#ff9f1c'/><stop offset='1' stop-color='#8a3a00'/></radialGradient></defs>
<g id='w' fill='url(#g)' stroke='#4a2e00' stroke-width='1.5' stroke-linejoin='round'>
<path d='M56 30 C44 22 28 8 3 9 C14 14 20 19 24 23 C16 21 10 21 4 23 C16 28 32 32 56 34 Z'/>
<path d='M50 34 C40 35 30 36 18 33 C26 38 40 39 52 37 Z'/>
<ellipse cx='36' cy='18' rx='5' ry='2.2' transform='rotate(-25 36 18)'/>
<ellipse cx='44' cy='23' rx='5' ry='2.2' transform='rotate(-35 44 23)'/>
</g>
<use href='#w' transform='translate(120 0) scale(-1 1)'/>
<circle cx='60' cy='25' r='9' fill='url(#j)' stroke='#4a2e00' stroke-width='2'/>
<path d='M60 3 L64 12 L60 16 L56 12 Z' fill='url(#g)' stroke='#4a2e00' stroke-width='1.5'/>
</svg>`;

// R371: an eye struck through, the mark of a face-down card the other player cannot see. A pale
// outline on a dark disc, so it reads by shape on any frame and to a colour-blind player alike.
const EYE_OFF = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<circle cx='32' cy='32' r='30' fill='#101522' stroke='#e8ecf8' stroke-width='3'/>
<path d='M10 32 C18 20 26 16 32 16 C38 16 46 20 54 32 C46 44 38 48 32 48 C26 48 18 44 10 32 Z' fill='none' stroke='#e8ecf8' stroke-width='4' stroke-linejoin='round'/>
<circle cx='32' cy='32' r='7' fill='#e8ecf8'/>
<path d='M14 50 L50 14' stroke='#101522' stroke-width='10' stroke-linecap='round'/>
<path d='M14 50 L50 14' stroke='#e8ecf8' stroke-width='4.5' stroke-linecap='round'/>
</svg>`;

// Patch v0.2.0's state badges (cardState.ts). Each is a pale outline on a dark plate, so it reads by
// shape on any frame and to a colour-blind player alike, as the struck eye does.

// B3.3, R385: a pane of glass with a crack running through it (Brittle); the count sits over it.
const BRITTLE = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<path d='M8 6 H56 V58 H8 Z' fill='#16303a' stroke='#d8f3ff' stroke-width='4' stroke-linejoin='round'/>
<path d='M8 6 H56 V58 H8 Z' fill='#9fe6ff' fill-opacity='.18'/>
<path d='M30 6 L26 22 L36 30 L24 42 L30 58 M26 22 L12 26 M36 30 L52 24 M24 42 L10 46 M36 30 L46 48' fill='none' stroke='#eafaff' stroke-width='3.2' stroke-linecap='round' stroke-linejoin='round'/>
</svg>`;

// B5 E39: an arrow curling back down onto a tray (Forever&'s return to hand).
const RETURN_HAND = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<circle cx='32' cy='32' r='30' fill='#101522' stroke='#e8ecf8' stroke-width='3'/>
<path d='M44 40 C50 30 46 16 32 16 C22 16 16 22 16 30' fill='none' stroke='#e8ecf8' stroke-width='5' stroke-linecap='round'/>
<path d='M8 26 L16 36 L24 26 Z' fill='#e8ecf8'/>
<path d='M18 46 H46' stroke='#e8ecf8' stroke-width='5' stroke-linecap='round'/>
</svg>`;

// B5 E39: a card with a four-point spark at its corner (Cast on draw).
const CAST_ON_DRAW = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<circle cx='32' cy='32' r='30' fill='#101522' stroke='#e8ecf8' stroke-width='3'/>
<path d='M18 18 H38 V50 H18 Z' fill='none' stroke='#e8ecf8' stroke-width='4' stroke-linejoin='round'/>
<path d='M44 10 L47 19 L56 22 L47 25 L44 34 L41 25 L32 22 L41 19 Z' fill='#e8ecf8'/>
</svg>`;

// B5 E39: crosshairs (targets enemies when it can).
const TARGET = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<circle cx='32' cy='32' r='30' fill='#101522' stroke='#e8ecf8' stroke-width='3'/>
<circle cx='32' cy='32' r='14' fill='none' stroke='#e8ecf8' stroke-width='4'/>
<path d='M32 8 V22 M32 42 V56 M8 32 H22 M42 32 H56' stroke='#e8ecf8' stroke-width='4' stroke-linecap='round'/>
<circle cx='32' cy='32' r='3.5' fill='#e8ecf8'/>
</svg>`;

// B3.1, R383: a cog (a card standing as a Unit), as the minion's Animated treatment draws one.
const COG = `<svg xmlns='${SVG_NS}' viewBox='0 0 64 64'>
<circle cx='32' cy='32' r='30' fill='#101522' stroke='#e8ecf8' stroke-width='3'/>
<path d='M29 10 H35 L36 17 L41 19 L46 14 L50 18 L45 23 L47 28 L54 29 V35 L47 36 L45 41 L50 46 L46 50 L41 45 L36 47 L35 54 H29 L28 47 L23 45 L18 50 L14 46 L19 41 L17 36 L10 35 V29 L17 28 L19 23 L14 18 L18 14 L23 19 L28 17 Z' fill='none' stroke='#e8ecf8' stroke-width='3' stroke-linejoin='round'/>
<circle cx='32' cy='32' r='6' fill='none' stroke='#e8ecf8' stroke-width='3.5'/>
</svg>`;

function dataUri(svg: string): string {
  return `data:image/svg+xml,${encodeURIComponent(svg.replace(/\n/g, ""))}`;
}

const ICON_URI: Readonly<Record<IconName, string>> = {
  sword: dataUri(SWORD),
  drop: dataUri(DROP),
  gem: dataUri(GEM),
  crest: dataUri(CREST),
  eyeOff: dataUri(EYE_OFF),
  brittle: dataUri(BRITTLE),
  returnHand: dataUri(RETURN_HAND),
  castOnDraw: dataUri(CAST_ON_DRAW),
  target: dataUri(TARGET),
  cog: dataUri(COG),
};

export function Icon({ name }: { name: IconName }): ReactElement {
  return (
    <img
      className={`cf-icon cf-icon--${name}`}
      src={ICON_URI[name]}
      alt=""
      aria-hidden="true"
      draggable={false}
      decoding="async"
    />
  );
}
