// A hand (BUILD M5-T1).
//
// `SideView.hand` is `CardView[] | { count: number }` and that union is the privacy boundary
// (SPEC §10.8): the viewer's hand is full cards with `hand-card-<instanceId>` testids, the
// opponent's hand is a number, so it renders exactly that many identical backs — no names, no
// def ids, no instance ids, nothing to read and nothing to click. The narrowing is
// `Array.isArray`, so there is no branch where a count could be rendered as cards.
//
// Polish task 7 (docs/polish/7-mobile-ux.md S6, S11, B26, B27): every card sits in a `.hand-slot`
// carrying its index as `--i`, and `.hand-cards` carries the hand size as `--n`, so board.css can
// lay the hand out as an overlapping fan without measuring anything. On the viewer's own hand a
// tap lifts a card (`data-lifted="true"`) so it can be read, legal or not — the lift is a
// capture-phase click, so it happens even when the card itself refuses the click. It is local
// view state, not a game action: a second tap, a press anywhere outside this hand, or the card
// leaving the hand lowers it. The selected hand card is always lifted. No card here is an HTML5
// drag source any more; drag to play is pointer events in game/drag/DragLayer.tsx, and the card a
// drop has just played is marked `data-landing` (drag/landing.ts), which board.css takes out of
// the fan while the board catches up with the play.
//
// R504: an empty hand keeps its place. Either seat's hand with no cards holds one card-sized
// outline (`hand-empty-<side>`, "Hand empty") and says `data-empty="true"`, so its row keeps the
// height a card gives it on every layout and the board does not jump when the last card leaves.
//
// R434: the game's end reveals both hands. Once the view shows the opponent's hand as cards (a
// finished game, `reveal.ts`), each back turns face up where it lay, one after another
// (`data-revealed="true"`; board.css flips it, and under reduced motion it is simply face up). A
// revealed card is read, never played: it has its own testid (`revealed-hand-card-<id>`), no click
// and no drag, and a resting mouse or a long-press opens it as any face does.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement, type ReactNode } from "react";

import type { CardView } from "@jackioh/shared";

import Card, { isSelected } from "./Card.tsx";
import { testid, type AnimatingMap, type ClickTarget, type Highlight, type Side } from "./contract.ts";
import { useLanding } from "./drag/landing.ts";
import { revealTestid } from "./reveal.ts";
import { useSetting } from "../settings/index.ts";

export type HandProps = {
  side: Side;
  hand: CardView[] | { count: number };
  /** R1143: this seat's hand size, once set for the rest of the game (Meditative #79). Shown beside the count. */
  cap?: number | undefined;
  /** R1141: on the opponent's seat, how many of their hand cards carry a mark. Never which. */
  marked?: number | undefined;
  /** #165: what a touch hold on a card opens (contract.ts's `touchHoldMode`). */
  touchHold?: "sheet" | "preview";
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
  /** R318: the card a full hand burned, drawn over the hand (OverflowNotices.tsx `BurnNotice`). */
  notice?: ReactNode;
};

export function handCount(hand: CardView[] | { count: number }): number {
  return Array.isArray(hand) ? hand.length : hand.count;
}

/** R504: what an empty hand's outline says. */
export const HAND_EMPTY_TEXT = "Hand empty";

/** R504: the testid of a seat's empty-hand outline. */
export function handEmptyTestid(side: Side): string {
  return `hand-empty-${side}`;
}

/**
 * R504: the outline an empty hand holds in place of its cards: one card's size (board.css, per
 * layout), dashed, with the words for a reader.
 */
function EmptyHand({ side }: { side: Side }): ReactElement {
  return (
    <div className="hand-empty" data-testid={handEmptyTestid(side)}>
      <span className="hand-empty-text">{HAND_EMPTY_TEXT}</span>
    </div>
  );
}

/**
 * R434: one of the opponent's cards at the game's end, a back turning over to its face. Both are
 * drawn; board.css shows the back first and flips to the face, and a reduced motion shows the face.
 */
function RevealedSlot({ card, index, touchHold }: { card: CardView; index: number; touchHold?: "sheet" | "preview" }): ReactElement {
  return (
    <div className="hand-slot hand-slot--revealed" style={{ "--i": index } as CSSProperties}>
      <div className="hand-flip">
        <div className="hand-flip-back" aria-hidden="true">
          <Card card={null} className="card-hand" />
        </div>
        <div className="hand-flip-face">
          <Card testId={revealTestid.handCard(card.instanceId)} card={card} className="card-hand" touchHold={touchHold} />
        </div>
      </div>
    </div>
  );
}

export default function Hand(props: HandProps): ReactElement {
  const { hand, side, cap, marked } = props;
  const count = handCount(hand);
  const yours = side === "you";
  // R434: the opponent's hand is cards only once the game is over; it is read, never played.
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
        <Card card={null} className="card-hand" />
      </div>
    ));
  } else if (revealed) {
    cards = hand.map((card, index) => <RevealedSlot key={card.instanceId} card={card} index={index} touchHold={props.touchHold} />);
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
        >
          <Card
            testId={handTestid}
            card={card}
            className="card-hand"
            target={{ on: "hand", instanceId: card.instanceId }}
            touchHold={props.touchHold}
            highlight={props.highlight}
            animating={props.animating}
            onClick={props.onClick}
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
      aria-label={`${side} hand`}
    >
      <span className="pile">
        <span className="pile-label">Hand</span>
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
            {marked} marked
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
