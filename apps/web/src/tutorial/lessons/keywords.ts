// Lesson 2 data (SPEC §9.10, R291, R2434001); ../lessons.ts defines its fields and ../scripts/keywords.ts
// is the coach. Units with Cries and keywords, and no spell, Aura, Death or backrow card. Deck order fixes
// this seed's deal (`scripts/lesson-deal.ts keywords`): reordering it changes the game, so rerun its tests.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "keywords",
  number: 2,
  title: "Cries and keywords",
  summary: "Units that act as they arrive, and the keywords they fight with.",
  discoveries: ["cry", "taunt", "armor", "divine-shield", "rush", "pierce"],
  seed: "core-keywords-3",
  humanSeat: "p1",
  poolHold: true,
  humanDeck: [
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-005", // Midlane Menace, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-044", // Duplicating Felinors, Rare
    "traininggrounds-023", // Quadrupling Felinors, Rare
    "traininggrounds-017", // Flaming Felinor, Rare
    "traininggrounds-041", // Bigot, Common
    "traininggrounds-033", // Guerilla Felinor, Epic
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-009", // Ravager, Common
    "traininggrounds-047", // Midrange Menace, Common
    "traininggrounds-018", // Knife Juggler, Rare
    "traininggrounds-016", // Celestial Caller, Rare
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-056", // Corpse Eater, Epic
    "traininggrounds-048", // Jlockeed Shredder-10, Common
  ],
  aiDeck: [
    "core-008", // Mr. Vanilla, Common
    "core-012", // Duplicating Felinors, Rare
    "core-019", // Midrange Menace, Common
    "core-025", // 4-mana 7/7, Common
    "core-032", // Prem Panther, Rare
    "core-053", // Reno, Common
    "core-068", // Twisted Sorcerer, Common
    "core-004", // Gary the Gambler, Common
    "core-030", // Archivist, Rare
    "core-061", // Prejudiced Postdoc, Rare
    "core-077", // Professor Curvature, Rare
    "core-015", // Me and Mr Token, Common
  ],
  retryTip: "Read the keywords: break a Divine Shield with a small hit, and attack the Taunt units first.",
};
