// One of R169's modifier badges beside a hero, readable in full on touch (#2321). The rail beside the
// portrait gives a badge a few words at most (board.css's `max-width`), so a long label is cut with an
// ellipsis, and its `title` is a hover a touch screen never shows. So the badge is a button: a tap, a
// click, Enter or Space opens its whole label in a bubble beside it, laid on the page (the hero row
// clips, and a label wrapped in place would push the row apart). It adds no key handler: the board's
// are frozen (R2126000), and Enter and Space are the button's own click. The press stops at the badge,
// so it never reaches the hero's own click (an emote menu, R1449); an attack or a drag still lands on
// the portrait. It reads the label the view gives and nothing else (§10.8, CLAUDE.md rule 7).

import { useEffect, useId, useRef, useState, type CSSProperties, type ReactElement } from "react";
import { createPortal } from "react-dom";

import type { ModifierView } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";

export function modifierFullTestid(id: string): string {
  return `modifier-full-${id}`;
}

/** The gutter the bubble keeps from the viewport's edges, and its gap from the badge (px). */
const EDGE = 16;
const GAP = 6;
/** The bubble's widest, before the viewport narrows it (px): a phone's 390 leaves 358. */
const BUBBLE_MAX = 320;

/** Where the bubble sits: under the badge in the top half of the screen, over it in the bottom half. */
function placement(badge: DOMRect): CSSProperties {
  const width = Math.min(BUBBLE_MAX, window.innerWidth - 2 * EDGE);
  const left = Math.max(EDGE, Math.min(badge.left, window.innerWidth - EDGE - width));
  const below = badge.top + badge.height / 2 < window.innerHeight / 2;
  return below
    ? { left, top: badge.bottom + GAP, maxWidth: width }
    : { left, bottom: window.innerHeight - badge.top + GAP, maxWidth: width };
}

export function ModifierBadge({ modifier }: { modifier: ModifierView }): ReactElement {
  const [at, setAt] = useState<CSSProperties | null>(null);
  const bubbleId = useId();
  const badge = useRef<HTMLButtonElement>(null);
  const bubble = useRef<HTMLSpanElement>(null);
  const label = t(modifier.label);
  const open = at !== null;

  useEffect(() => {
    if (!open) return;
    function close(): void {
      setAt(null);
    }
    function onPointer(event: PointerEvent): void {
      if (!(event.target instanceof Node)) return;
      if (badge.current?.contains(event.target) === true || bubble.current?.contains(event.target) === true) return;
      close();
    }
    window.addEventListener("pointerdown", onPointer);
    // The bubble is placed once; a scroll or a resize would leave it behind, so it goes.
    window.addEventListener("resize", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("pointerdown", onPointer);
      window.removeEventListener("resize", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [open]);

  return (
    <>
      <button
        type="button"
        ref={badge}
        className="modifier-badge"
        data-modifier-id={modifier.id}
        data-open={open ? "true" : undefined}
        title={label}
        aria-expanded={open}
        aria-controls={open ? bubbleId : undefined}
        onClick={(event) => {
          event.stopPropagation();
          const rect = badge.current?.getBoundingClientRect();
          setAt(open || rect === undefined ? null : placement(rect));
        }}
        // Focus leaving for anything but the bubble (Tab away) closes it; a touch screen may never
        // focus the button, and there the tap elsewhere closes it instead.
        onBlur={(event) => {
          if (open && !(event.relatedTarget instanceof Node && bubble.current?.contains(event.relatedTarget) === true)) setAt(null);
        }}
      >
        {label}
      </button>
      {at === null
        ? null
        : createPortal(
            // tabIndex -1: a click on the bubble focuses it, so reading it is no blur that closes it.
            <span ref={bubble} id={bubbleId} className="modifier-full" data-testid={modifierFullTestid(modifier.id)} role="note" tabIndex={-1} style={at}>
              {label}
            </span>,
            document.body,
          )}
    </>
  );
}
