// Every lesson's coach script, by lesson id (SPEC §9.10). The page loads these; the practice worker
// never does (it needs only the decks, lessons.ts).

import type { LessonScript } from "../coach.ts";
import { script as backrow } from "./backrow.ts";
import { script as basics } from "./basics.ts";
import { script as casting } from "./casting.ts";
import { script as finale } from "./finale.ts";
import { script as keywords } from "./keywords.ts";

export const LESSON_SCRIPTS: Readonly<Record<string, LessonScript>> = { basics, keywords, casting, backrow, finale };

export function scriptFor(lessonId: string): LessonScript | undefined {
  return LESSON_SCRIPTS[lessonId];
}
