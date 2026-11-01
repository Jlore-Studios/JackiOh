// A hero portrait (issue §1, R641): the portrait's card drawn in `CardArt`'s `oval` shape
// (`PortraitArt`, R1332), squashing and glinting as its menu or inspect view opens (R1331). R987: Luck
// rides above the health badge. Presentation only: it lives inside the `hero-<side>` element, which
// keeps the testid, the ClickTarget and every glow, highlight and damage shake.

import { useEffect, useRef, useState, type CSSProperties, type DOMAttributes, type ReactElement, type ReactNode } from "react";

import type { PortraitId } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import { PORTRAIT_REACT_MS } from "./config.ts";
import { PortraitArt } from "./PortraitArt.tsx";

/**
 * Whether the portrait is reacting now (R1331): true for `PORTRAIT_REACT_MS` after `open` turns true,
 * never under Reduce Motion (`reduced`). Closing does not react, nor a menu turning into the view
 * (`open` stays true), nor one already open when the hook mounted.
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
  press,
  children,
}: {
  portrait: PortraitId;
  /** The hero's true health; the badge draws it clamped to 0. */
  health: number;
  armor: number;
  /** The menu or the inspect view has just opened (R1331). */
  reacting?: boolean;
  /** R987: the side's Luck (`SideView.luck`); the badge draws only above 0. */
  luck?: number;
  /**
   * R1449: the hold's pointer handlers, on the whole oval: its badges sit over the open button (the
   * health badge covers the oval's center at some sizes), so a press on one is a press on the portrait.
   */
  press?: DOMAttributes<HTMLElement>;
  /** The emote bubble/sticker and the portrait's open button mount here, hung off the oval. */
  children?: ReactNode;
}): ReactElement {
  return (
    <span
      className="hero-portrait"
      data-portrait={portrait}
      data-reacting={reacting === true ? "true" : undefined}
      style={{ "--portrait-react": `${PORTRAIT_REACT_MS}ms` } as CSSProperties}
      {...press}
    >
      <PortraitArt portrait={portrait} className="hero-portrait-art" />
      {reacting === true && <span className="hero-portrait-glint" aria-hidden="true" />}
      {/* The badges sit on the art itself (issue §1): health lower right, armor lower left. The
          elements are the same `hero-health`/`hero-armor` the board always drew — their classes,
          `data-health`/`data-armor` and clamps are unchanged, only their parent moved. */}
      <span className="hero-health" data-health={health} title={t("Health")}>
        {Math.max(0, health)}
      </span>
      {armor > 0 && (
        <span className="hero-armor" data-armor={armor} title={t("Hero armor")}>
          {armor}
        </span>
      )}
      {luck !== undefined && luck > 0 && (
        <span
          className="hero-luck"
          data-testid="hero-luck"
          data-luck={luck}
          title={t("Luck {luck}: every roll your cards make that keeps a best rolls {luck} more times", { luck: String(luck) })}
        >
          {luck}
        </span>
      )}
      {children}
    </span>
  );
}
