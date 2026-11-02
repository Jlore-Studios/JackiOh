// R2722001: under Reduce motion (the panel's or the system's) a Mythic's entrance is a still reveal of
// its card over the board: no shake, no flash, no sweep and no hold. The card stands centred for
// TAKEOVER_STILL_MS and is gone. It reads only a card the seat's own event names (Game.tsx asks
// `entranceEntries`, R202), takes no pointer event, carries no `data-animating` and holds nothing, so
// the queue, practice's AI and `cy.settled()` never wait for it (R200). The music still plays (R1350).

import { useContext, useEffect, type ReactElement } from "react";

import type { PlayerView } from "@jackioh/shared";

import { CardFace } from "../cards/index.ts";
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
  return (
    <div className="takeover-still" data-testid="takeover-still" data-takeover-def={defId} aria-hidden="true">
      <span className="takeover-still-card">
        <CardFace face={face} layout="full" />
      </span>
    </div>
  );
}
