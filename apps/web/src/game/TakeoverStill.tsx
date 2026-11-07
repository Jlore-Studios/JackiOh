// R2722001: under Reduce motion (the panel's or the system's) a Mythic's entrance is a still reveal of
// its card over the board: no shake, no flash, no sweep and no hold. The card stands centred for
// TAKEOVER_STILL_MS and is gone. It reads only a card the seat's own event names (Game.tsx asks
// `entranceEntries`, R202), takes no pointer event, carries no `data-animating` and holds nothing, so
// the queue, practice's AI and `cy.settled()` never wait for it (R200). The music still plays (R1350).
// R2803000: the still echoes the reworked takeover at rest: the dark it lands in, a motionless rays
// disc behind the card and the landing ring around it. Nothing here animates, so it stays still.
// R3407000: a Mythic whose takeover draws its own picture (`takeoverArtOf`) shows that picture filling the
// screen behind the rays, still, as its takeover's still form.

import { useContext, useEffect, type ReactElement } from "react";

import type { PlayerView } from "@jackioh/shared";

import { CardFace } from "../cards/index.ts";
import { takeoverArtOf } from "../fx/takeover.ts";
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

export default function TakeoverStill({ defId, instanceId, view, onDone }: TakeoverStillProps): ReactElement | null {
  const lookup = useContext(CatalogContext);
  useEffect(() => {
    const timer = window.setTimeout(onDone, TAKEOVER_STILL_MS);
    return () => window.clearTimeout(timer);
  }, [defId, instanceId, onDone]);
  const face = namedFace(lookup, view, { defId, instanceId, radiant: false });
  if (face === null) return null;
  const art = takeoverArtOf(defId);
  return (
    <div className="takeover-still" data-testid="takeover-still" data-takeover-def={defId} aria-hidden="true">
      {art === undefined ? null : <img className="takeover-still-art" src={art} alt="" aria-hidden="true" draggable={false} decoding="async" />}
      <span className="takeover-still-rays" aria-hidden="true" />
      <span className="takeover-still-card">
        <CardFace face={face} layout="full" />
      </span>
      <span className="takeover-still-ring" aria-hidden="true" />
    </div>
  );
}
