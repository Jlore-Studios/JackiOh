// Lesson 5 data (SPEC §9.10, R291, R2205003); ../scripts/finale.ts is the coach. The final exam:
// the first lesson to deal Legendary and Mythic cards, all four of Training Grounds' (Seance, Tutorial
// Master, Call to Chaos (Training-Grounds Edition) and Goetzmann Game Design), and the only one with
// no pool hold, so random cards may be any card of every shipped set (TG #40 draws from all of
// them). Deck order fixes this seed's deal (`scripts/lesson-deal.ts finale`). The AI's deck holds
// no Legendary or Mythic card (R291). Re-ordered when the player going first opened on three cards
// again (R2368000): the deal is the four-card one with Clobberer moved from the opening hand to the
// bottom of the deck, so every draw comes on its turn.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "finale",
  number: 5,
  title: "The final exam",
  summary: "Legendary and Mythic cards, random effects, Quickdraw and copies: everything at once.",
  discoveries: ["quickdraw", "legendary", "random", "copying"],
  seed: "tg-finale-4",
  humanSeat: "p1",
  poolHold: false,
  humanDeck: [
    "traininggrounds-004", // Boulder Fist Ogre, Common
    "traininggrounds-040", // Goetzmann Game Design, Mythic
    "traininggrounds-037", // Seance, Legendary
    "traininggrounds-010", // Fan of Knives, Common
    "traininggrounds-034", // Dino-size, Epic
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-052", // Snom Bunny Mind Control, Rare
    "traininggrounds-056", // Corpse Eater, Epic
    "traininggrounds-048", // Jlockeed Shredder-10, Common
    "traininggrounds-055", // Reminisce, Rare
    "traininggrounds-039", // Call to Chaos (Training-Grounds Edition), Legendary
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-018", // Knife Juggler, Rare
    "traininggrounds-045", // Jlockeed's Weapons, Common
    "traininggrounds-038", // Tutorial Master, Legendary
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-009", // Ravager, Common
    "traininggrounds-057", // Recycling Initiative, Epic
  ],
  aiDeck: [
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-047", // Midrange Menace, Common
    "traininggrounds-044", // Duplicating Felinors, Rare
    "traininggrounds-053", // Prejudiced Postdoc, Rare
    "traininggrounds-014", // Explosive Trap, Common
    "traininggrounds-024", // Goliath Situation, Rare
    "traininggrounds-012", // Fortify, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
  ],
  retryTip: "Play your Legendary cards when they can do the most, and keep a unit back to copy or to make bigger.",
};
