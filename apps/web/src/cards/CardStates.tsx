// State badges: Brittle (R385), Temporary's count in a hand (#2437), tuned (R386), enchantments (B5 E39), and Animated (R383). A Created card's badge is never drawn on a face (R2647000).
// The board minion draws Brittle and Animated itself (keywordVisuals.ts, R438); `PileDepth` shows E21.
// Shape plus accessible words, not color, lets faces fit inside buttons (B12).

import { useState, type CSSProperties, type ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { PILE_WORDS, stateBadges, type StateBadge, type StateBadgeKind } from "./cardState.ts";
import { listSeparator } from "./chinese.ts";
import { STATE_BADGES_SMALL_MAX } from "./constants.ts";
import { Icon } from "./icons.tsx";
import { MARK_PALETTES, TEMPORARY_LAST_COLOR } from "./marks.ts";
import type { FaceModel } from "./model.ts";
import { STACK_TITLE, STACK_UNKNOWN_NAME, default as StackSheet } from "./wheel/StackSheet.tsx";

import "./cardstate.css";

export type CardStatesProps = { face: FaceModel; omit?: readonly StateBadgeKind[] };

const NONE: readonly StateBadgeKind[] = [];

/** R2647000: the states no face draws, rarely relevant in combat; the inspect state notes still list them (StateNotes.tsx). */
const NOT_ON_FACES: readonly StateBadgeKind[] = ["created"];

/** #2437: the Temporary badge's disc, the fuchsia of a last count's aura (marks.ts), for cardstate.css. */
const TEMPORARY_DISC = {
  "--cf-temporary-rim": MARK_PALETTES[TEMPORARY_LAST_COLOR].rim,
  "--cf-temporary-glow": MARK_PALETTES[TEMPORARY_LAST_COLOR].glow,
} as CSSProperties;

function Badge({ badge }: { badge: StateBadge }): ReactElement {
  return (
    <span
      className="cf-state"
      data-state={badge.kind}
      style={badge.kind === "temporary" ? TEMPORARY_DISC : undefined}
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
  useLocale();
  const badges = stateBadges(face).filter((badge) => !omit.includes(badge.kind) && !NOT_ON_FACES.includes(badge.kind));
  if (badges.length === 0) return null;
  const folded = badges.slice(STATE_BADGES_SMALL_MAX);
  return (
    <span className="cf-states" data-count={badges.length}>
      {badges.map((badge) => (
        <Badge key={`${badge.kind}:${badge.words}`} badge={badge} />
      ))}
      {folded.length > 0 && (
        <span className="cf-states-more" title={folded.map((badge) => badge.words).join(listSeparator())} aria-hidden="true">
          +{folded.length}
        </span>
      )}
    </span>
  );
}

/** B5 E21: a backrow pile's buried depth, never naming a face-down top card; a press opens the pile wheel. */
export function PileDepth({
  buried,
  top,
  className = "buried-badge backrow-pile",
}: {
  buried: number;
  top?: FaceModel | null;
  className?: string;
}): ReactElement | null {
  useLocale();
  const [open, setOpen] = useState(false);
  if (buried <= 0) return null;
  const face = top ?? null;
  return (
    <>
      <button
        type="button"
        className={className}
        data-buried={buried}
        title={t(PILE_WORDS)}
        aria-label={t("{words}: show pile", { words: t(PILE_WORDS) })}
        onClick={(event) => {
          event.stopPropagation();
          setOpen(true);
        }}
      >
        {buried}
      </button>
      {open ? (
        <StackSheet title={t(STACK_TITLE)} top={face} topName={face?.name ?? t(STACK_UNKNOWN_NAME)} buried={buried} onClose={() => setOpen(false)} />
      ) : null}
    </>
  );
}
