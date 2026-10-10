// A hero portrait (issue §1, R641): the portrait's card drawn in `CardArt`'s `oval` shape, with
// health badged on its lower right and armor on its lower left, the way Hearthstone mounts them.
// The oval is `PortraitArt` (R1332): the card's art under the portrait's own vivid layers. When the
// hero's inspect view opens, the oval answers with a squash and a glint (R1331), `reacting`.
// R987: Luck rides above the health badge when the side has any.
// The portrait is presentation only — it lives inside the existing `hero-<side>` element, which
// keeps the testid, the ClickTarget and every glow/highlight/damage-shake it always had, so a
// click or a drag attack lands exactly as before and no e2e spec changes.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement, type ReactNode } from "react";

import type { PortraitId } from "@jackioh/shared";

import { PORTRAIT_REACT_MS } from "./config.ts";
import { PortraitArt } from "./PortraitArt.tsx";

/**
 * Whether the portrait is reacting now (R1331): true for `PORTRAIT_REACT_MS` after `open` turns
 * true, and never under Reduce Motion (`reduced`, the panel's switch or the OS's). Closing the view
 * does not react, and a view that was already open when the hook mounted does not either.
 */
export function usePortraitReaction(open: boolean, reduced: boolean): boolean {
  const [reacting, setReacting] = useState(false);
  const wasOpen = useRef(open);
  useEffect(() => {
    const opened = open && !wasOpen.current;
    wasOpen.current = open;
    if (opened && !reduced) setReacting(true);
  }, [open, reduced]);
  useEffect(() => {
    if (!reacting) return undefined;
    const timer = window.setTimeout(() => setReacting(false), PORTRAIT_REACT_MS);
    return () => window.clearTimeout(timer);
  }, [reacting]);
  return reacting && !reduced;
}

export function HeroPortrait({
  portrait,
  health,
  armor,
  reacting,
  luck,
  children,
}: {
  portrait: PortraitId;
  /** The hero's true health; the badge draws it clamped to 0 like the hero always has. */
  health: number;
  armor: number;
  /** The inspect view has just opened: the oval squashes and a glint crosses it (R1331). */
  reacting?: boolean;
  /** R987: the side's Luck (`SideView.luck`); the badge draws only above 0. */
  luck?: number;
  /** The emote bubble/sticker and the portrait's open button mount here, hung off the oval. */
  children?: ReactNode;
}): ReactElement {
  return (
    <span
      className="hero-portrait"
      data-portrait={portrait}
      data-reacting={reacting === true ? "true" : undefined}
      style={{ "--portrait-react": `${PORTRAIT_REACT_MS}ms` } as CSSProperties}
    >
      <PortraitArt portrait={portrait} className="hero-portrait-art" />
      {reacting === true && <span className="hero-portrait-glint" aria-hidden="true" />}
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
      {luck !== undefined && luck > 0 && (
        <span
          className="hero-luck"
          data-testid="hero-luck"
          data-luck={luck}
          title={`Luck ${luck}: every roll your cards make that keeps a best rolls ${luck} more times`}
        >
          {luck}
        </span>
      )}
      {children}
    </span>
  );
}
