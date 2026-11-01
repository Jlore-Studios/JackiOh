// Lesson 5 data (SPEC §9.10, R291, R2205003, R2434001, R2533001); ../scripts/finale.ts is the coach. The
// final exam, everything at once: the first lesson to deal Legendary and Mythic cards, all four of
// Training Grounds' to the player (Seance, Tutorial Master, Call to Chaos (Training-Grounds Edition) and
// Goetzmann Game Design), and the only one with no pool hold, so random cards may be any card of every
// shipped set (TG #40 draws from all of them). The AI's deck is twelve Training Grounds cards of every
// kind the lessons taught, an Aura and a Death among them, none Legendary or Mythic (R2533001). Deck order
// fixes this seed's deal (`scripts/lesson-deal.ts finale`). Seed "-38" until #2521, whose cards change what
// Goetzmann Game Design's random effects find, so the coach's line there took 12 turns; "-59" deals
// Seance, Tutorial Master and Call to Chaos as early and wins within the lesson's 11.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "finale",
  number: 5,
  title: "The final exam",
  summary: "Legendary and Mythic cards, random effects, Quickdraw and copies: everything at once.",
  discoveries: ["quickdraw", "legendary", "random", "copying"],
  seed: "tg-finale-59",
  humanSeat: "p1",
  poolHold: false,
  humanDeck: [
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-040", // Goetzmann Game Design, Mythic
    "traininggrounds-037", // Seance, Legendary
    "traininggrounds-010", // Fan of Knives, Common
    "traininggrounds-034", // Dino-size, Epic
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-052", // Legacy: Snom Bunny Mind Control, Rare
    "traininggrounds-056", // Legacy: Corpse Eater, Epic
    "traininggrounds-048", // Legacy: Jlockeed Shredder-10, Common
    "traininggrounds-055", // Legacy: Reminisce, Rare
    "traininggrounds-039", // Call to Chaos (Training-Grounds Edition), Legendary
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-018", // Knife Juggler, Rare
    "traininggrounds-045", // Legacy: Jlockeed's Weapons, Common
    "traininggrounds-038", // Tutorial Master, Legendary
    "traininggrounds-043", // Legacy: Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-009", // Ravager, Common
    "traininggrounds-057", // Legacy: Recycling Initiative, Epic
  ],
  aiDeck: [
    "traininggrounds-003", // Group leader, Common
    "traininggrounds-021", // Flame Tongue Totem, Rare
    "traininggrounds-032", // Selfless Protector, Epic
    "traininggrounds-041", // Legacy: Bigot, Common
    "traininggrounds-048", // Legacy: Jlockeed Shredder-10, Common
    "traininggrounds-044", // Legacy: Duplicating Felinors, Rare
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-046", // Legacy: Hit Job, Common
    "traininggrounds-014", // Explosive Trap, Common
    "traininggrounds-024", // Goliath Situation, Rare
  ],
  retryTip: "Play your Legendary cards when they can do the most, and keep a unit back to copy or to make bigger.",
};
