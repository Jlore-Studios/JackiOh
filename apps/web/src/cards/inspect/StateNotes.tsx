// A face in play's states in words, beside the face an inspect overlay shows (SPEC §10.8). A hover
// preview takes no pointer events and a touch sheet has no hover, so the badges' tooltips cannot be
// read there. Covers the tuned ribbon (R386, R1320), the other badges' words (R385, E39, R383), a
// quest line (B5 E33, R404) and a staged quest's stages (R1854000), what a copier copies (B5 E14, R511), why the viewer's own unit cannot
// attack now (R849000) and why a card in hand cannot be played now (R979000). `LocLine` is E36's
// meta line.

import type { ReactElement } from "react";

import { t, useLocale } from "../../i18n/i18n.ts";
import { questProgress, stageWords, stateBadges } from "../cardState.ts";
import { MARK_GLYPH, markWords } from "../marks.ts";
import { Icon } from "../icons.tsx";
import { locWords, type FaceModel } from "../model.ts";
import { RulesText } from "../RulesText.tsx";
import { VERDICT_GLYPH, VERDICT_WORD, WAY_GLYPH, changeWords } from "../tuning.ts";
import { INSPECT_LOC, INSPECT_STATES, INSPECT_TUNED } from "./testids.ts";

/** R404: the glyphs of a quest and of an aura its line holds; R511: of a copier's note. */
const QUEST_GLYPH = "⚑";
/** R1854000: a stage's mark by its state — done, open now, still to come. */
const STAGE_GLYPH: Readonly<Record<"done" | "current" | "upcoming", string>> = { done: "✓", current: "▸", upcoming: "·" };
/** R1854000: a stage's state in words, beside its glyph for a reader that has no sight of it. */
const STAGE_STATE_WORDS: Readonly<Record<"done" | "current" | "upcoming", string>> = {
  done: "done",
  current: "now",
  upcoming: "to come",
};
const AURA_GLYPH = "✦";
const COPY_GLYPH = "“";
/** R849000: the glyph of a unit's reason for not attacking. */
const ATTACK_REASON_GLYPH = "⊘";
/** R979000: the glyph of a hand card's reason for not playing, the attack reason's sibling. */
const PLAY_REASON_GLYPH = "⊘";

/** R511: "Copying: Book of Knowledge", "Copying: Book of Knowledge (Radiant)". */
export function copyingWords(copying: { name: string; radiant: boolean }): string {
  return copying.radiant ? t("Copying: {name} (Radiant)", { name: copying.name }) : t("Copying: {name}", { name: copying.name });
}

export function hasStateNotes(face: FaceModel): boolean {
  return (
    stateBadges(face).length > 0 ||
    (face.quest?.stages ?? []).length > 0 ||
    (face.marks ?? []).length > 0 ||
    (face.quest?.auras ?? []).length > 0 ||
    (face.copying ?? null) !== null ||
    face.attackReason !== undefined ||
    face.playReason !== undefined
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
        {t(VERDICT_WORD[tuning.verdict])}
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
              {way === null ? null : <span className="inspect-sr">{` (${t(way)})`}</span>}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

export function StateNotes({ face }: { face: FaceModel }): ReactElement | null {
  useLocale();
  const others = stateBadges(face).filter((badge) => badge.kind !== "tuned" && badge.kind !== "quest" && badge.kind !== "questStage");
  const tuned = face.tuning !== undefined && face.tuning !== null;
  // R437: a mark's words, which on the board live only in its badge's tooltip (no hover on touch).
  const marks = face.marks ?? [];
  const quests = face.quest?.open ?? [];
  const auras = face.quest?.auras ?? [];
  const stages = face.quest?.stages ?? [];
  const copying = face.copying ?? null;
  const lines =
    others.length +
    marks.length +
    quests.length +
    (stages.length === 0 ? 0 : 1) +
    auras.length +
    (copying === null ? 0 : 1) +
    (face.attackReason === undefined ? 0 : 1) +
    (face.playReason === undefined ? 0 : 1);
  if (lines === 0 && !tuned) return null;
  return (
    <>
      <TunedRibbon face={face} />
      {lines === 0 ? null : (
        <ul className="inspect-states" data-testid={INSPECT_STATES}>
          {face.attackReason === undefined ? null : (
            <li className="inspect-state" data-state="attackReason">
              <span className="inspect-state-glyph" aria-hidden="true">
                {ATTACK_REASON_GLYPH}
              </span>
              <span className="inspect-state-words">{t(face.attackReason)}</span>
            </li>
          )}
          {face.playReason === undefined ? null : (
            <li className="inspect-state" data-state="playReason">
              <span className="inspect-state-glyph" aria-hidden="true">
                {PLAY_REASON_GLYPH}
              </span>
              <span className="inspect-state-words">{t(face.playReason)}</span>
            </li>
          )}
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
                {t("Quest: ")}
                <RulesText text={quest.text} /> <span className="inspect-quest-progress">({questProgress(quest)})</span>
                <span className="inspect-quest-label">{t("Rewards")}</span>
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
          {stages.length === 0 ? null : (
            <li className="inspect-state inspect-quest-stages" data-state="questStages">
              <span className="inspect-state-glyph" aria-hidden="true">
                {QUEST_GLYPH}
              </span>
              <span className="inspect-state-words">
                {stageWords(stages)}
                <ol className="inspect-quest-stage-list">
                  {stages.map((stage, index) => (
                    <li
                      key={`stage:${String(index)}`}
                      data-stage={index + 1}
                      data-stage-state={stage.state}
                      data-hear-back={stage.hearBack}
                    >
                      <span aria-hidden="true">{STAGE_GLYPH[stage.state]} </span>
                      <span className="inspect-quest-stage-name">{t(stage.name)}</span>
                      <span className="inspect-sr">{` (${t(STAGE_STATE_WORDS[stage.state])})`}</span>
                      <ul className="inspect-quest-stage-goals">
                        {stage.goals.map((goal) => (
                          <li key={goal}>
                            <RulesText text={goal} />
                          </li>
                        ))}
                      </ul>
                      <span className="inspect-quest-label">{t("Reward")}</span> <RulesText text={stage.reward} />
                    </li>
                  ))}
                </ol>
              </span>
            </li>
          )}
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
              <span className="inspect-state-words">{t(markWords(mark.mark).text)}</span>
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
