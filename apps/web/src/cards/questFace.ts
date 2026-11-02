// A Quest card's rules box (the Quest tag, R1837000; R2450000): it prints only the stage the card
// stands at, as Hearthstone's Questlines print the part they are on.
//
// A staged Quest (Stinkyscape #7 The Greatest Heist, #112 Job Application, R1854000) has a catalog line
// per stage, "Quest: Stage 2, Comradery. <goal>. Reward: <reward>." (or a Chinese face's, R1301), and
// prints one in their place: on the field, the stage the view's line stands at (`QuestView.stages`)
// with the goal it drew (R1854002) or, while it waits to hear back, the turns left (R1854004); off the
// field, its first. In Too Deep's tree (Classic #90, R404, R543) has no stage lines: on the field it
// prints its text, its open quests with their rewards and its auras; off the field, its text alone.
//
// All of it is the catalog face's words and the view's public quest line, the same on both seats
// (R1854000, CLAUDE.md rule 7). A Radiant face's marks (R277) and its tuned numbers in play (R386) move
// with their words (`assemble`). The whole text stays in the hover preview (inspect/Extended.tsx), the
// touch sheet and the detail view; the inspect overlay lists every stage (inspect/StateNotes.tsx).

import type { QuestView, Tag } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import { currentStage, questWords, stageTitle, stageWords } from "./cardState.ts";
import { listSeparator } from "./chinese.ts";
import type { FaceModel, FaceText } from "./model.ts";
import type { TextRange } from "./radiantDiff.ts";
import type { TunedRange } from "./tuning.ts";

/** What a Quest card's rules box prints, and whether it is less than the whole text (a staged Quest's one stage). */
export type QuestFace = FaceText & { less: boolean };

const QUEST_TAG: Tag = "Quest";

/** A printed stage line's head, and where its reward starts: English, and a Chinese face's (R1301). */
const STAGE_FORMS: readonly { head: RegExp; reward: string }[] = [
  { head: /^(Quest: )Stage (\d+), (.+?)(\. )/u, reward: "Reward: " },
  { head: /^(任务：)第(\d+)阶段，(.+?)(。)/u, reward: "奖励：" },
];

/** A stage line of the face's text, its offsets in the whole text. */
type StageLine = {
  number: number;
  name: string;
  start: number;
  /** Where its "Quest: " ends. */
  quest: number;
  /** Where its goal starts, after the stage's name. */
  goal: number;
  /** Where its reward starts, or its end when it prints none. */
  reward: number;
  end: number;
  /** What ends the stage's name, ". " or "。", and so each part the face prints of it. */
  stop: string;
};

type Line = { start: number; end: number; stage: StageLine | null };

/** A stretch of the face's text, or words the face prints in its place. */
type Piece = { from: number; to: number } | { words: string };

function stageOf(line: string, start: number): StageLine | null {
  for (const form of STAGE_FORMS) {
    const match = form.head.exec(line);
    if (match === null) continue;
    const [head = "", quest = "", number = "", name = "", stop = ""] = match;
    const reward = line.indexOf(form.reward, head.length);
    return {
      number: Number(number),
      name,
      start,
      quest: start + quest.length,
      goal: start + head.length,
      reward: reward < 0 ? start + line.length : start + reward,
      end: start + line.length,
      stop,
    };
  }
  return null;
}

function linesOf(full: string): Line[] {
  let start = 0;
  return full.split("\n").map((line) => {
    const at = start;
    start += line.length + 1;
    return { start: at, end: at + line.length, stage: stageOf(line, at) };
  });
}

/** The face's text made of `pieces`, its marks and tuned numbers kept where their words now stand (as model.ts `copiedText`). */
function assemble(text: FaceText, pieces: readonly Piece[]): FaceText {
  let full = "";
  const marks: TextRange[] = [];
  const tuned: TunedRange[] = [];
  for (const piece of pieces) {
    if ("words" in piece) {
      full += piece.words;
      continue;
    }
    const by = full.length - piece.from;
    for (const mark of text.marks) {
      const start = Math.max(mark.start, piece.from);
      const end = Math.min(mark.end, piece.to);
      if (start < end) marks.push({ start: start + by, end: end + by });
    }
    for (const range of text.tuned ?? []) {
      if (range.start >= piece.from && range.end <= piece.to) tuned.push({ ...range, start: range.start + by, end: range.end + by });
    }
    full += text.full.slice(piece.from, piece.to);
  }
  return { full, marks, ...(tuned.length === 0 ? {} : { tuned }) };
}

/** A stage as its line prints it, under `title`: "Stage 1 of 3, Initiation. Quest: <goal>. Reward: <reward>." */
function printedStage(line: StageLine, title: string): Piece[] {
  return [{ words: title + line.stop }, { from: line.start, to: line.quest }, { from: line.goal, to: line.end }];
}

/** What a staged Quest prints in place of its stage lines, or null when the view names no stage it prints. */
function stagePieces(lines: readonly StageLine[], quest: QuestView | null): Piece[] | null {
  if (quest === null || quest.stages === undefined) {
    const first = lines[0];
    return first === undefined ? null : printedStage(first, stageTitle(first.number, lines.length, first.name));
  }
  const stages = quest.stages;
  if (stages.length > 0 && stages.every((stage) => stage.state === "done")) return [{ words: stageWords(stages) }];
  const found = currentStage(stages);
  const line = found === null ? undefined : lines.find((one) => one.number === found.number);
  if (found === null || line === undefined) return null;
  const title = stageTitle(found.number, stages.length, line.name, found.stage.hearBack);
  const open = found.stage.state === "current" ? quest.open : [];
  if (open.length === 0) return printedStage(line, title);
  const goals = open.map((one) => questWords(one)).join(line.stop);
  return [{ words: title + line.stop + goals + line.stop }, { from: line.reward, to: line.end }];
}

function treeWords(quest: QuestView): string {
  const quests = quest.open.map((one) =>
    one.rewards.length === 0
      ? questWords(one)
      : t("{quest}. Rewards: {rewards}.", { quest: questWords(one), rewards: one.rewards.map((reward) => t(reward.text)).join(listSeparator()) }),
  );
  const lines = [...(quests.length === 0 ? [t("Quest complete")] : quests), ...quest.auras.map((aura) => t(aura.text))];
  return lines.map((line) => `\n${line}`).join("");
}

/**
 * R2450000: what a Quest card's rules box prints, or null for a face that prints its text as it is
 * (every other face, In Too Deep off the field, a staged face the view names no printed stage of).
 */
export function questFaceText(face: FaceModel): QuestFace | null {
  if (!face.tags.includes(QUEST_TAG)) return null;
  const lines = linesOf(face.text.full);
  const stages = lines.flatMap((line) => (line.stage === null ? [] : [line.stage]));
  if (stages.length > 0) {
    const printed = stagePieces(stages, face.quest ?? null);
    if (printed === null) return null;
    const pieces: Piece[] = [];
    let placed = false;
    for (const line of lines) {
      if (line.stage !== null && placed) continue;
      if (pieces.length > 0) pieces.push({ words: "\n" });
      if (line.stage === null) pieces.push({ from: line.start, to: line.end });
      else {
        pieces.push(...printed);
        placed = true;
      }
    }
    return { ...assemble(face.text, pieces), less: true };
  }
  const quest = face.quest;
  if (quest === undefined || quest === null) return null;
  return { ...assemble(face.text, [{ from: 0, to: face.text.full.length }, { words: treeWords(quest) }]), less: false };
}
