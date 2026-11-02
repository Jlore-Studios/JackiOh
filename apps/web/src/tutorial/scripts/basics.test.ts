// Lesson "basics" (SPEC §9.10; R293, R2205000, R2205001, R2205003, R2434001): testkit.ts's lessonSuite.

import { describe } from "vitest";

import { lessonSuite } from "./testkit.ts";

describe("R293 R2205000 R2205001 R2205003 R2434001 lesson basics", () => {
  lessonSuite("basics", { coachTurnsMax: 8, randomRuns: 8 });
});
