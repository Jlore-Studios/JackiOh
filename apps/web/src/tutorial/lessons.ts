// R291 lesson data (SPEC §9.10): fixed decks, seed and seat make R290 practice games repeatable.
// R2205003, R2434001: five lessons, one subject each, every deck Training Grounds cards. Lessons 1–4
// hold no Legendary or Mythic card and set `poolHold` (R2201001) on both seats, so none is generated
// either; lesson 5 deals them to the human alone (R2533001). Decks meet R184's sizes, respect R186 and
// bring nothing a later lesson teaches (`lessons.test.ts`). R2434000: the ids of the lessons before
// #2434 read as the lesson in their place on the path.

import { AI_TUTORIAL, TUTORIAL_HUMAN, type Handicap } from "@jackioh/engine/config";
import type { PlayerId } from "@jackioh/shared";

import { lesson as backrow } from "./lessons/backrow.ts";
import { lesson as basics } from "./lessons/basics.ts";
import { lesson as casting } from "./lessons/casting.ts";
import { lesson as finale } from "./lessons/finale.ts";
import { lesson as keywords } from "./lessons/keywords.ts";

export type TutorialLesson = {
  id: string;
  number: number;
  title: string;
  summary: string;
  /** R2205000: the Field Guide entries its moments set up, in the order they come; the path's chips. */
  discoveries: readonly string[];
  seed: string;
  humanSeat: PlayerId;
  humanDeck: readonly string[];
  aiDeck: readonly string[];
  poolHold: boolean;
  /** R291: an R186 exception a fixed lesson needs, with its reason. */
  aiShadowBanned?: Readonly<Record<string, string>>;
  retryTip: string;
};

export const TUTORIAL_LESSONS: readonly TutorialLesson[] = [basics, keywords, casting, backrow, finale];

/**
 * R2434000: the ids of lessons 2 to 4 before #2434 remade them, each read as the lesson now in its
 * place, so a player who won the old lesson n has won lesson n. `basics` and `finale` kept theirs.
 */
export const EARLIER_LESSON_IDS: Readonly<Record<string, string>> = { spells: "keywords", traps: "casting", advanced: "backrow" };

export function currentLessonId(id: string): string {
  return Object.hasOwn(EARLIER_LESSON_IDS, id) ? (EARLIER_LESSON_IDS[id] ?? id) : id;
}

export function lessonById(id: string): TutorialLesson | undefined {
  return TUTORIAL_LESSONS.find((lesson) => lesson.id === id);
}

export function nextLessonOf(id: string): TutorialLesson | undefined {
  const lesson = lessonById(id);
  return lesson === undefined ? undefined : TUTORIAL_LESSONS.find((candidate) => candidate.number === lesson.number + 1);
}

/** R290, R1705005, R2201000, R2201001, R2434001, R2533000: the tutorial opponent's handicap, its bar and all, and the human's, the pool hold on both when the lesson sets it. */
export function lessonHandicaps(lesson: TutorialLesson): { human: Handicap; ai: Handicap } {
  const hold = lesson.poolHold ? { poolHold: true } : {};
  return { human: { ...TUTORIAL_HUMAN, ...hold }, ai: { ...AI_TUTORIAL, ...hold } };
}
