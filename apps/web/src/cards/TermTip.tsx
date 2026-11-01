// A glossary term in a card's text shows its glossary line where the face takes the pointer
// (R2098000, issue #2098): on the board and in the hand, in the Almanac's and the deck builder's
// pool, and in the inspect views (the detail view, the touch sheet, a pile's dialog). A mouse or
// pen resting on the term shows it after TERM_HOVER_DELAY_MS; a touch held on it for LONG_PRESS_MS
// shows it too, and an ordinary tap still plays, aims, selects or opens the card, since the term's
// long press takes over from the card's and its closing click goes nowhere. The tooltip takes no
// pointer events, so it never blocks play. Where the surface allows a control the term is a tab
// stop: focus shows the tooltip, which is `role="tooltip"` and linked by `aria-describedby`, and
// blur or Escape closes it. Outside these surfaces terms print plain, as before.

import {
  createContext,
  useContext,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type ReactElement,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

import { TERM_HOVER_DELAY_MS } from "./constants.ts";
import { GLOSSARY, type GlossaryTermId } from "./glossary.ts";
import { LONG_PRESS_MS, LONG_PRESS_SLOP_PX } from "./inspect/constants.ts";
import { placePreview, rectOf, type Rect } from "./inspect/placement.ts";
import { useRefsInteractive } from "./refContext.tsx";

export type TermTipMode = "off" | "pointer" | "control";

export const TERM_TOOLTIP_TESTID = "card-term-tooltip";

const TERM_TIP_SELECTOR = ".cf-term[data-tip]";

const ModeContext = createContext<TermTipMode>("off");

type Timer = ReturnType<typeof setTimeout>;

/** The surfaces below make the terms they print show their rule; `focusable` also tabs to them. */
export function TermTips({ focusable = false, children }: { focusable?: boolean; children: ReactNode }): ReactElement {
  return <ModeContext.Provider value={focusable ? "control" : "pointer"}>{children}</ModeContext.Provider>;
}

/**
 * R2098000: where references are controls (CardDetail, InspectSheet) terms are controls too.
 * Where references are switched off (RefTooltip, CardHistory, References, the hover preview)
 * terms fall back to the context's mode, which is "off" everywhere those occur today.
 */
function useTermTipMode(): TermTipMode {
  const mode = useContext(ModeContext);
  return useRefsInteractive() ? "control" : mode;
}

/** Whether a press starts on a term that shows its rule: then it is the term's long press. */
export function startsOnTermTip(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(TERM_TIP_SELECTOR) !== null;
}

function viewport(): { width: number; height: number } {
  return { width: window.innerWidth, height: window.innerHeight };
}

function TermTooltip({ id, term, anchor }: { id: string; term: GlossaryTermId; anchor: Rect }): ReactElement {
  const ref = useRef<HTMLDivElement>(null);
  const entry = GLOSSARY[term];
  const placed = placePreview(anchor, viewport(), { width: 0, height: 0 }, "above");

  // Once laid out, place it again by its real size. jsdom has no layout and keeps the estimate.
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) return;
    const width = element.offsetWidth;
    const height = element.offsetHeight;
    if (width === 0 || height === 0) return;
    const measured = placePreview(anchor, viewport(), { width, height }, "above");
    element.style.left = `${String(measured.left)}px`;
    element.style.top = `${String(measured.top)}px`;
  });

  return createPortal(
    <div
      ref={ref}
      id={id}
      role="tooltip"
      className="cf-term-tip"
      data-testid={TERM_TOOLTIP_TESTID}
      data-term={term}
      style={{ position: "fixed", left: placed.left, top: placed.top }}
    >
      <strong className="cf-term-tip-label">{entry.label}</strong> {entry.rule}
    </div>,
    document.body,
  );
}

