// Lesson "basics" as data (SPEC §9.10, R291): ../lessons.ts says what every field means and the
// rules every lesson keeps. Its coach script is ../scripts/basics.ts.
//
// The deal. The seed's shuffle moves list positions, not cards, so the lists below are in the order
// that deals this (`scripts/lesson-deal.ts basics` prints it):
//
//  - You: Mr. Vanilla, Duplicating Felinors and Gary the Gambler in hand, then Gravedigger,
//    Jlockeed Shredder-10, 4-mana 7/7, Professor Curvature, Midrange Menace and Reno — a unit for
//    every mana on turns 1 to 4, and only units with a line of text or less in the nine cards the
//    coach line sees. The cards with prompts, Rush, Charge or more sit below them.
//  - The AI: Gravedigger, Archivist, Professor Curvature and Moths to the Flame in hand (and The
//    Coin), then Mr. Vanilla, Gary the Gambler, Carnivorous Cube, Prejudiced Postdoc and Duplicating
//    Felinors; Straaza, 4-mana 7/7 and the Big D-fender wall are at the bottom. Lesson 1 allows it
//    plain units only (R291), which leaves few. Patch v0.1.1 made Mr. Vanilla a 4/4 on both sides of
//    the table, and the old deal's AI then walled the player's Mr. Vanilla on its first turn (a
//    Postdoc copy in Defense Position), so the coach's turn-2 hero hit never came; this deal opens
//    the AI with 2-drops, so its first turn is The Coin and Archivist, its second Moths to the Flame
//    and an attack, and its Postdoc comes on its fourth.

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
