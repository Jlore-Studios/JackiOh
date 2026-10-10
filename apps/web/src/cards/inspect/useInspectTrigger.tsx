// One-card hover and long-press inspect (B22–B24). The caller renders `overlay` beside its trigger
// so the trigger's click capture cannot see it. Hover is settings-gated; long press may open a
// sheet, callback or hold preview, and suppresses its following click. Custom subjects render their
// overlay; `openSheet` opens a sheet directly.

import { useEffect, useLayoutEffect, useMemo, useRef, useSyncExternalStore } from "react";
import type { MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent, ReactElement } from "react";
import type { FaceModel } from "../model.ts";
import { readCardSettings, useCardSettings } from "../settings.ts";
// The settings panel's "Hover previews" switch also gates the preview (with `hoverPreviews`).
import { readSettings as readPanelSettings, useSetting as usePanelSetting } from "../../settings/store.ts";
import { CLICK_SUPPRESS_MS, HOVER_DELAY_MS, LONG_PRESS_MS, LONG_PRESS_SLOP_PX } from "./constants.ts";
import { HoverPreview } from "./HoverPreview.tsx";
import { InspectSheet } from "./InspectSheet.tsx";
import type { PreviewPrefer, Rect } from "./placement.ts";
import { closeHoverFor, closeInspect, inspectSnapshot, openInspect, subscribeInspect } from "./store.ts";
import "./inspect.css";

export type InspectOverlayState = { mode: "hover" | "sheet"; anchor: Rect; close: () => void };

/**
 * A card: the preview and the sheet draw its face. `note` is a line they print above it, in words
 * the view gave the caller (R371: "Face down — your opponent can't see this card").
 */
export type InspectSubject = { key: string; face: FaceModel; note?: string };

/** Anything else (a pile): the caller draws the overlay for each mode. */
export type InspectRenderSubject = { key: string; render: (state: InspectOverlayState) => ReactElement | null };

export type InspectOptions = {
  /** Default true: a mouse or pen hover opens the preview (also gated by settings.hoverPreviews). */
  hover?: boolean;
  /** Default true: a touch long-press opens the sheet, or calls onLongPress when given. */
  longPress?: boolean;
  onLongPress?: () => void;
  /** Without `onLongPress`, a hold opens the default sheet or a settings-gated preview; disabled previews leave a plain tap. */
  touchHold?: "sheet" | "preview";
  /** A mouse right-click calls it and prevents the native menu. The deck builder only: the board
      leaves right-click to the drag cancel. */
  onContextMenu?: () => void;
  /** Which side of the card the hover preview tries first; "beside" (B27's order) by default. */
  prefer?: PreviewPrefer;
  /** Default true: overlays show lines of code (E36); matches pass false because it is hidden. */
  showLoc?: boolean;
};

export type InspectHandlers = {
  onPointerEnter: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerLeave: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerDown: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerMove: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerUp: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerCancel: (event: ReactPointerEvent<HTMLElement>) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLElement>) => void;
  onClickCapture: (event: ReactMouseEvent<HTMLElement>) => void;
};

export type InspectBindings = {
  handlers: InspectHandlers;
  overlay: ReactElement | null;
  open: "hover" | "sheet" | null;
  /** Opens the sheet now on `element`; a no-op for a null subject. */
  openSheet: (element: Element) => void;
};

type Timer = ReturnType<typeof setTimeout>;

type Press = { x: number; y: number; pointerId: number };

/** idle; pending: a touch is down and its timer runs; fired: the long-press happened. */
type PressState = "idle" | "pending" | "fired";

function noop(): void {}

const NO_OPTIONS: InspectOptions = {};

const NOOP_HANDLERS: InspectHandlers = {
  onPointerEnter: noop,
  onPointerLeave: noop,
  onPointerDown: noop,
  onPointerMove: noop,
  onPointerUp: noop,
  onPointerCancel: noop,
  onContextMenu: noop,
  onClickCapture: noop,
};

/** Both switches that govern the hover preview, read now (handlers and timers outlive a render). */
function hoverAllowed(): boolean {
  return readCardSettings().hoverPreviews && readPanelSettings().hoverPreviews;
}

function hoverPointer(pointerType: string | undefined): boolean {
  return pointerType === undefined || pointerType === "" || pointerType === "mouse" || pointerType === "pen";
}

