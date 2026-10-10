// Lesson data (SPEC §9.10, R291); ../lessons.ts defines its fields and ../scripts/basics.ts is the coach.
// Deck order determines this seed's deal (`scripts/lesson-deal.ts basics`); changing it changes the
// scripted opening. The human gets units for turns 1–4, while Lesson 1's AI gets plain units only
// (R291): The Coin and Archivist first, then Moths to the Flame.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "basics",
  number: 1,
  title: "First steps",
  summary: "Play units, attack, and bring the enemy hero down to 0.",
  mechanics: ["Mana", "Playing units", "Lanes", "Summoning sickness", "Attacking", "Trading", "Hero health", "Winning"],
  seed: "tutorial-basics-1",
  humanSeat: "p1",
  humanDeck: [
    "core-015", // Me and Mr Token, Common
    "core-068", // Twisted Sorcerer, Common
    "core-012", // Duplicating Felinors, Rare
    "core-020", // Pointmaster, Common
    "core-011", // Tempo Timmy, Common
    "core-013", // Jlockeed Shredder-10, Common
    "core-066", // The Rock, Common
    "core-054", // Straaza, Common
    "core-077", // Professor Curvature, Rare
    "core-019", // Midrange Menace, Common
    "core-008", // Mr. Vanilla, Common
    "core-053", // Reno, Common
    "core-003", // Right-house defender, Common
    "core-004", // Gary the Gambler, Common
    "core-025", // 4-mana 7/7, Common
    "core-045", // Deft Duelist, Rare
    "core-030", // Archivist, Rare
    "core-037", // Gravedigger, Rare
    "core-056", // Jilliax, Common
    "core-032", // Prem Panther, Rare
  ],
  aiDeck: [
    "core-001", // Big D-fender, Common
    "core-025", // 4-mana 7/7, Common
    "core-077", // Professor Curvature, Rare
    "core-054", // Straaza, Common
    "core-008", // Mr. Vanilla, Common
    "core-004", // Gary the Gambler, Common
    "core-022", // Carnivorous Cube, Epic
    "core-012", // Duplicating Felinors, Rare
    "core-061", // Prejudiced Postdoc, Rare
    "core-030", // Archivist, Rare
    "core-037", // Gravedigger, Rare
    "core-009", // Moths to the Flame, Rare
  ],
  retryTip: "Play a unit every turn, trade only when your unit survives the hit, and send everything else at the enemy hero.",
};
