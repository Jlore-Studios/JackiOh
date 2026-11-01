// A hand (BUILD M5-T1).
//
// `SideView.hand` is `CardView[] | { count: number }` and that union is the privacy boundary
// (SPEC §10.8): the viewer's hand is full cards (`hand-card-<instanceId>`), the opponent's a number,
// drawn as that many identical backs with nothing to read or click. The narrowing is
// `Array.isArray`, so there is no branch where a count could be rendered as cards.
//
// The fan (docs/polish/7-mobile-ux.md S6, S11, B26, B27): each `.hand-slot` carries its index as `--i` and
// `.hand-cards` the hand size as `--n`, so board.css lays it out without measuring. On the viewer's own
// hand a tap lifts a card (`data-lifted`) so it can be read, legal or not: a capture-phase click, so it
// happens even when the card refuses the click. It is local view state, not a game action. Drag to play is
// game/drag/DragLayer.tsx, and the card a drop has just played is marked `data-landing` (drag/landing.ts),
// which board.css takes out of the fan while the board catches up with the play.
//
// A pointer on the viewer's own hand card is reported upward (`onHover`, #991) so the board can ring
// what its preview counts. It is view state, never an action.
//
// R504: an empty hand, either seat's, keeps its place with one card-sized outline
// (`hand-empty-<side>`) and `data-empty="true"`, so the board does not jump when the last card leaves.
//
// R434: the game's end reveals both hands. Once the view shows the opponent's hand as cards
// (`reveal.ts`), each back turns face up where it lay; a revealed card is read, never played (its
// own testid `revealed-hand-card-<id>`, no click, no drag).
//
// R1297000, R1834004: the opponent's backs wear their seat's card back (`seat`), and a card in a hand
// its seat's loadout where the board carries one and the card is not the viewer's own (game/wearer.ts):
// a revealed one, or the lower hand of a watched or replayed game.
//
// #2267: each of the viewer's own hand cards wears the hand hints that apply to it (HandHints.tsx,
// the list in handHints.ts), read off the card and the view alone; the opponent's backs and a
// revealed card (R434) wear none.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement, type ReactNode } from "react";

import type { CardView, PlayerId, PlayerView } from "@jackioh/shared";

import Card, { isSelected } from "./Card.tsx";
import HandHints from "./HandHints.tsx";
import type { HandHint } from "./handHints.ts";
import { testid, type AnimatingMap, type ClickTarget, type Highlight, type Side } from "./contract.ts";
import { useLanding } from "./drag/landing.ts";
import { revealTestid } from "./reveal.ts";
import { useSetting } from "../settings/index.ts";
import { t } from "../i18n/i18n.ts";

export type HandProps = {
  side: Side;
  seat?: PlayerId;
  hand: CardView[] | { count: number };
  /** R1143: this seat's hand size, once set for the rest of the game (Meditative #79). Shown beside the count. */
  cap?: number | undefined;
  /** R1141: on the opponent's seat, how many of their hand cards carry a mark. Never which. */
  marked?: number | undefined;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
  onHover?: (instanceId: string | null) => void;
  /** R318: the card a full hand burned, drawn over the hand (OverflowNotices.tsx `BurnNotice`). */
  notice?: ReactNode;
  /** #2267: the view the viewer's own hand cards' hints read; Board.tsx gives it to `side="you"` only. Without it, no hint is drawn. */
  view?: PlayerView | undefined;
  /** #2267: the hints to draw, handHints.ts `HAND_HINTS` when absent; a test passes its own. */
  hints?: readonly HandHint[] | undefined;
};

export function handCount(hand: CardView[] | { count: number }): number {
  return Array.isArray(hand) ? hand.length : hand.count;
}

export const HAND_EMPTY_TEXT = "Hand empty";

export function handEmptyTestid(side: Side): string {
  return `hand-empty-${side}`;
}

/** R504: the outline an empty hand holds in place of its cards: one card's size (board.css, per layout). */
function EmptyHand({ side }: { side: Side }): ReactElement {
  return (
    <div className="hand-empty" data-testid={handEmptyTestid(side)}>
      <span className="hand-empty-text">{t(HAND_EMPTY_TEXT)}</span>
    </div>
  );
}

/**
 * R434: one of the opponent's cards at the game's end, a back turning over to its face. Both are
 * drawn; board.css shows the back first and flips to the face, and a reduced motion shows the face.
 */
function RevealedSlot({
  card,
  index,
  seat,
}: {
  card: CardView;
  index: number;
  seat?: PlayerId;
}): ReactElement {
  return (
    <div className="hand-slot hand-slot--revealed" style={{ "--i": index } as CSSProperties}>
      <div className="hand-flip">
        <div className="hand-flip-back" aria-hidden="true">
          <Card card={null} className="card-hand" backOf={seat} />
        </div>
        <div className="hand-flip-face">
          <Card testId={revealTestid.handCard(card.instanceId)} card={card} className="card-hand" inHandOf={seat} />
        </div>
      </div>
    </div>
  );
}