function rectOf(element: Element): Rect {
  const box = element.getBoundingClientRect();
  return { left: box.left, top: box.top, width: box.width, height: box.height };
}

export function useInspectTrigger(
  subject: InspectSubject | InspectRenderSubject | null,
  options?: InspectOptions,
): InspectBindings {
  const key = subject === null ? null : subject.key;
  const settings = useCardSettings();
  const active = useSyncExternalStore(subscribeInspect, () => inspectSnapshot(key));

  // The latest props, for handlers and timers that outlive the render that made them.
  const live = useRef({ subject, options: options ?? NO_OPTIONS });
  useLayoutEffect(() => {
    live.current = { subject, options: options ?? NO_OPTIONS };
  });

  const timers = useRef<{ hover: Timer | null; press: Timer | null; suppress: Timer | null }>({
    hover: null,
    press: null,
    suppress: null,
  });
  const press = useRef<Press | null>(null);
  const pressState = useRef<PressState>("idle");
  const suppressClick = useRef(false);
  /** The key of the preview this hold opened, so its end never closes one a mouse's hover opened. */
  const holdPreview = useRef<string | null>(null);

  const handlers = useMemo<InspectHandlers>(() => {
    const t = timers.current;

    const clearHoverTimer = (): void => {
      if (t.hover !== null) clearTimeout(t.hover);
      t.hover = null;
    };
    const clearPress = (): void => {
      if (t.press !== null) clearTimeout(t.press);
      t.press = null;
      press.current = null;
      if (pressState.current === "pending") pressState.current = "idle";
    };
    const disarmSuppressor = (): void => {
      if (t.suppress !== null) clearTimeout(t.suppress);
      t.suppress = null;
      suppressClick.current = false;
      if (pressState.current === "fired") pressState.current = "idle";
    };
    const armSuppressor = (): void => {
      if (t.suppress !== null) clearTimeout(t.suppress);
      suppressClick.current = true;
      t.suppress = setTimeout(disarmSuppressor, CLICK_SUPPRESS_MS);
    };
    /** Ends the preview this hold opened (a no-op for any other preview or when none did). */
    const closeHoldPreview = (): void => {
      const opened = holdPreview.current;
      if (opened === null) return;
      holdPreview.current = null;
      closeHoverFor(opened);
    };

    return {
      onPointerEnter: (event) => {
        const { subject: current, options: opts } = live.current;
        if (current === null || opts.hover === false || !hoverAllowed()) return;
        if (!hoverPointer(event.pointerType)) return;
        clearHoverTimer();
        const element = event.currentTarget;
        const hoverKey = current.key;
        t.hover = setTimeout(() => {
          t.hover = null;
          const now = live.current;
          if (now.subject === null || now.subject.key !== hoverKey) return;
          if (now.options.hover === false || !hoverAllowed() || !element.isConnected) return;
          openInspect({ key: hoverKey, mode: "hover", anchor: rectOf(element) });
        }, HOVER_DELAY_MS);
      },

      onPointerLeave: () => {
        clearHoverTimer();
        const current = live.current.subject;
        if (current !== null) closeHoverFor(current.key);
      },

      onPointerDown: (event) => {
        clearHoverTimer();
        const { subject: current, options: opts } = live.current;
        if (current === null) return;
        closeHoverFor(current.key);
        holdPreview.current = null;
        disarmSuppressor();
        clearPress();
        if (event.pointerType !== "touch" || opts.longPress === false) return;
        const element = event.currentTarget;
        const pressKey = current.key;
        press.current = { x: event.clientX, y: event.clientY, pointerId: event.pointerId };
        pressState.current = "pending";
        t.press = setTimeout(() => {
          t.press = null;
          const now = live.current;
          if (now.subject === null || now.subject.key !== pressKey || !element.isConnected) {
            press.current = null;
            pressState.current = "idle";
            return;
          }
          const onLongPress = now.options.onLongPress;
          const previewHold = onLongPress === undefined && now.options.touchHold === "preview";
          if (previewHold && !hoverAllowed()) {
            // Disabled previews leave a plain tap, without swallowing its click.
            press.current = null;
            pressState.current = "idle";
            return;
          }
          pressState.current = "fired";
          armSuppressor();
          if (previewHold) {
            // `press` keeps the start point, so a drift past the slop still ends the look.
            holdPreview.current = pressKey;
            openInspect({ key: pressKey, mode: "hover", anchor: rectOf(element) });
            return;
          }
          press.current = null;
          if (onLongPress !== undefined) onLongPress();
          else openInspect({ key: pressKey, mode: "sheet", anchor: rectOf(element) });
        }, LONG_PRESS_MS);
      },

      onPointerMove: (event) => {
        const start = press.current;
        if (start === null) return;
        if (event.pointerId !== start.pointerId) return;
        const dx = event.clientX - start.x;
        const dy = event.clientY - start.y;
        if (Math.hypot(dx, dy) <= LONG_PRESS_SLOP_PX) return;
        // Past the slop is a drag: cancel the press and its open preview.
        closeHoldPreview();
        clearPress();
      },

      onPointerUp: () => {
        closeHoldPreview();
        press.current = null;
        if (pressState.current === "pending") clearPress();
        // The lift's click is on the way, so the swallow window runs from here.
        else if (pressState.current === "fired" && suppressClick.current) armSuppressor();
      },

      onPointerCancel: () => {
        closeHoldPreview();
        clearPress();
        clearHoverTimer();
      },

      onContextMenu: (event) => {
        // Newer browsers send contextmenu as a PointerEvent.
        const native: Event = event.nativeEvent;
        const nativeType = "pointerType" in native ? (native as PointerEvent).pointerType : undefined;
        if (pressState.current !== "idle" || nativeType === "touch") {
          event.preventDefault();
          return;
        }
        const { subject: current, options: opts } = live.current;
        if (current === null || opts.onContextMenu === undefined) return;
        event.preventDefault();
        clearHoverTimer();
        closeHoverFor(current.key);
        opts.onContextMenu();
      },

      onClickCapture: (event) => {
        if (!suppressClick.current) return;
        event.preventDefault();
        event.stopPropagation();
        disarmSuppressor();
      },
    };
  }, []);

  const openSheet = useMemo(
    () =>
      (element: Element): void => {
        const current = live.current.subject;
        if (current === null) return;
        openInspect({ key: current.key, mode: "sheet", anchor: rectOf(element) });
      },
    [],
  );

  useEffect(() => {
    const t = timers.current;
    return () => {
      if (t.hover !== null) clearTimeout(t.hover);
      if (t.press !== null) clearTimeout(t.press);
      if (t.suppress !== null) clearTimeout(t.suppress);
      t.hover = null;
      t.press = null;
      t.suppress = null;
      press.current = null;
      pressState.current = "idle";
      suppressClick.current = false;
      holdPreview.current = null;
    };
  }, []);

  // A card that unmounts, or changes key, takes its overlay with it.
  useEffect(() => {
    if (key === null) return;
    return () => closeInspect(key);
  }, [key]);

  const mode = active === null ? null : active.mode;
  const panelHover = usePanelSetting("hoverPreviews");
  const hoverPreviews = settings.hoverPreviews && panelHover;

  // The page closes open previews; captured scroll includes a scrolled zone.
  useEffect(() => {
    if (key === null || mode !== "hover") return;
    if (!hoverPreviews) {
      closeHoverFor(key);
      return;
    }
    const close = (): void => closeHoverFor(key);
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") close();
    };
    const onVisibility = (): void => {
      if (document.visibilityState === "hidden") close();
    };
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("pointerdown", close, true);
    window.addEventListener("blur", close);
    window.addEventListener("scroll", close, true);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("pointerdown", close, true);
      window.removeEventListener("blur", close);
      window.removeEventListener("scroll", close, true);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [key, mode, hoverPreviews]);

  let overlay: ReactElement | null = null;
  if (subject !== null && active !== null) {
    const closeKey = subject.key;
    const close = (): void => closeInspect(closeKey);
    if ("render" in subject) {
      overlay = subject.render({ mode: active.mode, anchor: active.anchor, close });
    } else {
      const face = options?.showLoc === false ? { ...subject.face, loc: null } : subject.face;
      overlay =
        active.mode === "hover" ? (
          <HoverPreview face={face} anchor={active.anchor} prefer={options?.prefer} note={subject.note} />
        ) : (
          <InspectSheet face={face} onClose={close} note={subject.note} />
        );
    }
  }

  return { handlers: subject === null ? NOOP_HANDLERS : handlers, overlay, open: mode, openSheet };
}