function LiveTerm({ term, mode, children }: { term: GlossaryTermId; mode: "pointer" | "control"; children: ReactNode }): ReactElement {
  const control = mode === "control";
  const tooltipId = useId();
  const ref = useRef<HTMLElement>(null);
  const hoverTimer = useRef<Timer | null>(null);
  const press = useRef<{ timer: Timer; release: () => void } | null>(null);
  const fired = useRef(false);
  const pressed = useRef(false);
  const [anchor, setAnchor] = useState<Rect | null>(null);

  const clearHover = (): void => {
    if (hoverTimer.current !== null) clearTimeout(hoverTimer.current);
    hoverTimer.current = null;
  };
  const endPress = (): void => {
    const pending = press.current;
    if (pending === null) return;
    press.current = null;
    clearTimeout(pending.timer);
    pending.release();
  };
  const open = (): void => {
    const element = ref.current;
    if (element !== null && element.isConnected) setAnchor(rectOf(element));
  };
  const close = (): void => {
    clearHover();
    endPress();
    setAnchor(null);
  };

  useEffect(() => () => {
    if (hoverTimer.current !== null) clearTimeout(hoverTimer.current);
    hoverTimer.current = null;
    const pending = press.current;
    press.current = null;
    if (pending !== null) {
      clearTimeout(pending.timer);
      pending.release();
    }
  }, []);

  // Scroll moves an open tooltip with its term. A press anywhere closes it, the term included.
  const isOpen = anchor !== null;
  useLayoutEffect(() => {
    if (!isOpen) return undefined;
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") close();
    };
    const onPress = (): void => {
      close();
    };
    const onMove = (): void => {
      const element = ref.current;
      if (element !== null && element.isConnected) setAnchor(rectOf(element));
      else close();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("pointerdown", onPress, true);
    window.addEventListener("scroll", onMove, true);
    window.addEventListener("resize", onMove);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("pointerdown", onPress, true);
      window.removeEventListener("scroll", onMove, true);
      window.removeEventListener("resize", onMove);
    };
  }, [isOpen]);

  const startPress = (event: ReactPointerEvent<HTMLElement>): void => {
    const startX = event.clientX;
    const startY = event.clientY;
    const pointerId = event.pointerId;
    // DragLayer captures the pointer on the card at 8 px, so the term would no longer see its
    // own moves: listen on the window while the press is pending.
    const onMove = (move: PointerEvent): void => {
      if (move.pointerId !== pointerId) return;
      if (Math.hypot(move.clientX - startX, move.clientY - startY) > LONG_PRESS_SLOP_PX) endPress();
    };
    const onUp = (up: PointerEvent): void => {
      if (up.pointerId !== pointerId) return;
      endPress();
    };
    const release = (): void => {
      window.removeEventListener("pointermove", onMove, true);
      window.removeEventListener("pointerup", onUp, true);
      window.removeEventListener("pointercancel", onUp, true);
    };
    window.addEventListener("pointermove", onMove, true);
    window.addEventListener("pointerup", onUp, true);
    window.addEventListener("pointercancel", onUp, true);
    press.current = {
      timer: setTimeout(() => {
        endPress();
        fired.current = true;
        open();
      }, LONG_PRESS_MS),
      release,
    };
  };

  return (
    <>
      <strong
        ref={ref}
        className="cf-term"
        data-term={term}
        data-tip={mode}
        data-tip-open={anchor === null ? undefined : "true"}
        tabIndex={control ? 0 : undefined}
        aria-describedby={anchor === null ? undefined : tooltipId}
        onPointerEnter={(event) => {
          if (event.pointerType === "touch") return;
          clearHover();
          hoverTimer.current = setTimeout(open, TERM_HOVER_DELAY_MS);
        }}
        onPointerLeave={(event) => {
          if (event.pointerType === "touch") return;
          close();
        }}
        onPointerDown={(event) => {
          pressed.current = true;
          fired.current = false;
          endPress();
          if (event.pointerType === "touch") startPress(event);
        }}
        onContextMenu={(event) => {
          if (press.current !== null || fired.current) {
            event.preventDefault();
            event.stopPropagation();
          }
        }}
        onClick={(event) => {
          // The click the long press ends with goes nowhere; any other click reaches the card.
          if (fired.current) {
            fired.current = false;
            event.preventDefault();
            event.stopPropagation();
          }
        }}
        onFocus={() => {
          if (control && !pressed.current) open();
        }}
        onBlur={() => {
          if (!control) return;
          pressed.current = false;
          close();
        }}
      >
        {children}
      </strong>
      {anchor === null ? null : <TermTooltip id={tooltipId} term={term} anchor={anchor} />}
    </>
  );
}

/** A glossary term in a card's text: plain everywhere, showing its rule where the surface allows. */
export function TermTip({ term, children }: { term: GlossaryTermId; children: ReactNode }): ReactElement {
  const mode = useTermTipMode();
  if (mode === "off") return <strong className="cf-term" data-term={term}>{children}</strong>;
  return (
    <LiveTerm term={term} mode={mode}>
      {children}
    </LiveTerm>
  );
}
