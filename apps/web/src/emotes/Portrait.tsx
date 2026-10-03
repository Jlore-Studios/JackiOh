// A hero portrait (issue §1, R640): the portrait's card drawn in `CardArt`'s `oval` shape, with
// health badged on its lower right and armor on its lower left, the way Hearthstone mounts them.
// The portrait is presentation only — it lives inside the existing `hero-<side>` element, which
// keeps the testid, the ClickTarget and every glow/highlight/damage-shake it always had, so a
// click or a drag attack lands exactly as before and no e2e spec changes.

import type { ReactElement, ReactNode } from "react";

import type { PortraitId } from "@jackioh/shared";

import { CardArt } from "../cards/art/CardArt.tsx";
import { PORTRAIT_DEFS } from "./portraits.ts";

export function HeroPortrait({
  portrait,
  health,
  armor,
  children,
}: {
  portrait: PortraitId;
  /** The hero's true health; the badge draws it clamped to 0 like the hero always has. */
  health: number;
  armor: number;
  /** The emote bubble/sticker and menus mount here, hung off the oval. */
  children?: ReactNode;
}): ReactElement {
  const { defId, def } = PORTRAIT_DEFS[portrait];
  return (
    <span className="hero-portrait" data-portrait={portrait}>
      <CardArt
        defId={defId}
        radiant={false}
        tags={def.tags}
        type={def.type}
        shape="oval"
        name={def.name}
        className="hero-portrait-art"
      />
      {/* The badges sit on the art itself (issue §1): health lower right, armor lower left. The
          elements are the same `hero-health`/`hero-armor` the board always drew — their classes,
          `data-health`/`data-armor` and clamps are unchanged, only their parent moved. */}
      <span className="hero-health" data-health={health} title="Health">
        {Math.max(0, health)}
      </span>
      {armor > 0 && (
        <span className="hero-armor" data-armor={armor} title="Hero armor">
          {armor}
        </span>
      )}
      {children}
    </span>
  );
}
