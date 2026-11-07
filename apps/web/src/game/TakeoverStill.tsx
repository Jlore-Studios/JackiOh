// R2722001: under Reduce motion (the panel's or the system's) a Mythic's entrance is a still reveal of
// its card over the board: no shake, no flash, no sweep and no hold. The card stands centred for
// TAKEOVER_STILL_MS and is gone. It reads only a card the seat's own event names (Game.tsx asks
// `entranceEntries`, R202), takes no pointer event, carries no `data-animating` and holds nothing, so
// the queue, practice's AI and `cy.settled()` never wait for it (R200). The music still plays (R1350).
// R2803000: the still echoes the reworked takeover at rest: the dark it lands in, a motionless rays
// disc behind the card and the landing ring around it. Nothing here animates, so it stays still.
// R3402000: a Mythic whose takeover has a scene of its own (fx/takeoverArt.ts) shows it at rest behind
// all of that, painted from its look's tint (board.css) and naming no card.

import { createElement, useContext, useEffect, type CSSProperties, type ReactElement } from "react";

import type { PlayerView } from "@jackioh/shared";

import { CardFace } from "../cards/index.ts";
import { TAKEOVER_TINTS, takeoverLookOf } from "../fx/takeover.ts";
import { TAKEOVER_ART, TAKEOVER_ART_ASPECT, TAKEOVER_ART_VIEWBOX } from "../fx/takeoverArt.ts";
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

/** R3402000: the Mythic's own takeover scene at rest, or nothing when its look has none. */
function StillArt({ defId }: { defId: string }): ReactElement | null {
  const look = takeoverLookOf(defId);
  const shapes = TAKEOVER_ART[look];
  if (shapes === undefined) return null;
  const tint = TAKEOVER_TINTS[look];
  const style = { "--takeover-rim": tint.rim, "--takeover-core": tint.core, "--takeover-glow": tint.glow } as CSSProperties;
  return (
    <svg
      className="takeover-still-art"
      data-look={look}
      viewBox={TAKEOVER_ART_VIEWBOX}
      preserveAspectRatio={TAKEOVER_ART_ASPECT}
      aria-hidden="true"
      style={style}
    >
      {/* A scene is a fixed list, so a shape's place in it is a stable key. */}
      {shapes.map((shape, index) => createElement(shape.tag, { key: index, className: `takeover-still-art-${shape.part}`, ...shape.attrs }))}
    </svg>
  );
}

export default function TakeoverStill({ defId, instanceId, view, onDone }: TakeoverStillProps): ReactElement | null {
  const lookup = useContext(CatalogContext);
  useEffect(() => {
    const timer = window.setTimeout(onDone, TAKEOVER_STILL_MS);
    return () => window.clearTimeout(timer);
  }, [defId, instanceId, onDone]);
  const face = namedFace(lookup, view, { defId, instanceId, radiant: false });
  if (face === null) return null;
  return (
    <div className="takeover-still" data-testid="takeover-still" data-takeover-def={defId} aria-hidden="true">
      <StillArt defId={defId} />
      <span className="takeover-still-rays" aria-hidden="true" />
      <span className="takeover-still-card">
        <CardFace face={face} layout="full" />
      </span>
      <span className="takeover-still-ring" aria-hidden="true" />
    </div>
  );
}
