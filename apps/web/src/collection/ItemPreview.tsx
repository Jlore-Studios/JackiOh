// R2730000 (#2730): the Collection's fullscreen preview of one held or inspected item, the
// only place a held token is spent. A hold (LONG_PRESS_MS, any pointer) or the inspect key opens
// it on any account cosmetic tile, card-cosmetic strip item or owned-cards grid card; a plain tap
// or click still equips or opens, as today, and the hold's click is swallowed. This decides
// nothing (CLAUDE.md rule 7): the preview draws the server's answers and a token's offer sends
// intent through `net/api.ts`. R2127000: Back closes it as Close does. R3016000: `usePreviewTrigger`
// adds a resting pointer's and a keyboard focus's own preview, non-modal, which closes as the pointer
// or the focus leaves; `controls` holds what the picture's preview lets the player change (its Radiant toggle).
//
// The client never enforces rules and never sees hidden information (CLAUDE.md rule 7).

import {
  useEffect,
  useLayoutEffect,
  useRef,
  type FocusEvent,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
  type ReactElement,
} from "react";

import { isInspectKey, useInspectTrigger, type InspectOverlayState } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/constants.ts";
import { rectOf, type Rect } from "../cards/inspect/placement.ts";
import { closeHoverFor, openInspect } from "../cards/inspect/store.ts";
import { t } from "../i18n/i18n.ts";
import { useBackLayer } from "../net/backLayers.ts";
import { collectionTestid as T } from "./collection.ts";

import "./collection.css";