export default function Hand(props: HandProps): ReactElement {
  const { hand, side, cap, marked, view } = props;
  const count = handCount(hand);
  const yours = side === "you";
  const revealed = !yours && Array.isArray(hand);
  const hoverPreviews = useSetting("hoverPreviews");
  const landing = useLanding();

  const rootRef = useRef<HTMLDivElement>(null);
  /** The instance id tapped up, or null. Only ever set on the viewer's own hand. */
  const [tapped, setTapped] = useState<string | null>(null);
  const tappedInHand = tapped !== null && Array.isArray(hand) && hand.some((card) => card.instanceId === tapped);

  // The card left the hand (played, discarded, stolen): forget it, so that the same instance
  // coming back later does not come back lifted.
  useEffect(() => {
    if (tapped !== null && !tappedInHand) setTapped(null);
  }, [tapped, tappedInHand]);

  // A press anywhere outside this hand lowers the tapped card. Capture phase on `window`, so a
  // press the board or a prompt swallows still counts, and so does one dispatched at the document.
  useEffect(() => {
    if (!tappedInHand) return undefined;
    const lower = (event: Event): void => {
      const root = rootRef.current;
      const target = event.target;
      if (root !== null && target instanceof Node && root.contains(target)) return;
      setTapped(null);
    };
    window.addEventListener("pointerdown", lower, true);
    return () => window.removeEventListener("pointerdown", lower, true);
  }, [tappedInHand]);

  function toggle(instanceId: string): void {
    setTapped((current) => (current === instanceId ? null : instanceId));
  }

  let cards: ReactNode;
  if (!Array.isArray(hand)) {
    cards = Array.from({ length: hand.count }, (_unused, index) => (
      <div key={`back-${index}`} className="hand-slot" style={{ "--i": index } as CSSProperties}>
        <Card card={null} className="card-hand" backOf={props.seat} />
      </div>
    ));
  } else if (revealed) {
    cards = hand.map((card, index) => <RevealedSlot key={card.instanceId} card={card} index={index} seat={props.seat} />);
  } else {
    cards = hand.map((card, index) => {
      const handTestid = testid.handCard(card.instanceId);
      const lifted = yours && ((tappedInHand && tapped === card.instanceId) || isSelected(props.highlight, handTestid));
      return (
        <div
          key={card.instanceId}
          className="hand-slot"
          style={{ "--i": index } as CSSProperties}
          data-lifted={lifted ? "true" : undefined}
          data-landing={yours && landing === card.instanceId ? "true" : undefined}
          onClickCapture={yours ? () => toggle(card.instanceId) : undefined}
          onPointerEnter={yours ? () => props.onHover?.(card.instanceId) : undefined}
          onPointerLeave={yours ? () => props.onHover?.(null) : undefined}
        >
          <Card
            testId={handTestid}
            card={card}
            className="card-hand"
            inOwnHand={yours}
            inHandOf={props.seat}
            target={{ on: "hand", instanceId: card.instanceId }}
            highlight={props.highlight}
            animating={props.animating}
            onClick={props.onClick}
            hintRow={yours && view !== undefined ? <HandHints card={card} view={view} hints={props.hints} /> : undefined}
          />
        </div>
      );
    });
  }

  return (
    <div
      ref={rootRef}
      className={`hand hand-${side}`}
      data-testid={`hand-${side}`}
      data-side={side}
      data-count={count}
      data-empty={count === 0 ? "true" : undefined}
      data-revealed={revealed && count > 0 ? "true" : undefined}
      data-hover-preview={yours ? (hoverPreviews ? "on" : "off") : undefined}
      data-marked={marked}
      aria-label={side === "you" ? t("Your hand") : t("Enemy hand")}
    >
      <span className="pile">
        <span className="pile-label">{t("Hand")}</span>
        <span className="pile-n" data-testid={`hand-count-${side}`}>
          {count}
        </span>
        {cap === undefined ? null : (
          <span className="pile-cap" data-testid={`hand-cap-${side}`}>
            /{cap}
          </span>
        )}
        {marked === undefined ? null : (
          <span className="pile-marked" data-testid={`hand-marked-${side}`}>
            {marked}
            {t(" marked")}
          </span>
        )}
      </span>
      <div className="hand-cards" style={{ "--n": count } as CSSProperties}>
        {count === 0 ? <EmptyHand side={side} /> : null}
        {cards}
      </div>
      {props.notice}
    </div>
  );
}
