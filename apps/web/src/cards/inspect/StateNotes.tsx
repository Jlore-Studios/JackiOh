// A face in play's states in words, beside the face an inspect overlay shows (SPEC §10.8, patch
// v0.2.0). The face itself carries them as badges and marks (CardStates.tsx, tuning.ts), but a hover
// preview takes no pointer events, so their tooltips cannot be read there, and a touch sheet has no
// hover at all: this spells each one out.
//
// - The tuned ribbon (R386, R1320): "Buffed", "Nerfed" or "Tuned" (every change better, every change
//   worse, or a mix), with its glyph, then each change in words with ▲ (better) or ▼ (worse) and the
//   word itself for a screen reader: "+2 Attack", "Gained Rush", "Damage 2 → 3", "Tribute −1".
// - Every other state badge's words: "Brittle 2: crumbles at 0" (R385), "Returns to hand · can't
//   cost less than (2)", "Cast on draw", "Targets enemies" (E39), and the animated mark (R383).
// - A quest line (B5 E33, R404, In Too Deep): each open quest's text with its progress, "1/2", and the
//   rewards it offers under it, then each aura the line holds. The texts are drawn by RulesText.
// - What a copier copies (B5 E14, R511, Echo): "Copying: Book of Knowledge", "(Radiant)" when it is.
//
// `LocLine` is E36's meta line: "27 lines of code", a fused card's being its ingredients' sum (the
// view's definition carries it). The detail view prints the same words in its own meta line
// (CardDetail.tsx).

import type { ReactElement } from "react";

import { questProgress, stateBadges } from "../cardState.ts";
import { MARK_GLYPH, markWords } from "../marks.ts";
import { Icon } from "../icons.tsx";
import { locWords, type FaceModel } from "../model.ts";
import { RulesText } from "../RulesText.tsx";
import { VERDICT_GLYPH, VERDICT_WORD, WAY_GLYPH, changeWords } from "../tuning.ts";
import { INSPECT_LOC, INSPECT_STATES, INSPECT_TUNED } from "./testids.ts";

/** R404: the glyphs of a quest and of an aura its line holds; R511: of a copier's note. */
const QUEST_GLYPH = "⚑";
const AURA_GLYPH = "✦";
const COPY_GLYPH = "“";

/** R511: "Copying: Book of Knowledge", "Copying: Book of Knowledge (Radiant)". */
export function copyingWords(copying: { name: string; radiant: boolean }): string {
  return `Copying: ${copying.name}${copying.radiant ? " (Radiant)" : ""}`;
}

/** Whether a face has anything for StateNotes to print. */
export function hasStateNotes(face: FaceModel): boolean {
  return (
    stateBadges(face).length > 0 ||
    (face.marks ?? []).length > 0 ||
    (face.quest?.auras ?? []).length > 0 ||
    (face.copying ?? null) !== null
  );
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
  const others = stateBadges(face).filter((badge) => badge.kind !== "tuned" && badge.kind !== "quest");
  const tuned = face.tuning !== undefined && face.tuning !== null;
  // R437: a mark's words, which on the board live only in its badge's tooltip (no hover on touch).
  const marks = face.marks ?? [];
  const quests = face.quest?.open ?? [];
  const auras = face.quest?.auras ?? [];
  const copying = face.copying ?? null;
  const lines = others.length + marks.length + quests.length + auras.length + (copying === null ? 0 : 1);
  if (lines === 0 && !tuned) return null;
  return (
    <>
      <TunedRibbon face={face} />
      {lines === 0 ? null : (
        <ul className="inspect-states" data-testid={INSPECT_STATES}>
          {copying === null ? null : (
            <li className="inspect-state" data-state="copies" data-copies={copying.defId}>
              <span className="inspect-state-glyph" aria-hidden="true">
                {COPY_GLYPH}
              </span>
              <span className="inspect-state-words">{copyingWords(copying)}</span>
            </li>
          )}
          {quests.map((quest) => (
            <li
              key={`quest:${quest.id}`}
              className="inspect-state inspect-quest"
              data-state="quest"
              data-quest={quest.id}
              data-progress={quest.progress}
              data-goal={quest.goal}
            >
              <span className="inspect-state-glyph" aria-hidden="true">
                {QUEST_GLYPH}
              </span>
              <span className="inspect-state-words">
                Quest: <RulesText text={quest.text} /> <span className="inspect-quest-progress">({questProgress(quest)})</span>
                <span className="inspect-quest-label">Rewards</span>
                <ul className="inspect-quest-rewards">
                  {quest.rewards.map((reward) => (
                    <li key={reward.id} data-reward={reward.id}>
                      <RulesText text={reward.text} />
                    </li>
                  ))}
                </ul>
              </span>
            </li>
          ))}
          {auras.map((aura) => (
            <li key={`aura:${aura.id}`} className="inspect-state" data-state="questAura" data-aura={aura.id}>
              <span className="inspect-state-glyph" aria-hidden="true">
                {AURA_GLYPH}
              </span>
              <span className="inspect-state-words">
                <RulesText text={aura.text} />
              </span>
            </li>
          ))}
          {marks.map((mark) => (
            <li key={`mark:${mark.mark}`} className="inspect-state" data-state="mark" data-mark={mark.mark} data-mark-color={mark.color}>
              <span className="inspect-state-glyph" aria-hidden="true">
                {MARK_GLYPH}
              </span>
              <span className="inspect-state-words">{markWords(mark.mark).text}</span>
            </li>
          ))}
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
