// A hero's inspect view (R1330, issue #544): the dialog a click, a long-press or Enter on either
// portrait opens. It shows the portrait large (`PortraitArt`, R1332), the portrait card's printed
// name, its title (the roster's line, which the deck builder's picker also shows), its flavour line
// (R660), and the hero's health and Armor from the view. It holds what the portrait's click opened
// before it: your emote menu, or the opponent's one-item Mute emotes menu (R643), drawn inline.
//
// It is the touch sheet's kind of overlay (cards/inspect/InspectSheet.tsx): portalled to the end of
// <body> over a scrim, focus on Close and back on close, Tab kept inside, Escape and the scrim and
// Close all close it (`useModalOverlay`). It reads only the portrait's id and the numbers the
// viewer's own `PlayerView` carries (CLAUDE.md rule 7); a portrait and its words are public (R642).

import { useId, useRef, type ReactElement } from "react";
import { createPortal } from "react-dom";

import type { EmoteGate, EmoteId, PortraitId } from "@jackioh/shared";
import { PORTRAITS } from "@jackioh/shared";

import { flavourFor } from "../cards/flavour.ts";
import { OVERLAY_ROOT_PROPS, useModalOverlay } from "../cards/inspect/store.ts";
import "../cards/inspect/inspect.css";
import { PortraitArt } from "./PortraitArt.tsx";
import { PORTRAIT_DEFS } from "./portraits.ts";
import { EmoteMenu, MuteMenu } from "./ui.tsx";
import "./portrait.css";

export const HERO_INSPECT = "hero-inspect";
export const HERO_INSPECT_SCRIM = "hero-inspect-scrim";
export const HERO_INSPECT_CLOSE = "hero-inspect-close";
export const HERO_INSPECT_NAME = "hero-inspect-name";
export const HERO_INSPECT_TITLE = "hero-inspect-title";
export const HERO_INSPECT_FLAVOUR = "hero-inspect-flavour";
export const HERO_INSPECT_HEALTH = "hero-inspect-health";
export const HERO_INSPECT_ARMOR = "hero-inspect-armor";

export type HeroInspectProps = {
  side: "you" | "opponent";
  portrait: PortraitId;
  /** The hero's true health from the view; the view draws it clamped to 0 like the badge. */
  health: number;
  armor: number;
  /** Which menu the view holds: your emotes, or the opponent's mute item (R643). */
  menu: "emotes" | "mute";
  /** R1343: the emotes your menu offers — your dealt hand, in the pool's order. */
  hand: readonly EmoteId[];
  gate: () => EmoteGate;
  muted: boolean;
  onPick: (emote: EmoteId) => void;
  onMute: () => void;
  onClose: () => void;
};

export function HeroInspect({
  side,
  portrait,
  health,
  armor,
  menu,
  hand,
  gate,
  muted,
  onPick,
  onMute,
  onClose,
}: HeroInspectProps): ReactElement {
  const closeRef = useRef<HTMLButtonElement>(null);
  const modal = useModalOverlay(onClose, closeRef);
  const nameId = useId();
  const { defId, def } = PORTRAIT_DEFS[portrait];
  const flavour = flavourFor(defId)?.flavour;

  return createPortal(
    <div className="inspect-layer hero-inspect-layer" {...OVERLAY_ROOT_PROPS}>
      <div className="inspect-scrim" data-testid={HERO_INSPECT_SCRIM} aria-hidden="true" {...modal.dismissProps} />
      <div
        className="hero-inspect"
        data-testid={HERO_INSPECT}
        data-side={side}
        data-portrait={portrait}
        role="dialog"
        aria-modal="true"
        aria-labelledby={nameId}
      >
        <span className="hero-inspect-portrait">
          <PortraitArt portrait={portrait} />
        </span>
        <span className="hero-inspect-seat">{side === "you" ? "Your hero" : "Opponent's hero"}</span>
        <h2 id={nameId} className="hero-inspect-name" data-testid={HERO_INSPECT_NAME}>
          {def.name}
        </h2>
        <p className="hero-inspect-title" data-testid={HERO_INSPECT_TITLE}>
          {PORTRAITS[portrait].flavour}
        </p>
        {flavour === undefined ? null : (
          <p className="hero-inspect-flavour" data-testid={HERO_INSPECT_FLAVOUR}>
            {flavour}
          </p>
        )}
        <dl className="hero-inspect-stats">
          <div className="hero-inspect-stat" data-testid={HERO_INSPECT_HEALTH} data-health={health}>
            <dt>Health</dt>
            <dd>{Math.max(0, health)}</dd>
          </div>
          <div className="hero-inspect-stat" data-testid={HERO_INSPECT_ARMOR} data-armor={armor}>
            <dt>Armor</dt>
            <dd>{armor}</dd>
          </div>
        </dl>
        {menu === "emotes" ? (
          <EmoteMenu inline side={side} hand={hand} gate={gate} onPick={onPick} onClose={modal.close} />
        ) : (
          <MuteMenu inline muted={muted} onMute={onMute} onClose={modal.close} />
        )}
        <button
          ref={closeRef}
          type="button"
          className="inspect-close"
          data-testid={HERO_INSPECT_CLOSE}
          {...modal.dismissProps}
        >
          Close
        </button>
      </div>
    </div>,
    document.body,
  );
}
