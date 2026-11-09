// R3334000: a Dormant Starship's shipyard idles on its tile: its dock light sweeps slowly across the hull, swinging
// about its lamp, and welding sparks flicker where its newest piece went in, both where its picture draws them
// (art/stinkyscape.ts `shipyardIdleOf`, in the picture's own units, framed as CardArt frames the picture). It takes one
// of the board's moving-art slots (`MinionFace`'s ranked `useArtMotion`, COSMETIC_FOIL_ANIMATED_MAX), so past the cap a
// yard holds still, and never moves off screen, under reduced motion, with Animated foil off or with no pieces yet.
// dormant.css moves it, by transform and opacity only. Presentation only (CLAUDE.md rule 7): the list is the view's,
// public on both seats (R1852005).

import type { CSSProperties, ReactElement } from "react";

import type { StarshipPieceRef } from "../wire/generated/StarshipPieceRef.ts";
import { fmt } from "./art/hash.ts";
import { SHIPYARD_LAMP, STARSHIP_DESIGNS, starshipArtOf } from "./art/starshipHulls.ts";
import { DEGREES, FULL_TURN } from "./art/stinkyDraw.ts";
import { shipyardIdleOf } from "./art/stinkyscape.ts";
import { SHIPYARD_IDLE_SPARKS, SHIPYARD_IDLE_SWEEP_MS, SHIPYARD_IDLE_WELD_MS } from "./constants.ts";

import "./dormant.css";

/** How strongly the sweeping beam lights what it crosses, over the yard's own still cone. */
const SWEEP_FILL_OPACITY = 0.22;
/** A welding spark's radius, and how far from the weld its ring of sparks sits before they fly, in picture units. */
const SPARK_RADIUS = 0.55;
const SPARK_REACH = 1.6;
/** The sparks are the yard's accent, as its still weld's are (starshipHulls.ts `shipyardMarkup`). */
const SPARK_FILL = STARSHIP_DESIGNS.shipyard.livery.stripe;

export function ShipyardIdle({ list, moving }: { list: readonly StarshipPieceRef[]; moving: boolean }): ReactElement | null {
  if (!moving || list.length === 0) return null;
  const { lamp, cone, newest } = shipyardIdleOf(starshipArtOf(list, false, undefined, false));
  const style = {
    "--cf-sweep-ms": `${String(SHIPYARD_IDLE_SWEEP_MS)}ms`,
    "--cf-weld-ms": `${String(SHIPYARD_IDLE_WELD_MS)}ms`,
    "--cf-sparks": String(SHIPYARD_IDLE_SPARKS),
  } as CSSProperties;
  return (
    <svg
      className="cf-shipyard-idle"
      data-shipyard-motion="animated"
      viewBox="0 0 100 100"
      preserveAspectRatio="xMidYMid slice"
      aria-hidden="true"
      focusable="false"
      style={style}
    >
      <polygon
        className="cf-shipyard-sweep"
        points={cone.map((point) => `${fmt(point.x)},${fmt(point.y)}`).join(" ")}
        fill={SHIPYARD_LAMP}
        fillOpacity={SWEEP_FILL_OPACITY}
        style={{ transformOrigin: `${fmt(lamp.x)}px ${fmt(lamp.y)}px` }}
      />
      {newest === null ? null : (
        <g className="cf-shipyard-sparks" transform={`translate(${fmt(newest.x)} ${fmt(newest.y)})`}>
          {Array.from({ length: SHIPYARD_IDLE_SPARKS }, (_, k) => {
            const turn = (FULL_TURN * DEGREES * k) / SHIPYARD_IDLE_SPARKS;
            return (
              <circle
                key={k}
                cx={fmt(SPARK_REACH * Math.cos(turn))}
                cy={fmt(SPARK_REACH * Math.sin(turn))}
                r={SPARK_RADIUS}
                fill={SPARK_FILL}
                style={{ "--cf-spark": String(k) } as CSSProperties}
              />
            );
          })}
        </g>
      )}
    </svg>
  );
}
