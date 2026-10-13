// R291 lesson data (SPEC §9.10): fixed decks, seed and seat make R290 practice games repeatable.
// Human decks meet §2.6; AI decks have R184's size, respect R186 and introduce spells and traps
// only after their lessons. `lessons.test.ts` enforces these constraints.

import type { PlayerId } from "@jackioh/shared";

import { lesson as advanced } from "./lessons/advanced.ts";
import { lesson as basics } from "./lessons/basics.ts";
import { lesson as spells } from "./lessons/spells.ts";
import { lesson as traps } from "./lessons/traps.ts";

export type TutorialLesson = {
  id: string;
  number: number;
  title: string;
  summary: string;
  mechanics: readonly string[];
  seed: string;
  humanSeat: PlayerId;
  humanDeck: readonly string[];
  aiDeck: readonly string[];
  /** R291: an R186 exception a fixed lesson needs, with its reason. */
  aiShadowBanned?: Readonly<Record<string, string>>;
  retryTip: string;
};

export const TUTORIAL_LESSONS: readonly TutorialLesson[] = [basics, spells, traps, advanced];

export function lessonById(id: string): TutorialLesson | undefined {
  return TUTORIAL_LESSONS.find((lesson) => lesson.id === id);
}

export function nextLessonOf(id: string): TutorialLesson | undefined {
  const lesson = lessonById(id);
  return lesson === undefined ? undefined : TUTORIAL_LESSONS.find((candidate) => candidate.number === lesson.number + 1);
}
