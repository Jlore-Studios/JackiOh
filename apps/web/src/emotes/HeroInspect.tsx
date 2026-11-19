// A hero's inspect view (R1330, issue #544): the dialog a hold on either portrait opens, or a
// right-click, the context-menu key or Shift+F10 on it (R1449; a click hangs the menu off the
// portrait instead). It shows the portrait large (`PortraitArt`, R1332), its printed words (R660),
// the hero's health and Armor and every other counter above 0 (#4251), and holds the click's
// menu inline (R643).
// It is the touch sheet's kind of overlay (`useModalOverlay`, cards/inspect/InspectSheet.tsx). It
// reads only the portrait's id and the viewer's own `PlayerView` (CLAUDE.md rule 7); a portrait and
// its words are public (R642).

import { useId, useRef, type ReactElement } from "react";
import { createPortal } from "react-dom";

import type { EmoteGate, EmoteId, PortraitId } from "@jackioh/shared";
import { PORTRAITS } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { flavourFor } from "../cards/flavour.ts";
import { useModalOverlay } from "../cards/inspect/store.ts";
import "../cards/inspect/inspect.css";
import { DialogClose } from "../game/Dialog.tsx";
import { t } from "../i18n/i18n.ts";
import { PortraitArt } from "./PortraitArt.tsx";
import { portraitDefs } from "./portraits.ts";
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
export const HERO_INSPECT_LUCK = "hero-inspect-luck";
export const HERO_INSPECT_JADE = "hero-inspect-jade";
export const HERO_INSPECT_BURN = "hero-inspect-burn";
export const HERO_INSPECT_TOXIC = "hero-inspect-toxic";
export const HERO_INSPECT_TIPSY = "hero-inspect-tipsy";
export const HERO_INSPECT_CREDITS = "hero-inspect-credits";

export type HeroInspectProps = {
  side: "you" | "opponent";
  portrait: PortraitId;
  /** The hero's true health from the view; the view draws it clamped to 0 like the badge. */
  health: number;
  armor: number;
  /** #4251: the seat's counters, each drawn only above 0 (Credits whenever the view carries
      them, 0 included, like the badge). */
  luck?: number;
  jade?: number;
  burn?: number;
  toxic?: number;
  tipsy?: number;
  marketCredits?: number;
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
  luck,
  jade,
  burn,
  toxic,
  tipsy,
  marketCredits,
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
  const { defId, def } = portraitDefs()[portrait];
  const flavour = flavourFor(defId)?.flavour;

  return createPortal(
    <div className="inspect-layer hero-inspect-layer" {...modal.layerProps}>
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
        <span className="hero-inspect-seat">{side === "you" ? t("Your hero") : t("Opponent's hero")}</span>
        <h2 id={nameId} className="hero-inspect-name" data-testid={HERO_INSPECT_NAME}>
          {readerName(def)}
        </h2>
        <p className="hero-inspect-title" data-testid={HERO_INSPECT_TITLE}>
          {t(PORTRAITS[portrait].flavour)}
        </p>
        {flavour === undefined ? null : (
          <p className="hero-inspect-flavour" data-testid={HERO_INSPECT_FLAVOUR}>
            {flavour}
          </p>
        )}
        <dl className="hero-inspect-stats">
          <div className="hero-inspect-stat" data-testid={HERO_INSPECT_HEALTH} data-health={health}>
            <dt>{t("Health")}</dt>
            <dd>{Math.max(0, health)}</dd>
          </div>
          <div className="hero-inspect-stat" data-testid={HERO_INSPECT_ARMOR} data-armor={armor}>
            <dt>{t("Armor")}</dt>
            <dd>{armor}</dd>
          </div>
          {(luck ?? 0) > 0 && (
            <div className="hero-inspect-stat" data-testid={HERO_INSPECT_LUCK} data-luck={luck}>
              <dt>{t("Luck")}</dt>
              <dd>{luck}</dd>
            </div>
          )}
          {(jade ?? 0) > 0 && (
            <div className="hero-inspect-stat" data-testid={HERO_INSPECT_JADE} data-jade={jade}>
              <dt>{t("Jade Counter")}</dt>
              <dd>{jade}</dd>
            </div>
          )}
          {(burn ?? 0) > 0 && (
            <div className="hero-inspect-stat" data-testid={HERO_INSPECT_BURN} data-burn={burn}>
              <dt>{t("Burn")}</dt>
              <dd>{burn}</dd>
            </div>
          )}
          {(toxic ?? 0) > 0 && (
            <div className="hero-inspect-stat" data-testid={HERO_INSPECT_TOXIC} data-toxic={toxic}>
              <dt>{t("Toxic")}</dt>
              <dd>{toxic}</dd>
            </div>
          )}
          {(tipsy ?? 0) > 0 && (
            <div className="hero-inspect-stat" data-testid={HERO_INSPECT_TIPSY} data-tipsy={tipsy}>
              <dt>{t("Tipsy")}</dt>
              <dd>{tipsy}</dd>
            </div>
          )}
          {marketCredits !== undefined && (
            <div
              className="hero-inspect-stat"
              data-testid={HERO_INSPECT_CREDITS}
              data-credits={marketCredits}
            >
              <dt>{t("Credits")}</dt>
              <dd>{marketCredits}</dd>
            </div>
          )}
        </dl>
        {menu === "emotes" ? (
          <EmoteMenu inline side={side} hand={hand} gate={gate} onPick={onPick} onClose={modal.close} />
        ) : (
          <MuteMenu inline muted={muted} onMute={onMute} onClose={modal.close} />
        )}
        {/* R3221000: the one ✕ every dialog closes by, at the top right. */}
        <DialogClose buttonRef={closeRef} testid={HERO_INSPECT_CLOSE} {...modal.dismissProps} />
      </div>
    </div>,
    document.body,
  );
}
