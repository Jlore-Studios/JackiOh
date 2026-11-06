// R3016000 (#3016): the Collection's hover preview of an owned-cards grid card. A resting mouse or pen,
// or the keyboard's focus, opens it after HOVER_DELAY_MS, the board's own hover delay (B22): a panel
// beside the card with its look as the card, in hand and on the board (LookViews.tsx) and a Radiant
// toggle. It stays open while the pointer is on the card or on the panel, so the toggle can be reached,
// and closes LOOK_HOVER_CLOSE_MS after the pointer has left both; at once when the focus leaves, on
// Escape, a scroll or the window's blur, and on a press on the card, which is a tap or a hold of its
// own (R2730000). Touch has no hover: a long press opens the fullscreen preview (ItemPreview.tsx), which
// shows the same views. Both "Hover previews" switches govern it, as they govern the board's
// (useInspectTrigger.tsx). Presentation only (CLAUDE.md rule 7). R3016001: the same panel shows one level of a card's
// variants (`level`) where a ladder's rung is hovered, and a grid card's carries "Equip or buy" (`onLooks`), which opens
// the card's equip and buy screen (LooksScreen.tsx).

import { useEffect, useLayoutEffect, useRef, useState, type FocusEvent, type PointerEvent, type ReactElement, type RefObject } from "react";

import type { CardDef } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/constants.ts";
import { placePreview, rectOf, type Rect } from "../cards/inspect/placement.ts";
import { readCardSettings } from "../cards/settings.ts";
import { readSettings as readPanelSettings } from "../settings/store.ts";
import { collectionTestid as T } from "./collection.ts";
import LookViews, { LooksButton, RadiantToggle } from "./LookViews.tsx";

/** Once the pointer has left the card, how long the panel waits for it to arrive on the panel. */
export const LOOK_HOVER_CLOSE_MS = 200;

/** The panel's size before it is laid out, for its first placement (jsdom never lays it out). */
const LOOK_HOVER_ESTIMATE = { width: 420, height: 310 } as const;

/** The card a hover preview is open on, and where that card was when it opened. */
export type LookHovered = { id: string; anchor: Rect };

/** What a grid card spreads on itself; `onPointerDown` goes before the card's own hold (ItemPreview.tsx `usePreviewHold`). */
export type LookHoverTrigger = {
  onPointerEnter: (event: PointerEvent<HTMLElement>) => void;
  onPointerLeave: () => void;
  onPointerDown: () => void;
  onFocus: (event: FocusEvent<HTMLElement>) => void;
  onBlur: (event: FocusEvent<HTMLElement>) => void;
};

export type LookHover = {
  hovered: LookHovered | null;
  trigger: (id: string) => LookHoverTrigger;
  /** The panel's: the pointer arrived on it, so it stays. */
  stay: () => void;
  /** The pointer left the panel or the card: it closes after LOOK_HOVER_CLOSE_MS. */
  leave: () => void;
  close: () => void;
  panel: RefObject<HTMLDivElement | null>;
};

type Timer = ReturnType<typeof setTimeout>;

/** Both switches that govern the board's hover preview, read now (timers outlive a render). */
function hoverAllowed(): boolean {
  return readCardSettings().hoverPreviews && readPanelSettings().hoverPreviews;
}

function hoverPointer(pointerType: string): boolean {
  return pointerType === "" || pointerType === "mouse" || pointerType === "pen";
}

/** Focus the keyboard brought (a click's is not), read in a try/catch for an engine without `:focus-visible`. */
function focusedByKeyboard(element: Element): boolean {
  try {
    return element.matches(":focus-visible");
  } catch {
    return false;
  }
}

