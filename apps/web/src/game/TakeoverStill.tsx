// R2722001: under Reduce motion (the panel's or the system's) a Mythic's entrance is a still reveal of
// its card over the board: no shake, no flash, no sweep and no hold. The card stands centred for
// TAKEOVER_STILL_MS and is gone. It reads only a card the seat's own event names (Game.tsx asks
// `entranceEntries`, R202), takes no pointer event, carries no `data-animating` and holds nothing, so
// the queue, practice's AI and `cy.settled()` never wait for it (R200). The music still plays (R1350).
// R2803000: the still echoes the reworked takeover at rest: the dark it lands in, a motionless rays
// disc behind the card and the landing ring around it. Nothing here animates, so it stays still.
// R3400000: a Mythic whose own takeover look draws art (fx/takeoverArt.ts: Bob's Plan's summit, Scratch
// Pad's canvas, Goetzmann Game Design's napkin) shows that art at rest behind the card, the same parts
// the live takeover mounts, painted by fx.css and never a card face (R202).
// R3389000: it wears its card's own takeover look (`data-takeover-look`), In Too Deep's abyss among them.
// R3407000: a Mythic whose takeover draws its own picture (`takeoverArtOf`) shows that picture filling the
// screen behind the rays, still, as its takeover's still form.
// R3399002: a Mythic whose look is drawn as a picture of its own (fx/takeoverArt.ts `TAKEOVER_ART_DRAWINGS`:
// Classic+'s four) shows that drawing at rest in the same art layer, filling the screen behind the card; it
// carries its look, never the card it is for. R3402000: and Core's four Mythics' scenes of shapes, each shape
// classed by its part as the live takeover's are, painted by fx.css from its look's tint, which the art layer carries.

import { createElement, useContext, useEffect, type CSSProperties, type ReactElement } from "react";

import type { PlayerView } from "@jackioh/shared";

import { CardFace } from "../cards/index.ts";
import { TAKEOVER_TINTS, takeoverArtOf, takeoverLookOf } from "../fx/takeover.ts";
import { TAKEOVER_ART_DRAWINGS, takeoverArtParts, type TakeoverDrawing } from "../fx/takeoverArt.ts";
import type { TakeoverLook } from "../fx/types.ts";
import { CatalogContext } from "./catalog.ts";
import { namedFace } from "./faces.ts";

/** R2722001: how long the still card stands over the board under Reduce motion. */
export const TAKEOVER_STILL_MS = 1_600;

export type TakeoverStillProps = {
  defId: string;
  instanceId: string;
  /** The view the card is read against, so a card it still lists is drawn as it stands. */
  view: PlayerView;
  /** Called once the still has stood its time. */
  onDone: () => void;
};

/** R3399002: a look's drawing at rest, its paths in their own colours; R3402000: or its scene's shapes, each classed by its part. */
function StillDrawing({ drawing }: { drawing: TakeoverDrawing }): ReactElement {
  return (
    <svg className="takeover-art-drawing" viewBox={drawing.viewBox} preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      {/* A drawing is a fixed list, so a shape's place in it is a stable key. */}
      {"paths" in drawing
        ? drawing.paths.map((path, index) => <path key={index} d={path.d} fill={path.fill} />)
        : drawing.shapes.map((shape, index) => createElement(shape.tag, { key: index, className: `takeover-drawing-${shape.part}`, ...shape.attrs }))}
    </svg>
  );
}

/** R3402000: a scene is painted from its look's tint, so its art layer carries the tint as the live takeover does. */
function sceneTint(look: TakeoverLook, drawing: TakeoverDrawing | undefined): CSSProperties | undefined {
  if (drawing === undefined || "paths" in drawing) return undefined;
  const tint = TAKEOVER_TINTS[look];
  return { "--fx-tint-rim": tint.rim, "--fx-tint-core": tint.core, "--fx-tint-glow": tint.glow } as CSSProperties;
}

export default function TakeoverStill({ defId, instanceId, view, onDone }: TakeoverStillProps): ReactElement | null {
  const lookup = useContext(CatalogContext);
  useEffect(() => {
    const timer = window.setTimeout(onDone, TAKEOVER_STILL_MS);
    return () => window.clearTimeout(timer);
  }, [defId, instanceId, onDone]);
  const face = namedFace(lookup, view, { defId, instanceId, radiant: false });
  if (face === null) return null;
  const look = takeoverLookOf(defId);
  const art = takeoverArtParts(look);
  const drawing = TAKEOVER_ART_DRAWINGS[look];
  const picture = takeoverArtOf(defId);
  return (
    <div className="takeover-still" data-testid="takeover-still" data-takeover-def={defId} data-takeover-look={look} aria-hidden="true">
      {picture === undefined ? null : <img className="takeover-still-art" src={picture} alt="" aria-hidden="true" draggable={false} decoding="async" />}
      {art.length > 0 || drawing !== undefined ? (
        <span className="takeover-art" data-look={look} data-still="true" aria-hidden="true" style={sceneTint(look, drawing)}>
          {art.map((part) => (
            <span key={part} className={`takeover-art-${part}`} />
          ))}
          {drawing === undefined ? null : <StillDrawing drawing={drawing} />}
        </span>
      ) : null}
      <span className="takeover-still-rays" aria-hidden="true" />
      <span className="takeover-still-card">
        <CardFace face={face} layout="full" />
      </span>
      <span className="takeover-still-ring" aria-hidden="true" />
    </div>
  );
}
