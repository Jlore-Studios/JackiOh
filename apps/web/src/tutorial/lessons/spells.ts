// Lesson data (SPEC §9.10, R291); ../lessons.ts defines its fields and ../scripts/spells.ts is the coach.
// Deck order determines this seed's deal (`lesson-deal.ts spells`); reordering it changes the game
// and requires rerunning this lesson's tests. It draws the cards the coach teaches when needed.
// The player has spells, keyword units, and finishers; the AI is deliberately weak and plain, with
// no backrow or material beyond lesson 3. GIGA Glowy Jelly Bean is deliberately uncast: the tutorial
// AI reaches at most 4 mana (cap 3 and The Coin).
// Prem Panther draws only after attacking and surviving (R426), so it stays below the scripted deal.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "spells",
  number: 2,
  title: "Spells and keywords",
  summary: "Cast spells at targets, and learn what Taunt, Rush, Charge and friends do.",
  mechanics: ["Spells and targets", "Cry", "Taunt", "Divine Shield", "Reborn", "Rush and Charge", "First Strike", "Defense Position"],
  seed: "tutorial-spells-367887",
  humanSeat: "p1",
  humanDeck: [
    "core-008", // Mr. Vanilla
    "core-011", // Tempo Timmy
    "core-035", // Lunar Eclipse
    "core-044", // True Strike
    "core-016", // Hit Job
    "core-068", // Twisted Sorcerer
    "core-001", // Big D-fender
    "core-012", // Duplicating Felinors
    "core-003", // Right-house defender
    "core-045", // Deft Duelist
    "core-019", // Midrange Menace
    "core-013", // Jlockeed Shredder-10
    "core-020", // Pointmaster
    "core-037", // Gravedigger
    "core-056", // Jilliax
    "core-032", // Prem Panther
    "core-025", // 4-mana 7/7
    "core-053", // Reno
    "core-005", // Stockpile
    "core-077", // Professor Curvature
  ],
  aiDeck: [
    "core-003", // Right-house defender
    "core-037", // Gravedigger
    "core-029", // GIGA Glowy Jelly Bean: costs (6), above the AI's 3 (+ The Coin), so it is never cast
    "core-015", // Me and Mr Token
    "core-008", // Mr. Vanilla
    "core-035", // Lunar Eclipse
    "core-030", // Archivist
    "core-077", // Professor Curvature
    "core-045", // Deft Duelist
    "core-005", // Stockpile
    "core-001", // Big D-fender
    "core-056", // Jilliax
  ],
  retryTip: "Break a Divine Shield with a small hit before a big one, and clear Taunts with spells, so your units can reach the hero.",
};
