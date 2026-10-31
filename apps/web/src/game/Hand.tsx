// Hand presentation (BUILD M5-T1; SPEC §10.8 privacy boundary: own hand is cards, opponent is count/backs).
// Fan layout and lifted cards: docs/polish/7-mobile-ux.md S6, S11, B26, B27; drag-to-play landing; #991 hover counted ring.
// R504 empty hand placeholder maintains row height; R434 game end reveals opponent hand with `revealed-hand-card-<id>`.
// Cosmetics (game/wearer.ts): R1297000, R1834004; #2267 hand hints (HandHints.tsx, omitted on opponent/revealed R434 cards).

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
  /** R1834004: the seat whose hand this is, whose card back its backs and whose loadout its revealed cards wear. */
  seat?: PlayerId;
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
  /** #991: the viewer's own hand card under the pointer, or null once the pointer leaves it (Board.tsx). */
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
  touchHold,
}: {
  card: CardView;
  index: number;
  seat?: PlayerId;
  touchHold?: "sheet" | "preview";
}): ReactElement {
  return (
    <div className="hand-slot hand-slot--revealed" style={{ "--i": index } as CSSProperties}>
      <div className="hand-flip">
        <div className="hand-flip-back" aria-hidden="true">
          <Card card={null} className="card-hand" backOf={seat} />
        </div>
        <div className="hand-flip-face">
          <Card testId={revealTestid.handCard(card.instanceId)} card={card} className="card-hand" touchHold={touchHold} inHandOf={seat} />
        </div>
      </div>
    </div>
  );
}

export default function Hand(props: HandProps): ReactElement {
  const { hand, side, cap, marked, view } = props;
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
        <Card card={null} className="card-hand" backOf={props.seat} />
      </div>
    ));
  } else if (revealed) {
    cards = hand.map((card, index) => <RevealedSlot key={card.instanceId} card={card} index={index} seat={props.seat} touchHold={props.touchHold} />);
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
            touchHold={props.touchHold}
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
