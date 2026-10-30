// Lesson 1 data (SPEC §9.10, R291, R2205003); ../lessons.ts defines its fields and ../scripts/basics.ts
// is the coach. Training Grounds cards only, none Legendary or Mythic, the pool hold on. Deck order
// fixes this seed's deal (`scripts/lesson-deal.ts basics`): reordering it changes the game, so rerun
// the lesson's tests. The player's deck is units; the AI's is twelve units with no keyword (R291) and
// no Aura or Death (R2205003): the set's three vanilla units and nine whose one ability is a Cry.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "basics",
  number: 1,
  title: "First steps",
  summary: "Play units, attack, and bring the enemy hero down to 0.",
  discoveries: ["mana", "play-unit", "summoning-sickness", "attack-hero", "trade", "hero-health", "winning"],
  seed: "tg-basics-1",
  humanSeat: "p1",
  poolHold: true,
  humanDeck: [
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-003", // Group leader, Common
    "traininggrounds-005", // Midlane Menace, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-009", // Ravager, Common
    "traininggrounds-047", // Midrange Menace, Common
    "traininggrounds-048", // Jlockeed Shredder-10, Common
    "traininggrounds-041", // Bigot, Common
    "traininggrounds-017", // Flaming Felinor, Rare
    "traininggrounds-018", // Knife Juggler, Rare
    "traininggrounds-021", // Flame Tongue Totem, Rare
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-044", // Duplicating Felinors, Rare
    "traininggrounds-023", // Quadrupling Felinors, Rare
    "traininggrounds-032", // Selfless Protector, Epic
  ],
  aiDeck: [
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-028", // Occidental, Epic
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-044", // Duplicating Felinors, Rare
    "traininggrounds-053", // Prejudiced Postdoc, Rare
    "traininggrounds-033", // Guerilla Felinor, Epic
    "traininggrounds-023", // Quadrupling Felinors, Rare
    "traininggrounds-041", // Bigot, Common
    "traininggrounds-051", // Big Felinor, Rare
    "traininggrounds-008", // Clobberer, Common
  ],
  retryTip: "Play a unit every turn, trade only when your unit survives the hit, and send everything else at the enemy hero.",
};
