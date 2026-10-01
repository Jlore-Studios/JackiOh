// The state rail (cardState.ts): the badges a face in play wears for its Brittle count (R385), its
// tuned mark (R386), its enchantments (B5 E39) and, on a hover preview of a card standing as a Unit,
// its animated mark (R383). One rail serves every face that shows a card's states — the tall card in
// a hand or an inspect overlay, a face-up backrow card, the board minion — and the stylesheet places it
// by layout (cardstate.css).
//
// Each badge is a glyph by shape (a cracked pane with the count, ▲ ▼ ◆, a return arrow, a spark, a
// crosshair, a cog), with its words as the tooltip and the accessible name, so nothing rests on colour.
// Spans and imgs only, so a face can still sit inside a button (B12). Nothing moves: the rail is the
// same under reduced motion.
//
// The board minion already draws Brittle's cracks and count and the Animated cog over its portrait
// (keywordVisuals.ts, R438), so it passes those kinds in `omit` and the rail does not draw them twice.
//
// `PileDepth` is a backrow pile's depth (E21), which game/Backrow.tsx draws beside the card.
//
// On a small face (a hand card) the rail keeps its first STATE_BADGES_SMALL_MAX badges and folds the
// rest into a "+n" chip (shown only there); its hover preview prints every one in words.

import type { ReactElement } from "react";

import { PILE_WORDS, stateBadges, type StateBadge, type StateBadgeKind } from "./cardState.ts";
import { STATE_BADGES_SMALL_MAX } from "./constants.ts";
import { Icon } from "./icons.tsx";
import type { FaceModel } from "./model.ts";

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
 * B5 E21: a backrow pile's depth — the face-down, dormant cards under the top one — beside the card in
 * its zone, read and drawn as a unit pile's is (`UnitView.buried`, game/Card.tsx). Nothing at none.
 */
export function PileDepth({ buried }: { buried: number }): ReactElement | null {
  if (buried <= 0) return null;
  return (
    <span className="buried-badge backrow-pile" data-buried={buried} title={PILE_WORDS}>
      {buried}
    </span>
  );
}
