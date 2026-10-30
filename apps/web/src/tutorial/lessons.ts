// R291 lesson data (SPEC §9.10): fixed decks, seed and seat make R290 practice games repeatable.
// R2205003: five lessons dealt from Training Grounds alone. Lessons 1–4 hold no Legendary or Mythic
// card and set the human's `poolHold` (R2201001), so none is generated either; lesson 5 deals them.
// Human decks meet §2.6 at `TUTORIAL_HUMAN`'s size; AI decks have R184's size, respect R186 and bring
// spells and the backrow only after the lessons that teach them. `lessons.test.ts` holds them to it.

import { AI_TUTORIAL, TUTORIAL_HUMAN, type Handicap } from "@jackioh/engine/config";
import type { PlayerId } from "@jackioh/shared";

import { lesson as advanced } from "./lessons/advanced.ts";
import { lesson as basics } from "./lessons/basics.ts";
import { lesson as finale } from "./lessons/finale.ts";
import { lesson as spells } from "./lessons/spells.ts";
import { lesson as traps } from "./lessons/traps.ts";

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
  /** R2201001, R2205003: hold Legendary and Mythic cards out of every random pool (lessons 1–4). */
  poolHold: boolean;
  /** R291: an R186 exception a fixed lesson needs, with its reason. */
  aiShadowBanned?: Readonly<Record<string, string>>;
  retryTip: string;
};

export const TUTORIAL_LESSONS: readonly TutorialLesson[] = [basics, spells, traps, advanced, finale];

export function lessonById(id: string): TutorialLesson | undefined {
  return TUTORIAL_LESSONS.find((lesson) => lesson.id === id);
}

export function nextLessonOf(id: string): TutorialLesson | undefined {
  const lesson = lessonById(id);
  return lesson === undefined ? undefined : TUTORIAL_LESSONS.find((candidate) => candidate.number === lesson.number + 1);
}

/**
 * R2433000: the tutorial opponent's handicap as today's lessons deal it, without its Core-only bar.
 * Today's opponent decks are Training Grounds cards, which the bar would keep in its hand; #2434 deals
 * it Core decks and drops this, so every lesson's opponent plays `AI_TUTORIAL` whole.
 */
function lessonOpponent(): Handicap {
  const { coreOnly: _bar, ...resources } = AI_TUTORIAL;
  return resources;
}

/** R290, R1705005, R2201000, R2201001: the tutorial opponent's handicap and the human's, the pool hold on it when the lesson sets it. */
export function lessonHandicaps(lesson: TutorialLesson): { human: Handicap; ai: Handicap } {
  return { human: lesson.poolHold ? { ...TUTORIAL_HUMAN, poolHold: true } : { ...TUTORIAL_HUMAN }, ai: lessonOpponent() };
}