/** Keeps Tab inside the dialog, which holds nothing but buttons. */
export function trapTab(event: KeyboardEvent<HTMLElement>): void {
  if (event.key !== "Tab") return;
  const buttons = [...event.currentTarget.querySelectorAll<HTMLElement>("button:not([disabled])")];
  const first = buttons[0];
  const last = buttons[buttons.length - 1];
  if (first === undefined || last === undefined) return;
  const active = document.activeElement;
  if (event.shiftKey && active === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
}

export type PreviewHold = {
  onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
  onContextMenu: (event: MouseEvent<HTMLElement>) => void;
  onClickCapture: (event: MouseEvent<HTMLElement>) => void;
  onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
};

const NO_OVERLAY = (): null => null;

/** R3016000: `PreviewHold`'s handlers, and those a hover preview opens and closes by. */
export type PreviewTriggerHandlers = PreviewHold & {
  onPointerEnter: (event: PointerEvent<HTMLElement>) => void;
  onPointerLeave: (event: PointerEvent<HTMLElement>) => void;
  onFocus: (event: FocusEvent<HTMLElement>) => void;
  onBlur: (event: FocusEvent<HTMLElement>) => void;
};

/** R3016000: the trigger's handlers, to spread on it, and its hover preview, to render beside it. */
export type PreviewTrigger = { handlers: PreviewTriggerHandlers; overlay: ReactElement | null };

/**
 * R2730000: a hold (LONG_PRESS_MS, any pointer) or the inspect key calls `onPreview`; the hold's click is
 * swallowed. R3016000: with `hover`, a mouse or pen resting on the trigger, or a keyboard's focus landing on it,
 * opens `hover`'s preview beside it after HOVER_DELAY_MS, as the game's hover preview opens (and with it, only
 * while the settings' hover previews are on); the pointer or the focus leaving, a press, Escape and a scroll
 * close it. A focus a press gave opens none: the press is the click's.
 */
export function usePreviewTrigger(key: string, onPreview: () => void, hover: ((anchor: Rect) => ReactElement) | null = null): PreviewTrigger {
  const inspectKey = `collection-preview-${key}`;
  const pressed = useRef<HTMLElement | null>(null);
  const pointerFocus = useRef(false);
  const focusTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { handlers, overlay } = useInspectTrigger(
    { key: inspectKey, render: hover === null ? NO_OVERLAY : ({ mode, anchor }: InspectOverlayState) => (mode === "hover" ? hover(anchor) : null) },
    { hover: hover !== null, holdAnyPointer: true, onLongPress: () => { pressed.current?.focus({ preventScroll: true }); onPreview(); } },
  );
  const clearFocusTimer = (): void => {
    if (focusTimer.current !== null) clearTimeout(focusTimer.current);
    focusTimer.current = null;
  };
  useEffect(() => clearFocusTimer, []);
  return {
    handlers: {
      onPointerEnter: handlers.onPointerEnter,
      onPointerLeave: handlers.onPointerLeave,
      onPointerDown: (event) => {
        pressed.current = event.currentTarget;
        pointerFocus.current = true;
        clearFocusTimer();
        handlers.onPointerDown(event);
      },
      onPointerMove: handlers.onPointerMove,
      onPointerUp: handlers.onPointerUp,
      onPointerCancel: handlers.onPointerCancel,
      onContextMenu: handlers.onContextMenu,
      onClickCapture: handlers.onClickCapture,
      onKeyDown: (event) => {
        if (!isInspectKey(event)) return;
        event.preventDefault();
        clearFocusTimer();
        closeHoverFor(inspectKey);
        onPreview();
      },
      onFocus: (event) => {
        const fromPress = pointerFocus.current;
        pointerFocus.current = false;
        if (hover === null || fromPress) return;
        const element = event.currentTarget;
        clearFocusTimer();
        focusTimer.current = setTimeout(() => {
          focusTimer.current = null;
          if (element.isConnected && document.activeElement === element) openInspect({ key: inspectKey, mode: "hover", anchor: rectOf(element) });
        }, HOVER_DELAY_MS);
      },
      onBlur: () => {
        pointerFocus.current = false;
        clearFocusTimer();
        closeHoverFor(inspectKey);
      },
    },
    overlay,
  };
}

/** R2730000: a hold (LONG_PRESS_MS, any pointer) or the inspect key calls `onPreview`; the hold's click is swallowed. */
export function usePreviewHold(key: string, onPreview: () => void): PreviewHold {
  const { handlers } = usePreviewTrigger(key, onPreview);
  return {
    onPointerDown: handlers.onPointerDown,
    onPointerMove: handlers.onPointerMove,
    onPointerUp: handlers.onPointerUp,
    onPointerCancel: handlers.onPointerCancel,
    onContextMenu: handlers.onContextMenu,
    onClickCapture: handlers.onClickCapture,
    onKeyDown: handlers.onKeyDown,
  };
}

/**
 * R2730000: one item fullscreen, modal, focus trapped: its picture large, its name, its rarity,
 * whether it is owned or locked, the token's offer where one covers it, and Close. R3016000: `controls`,
 * under the picture, change how it is drawn and nothing else (a card's Radiant toggle).
 */
export default function ItemPreview({
  id,
  name,
  rarity,
  owned,
  picture,
  controls = null,
  offer,
  onClose,
}: {
  id: string;
  name: string;
  rarity: string;
  owned: boolean;
  picture: ReactElement;
  controls?: ReactElement | null;
  offer: ReactElement | null;
  onClose: () => void;
}): ReactElement {
  const closeButton = useRef<HTMLButtonElement>(null);

  // Focus moves in and goes back to the item that opened the preview.
  useLayoutEffect(() => {
    const opener = document.activeElement;
    closeButton.current?.focus({ preventScroll: true });
    return () => {
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent): void => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose]);
  // R2127000: Back closes the preview as Close does, and the focus goes back as above.
  useBackLayer(onClose);

  return (
    <div
      className="collection-preview"
      data-testid={T.preview}
      data-item={id}
      data-owned={String(owned)}
      role="dialog"
      aria-modal="true"
      aria-label={name}
      onKeyDown={trapTab}
    >
      <div className="collection-preview__scrim" data-testid={T.previewScrim} aria-hidden="true" onClick={onClose} />
      <div className="collection-preview__panel">
        <div className="collection-preview__picture" aria-hidden="true">
          {picture}
        </div>
        {controls}
        <h3 className="collection-preview__name">{name}</h3>
        <p className="collection-preview__meta">{t(rarity)}</p>
        <p className="collection-preview__meta" data-testid={T.previewStatus}>
          {owned ? t("Owned") : t("Locked")}
        </p>
        {offer}
        <button type="button" ref={closeButton} data-testid={T.previewClose} onClick={onClose}>
          {t("Close")}
        </button>
      </div>
    </div>
  );
}
