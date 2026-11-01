// Lesson 1 data (SPEC §9.10, R291, R2434001, R2533001); ../lessons.ts defines its fields and
// ../scripts/basics.ts is the coach. The first steps, and nothing else: mostly vanilla units, at most
// `deckCopies` of each (R2433001; the AI's own, R2533001), and no spell, keyword but Rush, backrow card,
// Legendary or Mythic, the pool hold on. Deck order fixes this seed's deal (`scripts/lesson-deal.ts
// basics`): reordering it changes the game, so rerun the lesson's tests.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "basics",
  number: 1,
  title: "First steps",
  summary: "Play units, attack, and bring the enemy hero down to 0.",
  discoveries: ["mana", "play-unit", "attack-hero", "trade", "winning"],
  seed: "tg-basics-1",
  humanSeat: "p1",
  poolHold: true,
  humanDeck: [
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-044", // Legacy: Duplicating Felinors, Rare
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-023", // Quadrupling Felinors, Rare
    "traininggrounds-005", // Midlane Menace, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-033", // Guerilla Felinor, Epic
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
  ],
  aiDeck: [
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-044", // Legacy: Duplicating Felinors, Rare
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
  ],
  retryTip: "Play a unit every turn, trade only when your unit survives the hit, and send everything else at the enemy hero.",
};
