// Lesson 4 data (SPEC §9.10, R291, R2434001); ../lessons.ts defines its fields and ../scripts/backrow.ts
// is the coach. The player's deck is ten Training Grounds units the first lessons taught, three spells,
// four Field Spells, two Traps and a Field Trap; the AI's is seven Core units, two Traps, a Field Trap
// and two Field Spells. No Legendary or Mythic, the pool hold on. Deck order fixes this seed's deal
// (`scripts/lesson-deal.ts backrow`).

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "backrow",
  number: 4,
  title: "Field Spells and Traps",
  summary: "Cards that wait face-down in the backrow, and cards that keep working there.",
  discoveries: ["trap", "enemy-backrow", "field-spell", "field-trap"],
  seed: "core-backrow-36",
  humanSeat: "p1",
  poolHold: true,
  humanDeck: [
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-047", // Legacy: Midrange Menace, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-044", // Legacy: Duplicating Felinors, Rare
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-014", // Explosive Trap, Common
    "traininggrounds-013", // Explosive Runes, Common
    "traininggrounds-024", // Goliath Situation, Rare
    "traininggrounds-060", // Legacy: Rush Token Farm, Common
    "traininggrounds-019", // Extra Rations, Rare
    "traininggrounds-035", // Corpse Recycler, Epic
    "traininggrounds-012", // Fortify, Common
    "traininggrounds-020", // Recruitment Posters, Rare
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-046", // Legacy: Hit Job, Common
  ],
  aiDeck: [
    "core-008", // Mr. Vanilla, Common
    "core-012", // Duplicating Felinors, Rare
    "core-019", // Midrange Menace, Common
    "core-025", // 4-mana 7/7, Common
    "core-032", // Prem Panther, Rare
    "core-053", // Reno, Common
    "core-068", // Twisted Sorcerer, Common
    "core-041", // Sheepish, Epic
    "core-060", // Bear Honeypot, Epic
    "core-071", // Intern Stimmy, Rare
    "core-058", // Rush Token Farm, Common
    "core-084", // Going Long, Rare
  ],
  retryTip: "Set a Trap before you end your turn, and keep a Field Spell working for you every turn.",
};
