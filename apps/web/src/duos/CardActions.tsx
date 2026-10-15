// The card bar and the ping markers (#936 B62, B64), drawn over whichever board the Duos board shows.
//
// THE BAR. Pointing at a hero, a zone or a hand card (a mouse over it, a tap on it, the keyboard's
// focus on it) puts a small bar on it: the ping wheel (`duos-ping-wheel`, the five pings) and, on a
// card of the seat's own hand, the pass control (`duos-pass`) with its cost and the passes left. The
// pass is live exactly when `legal` lists that card's `pass` (B19, rule 7); otherwise it is greyed and
// sends nothing. The bar stays DUOS_HOVER_GRACE_MS after the pointer leaves its card, so the pointer
// can reach it, and goes on Escape, on a press elsewhere, after a pick, and while a card is dragged.
//
// THE MARKERS. A ping marks its place (`duos-ping-marker`) on this board when it names this board's
// duel; the caller keeps each one PING_SHOW_MS.
//
// Game.tsx stays the one board: nothing here is inside it. The bar and the markers are laid over the
// board in its container, at the box of the element the board draws for the spot (`spots.ts`),
// re-measured every frame while any is up, since cards move as the board animates.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactElement } from "react";

import type { PlayerView } from "@jackioh/shared";

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { PING_KINDS, type PingKind, type PingTarget } from "../net/duos.ts";
import { DUOS_HOVER_GRACE_MS } from "./config.ts";
import { PING_GLYPH, PING_LABEL } from "./pings.ts";
import { seatLabel } from "./seats.ts";
import { sameSpot, spotOfElement, spotTestid, targetOfSpot, type BoardSpot } from "./spots.ts";
import { duosTestid } from "./testids.ts";

/** What the pass control shows on the seat's own hand cards (B62). */
export type PassOffer = {
  /** The hand cards `legal` lists a `pass` for, by instance id: only these are live. */
  legal: ReadonlySet<string>;
  /** The team's `pass_cost`, in mana. */
  cost: number;
  /** `pass_cap` less this round's passes (B22). */
  left: number;
};

/** One ping to mark on this board. */
export type PingMarker = { key: string; spot: BoardSpot; ping: PingKind; from: DuosSeat; mine: boolean };

export type CardActionsProps = {
  /**
   * The element the board is drawn in, once it is mounted; the bar and the markers are positioned
   * inside it. An element, not a ref: a parent's ref is attached after its children's layout effects
   * run, so a ref read there on the first render would be empty and a ping already up would wait a
   * frame for its marker.
   */
  container: HTMLDivElement | null;
  /** The view the board is showing, and the duel it is of. */
  view: PlayerView;
  duel: 0 | 1;
  /** Whether the bar is offered at all: not once the game is over. */
  enabled: boolean;
  /** The pass control, on the seat's own board; null on the teammate's. */
  pass: PassOffer | null;
  onPass: (instanceId: string) => void;
  onPing: (ping: PingKind, target: PingTarget) => void;
  markers: readonly PingMarker[];
};

type Box = { left: number; top: number; width: number; height: number };

const BAR_CLASS = "duos-card-actions";
const NO_BOXES: ReadonlyMap<string, Box> = new Map();

function onBar(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(`.${BAR_CLASS}`) !== null;
}

function elementOf(target: EventTarget | null): Element | null {
  return target instanceof Element ? target : null;
}

function sameBoxes(a: ReadonlyMap<string, Box>, b: ReadonlyMap<string, Box>): boolean {
  if (a.size !== b.size) return false;
  for (const [id, box] of a) {
    const other = b.get(id);
    if (other === undefined) return false;
    if (other.left !== box.left || other.top !== box.top || other.width !== box.width || other.height !== box.height) {
      return false;
    }
  }
  return true;
}

/** `[data-testid="…"]` for an id read off the board, quoted as DragLayer quotes it. */
function byTestid(id: string): string {
  return `[data-testid="${id.replace(/["\\]/g, "\\$&")}"]`;
}

/**
 * The boxes of the elements `idsKey` names ("|"-joined testids), relative to `container`, kept
 * current every animation frame while there are any.
 */
function useBoxes(host: HTMLDivElement | null, idsKey: string): ReadonlyMap<string, Box> {
  const [boxes, setBoxes] = useState<ReadonlyMap<string, Box>>(NO_BOXES);
  useLayoutEffect(() => {
    if (idsKey === "") {
      setBoxes(NO_BOXES);
      return undefined;
    }
    const ids = idsKey.split("|");
    let live = true;
    let frame: number | null = null;
    const measure = (): void => {
      if (!live) return;
      if (host !== null) {
        const origin = host.getBoundingClientRect();
        const next = new Map<string, Box>();
        for (const id of ids) {
          const element = host.querySelector(byTestid(id));
          if (element === null) continue;
          const rect = element.getBoundingClientRect();
          next.set(id, { left: rect.left - origin.left, top: rect.top - origin.top, width: rect.width, height: rect.height });
        }
        setBoxes((previous) => (sameBoxes(previous, next) ? previous : next));
      }
      // As the tutorial's coach measures (Coach.tsx): every frame where there are frames.
      frame = typeof requestAnimationFrame === "function" ? requestAnimationFrame(measure) : null;
    };
    measure();
    return () => {
      live = false;
      if (frame !== null && typeof cancelAnimationFrame === "function") cancelAnimationFrame(frame);
    };
  }, [host, idsKey]);
  return boxes;
}