/** R3016000: the hover preview's state and its handlers, one panel open at a time. */
export function useLookHover(): LookHover {
  const [hovered, setHovered] = useState<LookHovered | null>(null);
  const shown = useRef<LookHovered | null>(null);
  const timers = useRef<{ open: Timer | null; close: Timer | null }>({ open: null, close: null });
  const panel = useRef<HTMLDivElement | null>(null);

  const [handlers] = useState(() => {
    const t = timers.current;
    const show = (next: LookHovered | null): void => {
      shown.current = next;
      setHovered(next);
    };
    const cancel = (): void => {
      if (t.open !== null) clearTimeout(t.open);
      if (t.close !== null) clearTimeout(t.close);
      t.open = null;
      t.close = null;
    };
    const close = (): void => {
      cancel();
      if (shown.current !== null) show(null);
    };
    const leave = (): void => {
      cancel();
      if (shown.current === null) return;
      t.close = setTimeout(() => {
        t.close = null;
        show(null);
      }, LOOK_HOVER_CLOSE_MS);
    };
    const arm = (id: string, element: HTMLElement): void => {
      cancel();
      t.open = setTimeout(() => {
        t.open = null;
        if (!element.isConnected || !hoverAllowed()) return;
        show({ id, anchor: rectOf(element) });
      }, HOVER_DELAY_MS);
    };
    const trigger = (id: string): LookHoverTrigger => ({
      onPointerEnter: (event) => {
        if (!hoverPointer(event.pointerType) || !hoverAllowed()) return;
        if (shown.current?.id === id) cancel();
        else arm(id, event.currentTarget);
      },
      onPointerLeave: leave,
      onPointerDown: close,
      // The focus may land on a button inside the trigger (a rung's Equip or Buy): it is that one the keyboard brought.
      onFocus: (event) => {
        if (!hoverAllowed() || !(event.target instanceof Element) || !focusedByKeyboard(event.target)) return;
        arm(id, event.currentTarget);
      },
      onBlur: (event) => {
        const to = event.relatedTarget;
        if (to instanceof Node && panel.current?.contains(to) === true) return;
        close();
      },
    });
    return { trigger, stay: cancel, leave, close, cancel };
  });

  // Open, the page closes it; unmounted, its timers go.
  useEffect(() => {
    if (hovered === null) return undefined;
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") handlers.close();
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("scroll", handlers.close, true);
    window.addEventListener("blur", handlers.close);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("scroll", handlers.close, true);
      window.removeEventListener("blur", handlers.close);
    };
  }, [hovered, handlers]);
  useEffect(() => handlers.cancel, [handlers]);

  return { hovered, trigger: handlers.trigger, stay: handlers.stay, leave: handlers.leave, close: handlers.close, panel };
}

/**
 * R3016000: the hover preview of `def`, beside `anchor`: its three views and a Radiant toggle, base first. R3016001:
 * `level` the one level it shows (a hovered rung's), and `onLooks` an "Equip or buy" that opens the card's screen.
 */
export function LookHoverPanel({
  def,
  owned,
  anchor,
  hover,
  level,
  onLooks,
}: {
  def: CardDef;
  owned: boolean;
  anchor: Rect;
  hover: LookHover;
  level?: string | undefined;
  onLooks?: (() => void) | undefined;
}): ReactElement {
  const [radiant, setRadiant] = useState(false);
  const placed = placePreview(anchor, { width: window.innerWidth, height: window.innerHeight }, LOOK_HOVER_ESTIMATE);

  // Once laid out, place it again by its real size, before it paints.
  useLayoutEffect(() => {
    const element = hover.panel.current;
    if (element === null) return;
    const width = element.offsetWidth;
    const height = element.offsetHeight;
    if (width === 0 || height === 0) return;
    const measured = placePreview(anchor, { width: window.innerWidth, height: window.innerHeight }, { width, height });
    element.style.left = `${String(measured.left)}px`;
    element.style.top = `${String(measured.top)}px`;
    element.dataset.side = measured.side;
  });

  return (
    <div
      ref={hover.panel}
      className="collection-look-hover"
      data-testid={T.lookHover}
      data-item={def.id}
      data-owned={String(owned)}
      data-level={level}
      data-side={placed.side}
      role="group"
      aria-label={readerName(def)}
      style={{ left: placed.left, top: placed.top }}
      onPointerEnter={hover.stay}
      onPointerLeave={hover.leave}
    >
      <div aria-hidden="true">
        <LookViews def={def} radiant={radiant} worn={owned} level={level} />
      </div>
      <div className="collection-look__actions">
        <RadiantToggle
          radiant={radiant}
          onToggle={() => {
            setRadiant((shown) => !shown);
          }}
        />
        {onLooks === undefined ? null : <LooksButton onLooks={onLooks} />}
      </div>
    </div>
  );
}
