// Lesson 3 data (SPEC §9.10, R291, R2434001); ../lessons.ts defines its fields and ../scripts/casting.ts
// is the coach. Units the first two lessons taught, and spells; no Aura, Death or backrow card. Deck order
// fixes this seed's deal (`scripts/lesson-deal.ts casting`).

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "casting",
  number: 3,
  title: "Spells",
  summary: "Cards that act once and are gone: damage, removal, healing and more.",
  discoveries: ["spell", "removal"],
  seed: "core-casting-11",
  humanSeat: "p1",
  poolHold: true,
  humanDeck: [
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-047", // Midrange Menace, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-044", // Duplicating Felinors, Rare
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-046", // Hit Job, Common
    "traininggrounds-026", // Path of Exile, Rare
    "traininggrounds-058", // True Strike, Common
    "traininggrounds-010", // Fan of Knives, Common
    "traininggrounds-012", // Fortify, Common
    "traininggrounds-042", // Stockpile, Common
    "traininggrounds-059", // Fig of Life, Common
    "traininggrounds-034", // Dino-size, Epic
    "traininggrounds-015", // In a hurry, Common
  ],
  aiDeck: [
    "core-008", // Mr. Vanilla, Common
    "core-012", // Duplicating Felinors, Rare
    "core-019", // Midrange Menace, Common
    "core-025", // 4-mana 7/7, Common
    "core-032", // Prem Panther, Rare
    "core-053", // Reno, Common
    "core-068", // Twisted Sorcerer, Common
    "core-005", // Stockpile, Common
    "core-016", // Hit Job, Common
    "core-044", // True Strike, Common
    "core-047", // Fig of Life, Common
    "core-035", // Lunar Eclipse, Rare
  ],
  retryTip: "Save a removal spell for a unit you can't beat in a fight, and keep your units attacking.",
};
