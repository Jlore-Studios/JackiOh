// Lesson "finale" (SPEC §9.10; R293, R2205000, R2205001, R2205003, R2434001): testkit.ts's lessonSuite, played
// through the REAL practice core (harness.ts): the engine, the card scripts and the AI.

import { describe } from "vitest";

import { lessonSuite } from "./testkit.ts";

describe("R293 R2205000 R2205001 R2205003 R2434001 lesson finale", () => {
  lessonSuite("finale", { coachTurnsMax: 11, randomRuns: 8 });
});
