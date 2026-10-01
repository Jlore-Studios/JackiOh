// A face in play's states in words, beside the face an inspect overlay shows (SPEC §10.8, patch
// v0.2.0). The face itself carries them as badges and marks (CardStates.tsx, tuning.ts), but a hover
// preview takes no pointer events, so their tooltips cannot be read there, and a touch sheet has no
// hover at all: this spells each one out.
//
// - The tuned ribbon (R386): "Upgraded", "Degraded" or "Tuned" (every change better, every change
//   worse, or a mix), with its glyph, then each change in words with ▲ (better) or ▼ (worse) and the
//   word itself for a screen reader: "+2 Attack", "Gained Rush", "Damage 2 → 3", "Tribute −1".
// - Every other state badge's words: "Brittle 2: crumbles at 0" (R385), "Returns to hand · can't
//   cost less than (2)", "Cast on draw", "Targets enemies" (E39), and the animated mark (R383).
//
// `LocLine` is E36's meta line: "27 lines of code", a fused card's being its ingredients' sum (the
// view's definition carries it). The detail view prints the same words in its own meta line
// (CardDetail.tsx).

import type { ReactElement } from "react";

import { stateBadges } from "../cardState.ts";
import { Icon } from "../icons.tsx";
import { locWords, type FaceModel } from "../model.ts";
import { VERDICT_GLYPH, VERDICT_WORD, WAY_GLYPH, changeWords } from "../tuning.ts";
import { INSPECT_LOC, INSPECT_STATES, INSPECT_TUNED } from "./testids.ts";

/** Whether a face has anything for StateNotes to print. */
export function hasStateNotes(face: FaceModel): boolean {
  return stateBadges(face).length > 0;
}

function TunedRibbon({ face }: { face: FaceModel }): ReactElement | null {
  const tuning = face.tuning;
  if (tuning === undefined || tuning === null) return null;
  return (
    <div className="inspect-tuned" data-testid={INSPECT_TUNED} data-tuned={tuning.verdict}>
      <p className="inspect-tuned-ribbon">
        <span className="inspect-tuned-glyph" aria-hidden="true">
          {VERDICT_GLYPH[tuning.verdict]}
        </span>
        {VERDICT_WORD[tuning.verdict]}
      </p>
      <ul className="inspect-tuned-changes">
        {tuning.changes.map((change, index) => {
          const words = changeWords(change);
          const way = change.kind === "set" ? null : change.way;
          return (
            <li key={`${String(index)}:${words}`} className="inspect-tuned-change" data-change={change.kind} data-way={way ?? undefined}>
              <span className="inspect-tuned-glyph" aria-hidden="true">
                {way === null ? VERDICT_GLYPH.tuned : WAY_GLYPH[way]}
              </span>
              {words}
              {way === null ? null : <span className="inspect-sr">{` (${way})`}</span>}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

export function StateNotes({ face }: { face: FaceModel }): ReactElement | null {
  const others = stateBadges(face).filter((badge) => badge.kind !== "tuned");
  const tuned = face.tuning !== undefined && face.tuning !== null;
  if (others.length === 0 && !tuned) return null;
  return (
    <>
      <TunedRibbon face={face} />
      {others.length === 0 ? null : (
        <ul className="inspect-states" data-testid={INSPECT_STATES}>
          {others.map((badge) => (
            <li key={`${badge.kind}:${badge.words}`} className="inspect-state" data-state={badge.kind}>
              <span className="inspect-state-glyph" aria-hidden="true">
                {badge.icon === null ? badge.text : <Icon name={badge.icon} />}
              </span>
              <span className="inspect-state-words">{badge.words}</span>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

/** E36: "N lines of code", or nothing for a card whose script nobody counted. */
export function LocLine({ face }: { face: FaceModel }): ReactElement | null {
  const loc = face.loc;
  if (loc === undefined || loc === null) return null;
  return (
    <p className="inspect-meta inspect-loc" data-testid={INSPECT_LOC} data-loc={loc}>
      {locWords(loc)}
    </p>
  );
}
