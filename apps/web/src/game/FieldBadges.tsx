// #2932, #3257: on a phone held upright a field card's Activate control and keyword marks stand in the
// top-left inner corner of the card's own zone instead of sitting on its face, so the face keeps its
// whole width for the name, art, attack and health (board.css, "Phone portrait"; the card is not made
// wider). The stack never leaves its zone: nothing of it reaches into the lane beside it. It may cover
// the card's art and its cost, whose gem board.css hides while a mark is there (#3257).
//
// The stack is a sibling of the card root, as the switch (`SwitchButton`, Card.tsx) is, so DEF's
// rotation never turns or moves it: it starts at the same corner in ATK and DEF. It stops above the
// card's name plate (board.css, `max-height`), or, for a DEF card, above the turned card, so it never
// covers the name, the attack or the health, and the marks that do not fit there fold into the face's
// "+n": the face's own cap (`keywordChips`) first, then as many more as the measure below finds the
// room cannot hold. The Activate control and, while the unit has a mark to draw, the "+n" always stay:
// where the room is shorter than they are (a DEF card on any phone), they hang over the turned card's
// top-left by the difference, which extends #3268's call on the control to the "+n", so a unit's marks
// never vanish with no sign. A named mark is drawn only where it fits. It draws view data only
// (CLAUDE.md rule 7): the control is `ActivateControls`, which decides nothing, and the marks are the
// face's own (`KeywordIcons`' markup, which cards.css styles by class). Card.tsx renders it only for a
// field card on the phone layout (`usePhonePortrait`); every other card keeps both on its face.

import { useLayoutEffect, useRef, useState, type ReactElement } from "react";

import { keywordKey, type CardView, type Keyword, type UnitView } from "@jackioh/shared";

import { KEYWORD_MARK } from "../cards/index.ts";
import { keywordChips } from "../cards/MinionFace.tsx";
import ActivateControls from "./ActivateControl.tsx";
import type { AnimatingMap, ClickTarget, Highlight } from "./contract.ts";

/** What the stack holds: the Activate control and the marks, each a flex item of it (board.css). */
const MARKS = ".activate-control, .keyword, .cf-kw-more";

/** Half a pixel of rounding in the layout does not count as running out. */
const SLACK_PX = 0.5;

/**
 * Whether a mark ends below the stack's `max-height` (board.css: the room above the name plate).
 * The marks' own boxes are read, not the stack's `scrollHeight`: the control's 44px press band
 * (activate.css) would inflate that. A layout with no `max-height` (jsdom has none) never overflows.
 */
function overflows(stack: HTMLElement): boolean {
  const room = parseFloat(getComputedStyle(stack).maxHeight);
  if (Number.isNaN(room)) return false;
  let bottom = 0;
  for (const mark of stack.querySelectorAll<HTMLElement>(MARKS)) bottom = Math.max(bottom, mark.offsetTop + mark.offsetHeight);
  return bottom > room + SLACK_PX;
}

export type FieldBadgesProps = {
  /** The card as the view lists it: the `UnitView` of a unit, else the face-up `BackrowView`; its `activations` are the control's. */
  card: CardView;
  unit: UnitView | null | undefined;
  /** The unit's keywords in the face's words, in order (`FaceModel.keywordWords`; R1301: a Chinese unit's are the table's). */
  words: readonly string[];
  /** What joins a list of them (`FaceModel.keywordJoin`). */
  join: string;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
};

export default function FieldBadges(props: FieldBadgesProps): ReactElement | null {
  const { card, unit } = props;
  // A unit whose keywords the face draws as states (Taunt's frame, a Divine Shield's bubble) or that do nothing on the field has no mark to draw.
  const { shown, folded } = keywordChips(unit?.keywords ?? [], unit?.armor ?? 0);
  const activations = (card.activations ?? []).length;
  if (activations === 0 && shown.length === 0 && folded.length === 0) return null;
  const chips = [...shown, ...folded];
  // Keyed by what it draws and by the card's position (a DEF card's room is shorter, board.css), so a change of either starts a fresh measure.
  return <Stack key={`${unit?.position}:${activations > 0}:${chips.map(keywordKey).join("|")}`} {...props} chips={chips} drawn={shown.length} />;
}

function Stack({
  card,
  unit,
  words,
  join,
  highlight,
  animating,
  onClick,
  chips,
  drawn,
}: FieldBadgesProps & {
  /** Every mark the unit may draw, in order: the face's `shown`, then its `folded`. */
  chips: Keyword[];
  /** How many of them the face itself draws before its "+n". */
  drawn: number;
}): ReactElement {
  const stack = useRef<HTMLSpanElement>(null);
  // `room` counts the named marks drawn, from the face's own cap down to 0, where the "+n" alone stands for every mark. The "+n" is
  // never dropped while the unit has a mark to draw, as the control is not, so the measure stops at 0 even if it still overflows.
  // A new object forces a measure.
  const [fit, setFit] = useState({ room: drawn });
  useLayoutEffect(() => {
    const element = stack.current;
    if (element !== null && fit.room > 0 && overflows(element)) setFit({ room: fit.room - 1 });
  }, [fit]);
  // A zone that changes size measures again, from the face's cap.
  useLayoutEffect(() => {
    const zone = stack.current?.parentElement;
    if (zone == null || typeof ResizeObserver === "undefined") return undefined;
    let size = `${zone.clientWidth}x${zone.clientHeight}`;
    const observer = new ResizeObserver(() => {
      const now = `${zone.clientWidth}x${zone.clientHeight}`;
      if (now === size) return;
      size = now;
      setFit({ room: drawn });
    });
    observer.observe(zone);
    return () => observer.disconnect();
  }, [drawn]);
  // R1301: a unit's keywords are named in the face's words (`words` follows `unit.keywords`' order).
  const keywords = unit?.keywords ?? [];
  const wordsOf = (keyword: Keyword): string => words[keywords.indexOf(keyword)] ?? keywordKey(keyword);
  const rest = chips.slice(fit.room);
  return (
    <span className="field-badges" ref={stack}>
      <ActivateControls card={card} highlight={highlight} animating={animating} onClick={onClick} />
      {unit != null && chips.length > 0 && (
        <span className="keywords">
          {chips.slice(0, fit.room).map((keyword, index) => (
            <span
              key={`${keywordKey(keyword)}-${index}`}
              className="keyword keyword-icon"
              role="img"
              aria-label={wordsOf(keyword)}
              data-keyword={keyword.kind}
              data-n={"n" in keyword ? keyword.n : undefined}
              title={wordsOf(keyword)}
            >
              {KEYWORD_MARK[keyword.kind]}
              {"n" in keyword ? ` ${keyword.n}` : ""}
            </span>
          ))}
          {rest.length > 0 && (
            <span className="cf-kw-more" role="img" aria-label={rest.map(wordsOf).join(join)} title={rest.map(wordsOf).join(join)}>
              +{rest.length}
            </span>
          )}
        </span>
      )}
    </span>
  );
}
