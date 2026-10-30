// A Dormant card's binding seal (#2435, R1839000): the grey veil and the night-violet ribbons
// with their crescent, drawn on a minion the view says is Dormant (`FaceModel.dormant`, from
// `UnitView.slumber`, faces.ts). MinionFace mounts this right before the state rail, so the veil's
// `backdrop-filter` greys only what is painted before it and the rail (the Dormant badge, its
// countdown, the Starship piece count) stays in colour on top.
//
// The seal has three layers: the veil (the whole face greyed and dimmed), the two still ribbons
// (the bound mark, crossing over the portrait's middle), and the two looping parts (the runes
// drifting along the bands and the crescent seal where they cross). Each part is an inline,
// aria-hidden SVG with no <text> or <title> (a face is span, strong, img and aria-hidden SVG,
// docs/polish/6-cards.md B12). A part that loops carries `cf-dormant-anim`; dormant.css runs its
// keyframes always, and never under reduced motion. A card beneath a Stack (R1839003) is not drawn,
// so nothing changes there; only a Dormant card on top of its zone gets the treatment.
//
// The release lives in animations.css (`jk-dormant-break`, `jk-dormant-lift`) and game/awakening.ts:
// on `awakened` (R1839002) the ribbons burst and the veil lifts, and `data-awakening` holds the seal
// gone through a Starship's launch (R1852003).

import type { CSSProperties, ReactElement } from "react";

import { DORMANT_DIM, DORMANT_GREY, DORMANT_PULSE_MS } from "./constants.ts";
import type { FaceModel } from "./model.ts";

import "./dormant.css";

/** The custom properties dormant.css reads; the numbers are constants.ts's (CLAUDE.md rule 9). */
export const DORMANT_STYLE = {
  "--cf-dormant-grey": String(DORMANT_GREY),
  "--cf-dormant-dim": String(DORMANT_DIM),
  "--cf-dormant-pulse": `${String(DORMANT_PULSE_MS)}ms`,
} as CSSProperties;

/** A minion's `data-dormant`: "true" when no countdown wakes it, else the turns left; undefined while awake. */
export function dormantAttr(dormant: FaceModel["dormant"]): string | undefined {
  if (dormant === undefined || dormant === null) return undefined;
  return dormant.turns === null ? "true" : String(dormant.turns);
}

/** The two ribbons, crossing at (50, 36): over the portrait's middle, clear of the rail above the name plate. */
const RIBBONS = ["M-6 14 L106 58", "M-6 58 L106 14"] as const;

export function DormantSeal(): ReactElement {
  return (
    <span className="cf-dormant" style={DORMANT_STYLE} aria-hidden="true">
      <span className="cf-dormant-veil" />
      <span className="cf-dormant-bands">
        <span className="cf-dormant-part cf-dormant-ribbons">
          <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true" focusable="false">
            {RIBBONS.map((d) => (
              <g key={d}>
                <path d={d} fill="none" stroke="#0b0d12" strokeWidth="14" strokeOpacity="0.55" />
                <path d={d} fill="none" stroke="#261b5c" strokeWidth="11" />
                <path d={d} fill="none" stroke="#8a7bd8" strokeWidth="7" />
              </g>
            ))}
          </svg>
        </span>
        <span className="cf-dormant-part cf-dormant-runes cf-dormant-anim">
          <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true" focusable="false">
            {RIBBONS.map((d) => (
              <path key={d} d={d} fill="none" stroke="#e6e0ff" strokeWidth="2.2" strokeDasharray="1.6 6" strokeLinecap="round" />
            ))}
          </svg>
        </span>
        <span className="cf-dormant-part cf-dormant-moon cf-dormant-anim">
          <svg viewBox="0 0 100 100" preserveAspectRatio="xMidYMid meet" aria-hidden="true" focusable="false">
            <circle cx="50" cy="50" r="42" fill="#261b5c" stroke="#0b0d12" strokeWidth="6" />
            <circle cx="50" cy="50" r="42" fill="none" stroke="#8a7bd8" strokeWidth="4" />
            <path d="M65 24 A30 30 0 1 0 65 76 A28 28 0 0 1 65 24 Z" fill="#e6e0ff" />
          </svg>
        </span>
      </span>
    </span>
  );
}
