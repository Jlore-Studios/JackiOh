// Lesson 2 data (SPEC §9.10, R291, R2434001, R2533001); ../lessons.ts defines its fields and
// ../scripts/keywords.ts is the coach: units, many with a Cry or a keyword, and no spell, Aura, Death,
// backrow card, Legendary or Mythic, the pool hold on. Deck order fixes this seed's deal
// (`scripts/lesson-deal.ts keywords`): reordering it changes the game, so rerun its tests.

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
    "traininggrounds-044", // Legacy: Duplicating Felinors, Rare
    "traininggrounds-023", // Quadrupling Felinors, Rare
    "traininggrounds-017", // Flaming Felinor, Rare
    "traininggrounds-041", // Legacy: Bigot, Common
    "traininggrounds-033", // Guerilla Felinor, Epic
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-009", // Ravager, Common
    "traininggrounds-047", // Legacy: Midrange Menace, Common
    "traininggrounds-018", // Knife Juggler, Rare
    "traininggrounds-016", // Celestial Caller, Rare
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-056", // Legacy: Corpse Eater, Epic
    "traininggrounds-048", // Legacy: Jlockeed Shredder-10, Common
  ],
  aiDeck: [
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-047", // Legacy: Midrange Menace, Common
    "traininggrounds-018", // Knife Juggler, Rare
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-005", // Midlane Menace, Common
    "traininggrounds-044", // Legacy: Duplicating Felinors, Rare
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-023", // Quadrupling Felinors, Rare
    "traininggrounds-033", // Guerilla Felinor, Epic
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-001", // Spikey Steven, Common
  ],
  retryTip: "Read the keywords: break a Divine Shield with a small hit, and attack the Taunt units first.",
};
