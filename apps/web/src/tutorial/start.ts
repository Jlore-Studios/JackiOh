// How a lesson starts: the practice start config that plays it (SPEC §9.10, R291). The worker's core
// plays the lesson's own decks with `AI_TUTORIAL` (R290) and `TUTORIAL_HUMAN` (R1705005) and ignores
// `deck` and `difficulty`'s handicap, so those carry placeholders here. The seed and the seat are the
// lesson's own, so it deals the same hands every time; the page never lets `?seed=` override them.

import type { PracticeStartConfig } from "../practice/protocol.ts";
import type { TutorialLesson } from "./lessons.ts";

export function lessonStartConfig(lesson: TutorialLesson): PracticeStartConfig {
  return {
    seed: lesson.seed,
    difficulty: "easy",
    humanSeat: lesson.humanSeat,
    deck: { kind: "random" },
    lesson: lesson.id,
  };
}
