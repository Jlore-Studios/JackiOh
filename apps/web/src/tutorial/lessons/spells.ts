// Lesson 2 data (SPEC §9.10, R291, R2205003); ../scripts/spells.ts is the coach. Training Grounds
// cards only, none Legendary or Mythic, the pool hold on. Deck order fixes this seed's deal
// (`scripts/lesson-deal.ts spells`). The player's spells and keyword units each meet a moment; the
// AI brings Taunt, Armor and Divine Shield to meet, and spells but no backrow or Aura (R291,
// R2205003). Re-ordered when the player going first opened on three cards again (R2368000): the deal
// is the four-card one with Fortify moved from the opening hand to the bottom of the deck, so every
// draw comes on its turn.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "spells",
  number: 2,
  title: "Spells and keywords",
  summary: "Cast spells at targets, and find out what Taunt, Rush, Divine Shield and friends do.",
  discoveries: ["spell", "removal", "cry", "taunt", "armor", "divine-shield", "rush", "pierce", "defense-position"],
  seed: "tg-spells-20",
  humanSeat: "p1",
  poolHold: true,
  humanDeck: [
    "traininggrounds-059", // Fig of Life, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-058", // True Strike, Common
    "traininggrounds-018", // Knife Juggler, Rare
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-042", // Stockpile, Common
    "traininggrounds-026", // Path of Exile, Rare
    "traininggrounds-016", // Celestial Caller, Rare
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-012", // Fortify, Common
    "traininggrounds-010", // Fan of Knives, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-009", // Ravager, Common
    "traininggrounds-041", // Bigot, Common
    "traininggrounds-046", // Hit Job, Common
    "traininggrounds-017", // Flaming Felinor, Rare
    "traininggrounds-004", // Boulder Fist Ogre, Common
  ],
  aiDeck: [
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-047", // Midrange Menace, Common
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-044", // Duplicating Felinors, Rare
    "traininggrounds-053", // Prejudiced Postdoc, Rare
    "traininggrounds-012", // Fortify, Common
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-023", // Quadrupling Felinors, Rare
  ],
  retryTip: "Break a Divine Shield with a small hit before a big one, and clear Taunts with spells, so your units can reach the hero.",
};
