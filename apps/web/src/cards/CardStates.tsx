// The state rail (cardState.ts): the badges a face in play wears for its Brittle count (R385), its
// tuned mark (R386), its enchantments (B5 E39) and, on a hover preview of a card standing as a Unit,
// its animated mark (R383). One rail serves every face that shows a card's states; the stylesheet
// places it by layout (cardstate.css).
//
// Each badge is a glyph by shape with its words as tooltip and accessible name, so nothing rests on
// colour. Spans and imgs only, so a face can still sit inside a button (B12); nothing moves.
//
// The board minion draws Brittle's cracks and the Animated cog itself (keywordVisuals.ts, R438), so
// it passes those kinds in `omit`. `PileDepth` is a backrow pile's depth (E21), drawn by
// game/Backrow.tsx. On a small face the rail keeps its first STATE_BADGES_SMALL_MAX badges and folds
// the rest into a "+n" chip; the hover preview prints every one in words.

import { useState, type ReactElement } from "react";

import { PILE_WORDS, stateBadges, type StateBadge, type StateBadgeKind } from "./cardState.ts";
import { STATE_BADGES_SMALL_MAX } from "./constants.ts";
import { Icon } from "./icons.tsx";
import type { FaceModel } from "./model.ts";
import { STACK_TITLE, STACK_UNKNOWN_NAME, default as StackSheet } from "./wheel/StackSheet.tsx";

import "./cardstate.css";

export type CardStatesProps = { face: FaceModel; omit?: readonly StateBadgeKind[] };

const NONE: readonly StateBadgeKind[] = [];

function Badge({ badge }: { badge: StateBadge }): ReactElement {
  return (
    <span
      className="cf-state"
      data-state={badge.kind}
      role="img"
      aria-label={badge.words}
      title={badge.words}
      {...badge.data}
    >
      {badge.icon === null ? null : <Icon name={badge.icon} />}
      {badge.text === null ? null : (
        <span className="cf-state-text" aria-hidden="true">
          {badge.text}
        </span>
      )}
    </span>
  );
}

export function CardStates({ face, omit = NONE }: CardStatesProps): ReactElement | null {
  const badges = stateBadges(face).filter((badge) => !omit.includes(badge.kind));
  if (badges.length === 0) return null;
  const folded = badges.slice(STATE_BADGES_SMALL_MAX);
  return (
    <span className="cf-states" data-count={badges.length}>
      {badges.map((badge) => (
        <Badge key={`${badge.kind}:${badge.words}`} badge={badge} />
      ))}
      {folded.length > 0 && (
        <span className="cf-states-more" title={folded.map((badge) => badge.words).join("; ")} aria-hidden="true">
          +{folded.length}
        </span>
      )}
    </span>
  );
}

/**
 * B5 E21: a pile's depth — the face-down, dormant cards under the top one — beside the card in its
 * zone, a backrow pile's as a unit pile's (`UnitView.buried`, game/Card.tsx). Nothing at none. A
 * press opens the pile as a wheel (cards/wheel), the top with its face and every buried card as a
 * back, so what is above and below what is obvious.
 */
export function PileDepth({
  buried,
  top,
  className = "buried-badge backrow-pile",
}: {
  buried: number;
  /** The pile's top card with its face; null for a back on top (a face-down pile names nothing). */
  top?: FaceModel | null;
  className?: string;
}): ReactElement | null {
  const [open, setOpen] = useState(false);
  if (buried <= 0) return null;
  const face = top ?? null;
  return (
    <>
      <button
        type="button"
        className={className}
        data-buried={buried}
        title={PILE_WORDS}
        aria-label={`${PILE_WORDS}: show pile`}
        onClick={(event) => {
          event.stopPropagation();
          setOpen(true);
        }}
      >
        {buried}
      </button>
      {open ? (
        <StackSheet title={STACK_TITLE} top={face} topName={face?.name ?? STACK_UNKNOWN_NAME} buried={buried} onClose={() => setOpen(false)} />
      ) : null}
    </>
  );
}
