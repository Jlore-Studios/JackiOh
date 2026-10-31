// Lesson 3 data (SPEC §9.10, R291, R2434001, R2533001); ../lessons.ts defines its fields and
// ../scripts/casting.ts is the coach. The player's deck is ten Training Grounds units the first two
// lessons taught and ten spells; the AI's is seven Training Grounds units and five spells. No Aura, Death,
// backrow card, Legendary or Mythic, the pool hold on. Deck order fixes this seed's deal
// (`scripts/lesson-deal.ts casting`).

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "casting",
  number: 3,
  title: "Spells",
  summary: "Cards that act once and are gone: damage, removal, healing and more.",
  discoveries: ["spell", "removal"],
  seed: "tg-casting-9",
  humanSeat: "p1",
  poolHold: true,
  humanDeck: [
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-044", // Legacy: Duplicating Felinors, Rare
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-046", // Legacy: Hit Job, Common
    "traininggrounds-026", // Path of Exile, Rare
    "traininggrounds-058", // Legacy: True Strike, Common
    "traininggrounds-010", // Fan of Knives, Common
    "traininggrounds-012", // Fortify, Common
    "traininggrounds-042", // Legacy: Stockpile, Common
    "traininggrounds-059", // Legacy: Fig of Life, Common
    "traininggrounds-034", // Dino-size, Epic
    "traininggrounds-015", // In a hurry, Common
  ],
  aiDeck: [
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-044", // Legacy: Duplicating Felinors, Rare
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-005", // Midlane Menace, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-033", // Guerilla Felinor, Epic
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-058", // Legacy: True Strike, Common
    "traininggrounds-010", // Fan of Knives, Common
    "traininggrounds-015", // In a hurry, Common
    "traininggrounds-042", // Legacy: Stockpile, Common
  ],
  retryTip: "Save a removal spell for a unit you can't beat in a fight, and keep your units attacking.",
};
