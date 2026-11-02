// Lesson "keywords" (SPEC §9.10; R293, R2205000, R2205001, R2205003, R2434001): testkit.ts's lessonSuite.

import { describe } from "vitest";

import { lessonSuite } from "./testkit.ts";

describe("R293 R2205000 R2205001 R2205003 R2434001 lesson keywords", () => {
  lessonSuite("keywords", { coachTurnsMax: 12, randomRuns: 8 });
});