export function CardActions({
  container,
  view,
  duel,
  enabled,
  pass,
  onPass,
  onPing,
  markers,
}: CardActionsProps): ReactElement {
  useLocale();
  const [focus, setFocus] = useState<BoardSpot | null>(null);
  const clearTimer = useRef<number | null>(null);

  useEffect(() => {
    const host = container;
    if (host === null) return undefined;
    const cancelClear = (): void => {
      if (clearTimer.current === null) return;
      window.clearTimeout(clearTimer.current);
      clearTimer.current = null;
    };
    const clearSoon = (): void => {
      if (clearTimer.current !== null) return;
      clearTimer.current = window.setTimeout(() => {
        clearTimer.current = null;
        setFocus(null);
      }, DUOS_HOVER_GRACE_MS);
    };
    const point = (spot: BoardSpot): void => {
      cancelClear();
      setFocus((current) => (sameSpot(current, spot) ? current : spot));
    };
    // A pointer or the focus arriving: the bar follows it onto a spot, and waits while it is on the bar.
    const over = (event: Event): void => {
      if (onBar(event.target)) {
        cancelClear();
        return;
      }
      const spot = spotOfElement(elementOf(event.target));
      if (spot === null) clearSoon();
      else point(spot);
    };
    // A press (a tap on a phone, where nothing hovers) moves the bar to its spot, or closes it.
    const down = (event: Event): void => {
      if (onBar(event.target)) return;
      const spot = spotOfElement(elementOf(event.target));
      if (spot !== null) {
        point(spot);
        return;
      }
      cancelClear();
      setFocus(null);
    };
    const leave = (): void => {
      clearSoon();
    };
    const key = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      cancelClear();
      setFocus(null);
    };
    host.addEventListener("pointerover", over);
    host.addEventListener("mouseover", over);
    host.addEventListener("focusin", over);
    host.addEventListener("pointerdown", down);
    host.addEventListener("pointerleave", leave);
    host.addEventListener("mouseleave", leave);
    window.addEventListener("keydown", key);
    return () => {
      host.removeEventListener("pointerover", over);
      host.removeEventListener("mouseover", over);
      host.removeEventListener("focusin", over);
      host.removeEventListener("pointerdown", down);
      host.removeEventListener("pointerleave", leave);
      host.removeEventListener("mouseleave", leave);
      window.removeEventListener("keydown", key);
      cancelClear();
    };
  }, [container]);

  const shownFocus = enabled ? focus : null;
  const idsKey = useMemo(() => {
    const ids = new Set<string>();
    if (shownFocus !== null) ids.add(spotTestid(shownFocus));
    for (const marker of markers) ids.add(spotTestid(marker.spot));
    return [...ids].sort().join("|");
  }, [shownFocus, markers]);
  const boxes = useBoxes(container, idsKey);

  const close = (): void => {
    if (clearTimer.current !== null) {
      window.clearTimeout(clearTimer.current);
      clearTimer.current = null;
    }
    setFocus(null);
  };

  const box = shownFocus === null ? undefined : boxes.get(spotTestid(shownFocus));
  const handCard = shownFocus?.at === "hand" ? shownFocus.instanceId : null;
  const ownHand = Array.isArray(view.you.hand) ? view.you.hand : [];
  const passCard = pass !== null && handCard !== null && ownHand.some((card) => card.instanceId === handCard) ? handCard : null;

  return (
    <>
      {markers.map((marker) => {
        const at = boxes.get(spotTestid(marker.spot));
        if (at === undefined) return null;
        return (
          <span
            key={marker.key}
            className="duos-ping-marker"
            data-testid={duosTestid.pingMarker}
            data-ping={marker.ping}
            data-from={marker.from}
            data-mine={marker.mine ? "true" : undefined}
            style={{ left: `${at.left}px`, top: `${at.top}px`, width: `${at.width}px`, height: `${at.height}px` }}
            role="img"
            aria-label={t("{seat}: {ping}", { seat: seatLabel(marker.from), ping: t(PING_LABEL[marker.ping]) })}
          >
            <span className="duos-ping-marker__glyph" aria-hidden="true">
              {PING_GLYPH[marker.ping]}
            </span>
          </span>
        );
      })}
      {shownFocus === null || box === undefined ? null : (
        <div
          className={BAR_CLASS}
          data-testid={duosTestid.cardActions}
          data-spot={spotTestid(shownFocus)}
          style={{ left: `${box.left + box.width / 2}px`, top: `${box.top}px` }}
        >
          <div className="duos-ping-wheel" data-testid={duosTestid.pingWheel} role="group" aria-label={t("Ping your teammate")}>
            {PING_KINDS.map((kind) => (
              <button
                key={kind}
                type="button"
                className="duos-ping-wheel__ping"
                data-testid={duosTestid.ping(kind)}
                data-ping={kind}
                aria-label={t(PING_LABEL[kind])}
                title={t(PING_LABEL[kind])}
                onClick={() => {
                  const target = targetOfSpot(shownFocus, view, duel);
                  if (target !== null) onPing(kind, target);
                  close();
                }}
              >
                <span aria-hidden="true">{PING_GLYPH[kind]}</span>
              </button>
            ))}
          </div>
          {pass === null || passCard === null ? null : (
            <button
              type="button"
              className="duos-pass"
              data-testid={duosTestid.pass}
              data-instance-id={passCard}
              data-legal={pass.legal.has(passCard) ? "true" : "false"}
              disabled={!pass.legal.has(passCard)}
              onClick={() => {
                if (!pass.legal.has(passCard)) return;
                onPass(passCard);
                close();
              }}
            >
              <span className="duos-pass__label">{t("Pass to teammate")}</span>
              <span className="duos-pass__meta">{t("{cost} mana · {left} left", { cost: pass.cost, left: pass.left })}</span>
            </button>
          )}
        </div>
      )}
    </>
  );
}
