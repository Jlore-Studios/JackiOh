// State badges: Brittle (R385), tuned (R386), enchantments (B5 E39), and Animated (R383).
// The board minion draws Brittle and Animated itself (keywordVisuals.ts, R438); `PileDepth` shows E21.
// Shape plus accessible words, not colour, lets faces fit inside buttons (B12).

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

/** B5 E21: show a backrow pile's buried depth without naming a face-down top card.
 * A press opens the pile wheel so stacking order is clear.
 */
export function PileDepth({
  buried,
  top,
  className = "buried-badge backrow-pile",
}: {
  buried: number;
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
